use crate::{Candidate, MelodySelection};
use serde::Serialize;
use skysheet_music::{TrackRanking, TrackRoleEstimate};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DropReason {
    MidiOutOfRange,
    BelowInstrumentRange,
    AboveInstrumentRange,
    NotInInstrumentKeys,
}

#[derive(Debug, Clone, Serialize)]
pub struct NoteMapping {
    pub source_track: usize,
    pub source_note_id: String,
    pub source_event_index: Option<usize>,
    pub source_midi: u8,
    pub source_pitch: String,
    pub transpose_semitones: i16,
    pub target_midi: i16,
    pub target_pitch: Option<String>,
    /// Zero-based index; game key number is this value + 1.
    pub key_index: Option<usize>,
    pub mapped: bool,
    pub drop_reason: Option<DropReason>,
    pub source_start_tick: u64,
    pub source_duration_ticks: u64,
    pub projected_start_tick: u64,
    pub projected_duration_ticks: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TrackReport {
    pub source_track: usize,
    pub output_track_id: String,
    pub name: Option<String>,
    pub original_notes: usize,
    pub source_lowest_midi: Option<u8>,
    pub source_highest_midi: Option<u8>,
    pub transposed_lowest_midi: Option<i16>,
    pub transposed_highest_midi: Option<i16>,
    pub kept: usize,
    pub dropped: usize,
    pub retention: Option<f64>,
    pub phase3_role: TrackRoleEstimate,
    pub confidence_explanation: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ArrangementReport {
    pub arrangement_report_version: u8,
    pub source_file: Option<String>,
    pub source_document_id: String,
    pub target_instrument: String,
    pub mode: &'static str,
    pub transpose_semitones: Option<i16>,
    pub original_notes: usize,
    pub kept: usize,
    pub dropped: usize,
    pub retention: Option<f64>,
    pub melody: MelodySelection,
    pub melody_retention: Option<f64>,
    pub accompaniment_retention: Option<f64>,
    pub source_ppq: u16,
    pub output_timeline: skysheet_score::Timeline,
    pub timing_report: skysheet_midi::Report,
    pub phase3_track_ranking: Vec<TrackRanking>,
    pub candidates: Vec<Candidate>,
    pub tracks: Vec<TrackReport>,
    pub note_mappings: Vec<NoteMapping>,
    pub diagnostics: Vec<String>,
    pub score_generated: bool,
    pub score_file: Option<String>,
    pub report_file: Option<String>,
    pub score_validation: bool,
    pub time_preserved: bool,
    pub pitch_mapping_valid: bool,
}
