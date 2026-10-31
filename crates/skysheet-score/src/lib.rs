//! SkySheet's instrument-adapted score model.
//!
//! This crate describes note events and score timing only. It does not
//! schedule key presses or implement playback automation.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::HashSet;
use std::fmt;

pub const SCORE_FORMAT_VERSION: &str = "1.0";
pub const PPQ: u16 = 960;
pub const SKY_15_INSTRUMENT_ID: &str = "sky-15";

pub const SKY_15_KEYS: [&str; 15] = [
    "C4", "D4", "E4", "F4", "G4", "A4", "B4",
    "C5", "D5", "E5", "F5", "G5", "A5", "B5", "C6",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Score {
    pub format_version: String,
    pub metadata: Metadata,
    pub instrument: Instrument,
    pub timeline: Timeline,
    pub tracks: Vec<Track>,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Instrument {
    pub id: String,
    pub name: String,
    pub playback_mode: PlaybackMode,
    pub keys: Vec<Pitch>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackMode {
    Trigger,
    Sustain,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Timeline {
    pub ppq: u16,
    pub tempo: f64,
    pub time_signature: TimeSignature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeSignature {
    pub numerator: u16,
    pub denominator: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub role: TrackRole,
    pub notes: Vec<Note>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Note {
    pub id: String,
    pub pitch: Pitch,
    pub start_tick: u64,
    pub duration_ticks: u64,
}

/// Scientific pitch notation with an explicit convention: C4 = MIDI 60.
/// Sharps are canonical; flats are not accepted.
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

        let octave: i16 = value[octave_start..]
            .parse()
            .map_err(|_| format!("invalid octave in pitch '{value}'"))?;
        let accidental = i16::from(octave_start == 2);
        let midi = (octave + 1) * 12 + natural + accidental;

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
        let accidental = i16::from(octave_start == 2);
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
                    issues.push(format!("note '{}' duration_ticks must be at least 1", note.id));
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
        self.instrument.keys.iter().position(|key| key == pitch).map(|i| i + 1)
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
        assert!(score.validate().unwrap_err().to_string().contains("not playable"));
    }

    #[test]
    fn rejects_duplicate_note_ids() {
        let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.tracks[1].notes[0].id = "note-001".into();
        assert!(score.validate().unwrap_err().to_string().contains("duplicate note id"));
    }

    #[test]
    fn rejects_zero_duration() {
        let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.tracks[0].notes[0].duration_ticks = 0;
        assert!(score.validate().unwrap_err().to_string().contains("duration_ticks"));
    }
}
