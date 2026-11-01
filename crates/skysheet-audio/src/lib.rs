//! Offline audio transcription. Unrestricted notes and their provenance remain
//! separate from Score v1. The generated SMF enters the existing MIDI pipeline.
mod decode;
mod midi;
mod transcribe;

pub use decode::{Audio, decode};
use serde::Serialize;
pub use transcribe::transcribe;

pub const SAMPLE_RATE: u32 = 22_050;
pub const MAX_FILE_BYTES: u64 = 128 * 1024 * 1024;
pub const MAX_DURATION_SECONDS: u32 = 600;

#[derive(Debug, Clone, Serialize)]
pub struct Progress {
    pub stage: &'static str,
    pub completed: u64,
    pub total: Option<u64>,
}

/// Return false to cancel. Callbacks may run on inference worker threads.
pub type Observer<'a> = dyn Fn(Progress) -> bool + Sync + 'a;

pub(crate) fn progress(
    observer: &Observer<'_>,
    stage: &'static str,
    completed: u64,
    total: Option<u64>,
) -> Result<(), Error> {
    if observer(Progress {
        stage,
        completed,
        total,
    }) {
        Ok(())
    } else {
        Err(Error::new("cancelled", "Audio transcription cancelled"))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}

impl Error {
    pub(crate) fn new(code: &'static str, message: impl ToString) -> Self {
        Self {
            code,
            message: message.to_string(),
        }
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}
impl std::error::Error for Error {}

#[derive(Debug, Clone, Serialize)]
pub struct Options {
    pub onset_threshold: f64,
    pub frame_threshold: f64,
    pub minimum_note_ms: f64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            onset_threshold: 0.5,
            frame_threshold: 0.3,
            minimum_note_ms: 127.7,
        }
    }
}
impl Options {
    pub fn validate(&self) -> Result<(), Error> {
        for (field, value) in [
            ("onset_threshold", self.onset_threshold),
            ("frame_threshold", self.frame_threshold),
        ] {
            if !value.is_finite() || !(0.05..=0.95).contains(&value) {
                return Err(Error::new(
                    "invalid_options",
                    format!("{field} must be finite and in 0.05..=0.95"),
                ));
            }
        }
        if !self.minimum_note_ms.is_finite() || !(30.0..=1000.0).contains(&self.minimum_note_ms) {
            return Err(Error::new(
                "invalid_options",
                "minimum_note_ms must be in 30..=1000",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Note {
    pub pitch: u8,
    pub start_sample: u64,
    pub end_sample: u64,
    pub start_tick: u64,
    pub end_tick: u64,
    pub velocity: u8,
    /// Mean model activation; not a calibrated probability or measured velocity.
    pub activation: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub report_version: u8,
    pub engine: &'static str,
    pub model_sha256: &'static str,
    pub source_sample_rate: u32,
    pub source_channels: usize,
    pub sample_rate: u32,
    pub sample_count: usize,
    pub duration_seconds: f64,
    pub reference_tempo: u16,
    pub ppq: u16,
    pub options: Options,
    pub notes: Vec<Note>,
    pub diagnostics: Vec<String>,
}

pub struct Transcription {
    pub midi: Vec<u8>,
    pub report: Report,
}
