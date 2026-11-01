use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    Source,
    ProtocolDefault,
    Inferred,
    UserProvided,
    Arrangement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Annotation {
    pub origin: Origin,
    pub text: String,
    pub note_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EventLocation {
    pub track: usize,
    pub event: usize,
    pub tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Diagnostic {
    pub category: String,
    pub code: String,
    pub message: String,
    pub location: Option<EventLocation>,
}
impl Diagnostic {
    pub(crate) fn analysis(code: &str, message: &str) -> Self {
        Self {
            category: "analysis_limitation".into(),
            code: code.into(),
            message: message.into(),
            location: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct MidiPitch(u8);
impl MidiPitch {
    pub fn new(value: u8) -> Result<Self, MusicError> {
        if value <= 127 {
            Ok(Self(value))
        } else {
            Err(MusicError(format!("MIDI pitch {value}: expected 0..127")))
        }
    }
    pub fn value(self) -> u8 {
        self.0
    }
    pub fn pitch_class(self) -> u8 {
        self.0 % 12
    }
    pub fn octave(self) -> i8 {
        (self.0 / 12) as i8 - 1
    }
    pub fn semitones_to(self, other: Self) -> i16 {
        i16::from(other.0) - i16::from(self.0)
    }
    pub fn name(self) -> String {
        const NAMES: [&str; 12] = [
            "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
        ];
        format!(
            "{}{}",
            NAMES[usize::from(self.pitch_class())],
            self.octave()
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MidiNoteSource {
    pub velocity: u8,
    pub channel: u8,
    pub on_event: usize,
    pub off_event: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MusicNote {
    pub id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub pitch: MidiPitch,
    pub track: usize,
    /// Absent for authored/transcribed notes; no invented channel or velocity.
    pub midi: Option<MidiNoteSource>,
    pub diagnostics: Vec<Diagnostic>,
}
impl MusicNote {
    pub fn channel(&self) -> Option<u8> {
        self.midi.as_ref().map(|m| m.channel)
    }
    pub(crate) fn event_order(&self) -> Option<usize> {
        self.midi.as_ref().map(|m| m.on_event)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MusicTrack {
    pub index: usize,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TempoEvent {
    pub location: EventLocation,
    pub microseconds_per_quarter: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeterEvent {
    pub location: EventLocation,
    pub numerator: u8,
    pub denominator_power: u8,
    pub clocks_per_click: u8,
    pub thirty_seconds_per_quarter: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SourceMetadata {
    pub location: EventLocation,
    pub kind: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MusicDocument {
    /// Caller-owned identity; MIDI adapter uses the entire byte sequence's stable
    /// noncryptographic fingerprint plus length (not a security/content hash).
    pub id: String,
    pub source_name: Option<String>,
    pub source_format: String,
    pub ppq: u16,
    pub tracks: Vec<MusicTrack>,
    pub notes: Vec<MusicNote>,
    /// Original order, duplicates and event locations, never effective-map rewriting.
    pub tempo_map: Vec<TempoEvent>,
    pub time_signature_map: Vec<MeterEvent>,
    pub metadata: Vec<SourceMetadata>,
    pub annotations: Vec<Annotation>,
    pub analysis_refs: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
    /// Exact source bytes retain release velocity and unmodeled event encoding.
    /// Deliberately excluded from report JSON; not a document persistence format.
    #[serde(skip_serializing)]
    pub original_midi: Option<Vec<u8>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicError(pub String);
impl std::fmt::Display for MusicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for MusicError {}

impl MusicDocument {
    pub fn validate(&self) -> Result<(), MusicError> {
        let fail = |s: String| Err(MusicError(s));
        if self.id.is_empty() || self.ppq == 0 {
            return fail("document id must be nonempty and PPQ positive".into());
        }
        for (i, t) in self.tracks.iter().enumerate() {
            if t.index != i {
                return fail(format!(
                    "track {i}: expected original contiguous track index"
                ));
            }
        }
        let mut ids = BTreeSet::new();
        let mut events = BTreeSet::new();
        for n in &self.notes {
            if n.id.is_empty() || !ids.insert(&n.id) {
                return fail(format!(
                    "note {}: empty/duplicate identity or onset event",
                    n.id
                ));
            }
            if n.track >= self.tracks.len() || n.end_tick <= n.start_tick {
                return fail(format!(
                    "note {}: invalid track/channel/velocity/endpoints/event order",
                    n.id
                ));
            }
            if let Some(m) = &n.midi
                && (m.channel > 15
                    || m.velocity == 0
                    || m.velocity > 127
                    || m.off_event <= m.on_event
                    || !events.insert((n.track, m.on_event))
                    || !events.insert((n.track, m.off_event)))
            {
                return fail(format!(
                    "note {}: invalid MIDI provenance or duplicate source event",
                    n.id
                ));
            }
        }
        let mut tempos = BTreeMap::new();
        for t in &self.tempo_map {
            if t.location.track >= self.tracks.len()
                || t.microseconds_per_quarter == 0
                || t.microseconds_per_quarter > 0xff_ffff
            {
                return fail(format!(
                    "tempo at {:?}: invalid track or MIDI tempo",
                    t.location
                ));
            }
            if let Some(old) = tempos.insert(t.location.tick, t.microseconds_per_quarter)
                && old != t.microseconds_per_quarter
            {
                return fail(format!(
                    "tempo conflict at tick {}: {old} vs {}",
                    t.location.tick, t.microseconds_per_quarter
                ));
            }
        }
        let mut meters = BTreeMap::new();
        for m in &self.time_signature_map {
            if m.location.track >= self.tracks.len()
                || m.numerator == 0
                || m.thirty_seconds_per_quarter != 8
            {
                return fail(format!(
                    "meter at {:?}: invalid track/numerator or unsupported notation unit",
                    m.location
                ));
            }
            let value = (
                m.numerator,
                m.denominator_power,
                m.clocks_per_click,
                m.thirty_seconds_per_quarter,
            );
            if let Some(old) = meters.insert(m.location.tick, value)
                && old != value
            {
                return fail(format!("meter conflict at tick {}", m.location.tick));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Confidence {
    pub probability: Option<f64>,
    pub calibration: &'static str,
}
impl Confidence {
    pub(crate) fn uncalibrated() -> Self {
        Self {
            probability: None,
            calibration: "uncalibrated_heuristic",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Evidence {
    pub origin: Origin,
    pub method: &'static str,
    pub source_document_id: String,
    pub tracks: Vec<usize>,
    pub note_ids: Vec<String>,
}
impl Evidence {
    pub(crate) fn inferred(
        doc: &MusicDocument,
        method: &'static str,
        notes: &[&MusicNote],
    ) -> Self {
        Self {
            origin: Origin::Inferred,
            method,
            source_document_id: doc.id.clone(),
            tracks: notes
                .iter()
                .map(|n| n.track)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
            note_ids: notes.iter().map(|n| n.id.clone()).collect(),
        }
    }
}
