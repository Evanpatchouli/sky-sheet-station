//! Source music and read-only, instrument-independent analysis.
//! No Score, MIDI parser, playback or target-instrument dependency.
mod model;
mod statistics;
mod structure;
mod tonality;
mod voices;
pub use structure::*;

pub use model::*;
use serde::Serialize;
pub use statistics::*;
pub use tonality::*;
pub use voices::*;

#[derive(Debug, Clone, Serialize)]
pub struct MusicAnalysisReport {
    pub report_version: u8,
    pub analyzer_version: &'static str,
    pub source_document_id: String,
    /// Auditable facts, including all notes and unmodified source maps.
    pub source: MusicDocument,
    pub note_statistics: NoteStatistics,
    pub pitch_statistics: PitchStatistics,
    pub tempo_analysis: TempoAnalysis,
    pub rhythm_analysis: RhythmAnalysis,
    pub key_candidates: KeyAnalysis,
    pub melody_candidates: Vec<MelodyCandidate>,
    pub track_roles: Vec<TrackRoleEstimate>,
    pub track_ranking: Vec<TrackRanking>,
    pub structure_analysis: StructureAnalysis,
    pub confidence: Confidence,
    pub diagnostics: Vec<Diagnostic>,
}

/// Analyze without mutating, quantizing, filtering or arranging the document.
#[derive(Debug, Clone, Serialize)]
pub struct MusicInspectionReport {
    pub report_version: u8,
    pub source_document_id: String,
    pub source: MusicDocument,
    pub note_statistics: NoteStatistics,
    pub pitch_statistics: PitchStatistics,
    pub tempo_analysis: TempoAnalysis,
    pub structure_analysis: StructureAnalysis,
    pub diagnostics: Vec<Diagnostic>,
}
pub fn inspect(document: &MusicDocument) -> Result<MusicInspectionReport, MusicError> {
    if document.notes.len() > 100_000 {
        return Err(MusicError("inspection limit: at most 100000 notes".into()));
    }
    document.validate()?;
    let mut diagnostics = document.diagnostics.clone();
    let structure_analysis = structure::structure(document, &mut diagnostics);
    Ok(MusicInspectionReport {
        report_version: 2,
        source_document_id: document.id.clone(),
        source: document.clone(),
        note_statistics: statistics::notes(document),
        pitch_statistics: tonality::pitches(document),
        tempo_analysis: statistics::tempos(document),
        structure_analysis,
        diagnostics,
    })
}

/// Analyze without mutating, quantizing, filtering or arranging the document.
pub fn analyze(document: &MusicDocument) -> Result<MusicAnalysisReport, MusicError> {
    // Bound heuristic work and report size independently of MIDI byte limits.
    if document.notes.len() > 100_000 {
        return Err(MusicError("analysis limit: at most 100000 paired notes; split the source explicitly before analysis".into()));
    }
    document.validate()?;
    let mut diagnostics = document.diagnostics.clone();
    diagnostics.push(Diagnostic::analysis("uncalibrated_inference", "Heuristic scores rank candidates, not probabilities. Confidence probabilities are unavailable; review candidates before arrangement."));
    let note_statistics = statistics::notes(document);
    let pitch_statistics = tonality::pitches(document);
    let tempo_analysis = statistics::tempos(document);
    let rhythm_analysis = statistics::rhythm(document);
    let key_candidates = tonality::keys(document, &pitch_statistics);
    let (melody_candidates, track_roles) = voices::estimate(document, &mut diagnostics);
    let track_ranking = voices::rank_tracks(document, &melody_candidates, &mut diagnostics);
    let structure_analysis = structure::structure(document, &mut diagnostics);
    Ok(MusicAnalysisReport {
        report_version: 2,
        analyzer_version: "primitives-2",
        source_document_id: document.id.clone(),
        source: document.clone(),
        note_statistics,
        pitch_statistics,
        tempo_analysis,
        rhythm_analysis,
        key_candidates,
        melody_candidates,
        track_roles,
        track_ranking,
        structure_analysis,
        confidence: Confidence::uncalibrated(),
        diagnostics,
    })
}
