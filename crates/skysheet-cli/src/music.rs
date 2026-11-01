use crate::{Diagnostic, INTERNAL, INVALID_SCORE, args::Command};
use serde_json::{Value, json};
use skysheet_music::{MusicDocument, NoteStatistics, StructureAnalysis, TempoAnalysis};

fn summary(
    doc: &MusicDocument,
    n: &NoteStatistics,
    s: &StructureAnalysis,
    t: &TempoAnalysis,
) -> String {
    let mut out = format!(
        "Document: {}\nSource: {}\nTracks: {}\nNotes: {}\nSource PPQ: {}\nMIDI pitch range: {:?}..{:?}\nMaximum polyphony: {}\nDensity: {:?} notes/quarter\nRegister counts (low/middle/high): {}/{}/{}\n",
        doc.id,
        doc.source_name.as_deref().unwrap_or("unknown"),
        doc.tracks.len(),
        n.total_notes,
        doc.ppq,
        n.lowest_pitch,
        n.highest_pitch,
        n.max_polyphony,
        n.onsets_per_quarter,
        s.registers.low,
        s.registers.middle,
        s.registers.high
    );
    for track in &s.tracks {
        let range = track
            .pitch_range
            .as_ref()
            .map(|r| {
                format!(
                    "{}..{} ({}..{})",
                    r.lowest_name, r.highest_name, r.lowest, r.highest
                )
            })
            .unwrap_or_else(|| "unknown".into());
        out.push_str(&format!(
            "Track {} {:?}: {} notes, range {}, {} active ticks\n",
            track.track, track.name, track.note_count, range, track.active_ticks
        ));
    }
    out.push_str("Tempo Map (source ticks, us/quarter):\n");
    for tempo in &t.effective_map {
        out.push_str(&format!(
            "  {}: {} ({:?})\n",
            tempo.start_tick, tempo.microseconds_per_quarter, tempo.origin
        ));
    }
    out.push_str("Meter Map (nominal grid):\n");
    for meter in &s.meter_segments {
        out.push_str(&format!(
            "  {}: {}/2^{} ({:?}), bar {:?}\n",
            meter.start_tick,
            meter.numerator,
            meter.denominator_power,
            meter.origin,
            meter.bar_at_start
        ));
    }
    out
}
pub(crate) fn execute(
    command: Command,
    doc: &MusicDocument,
) -> Result<(Value, String), Diagnostic> {
    let failure = |e: skysheet_music::MusicError| {
        Diagnostic::new("music_analysis", e.to_string(), INVALID_SCORE)
    };
    let encode = |e: serde_json::Error| Diagnostic::new("internal", e.to_string(), INTERNAL);
    if command == Command::InspectMusic {
        let report = skysheet_music::inspect(doc).map_err(failure)?;
        let mut human = summary(
            doc,
            &report.note_statistics,
            &report.structure_analysis,
            &report.tempo_analysis,
        );
        for d in &report.diagnostics {
            human.push_str(&format!("[{}] {}: {}\n", d.category, d.code, d.message));
        }
        Ok((
            json!({"report":serde_json::to_value(&report).map_err(encode)?}),
            human,
        ))
    } else {
        let report = skysheet_music::analyze(doc).map_err(failure)?;
        let mut human = summary(
            doc,
            &report.note_statistics,
            &report.structure_analysis,
            &report.tempo_analysis,
        );
        human.push_str(&format!("Rhythm: {}, {} distinct onsets\nKey: {} ({} candidates)\nTrack melody ranking (heuristic, not probability):\n",report.rhythm_analysis.stability,report.rhythm_analysis.unique_onsets,report.key_candidates.status,report.key_candidates.candidates.len()));
        for rank in &report.track_ranking {
            human.push_str(&format!(
                "  #{} track {}: {:.3}\n",
                rank.rank, rank.track, rank.score
            ));
            if let Some(score) = &rank.scoring {
                for c in &score.components {
                    human.push_str(&format!(
                        "    {}: {:.4} * {:.2} = {:.4}\n",
                        c.name, c.raw, c.weight, c.contribution
                    ));
                }
            }
        }
        for role in &report.track_roles {
            human.push_str(&format!(
                "Track {} role: {:?}; candidates {:?}\n",
                role.track,
                role.role,
                role.candidates
                    .iter()
                    .map(|r| (r.role, r.score))
                    .collect::<Vec<_>>()
            ));
        }
        for d in &report.diagnostics {
            human.push_str(&format!("[{}] {}: {}\n", d.category, d.code, d.message));
        }
        Ok((
            json!({"report":serde_json::to_value(&report).map_err(encode)?}),
            human,
        ))
    }
}
