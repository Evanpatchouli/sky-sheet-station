//! SkySheet's instrument-adapted score model.
//!
//! This crate describes note events and score timing only. It does not
//! schedule key presses or implement playback automation.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;

mod object;
use object::score_object;

pub const SCORE_FORMAT_VERSION: &str = "1.0";
pub const PPQ: u16 = 960;
/// Largest exact interoperable JSON tick value, including note end positions.
pub const MAX_TICK: u64 = (1_u64 << 53) - 1;
pub const SKY_15_INSTRUMENT_ID: &str = "sky-15";

pub const SKY_15_KEYS: [&str; 15] = [
    "C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5", "D5", "E5", "F5", "G5", "A5", "B5", "C6",
];

score_object! {
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct Score {
        pub format_version: String,
        pub metadata: Metadata,
        pub instrument: Instrument,
        pub timeline: Timeline,
        pub tracks: Vec<Track>,
        #[serde(default)]
        pub text: Option<String>,
    }
}

score_object! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Metadata {
        pub id: String,
        pub name: String,
    }
}

score_object! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Instrument {
        pub id: String,
        pub name: String,
        pub playback_mode: PlaybackMode,
        pub keys: Vec<Pitch>,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    Trigger,
    Sustain,
}

score_object! {
    #[derive(Debug, Clone, PartialEq, Serialize)]
    pub struct Timeline {
        pub ppq: u16,
        #[serde(deserialize_with = "deserialize_tempo")]
        pub tempo: f64,
        pub time_signature: TimeSignature,
    }
}

// JSON-specific: preserve the complete token until the standard library performs
// nearest-binary64 conversion. Box supports both string and reader inputs; this
// adapter does not promise compatibility with arbitrary non-JSON deserializers.
// Positivity remains a Score::validate rule, including underflow to zero.
fn deserialize_tempo<'de, D: Deserializer<'de>>(deserializer: D) -> Result<f64, D::Error> {
    let raw = Box::<serde_json::value::RawValue>::deserialize(deserializer)?;
    let value = raw.get().parse::<f64>().map_err(|error| {
        serde::de::Error::custom(format!("timeline.tempo: expected a JSON number: {error}"))
    })?;
    if !value.is_finite() {
        return Err(serde::de::Error::custom(
            "timeline.tempo: number out of range",
        ));
    }
    Ok(value)
}

score_object! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
    pub struct TimeSignature {
        /// 1 through 65535; zero is rejected by `Score::validate`.
        pub numerator: u16,
        /// One of 1, 2, 4, 8, 16, 32, 64, checked by `Score::validate`.
        pub denominator: u16,
    }
}

score_object! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Track {
        pub id: String,
        pub name: String,
        pub role: TrackRole,
        pub notes: Vec<Note>,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    Melody,
    Harmony,
    Bass,
    Accompaniment,
    Percussion,
    Unknown,
}

score_object! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize)]
    pub struct Note {
        pub id: String,
        pub pitch: Pitch,
        /// Absolute position, at most `MAX_TICK`; the note end must also fit.
        pub start_tick: u64,
        /// 1 through `MAX_TICK`, checked together with the end by `Score::validate`.
        pub duration_ticks: u64,
    }
}

/// Scientific pitch notation with an explicit convention: C4 = MIDI 60.
/// Sharps are canonical; flats are not accepted.
/// Both instrument keys and note pitches must map to MIDI numbers 0 through 127.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Pitch(String);

impl Pitch {
    pub fn parse(value: impl Into<String>) -> Result<Self, String> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() < 2 {
            return Err(format!("invalid pitch '{value}'"));
        }

        let natural = match bytes[0] as char {
            'C' => 0_i16,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => return Err(format!("invalid pitch letter in '{value}'")),
        };

        let octave_start = if bytes.get(1) == Some(&b'#') { 2 } else { 1 };
        if octave_start >= bytes.len() {
            return Err(format!("missing octave in pitch '{value}'"));
        }

        let octave_text = &value[octave_start..];
        let digits = octave_text.strip_prefix('-').unwrap_or(octave_text);
        if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(format!("invalid octave in pitch '{value}'"));
        }
        let octave: i16 = octave_text
            .parse()
            .map_err(|_| format!("invalid octave in pitch '{value}'"))?;
        let accidental = if octave_start == 2 { 1_i16 } else { 0_i16 };
        // Widen before arithmetic so even i16 boundary inputs return an error
        // instead of panicking or wrapping into the playable range.
        let midi = (i32::from(octave) + 1) * 12 + i32::from(natural + accidental);

        if !(0..=127).contains(&midi) {
            return Err(format!("pitch '{value}' is outside MIDI range 0..=127"));
        }

        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn midi_number(&self) -> u8 {
        let bytes = self.0.as_bytes();
        let natural = match bytes[0] as char {
            'C' => 0_i16,
            'D' => 2,
            'E' => 4,
            'F' => 5,
            'G' => 7,
            'A' => 9,
            'B' => 11,
            _ => unreachable!("Pitch is validated at construction"),
        };
        let octave_start = if bytes.get(1) == Some(&b'#') { 2 } else { 1 };
        let octave: i16 = self.0[octave_start..]
            .parse()
            .expect("Pitch is validated at construction");
        let accidental = if octave_start == 2 { 1_i16 } else { 0_i16 };
        ((octave + 1) * 12 + natural + accidental) as u8
    }
}

impl fmt::Display for Pitch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for Pitch {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Pitch {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationErrors {
    pub issues: Vec<String>,
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, issue) in self.issues.iter().enumerate() {
            if index > 0 {
                f.write_str("; ")?;
            }
            f.write_str(issue)?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

impl Score {
    /// Validate cross-field constraints that JSON Schema cannot fully express.
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        let mut issues = Vec::new();

        if self.format_version != SCORE_FORMAT_VERSION {
            issues.push(format!(
                "unsupported format_version '{}'; expected '{}'",
                self.format_version, SCORE_FORMAT_VERSION
            ));
        }
        if self.metadata.id.is_empty() {
            issues.push("metadata.id must not be empty".into());
        }
        if self.metadata.name.is_empty() {
            issues.push("metadata.name must not be empty".into());
        }
        if self.timeline.ppq != PPQ {
            issues.push(format!("timeline.ppq must be {PPQ}"));
        }
        if !self.timeline.tempo.is_finite() || self.timeline.tempo <= 0.0 {
            issues.push("timeline.tempo must be finite and greater than zero".into());
        }

        let meter = self.timeline.time_signature;
        if meter.numerator == 0
            || meter.denominator == 0
            || meter.denominator > 64
            || !meter.denominator.is_power_of_two()
        {
            issues.push("time_signature must have a positive numerator and a power-of-two denominator from 1 through 64".into());
        }

        if self.instrument.id.is_empty() {
            issues.push("instrument.id must not be empty".into());
        }
        if self.instrument.name.is_empty() {
            issues.push("instrument.name must not be empty".into());
        }
        if self.instrument.keys.is_empty() {
            issues.push("instrument.keys must not be empty".into());
        }

        let mut key_set = HashSet::new();
        for key in &self.instrument.keys {
            if !key_set.insert(key) {
                issues.push(format!("duplicate instrument pitch '{key}'"));
            }
        }

        if self.instrument.id == SKY_15_INSTRUMENT_ID {
            let expected: Vec<Pitch> = SKY_15_KEYS
                .iter()
                .map(|key| Pitch::parse(*key).expect("built-in pitch is valid"))
                .collect();
            if self.instrument.keys != expected {
                issues.push("sky-15 keys must match the canonical 15 pitches and order".into());
            }
        }

        let mut track_ids = HashSet::new();
        let mut note_ids = HashSet::new();
        for track in &self.tracks {
            if track.id.is_empty() {
                issues.push("track.id must not be empty".into());
            }
            if track.name.is_empty() {
                issues.push(format!("track '{}' name must not be empty", track.id));
            }
            if !track_ids.insert(&track.id) {
                issues.push(format!("duplicate track id '{}'", track.id));
            }

            for note in &track.notes {
                if note.id.is_empty() {
                    issues.push("note.id must not be empty".into());
                }
                if !note_ids.insert(&note.id) {
                    issues.push(format!("duplicate note id '{}'", note.id));
                }
                if note.duration_ticks == 0 {
                    issues.push(format!(
                        "note '{}' duration_ticks must be at least 1",
                        note.id
                    ));
                }
                if note.start_tick > MAX_TICK {
                    issues.push(format!(
                        "note '{}' start_tick {} must be at most {MAX_TICK}",
                        note.id, note.start_tick
                    ));
                }
                if note.duration_ticks > MAX_TICK {
                    issues.push(format!(
                        "note '{}' duration_ticks {} must be at most {MAX_TICK}",
                        note.id, note.duration_ticks
                    ));
                }
                match note.start_tick.checked_add(note.duration_ticks) {
                    Some(end_tick) if end_tick > MAX_TICK => issues.push(format!(
                        "note '{}' end_tick {end_tick} (start_tick + duration_ticks) must be at most {MAX_TICK}",
                        note.id
                    )),
                    None => issues.push(format!(
                        "note '{}' end_tick overflows u64 (start_tick {} + duration_ticks {}); must be at most {MAX_TICK}",
                        note.id, note.start_tick, note.duration_ticks
                    )),
                    _ => {}
                }
                if !self.instrument.keys.contains(&note.pitch) {
                    issues.push(format!(
                        "note '{}' pitch '{}' is not playable on instrument '{}'",
                        note.id, note.pitch, self.instrument.id
                    ));
                }
            }
        }

        if issues.is_empty() {
            Ok(())
        } else {
            Err(ValidationErrors { issues })
        }
    }

    /// Derive the 1-based game key number from instrument key ordering.
    pub fn key_number_for(&self, pitch: &Pitch) -> Option<usize> {
        self.instrument
            .keys
            .iter()
            .position(|key| key == pitch)
            .map(|i| i + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");

    #[test]
    fn pitch_mapping_uses_c4_as_midi_60() {
        assert_eq!(Pitch::parse("C4").unwrap().midi_number(), 60);
        assert_eq!(Pitch::parse("C#4").unwrap().midi_number(), 61);
        assert_eq!(Pitch::parse("C5").unwrap().midi_number(), 72);
        assert_eq!(Pitch::parse("C6").unwrap().midi_number(), 84);
    }

    #[test]
    fn pitch_rejects_flats_and_invalid_values() {
        assert!(Pitch::parse("Db4").is_err());
        assert!(Pitch::parse("H4").is_err());
        assert!(Pitch::parse("C#").is_err());
        assert!(Pitch::parse("C10").is_err());
    }

    #[test]
    fn pitch_rejects_overflowing_octaves_without_panicking() {
        for value in [
            "C32767",
            "C-32768",
            "C5460",
            "C5461",
            "C32768",
            "C99999999999999999999",
        ] {
            assert!(Pitch::parse(value).is_err(), "accepted {value}");
            let json = serde_json::to_string(value).unwrap();
            assert!(
                serde_json::from_str::<Pitch>(&json).is_err(),
                "accepted {json}"
            );
        }
    }

    #[test]
    fn pitch_requires_schema_octave_syntax() {
        for value in [
            "C+4", "C#+4", "C-", "C#-", "C 4", "C4\n", "C４", "C💫", "💫4",
        ] {
            assert!(Pitch::parse(value).is_err(), "accepted {value:?}");
            let json = serde_json::to_string(value).unwrap();
            assert!(
                serde_json::from_str::<Pitch>(&json).is_err(),
                "accepted {json}"
            );
        }
    }

    #[test]
    fn pitch_preserves_valid_spelling_and_midi_boundaries() {
        for (value, midi) in [("C-1", 0), ("G9", 127), ("F#4", 66), ("C04", 60)] {
            let pitch = Pitch::parse(value).unwrap();
            assert_eq!(pitch.midi_number(), midi);
            assert_eq!(pitch.as_str(), value);
            let json = serde_json::to_string(&pitch).unwrap();
            assert_eq!(json, serde_json::to_string(value).unwrap());
            assert_eq!(serde_json::from_str::<Pitch>(&json).unwrap(), pitch);
        }
        for value in ["B-2", "G#9", "C10"] {
            assert!(Pitch::parse(value).is_err());
        }
    }

    #[test]
    fn documented_example_deserializes_validates_and_round_trips() {
        let score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.validate().unwrap();
        let encoded = serde_json::to_string_pretty(&score).unwrap();
        let decoded: Score = serde_json::from_str(&encoded).unwrap();
        assert_eq!(score, decoded);
    }

    #[test]
    fn game_key_number_is_derived_from_pitch_order() {
        let score: Score = serde_json::from_str(EXAMPLE).unwrap();
        assert_eq!(score.key_number_for(&Pitch::parse("C4").unwrap()), Some(1));
        assert_eq!(score.key_number_for(&Pitch::parse("C5").unwrap()), Some(8));
        assert_eq!(score.key_number_for(&Pitch::parse("C6").unwrap()), Some(15));
    }

    #[test]
    fn rejects_unplayable_pitch() {
        let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.tracks[0].notes[0].pitch = Pitch::parse("C#4").unwrap();
        assert!(
            score
                .validate()
                .unwrap_err()
                .to_string()
                .contains("not playable")
        );
    }

    #[test]
    fn rejects_duplicate_note_ids() {
        let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.tracks[1].notes[0].id = "note-001".into();
        assert!(
            score
                .validate()
                .unwrap_err()
                .to_string()
                .contains("duplicate note id")
        );
    }

    #[test]
    fn rejects_zero_duration() {
        let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.tracks[0].notes[0].duration_ticks = 0;
        assert!(
            score
                .validate()
                .unwrap_err()
                .to_string()
                .contains("duration_ticks")
        );
    }

    fn example_score() -> Score {
        serde_json::from_str(EXAMPLE).unwrap()
    }

    fn assert_invalid(score: &Score, expected: &str) {
        let errors = score.validate().unwrap_err().to_string();
        assert!(
            errors.contains(expected),
            "expected {expected:?} in {errors:?}"
        );
    }

    #[test]
    fn rejects_duplicate_track_ids() {
        let mut score = example_score();
        score.tracks[1].id = score.tracks[0].id.clone();
        assert_invalid(&score, "duplicate track id");
    }

    #[test]
    fn rejects_invalid_version_and_timing_metadata() {
        let mut score = example_score();
        score.format_version = "2.0".into();
        assert_invalid(&score, "format_version");
        score = example_score();
        score.timeline.ppq = 480;
        assert_invalid(&score, "timeline.ppq");
        for tempo in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            score = example_score();
            score.timeline.tempo = tempo;
            assert_invalid(&score, "timeline.tempo");
        }
        for (numerator, denominator) in [(0, 4), (4, 0), (4, 3)] {
            score = example_score();
            score.timeline.time_signature = TimeSignature {
                numerator,
                denominator,
            };
            assert_invalid(&score, "time_signature");
        }
    }

    #[test]
    fn rejects_empty_and_duplicate_instrument_keys() {
        let mut score = example_score();
        score.instrument.keys.clear();
        assert_invalid(&score, "instrument.keys must not be empty");
        score = example_score();
        score.instrument.keys[1] = score.instrument.keys[0].clone();
        assert_invalid(&score, "duplicate instrument pitch");
    }

    #[test]
    fn rejects_wrong_sky_key_order_and_count() {
        let mut score = example_score();
        score.instrument.keys.swap(0, 1);
        assert_invalid(&score, "canonical 15 pitches and order");
        score = example_score();
        score.instrument.keys.pop();
        assert_invalid(&score, "canonical 15 pitches and order");
    }

    #[test]
    fn rejects_empty_required_strings() {
        for pointer in [
            "/metadata/id",
            "/metadata/name",
            "/instrument/id",
            "/instrument/name",
            "/tracks/0/id",
            "/tracks/0/name",
            "/tracks/0/notes/0/id",
        ] {
            let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
            *value.pointer_mut(pointer).unwrap() = serde_json::json!("");
            let score: Score = serde_json::from_value(value).unwrap();
            assert_invalid(&score, "must not be empty");
        }
    }

    #[test]
    fn rejects_unknown_fields_at_every_object_level() {
        for pointer in [
            "",
            "/metadata",
            "/instrument",
            "/timeline",
            "/timeline/time_signature",
            "/tracks/0",
            "/tracks/0/notes/0",
        ] {
            let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
            value
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("unexpected".into(), serde_json::json!(true));
            let error = serde_json::from_value::<Score>(value).unwrap_err();
            assert!(
                error.to_string().contains("unknown field"),
                "{pointer}: {error}"
            );
        }
        let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
        value["tracks"][0]["notes"][0]["velocity"] = serde_json::json!(100);
        assert!(
            serde_json::from_value::<Score>(value)
                .unwrap_err()
                .to_string()
                .contains("velocity")
        );
    }

    #[test]
    fn rejects_invalid_json_timing_and_enums() {
        for (pointer, invalid) in [
            ("/tracks/0/notes/0/start_tick", serde_json::json!(-1)),
            ("/tracks/0/notes/0/start_tick", serde_json::json!(0.5)),
            ("/tracks/0/notes/0/duration_ticks", serde_json::json!(-1)),
            ("/tracks/0/notes/0/duration_ticks", serde_json::json!(1.5)),
            ("/instrument/playback_mode", serde_json::json!("hold")),
            ("/tracks/0/role", serde_json::json!("lead")),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
            *value.pointer_mut(pointer).unwrap() = invalid;
            assert!(
                serde_json::from_value::<Score>(value).is_err(),
                "accepted {pointer}"
            );
        }
    }

    #[test]
    fn optional_text_accepts_omitted_null_and_string() {
        for text in [
            None,
            Some(serde_json::Value::Null),
            Some(serde_json::json!("user text")),
        ] {
            let mut value: serde_json::Value = serde_json::from_str(EXAMPLE).unwrap();
            value.as_object_mut().unwrap().remove("text");
            if let Some(text) = text {
                value["text"] = text;
            }
            let score: Score = serde_json::from_value(value.clone()).unwrap();
            assert_eq!(
                score.text.as_deref(),
                value.get("text").and_then(|text| text.as_str())
            );
            score.validate().unwrap();
            let encoded = serde_json::to_string(&score).unwrap();
            assert_eq!(serde_json::from_str::<Score>(&encoded).unwrap(), score);
        }
    }

    #[test]
    fn accepts_sustain_overlapping_notes_and_all_track_roles() {
        let mut score = example_score();
        score.instrument.playback_mode = PlaybackMode::Sustain;
        score.tracks[0].notes[0].duration_ticks = 1920;
        for (role, spelling) in [
            (TrackRole::Melody, "melody"),
            (TrackRole::Harmony, "harmony"),
            (TrackRole::Bass, "bass"),
            (TrackRole::Accompaniment, "accompaniment"),
            (TrackRole::Percussion, "percussion"),
            (TrackRole::Unknown, "unknown"),
        ] {
            score.tracks[0].role = role;
            score.validate().unwrap();
            let encoded = serde_json::to_value(&score).unwrap();
            assert_eq!(encoded["instrument"]["playback_mode"], "sustain");
            assert_eq!(encoded["tracks"][0]["role"], spelling);
            assert_eq!(serde_json::from_value::<Score>(encoded).unwrap(), score);
        }
        assert_eq!(score.key_number_for(&Pitch::parse("C#4").unwrap()), None);
    }
}
