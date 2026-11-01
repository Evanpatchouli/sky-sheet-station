use serde::Serialize;
use skysheet_music::{MusicAnalysisReport, MusicDocument, MusicError, TrackRole};
use skysheet_score::{Instrument, Pitch, PlaybackMode, SKY_15_INSTRUMENT_ID, SKY_15_KEYS};
use std::cmp::{Ordering, Reverse};

pub const SCORING_RULE: &str = "lexicographic: melody kept descending (only when selected), total kept descending, accompaniment kept descending, absolute semitones ascending, preserved adjacent melody pairs descending, retained melody leap sum ascending, signed semitones ascending; ratios and dropped counts have fixed denominators and are reported, not weighted again";

#[derive(Debug, Clone, Copy, Default)]
pub enum TransposeMode {
    #[default]
    Auto,
    Fixed(i16),
}

pub fn sky_15_instrument() -> Instrument {
    Instrument {
        id: SKY_15_INSTRUMENT_ID.into(),
        name: "Sky 15-key instrument".into(),
        playback_mode: PlaybackMode::Trigger,
        keys: SKY_15_KEYS
            .iter()
            .map(|s| Pitch::parse(*s).expect("Score profile"))
            .collect(),
    }
}

#[derive(Debug)]
pub struct TranspositionSearch {
    pub analysis: MusicAnalysisReport,
    pub melody: MelodySelection,
    pub candidates: Vec<Candidate>,
    pub selected_index: usize,
}

/// Evaluate every offset capable of mapping a MIDI pitch to another MIDI pitch.
/// Invalid shifted notes are diagnosed by the arrangement, never clamped.
pub fn search_transposition(
    doc: &MusicDocument,
    mode: TransposeMode,
) -> Result<TranspositionSearch, MusicError> {
    if doc.tracks.len() > 4096 {
        return Err(MusicError(
            "arrangement limit: at most 4096 tracks; no tracks were omitted".into(),
        ));
    }
    if let TransposeMode::Fixed(shift) = mode
        && !(-127..=127).contains(&shift)
    {
        return Err(MusicError("transpose_semitones: expected -127..127".into()));
    }
    let analysis = skysheet_music::analyze(doc)?;
    let melody = select_melody(&analysis);
    let accompaniment: Vec<_> = analysis
        .track_roles
        .iter()
        .map(|r| {
            Some(r.track) != melody.track
                && matches!(
                    r.role,
                    TrackRole::Bass
                        | TrackRole::Harmony
                        | TrackRole::Accompaniment
                        | TrackRole::Rhythm
                )
        })
        .collect();
    let range = match mode {
        TransposeMode::Auto => -127..=127,
        TransposeMode::Fixed(s) => s..=s,
    };
    let instrument = sky_15_instrument();
    let mut candidates: Vec<_> = range
        .map(|s| evaluate(doc, &instrument, melody.track, &accompaniment, s))
        .collect();
    let selected_index = candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| compare(a, b))
        .unwrap()
        .0;
    candidates[selected_index].selected = true;
    Ok(TranspositionSearch {
        analysis,
        melody,
        candidates,
        selected_index,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct MelodySelection {
    pub status: &'static str,
    pub track: Option<usize>,
    pub reason: &'static str,
    pub minimum_score: f64,
    pub minimum_margin: f64,
    pub confidence: &'static str,
}

pub(crate) fn select_melody(analysis: &MusicAnalysisReport) -> MelodySelection {
    let mut selection = MelodySelection {
        status: "unknown",
        track: None,
        reason: "No eligible, sufficiently separated Phase 3 melody candidate; prioritize all notes.",
        minimum_score: 0.65,
        minimum_margin: 0.10,
        confidence: "uncalibrated_heuristic_not_probability",
    };
    if let Some(top) = analysis.track_ranking.first() {
        let margin = analysis.track_ranking.get(1).map(|r| top.score - r.score);
        if top.score >= selection.minimum_score
            && margin.is_some_and(|v| v < selection.minimum_margin)
        {
            selection.reason = "Competing track melody scores differ by less than 0.10; melody is Unknown and total retention takes priority.";
        } else if top.score >= selection.minimum_score
            && top.scoring.as_ref().is_some_and(|s| s.eligible)
            && analysis.track_roles[top.track].role == TrackRole::Melody
        {
            selection.status = "heuristic_candidate";
            selection.track = Some(top.track);
            selection.reason = "Top Phase 3 track is eligible, classified as melody, score >= 0.65, and leads the next track by >= 0.10; provisional, not a musical fact.";
        }
    }
    selection
}

pub fn ratio(kept: usize, total: usize) -> Option<f64> {
    (total > 0).then(|| kept as f64 / total as f64)
}

#[derive(Debug, Clone, Serialize)]
pub struct Candidate {
    pub transpose_semitones: i16,
    pub melody_total: Option<usize>,
    pub melody_kept: Option<usize>,
    pub melody_retention: Option<f64>,
    pub total_kept: usize,
    pub accompaniment_total: usize,
    pub accompaniment_kept: usize,
    pub accompaniment_retention: Option<f64>,
    pub dropped: usize,
    pub absolute_semitones: u16,
    pub preserved_melody_pairs: usize,
    pub retained_melody_leap_sum: u64,
    pub selected: bool,
    pub scoring_rule: &'static str,
}

pub(crate) fn key_index(instrument: &Instrument, target: i16) -> Option<usize> {
    u8::try_from(target)
        .ok()
        .filter(|p| *p <= 127)
        .and_then(|p| {
            instrument
                .keys
                .iter()
                .position(|key| key.midi_number() == p)
        })
}

pub(crate) fn evaluate(
    doc: &MusicDocument,
    instrument: &Instrument,
    melody: Option<usize>,
    accompaniment: &[bool],
    shift: i16,
) -> Candidate {
    let mut c = Candidate {
        transpose_semitones: shift,
        melody_total: melody.map(|_| 0),
        melody_kept: melody.map(|_| 0),
        melody_retention: None,
        total_kept: 0,
        accompaniment_total: 0,
        accompaniment_kept: 0,
        accompaniment_retention: None,
        dropped: 0,
        absolute_semitones: shift.unsigned_abs(),
        preserved_melody_pairs: 0,
        retained_melody_leap_sum: 0,
        selected: false,
        scoring_rule: SCORING_RULE,
    };
    let mut last_melody = None;
    let mut last_kept_pitch: Option<u8> = None;
    for n in &doc.notes {
        let kept = key_index(instrument, i16::from(n.pitch.value()) + shift).is_some();
        c.total_kept += usize::from(kept);
        if accompaniment[n.track] {
            c.accompaniment_total += 1;
            c.accompaniment_kept += usize::from(kept);
        }
        if Some(n.track) == melody {
            *c.melody_total.as_mut().unwrap() += 1;
            *c.melody_kept.as_mut().unwrap() += usize::from(kept);
            if last_melody == Some(true) && kept {
                c.preserved_melody_pairs += 1;
            }
            last_melody = Some(kept);
            if kept {
                if let Some(last) = last_kept_pitch {
                    c.retained_melody_leap_sum += u64::from(last.abs_diff(n.pitch.value()));
                }
                last_kept_pitch = Some(n.pitch.value());
            }
        }
    }
    c.dropped = doc.notes.len() - c.total_kept;
    c.melody_retention = c
        .melody_kept
        .zip(c.melody_total)
        .and_then(|(k, t)| ratio(k, t));
    c.accompaniment_retention = ratio(c.accompaniment_kept, c.accompaniment_total);
    c
}

pub(crate) fn compare(a: &Candidate, b: &Candidate) -> Ordering {
    let key = |c: &Candidate| {
        (
            c.melody_kept,
            c.total_kept,
            c.accompaniment_kept,
            Reverse(c.absolute_semitones),
            c.preserved_melody_pairs,
            Reverse(c.retained_melody_leap_sum),
            Reverse(c.transpose_semitones),
        )
    };
    key(a).cmp(&key(b))
}
