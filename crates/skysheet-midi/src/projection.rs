//! D6 projection before pitch adaptation; never edits MusicDocument.
use crate::{Failure, Report};
use serde::Serialize;
use skysheet_music::{MusicDocument, MusicNote};
use skysheet_score::Timeline;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectedNote {
    pub source_note_id: String,
    pub source_track: usize,
    pub source_midi: u8,
    pub source_start_tick: u64,
    pub source_end_tick: u64,
    pub start_tick: u64,
    pub duration_ticks: u64,
}
impl ProjectedNote {
    fn matches(&self, n: &MusicNote) -> bool {
        self.source_note_id == n.id
            && self.source_track == n.track
            && self.source_midi == n.pitch.value()
            && self.source_start_tick == n.start_tick
            && self.source_end_tick == n.end_tick
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreTimeProjection {
    source_document_id: String,
    source_ppq: u16,
    source_tempos: Vec<skysheet_music::TempoEvent>,
    source_meters: Vec<skysheet_music::MeterEvent>,
    timeline: Timeline,
    notes: Vec<ProjectedNote>,
    report: Report,
}
impl ScoreTimeProjection {
    pub fn timeline(&self) -> &Timeline {
        &self.timeline
    }
    pub fn notes(&self) -> &[ProjectedNote] {
        &self.notes
    }
    pub fn report(&self) -> &Report {
        &self.report
    }
    pub fn matches(&self, doc: &MusicDocument) -> bool {
        self.source_document_id == doc.id
            && self.source_ppq == doc.ppq
            && self.source_tempos == doc.tempo_map
            && self.source_meters == doc.time_signature_map
            && self.notes.len() == doc.notes.len()
            && self.notes.iter().zip(&doc.notes).all(|(p, n)| p.matches(n))
    }
}

pub fn project_music(doc: &MusicDocument) -> Result<ScoreTimeProjection, Failure> {
    let mut report = Report::new();
    if doc.notes.len() > 100_000 || doc.tracks.len() > 4096 {
        report.push(
            "error",
            "projection_resource_limit",
            "at most 100000 notes and 4096 tracks; no partial projection",
            None,
        );
        return Err(report.failure());
    }
    report.source_ppq = Some(doc.ppq);
    report.note_count = doc.notes.len();
    report.track_count = doc.tracks.len();
    if let Err(e) = doc.validate() {
        report.push("error", "music_document", e.to_string(), None);
        return Err(report.failure());
    }
    report.tempos = doc
        .tempo_map
        .iter()
        .map(|t| crate::TempoEvent {
            track: t.location.track,
            event: t.location.event,
            tick: t.location.tick,
            microseconds_per_quarter: t.microseconds_per_quarter,
        })
        .collect();
    report.meters = doc
        .time_signature_map
        .iter()
        .map(|m| crate::MeterEvent {
            track: m.location.track,
            event: m.location.event,
            tick: m.location.tick,
            numerator: m.numerator,
            denominator_power: m.denominator_power,
            clocks_per_click: m.clocks_per_click,
            thirty_seconds_per_quarter: m.thirty_seconds_per_quarter,
        })
        .collect();
    let (timing, timeline) = crate::import::prepare_timing(&mut report, None)?;
    let mut notes = Vec::with_capacity(doc.notes.len());
    for n in &doc.notes {
        let loc = Some((
            n.track,
            n.midi.as_ref().map_or(0, |m| m.on_event),
            n.start_tick,
        ));
        match (timing.endpoint(n.start_tick), timing.endpoint(n.end_tick)) {
            (Ok((start, a)), Ok((end, b))) if end > start => {
                if a || b {
                    report.push("warning", "time_quantized", format!("{}: source {}..{} -> projected {start}..{end}; absolute endpoints, half up",n.id,n.start_tick,n.end_tick), loc);
                }
                notes.push(ProjectedNote {
                    source_note_id: n.id.clone(),
                    source_track: n.track,
                    source_midi: n.pitch.value(),
                    source_start_tick: n.start_tick,
                    source_end_tick: n.end_tick,
                    start_tick: start,
                    duration_ticks: end - start,
                });
            }
            (Ok(_), Ok(_)) => report.push(
                "error",
                "quantized_zero_duration",
                format!("{}: D6 projected duration is not positive", n.id),
                loc,
            ),
            (Err(e), _) | (_, Err(e)) => report.push("error", "output_tick_range", e, loc),
        }
    }
    if report.failed() {
        return Err(report.failure());
    }
    Ok(ScoreTimeProjection {
        source_document_id: doc.id.clone(),
        source_ppq: doc.ppq,
        source_tempos: doc.tempo_map.clone(),
        source_meters: doc.time_signature_map.clone(),
        timeline,
        notes,
        report,
    })
}
