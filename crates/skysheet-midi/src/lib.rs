//! SMF interoperability. Source provenance and conversion losses stay outside Score v1.
mod export;
mod import;
mod music;
mod structure;
mod timing;

pub use music::read_music;

pub use export::export;
pub use import::{ImportOptions, import, inspect};

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub severity: &'static str,
    pub code: &'static str,
    pub message: String,
    pub track: Option<usize>,
    pub event: Option<usize>,
    pub source_tick: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceNote {
    pub track: usize,
    pub channel: u8,
    pub key: u8,
    pub pitch: String,
    pub velocity: u8,
    pub on_event: usize,
    pub off_event: usize,
    pub start_tick: u64,
    pub end_tick: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct MetadataEvent {
    pub track: usize,
    pub event: usize,
    pub tick: u64,
    pub kind: String,
    pub data: Vec<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TempoEvent {
    pub track: usize,
    pub event: usize,
    pub tick: u64,
    pub microseconds_per_quarter: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MeterEvent {
    pub track: usize,
    pub event: usize,
    pub tick: u64,
    pub numerator: u8,
    pub denominator_power: u8,
    pub clocks_per_click: u8,
    pub thirty_seconds_per_quarter: u8,
}

#[derive(Debug, Default, Clone, Serialize)]
pub struct Report {
    pub report_version: u8,
    pub format: Option<u16>,
    pub source_ppq: Option<u16>,
    pub track_count: usize,
    pub note_count: usize,
    pub reference_tempo: Option<f64>,
    pub output_microseconds_per_quarter: Option<u32>,
    pub tempos: Vec<TempoEvent>,
    pub meters: Vec<MeterEvent>,
    pub source_notes: Vec<SourceNote>,
    pub metadata: Vec<MetadataEvent>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Report {
    fn new() -> Self {
        Self {
            report_version: 1,
            ..Self::default()
        }
    }

    fn push(
        &mut self,
        severity: &'static str,
        code: &'static str,
        message: impl Into<String>,
        location: Option<(usize, usize, u64)>,
    ) {
        self.diagnostics.push(Diagnostic {
            severity,
            code,
            message: message.into(),
            track: location.map(|v| v.0),
            event: location.map(|v| v.1),
            source_tick: location.map(|v| v.2),
        });
    }

    fn failed(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == "error")
    }

    fn failure(self) -> Failure {
        Failure {
            report: Box::new(self),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub report: Box<Report>,
}

impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for d in &self.report.diagnostics {
            if d.severity == "error" {
                writeln!(f, "{}: {}", d.code, d.message)?;
            }
        }
        Ok(())
    }
}
impl std::error::Error for Failure {}
