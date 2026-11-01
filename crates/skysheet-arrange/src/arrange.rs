use crate::{
    ArrangementReport, DropReason, NoteMapping, TrackReport, TransposeMode, search_transposition,
    sky_15_instrument,
};
use skysheet_midi::ScoreTimeProjection;
use skysheet_music::MusicDocument;
use skysheet_score::{Metadata, Note, SCORE_FORMAT_VERSION, Score, Track, TrackRole};

#[derive(Debug)]
pub struct Arrangement {
    pub score: Score,
    pub report: ArrangementReport,
}
#[derive(Debug)]
pub struct ArrangementError {
    pub message: String,
    pub report: Option<Box<ArrangementReport>>,
}
impl From<String> for ArrangementError {
    fn from(message: String) -> Self {
        Self {
            message,
            report: None,
        }
    }
}
impl std::fmt::Display for ArrangementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for ArrangementError {}

/// Pitch adaptation consumes an immutable, validated D6 projection. No time math.
pub fn arrange(
    doc: &MusicDocument,
    projection: &ScoreTimeProjection,
    mode: TransposeMode,
) -> Result<Arrangement, ArrangementError> {
    if !projection.matches(doc) {
        return Err(
            "time projection does not match source document, notes or maps"
                .to_string()
                .into(),
        );
    }
    let search = search_transposition(doc, mode).map_err(|e| e.to_string())?;
    let selected = &search.candidates[search.selected_index];
    let shift = selected.transpose_semitones;
    let instrument = sky_15_instrument();
    let mut tracks: Vec<_> = doc
        .tracks
        .iter()
        .map(|t| {
            let role = if Some(t.index) == search.melody.track {
                TrackRole::Melody
            } else {
                match search.analysis.track_roles[t.index].role {
                    skysheet_music::TrackRole::Bass => TrackRole::Bass,
                    skysheet_music::TrackRole::Harmony => TrackRole::Harmony,
                    skysheet_music::TrackRole::Accompaniment => TrackRole::Accompaniment,
                    skysheet_music::TrackRole::Rhythm => TrackRole::Percussion,
                    _ => TrackRole::Unknown,
                }
            };
            Track {
                id: format!("source-track-{}", t.index),
                name: t
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| format!("Track {}", t.index + 1)),
                role,
                notes: vec![],
            }
        })
        .collect();
    let low = instrument
        .keys
        .iter()
        .map(|p| i16::from(p.midi_number()))
        .min()
        .unwrap();
    let high = instrument
        .keys
        .iter()
        .map(|p| i16::from(p.midi_number()))
        .max()
        .unwrap();
    let mut mappings = Vec::with_capacity(doc.notes.len());
    let mut ranges = vec![(None::<u8>, None::<u8>, 0usize); tracks.len()];
    for (n, time) in doc.notes.iter().zip(projection.notes()) {
        let target = i16::from(n.pitch.value()) + shift;
        let key_index = crate::search::key_index(&instrument, target);
        let drop_reason = if key_index.is_some() {
            None
        } else if !(0..=127).contains(&target) {
            Some(DropReason::MidiOutOfRange)
        } else if target < low {
            Some(DropReason::BelowInstrumentRange)
        } else if target > high {
            Some(DropReason::AboveInstrumentRange)
        } else {
            Some(DropReason::NotInInstrumentKeys)
        };
        let target_pitch = key_index.map(|k| instrument.keys[k].clone());
        if let Some(pitch) = &target_pitch {
            tracks[n.track].notes.push(Note {
                id: n.id.clone(),
                pitch: pitch.clone(),
                start_tick: time.start_tick,
                duration_ticks: time.duration_ticks,
            });
        }
        let range = &mut ranges[n.track];
        range.0 = Some(range.0.map_or(n.pitch.value(), |v| v.min(n.pitch.value())));
        range.1 = Some(range.1.map_or(n.pitch.value(), |v| v.max(n.pitch.value())));
        range.2 += 1;
        mappings.push(NoteMapping {
            source_track: n.track,
            source_note_id: n.id.clone(),
            source_event_index: n.midi.as_ref().map(|m| m.on_event),
            source_midi: n.pitch.value(),
            source_pitch: n.pitch.name(),
            transpose_semitones: shift,
            target_midi: target,
            target_pitch: target_pitch.map(|p| p.to_string()),
            key_index,
            mapped: key_index.is_some(),
            drop_reason,
            source_start_tick: n.start_tick,
            source_duration_ticks: n.end_tick - n.start_tick,
            projected_start_tick: time.start_tick,
            projected_duration_ticks: time.duration_ticks,
        });
    }
    let track_reports = doc.tracks.iter().map(|t| {
        let (low, high, total) = ranges[t.index];
        let kept = tracks[t.index].notes.len();
        TrackReport { source_track: t.index, output_track_id: tracks[t.index].id.clone(), name: t.name.clone(), original_notes: total,
            source_lowest_midi: low, source_highest_midi: high, transposed_lowest_midi: low.map(|n| i16::from(n)+shift), transposed_highest_midi: high.map(|n|i16::from(n)+shift),
            kept, dropped: total-kept, retention: crate::search::ratio(kept,total), phase3_role: search.analysis.track_roles[t.index].clone(), confidence_explanation: "Uncalibrated Phase 3 role hypothesis; Unknown is not accompaniment. Selected melody remains provisional." }
    }).collect();
    let mut report = ArrangementReport {
        arrangement_report_version: 1,
        source_file: doc.source_name.clone(),
        source_document_id: doc.id.clone(),
        target_instrument: instrument.id.clone(),
        mode: match mode {
            TransposeMode::Auto => "auto",
            TransposeMode::Fixed(_) => "fixed",
        },
        transpose_semitones: Some(shift),
        original_notes: doc.notes.len(),
        kept: selected.total_kept,
        dropped: selected.dropped,
        retention: crate::search::ratio(selected.total_kept, doc.notes.len()),
        melody_retention: selected.melody_retention,
        accompaniment_retention: selected.accompaniment_retention,
        source_ppq: doc.ppq,
        output_timeline: projection.timeline().clone(),
        timing_report: projection.report().clone(),
        melody: search.melody,
        phase3_track_ranking: search.analysis.track_ranking,
        candidates: search.candidates,
        tracks: track_reports,
        note_mappings: mappings,
        diagnostics: search
            .analysis
            .diagnostics
            .iter()
            .map(|d| format!("{}: {}", d.code, d.message))
            .collect(),
        score_generated: false,
        score_file: None,
        report_file: None,
        score_validation: false,
        time_preserved: false,
        pitch_mapping_valid: false,
    };
    report.diagnostics.push("Timing equality is against the independent D6 projection, not raw MIDI ticks. No Arrange quantization, octave folding, pitch snapping, note merging or polyphony reduction.".into());
    if report.dropped > 0 {
        report.diagnostics.push(format!(
            "{} of {} source notes could not map; every loss is listed in note_mappings.",
            report.dropped, report.original_notes
        ));
    }
    let fail = |message: String, report: ArrangementReport| ArrangementError {
        message,
        report: Some(Box::new(report)),
    };
    if report.kept == 0 {
        return Err(fail(
            "no playable notes retained; no empty Score produced".into(),
            report,
        ));
    }
    let score = Score {
        format_version: SCORE_FORMAT_VERSION.into(),
        metadata: Metadata {
            id: format!("{}-sky-{shift}", doc.id),
            name: doc
                .source_name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| "Sky arrangement".into()),
        },
        instrument,
        timeline: projection.timeline().clone(),
        tracks,
        text: None,
    };
    let verify = || -> Result<(), String> {
        score.validate().map_err(|e| e.to_string())?;
        let bytes = serde_json::to_vec(&score).map_err(|e| e.to_string())?;
        let reread: Score = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        reread.validate().map_err(|e| e.to_string())?;
        if reread != score
            || reread.timeline.tempo.to_bits() != projection.timeline().tempo.to_bits()
        {
            return Err("Score raw-byte round-trip mismatch".into());
        }
        let by_id: std::collections::BTreeMap<_, _> = score
            .tracks
            .iter()
            .enumerate()
            .flat_map(|(t, track)| track.notes.iter().map(move |n| (n.id.as_str(), (t, n))))
            .collect();
        for m in &report.note_mappings {
            if !m.mapped {
                continue;
            }
            let (t, n) = by_id
                .get(m.source_note_id.as_str())
                .ok_or("missing output note")?;
            if *t != m.source_track
                || n.start_tick != m.projected_start_tick
                || n.duration_ticks != m.projected_duration_ticks
                || i16::from(n.pitch.midi_number()) != m.target_midi
                || score.key_number_for(&n.pitch) != m.key_index.map(|i| i + 1)
            {
                return Err(format!(
                    "mapping/time invariant failed for {}",
                    m.source_note_id
                ));
            }
        }
        Ok(())
    };
    if let Err(e) = verify() {
        return Err(fail(e, report));
    }
    report.score_generated = true;
    report.score_validation = true;
    report.time_preserved = true;
    report.pitch_mapping_valid = true;
    Ok(Arrangement { score, report })
}
