use crate::{Confidence, Evidence, MusicDocument, MusicNote, Origin};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct ActivityWindow {
    pub quarter_index: u64,
    pub onset_count: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct NoteStatistics {
    pub total_notes: usize,
    pub lowest_pitch: Option<u8>,
    pub highest_pitch: Option<u8>,
    pub pitch_range_semitones: Option<u8>,
    pub duration_ticks_distribution: BTreeMap<u64, usize>,
    /// Duration in source ticks for each active-note count, including silence
    /// from zero to the last note end. Notes occupy [start,end).
    pub polyphony_ticks_distribution: BTreeMap<usize, u64>,
    pub max_polyphony: usize,
    pub overlapping_note_pairs: u64,
    pub per_track_notes: Vec<usize>,
    pub onsets_per_quarter: Option<f64>,
    pub activity_windows: Vec<ActivityWindow>,
    pub principal_activity_quarters: Vec<u64>,
}

pub(crate) fn sorted_notes(doc: &MusicDocument) -> Vec<&MusicNote> {
    let mut notes: Vec<_> = doc.notes.iter().collect();
    notes.sort_by_key(|n| (n.start_tick, n.track, n.event_order(), &n.id));
    notes
}

pub(crate) fn notes(doc: &MusicDocument) -> NoteStatistics {
    let mut durations = BTreeMap::new();
    let mut sweep: BTreeMap<u64, (usize, usize)> = BTreeMap::new();
    let mut windows = BTreeMap::new();
    let mut per_track = vec![0; doc.tracks.len()];
    for n in &doc.notes {
        *durations.entry(n.end_tick - n.start_tick).or_default() += 1;
        sweep.entry(n.start_tick).or_default().0 += 1;
        sweep.entry(n.end_tick).or_default().1 += 1;
        *windows
            .entry(n.start_tick / u64::from(doc.ppq))
            .or_insert(0_usize) += 1;
        per_track[n.track] += 1;
    }
    let (mut previous, mut active, mut pairs, mut max_polyphony) = (0, 0_usize, 0_u64, 0);
    let mut histogram = BTreeMap::new();
    for (tick, (starts, ends)) in sweep {
        if tick > previous {
            *histogram.entry(active).or_insert(0) += tick - previous;
        }
        active -= ends;
        pairs +=
            active as u64 * starts as u64 + starts as u64 * starts.saturating_sub(1) as u64 / 2;
        active += starts;
        max_polyphony = max_polyphony.max(active);
        previous = tick;
    }
    let first = doc.notes.iter().map(|n| n.start_tick).min();
    let last = doc.notes.iter().map(|n| n.end_tick).max();
    let density = first
        .zip(last)
        .map(|(s, e)| doc.notes.len() as f64 * f64::from(doc.ppq) / (e - s) as f64);
    let low = doc.notes.iter().map(|n| n.pitch.value()).min();
    let high = doc.notes.iter().map(|n| n.pitch.value()).max();
    let principal = windows
        .iter()
        .filter(|(_, count)| **count * windows.len() >= doc.notes.len())
        .map(|(q, _)| *q)
        .collect();
    NoteStatistics {
        total_notes: doc.notes.len(),
        lowest_pitch: low,
        highest_pitch: high,
        pitch_range_semitones: low.zip(high).map(|(l, h)| h - l),
        duration_ticks_distribution: durations,
        polyphony_ticks_distribution: histogram,
        max_polyphony,
        overlapping_note_pairs: pairs,
        per_track_notes: per_track,
        onsets_per_quarter: density,
        activity_windows: windows
            .into_iter()
            .map(|(quarter_index, onset_count)| ActivityWindow {
                quarter_index,
                onset_count,
            })
            .collect(),
        principal_activity_quarters: principal,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EffectiveTempo {
    pub start_tick: u64,
    pub microseconds_per_quarter: u32,
    pub origin: Origin,
    pub source_events: Vec<crate::EventLocation>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TempoAnalysis {
    pub effective_map: Vec<EffectiveTempo>,
    pub principal_microseconds_per_quarter: Option<u32>,
    pub principal_bpm: Option<f64>,
    pub principal_method: &'static str,
    pub change_positions: Vec<u64>,
    pub change_count: usize,
    pub stability: &'static str,
    pub uses_protocol_default: bool,
}
pub(crate) fn tempos(doc: &MusicDocument) -> TempoAnalysis {
    let mut map: BTreeMap<u64, EffectiveTempo> = BTreeMap::new();
    for t in &doc.tempo_map {
        map.entry(t.location.tick)
            .or_insert_with(|| EffectiveTempo {
                start_tick: t.location.tick,
                microseconds_per_quarter: t.microseconds_per_quarter,
                origin: Origin::Source,
                source_events: Vec::new(),
            })
            .source_events
            .push(t.location.clone());
    }
    let uses_default = !map.contains_key(&0);
    map.entry(0).or_insert(EffectiveTempo {
        start_tick: 0,
        microseconds_per_quarter: 500_000,
        origin: Origin::ProtocolDefault,
        source_events: Vec::new(),
    });
    let effective: Vec<_> = map.into_values().collect();
    let changes: Vec<_> = effective
        .windows(2)
        .filter(|w| w[0].microseconds_per_quarter != w[1].microseconds_per_quarter)
        .map(|w| w[1].start_tick)
        .collect();
    let horizon = doc.notes.iter().map(|n| n.end_tick).max().unwrap_or(0);
    let mut weights = BTreeMap::<u32, u64>::new();
    for (i, t) in effective.iter().enumerate() {
        let end = effective
            .get(i + 1)
            .map_or(horizon, |n| n.start_tick.min(horizon));
        if end > t.start_tick {
            *weights.entry(t.microseconds_per_quarter).or_default() += end - t.start_tick;
        }
    }
    let principal = weights
        .iter()
        .max_by_key(|(us, ticks)| (**ticks, std::cmp::Reverse(**us)))
        .map(|(us, _)| *us);
    TempoAnalysis {
        principal_microseconds_per_quarter: principal,
        principal_bpm: principal.map(|us| 60_000_000.0 / f64::from(us)),
        principal_method: "largest accumulated source-tick span from zero to last note end; ties choose smaller us/qn; BPM is a display ratio",
        change_count: changes.len(),
        stability: if changes.is_empty() {
            "constant_effective_map"
        } else {
            "changing_effective_map"
        },
        change_positions: changes,
        uses_protocol_default: uses_default,
        effective_map: effective,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RhythmPattern {
    pub intervals_ticks: Vec<u64>,
    pub occurrences: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct BeatCandidate {
    pub period_ticks: u64,
    pub phase_tick: u64,
    pub interval_support_fraction: f64,
    pub confidence: Confidence,
    pub evidence: Evidence,
}
#[derive(Debug, Clone, Serialize)]
pub struct RhythmAnalysis {
    pub unique_onsets: usize,
    pub inter_onset_ticks_distribution: BTreeMap<u64, usize>,
    pub onsets_per_quarter: Option<f64>,
    pub repeated_patterns: Vec<RhythmPattern>,
    pub repeated_patterns_truncated: bool,
    pub modal_interval_fraction: Option<f64>,
    pub stability: &'static str,
    pub near_nominal_downbeat_onsets: Option<usize>,
    pub downbeat_method: &'static str,
    pub beat_candidates: Vec<BeatCandidate>,
    pub evidence: Evidence,
}
pub(crate) fn rhythm(doc: &MusicDocument) -> RhythmAnalysis {
    let sorted = sorted_notes(doc);
    let ticks: Vec<_> = sorted
        .iter()
        .map(|n| n.start_tick)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let intervals: Vec<_> = ticks.windows(2).map(|w| w[1] - w[0]).collect();
    let mut hist = BTreeMap::new();
    for v in &intervals {
        *hist.entry(*v).or_insert(0_usize) += 1;
    }
    let mut patterns = BTreeMap::new();
    for window in intervals.windows(3) {
        *patterns.entry(window.to_vec()).or_insert(0_usize) += 1;
    }
    let mut repeated: Vec<_> = patterns
        .into_iter()
        .filter(|(_, n)| *n >= 2)
        .map(|(intervals_ticks, occurrences)| RhythmPattern {
            intervals_ticks,
            occurrences,
        })
        .collect();
    repeated.sort_by_key(|p| (std::cmp::Reverse(p.occurrences), p.intervals_ticks.clone()));
    let truncated = repeated.len() > 32;
    repeated.truncate(32);
    let mut ranked: Vec<_> = hist.iter().collect();
    ranked.sort_by_key(|(ticks, count)| (std::cmp::Reverse(**count), **ticks));
    let fraction = ranked
        .first()
        .map(|(_, n)| **n as f64 / intervals.len() as f64);
    let evidence = Evidence::inferred(
        doc,
        "distinct onset intervals; exact length-3 windows; no quantization",
        &sorted,
    );
    let candidates = ranked
        .into_iter()
        .take(3)
        .map(|(period, count)| BeatCandidate {
            period_ticks: *period,
            phase_tick: ticks[0] % period,
            interval_support_fraction: *count as f64 / intervals.len() as f64,
            confidence: Confidence::uncalibrated(),
            evidence: evidence.clone(),
        })
        .collect();
    // Meter changes reset a nominal bar origin. This is an explicitly labeled
    // analytical convention, not a recovered bar/phrase boundary.
    let meter = crate::structure::grid_meters(doc);
    let mut near = Some(0);
    for tick in &ticks {
        let (&origin, &(num, pow, _)) = meter.range(..=*tick).next_back().expect("initial meter");
        if pow > 31 {
            near = None;
            break;
        }
        let denom = 1_u128 << pow;
        let bar = u128::from(num) * 4 * u128::from(doc.ppq);
        let remainder = (u128::from(*tick - origin) * denom) % bar;
        if remainder.min(bar - remainder) * 16 <= u128::from(doc.ppq) * denom {
            *near.as_mut().unwrap() += 1;
        }
    }
    let span = ticks
        .first()
        .zip(ticks.last())
        .map(|(s, e)| e - s)
        .filter(|n| *n > 0);
    RhythmAnalysis {
        unique_onsets: ticks.len(),
        inter_onset_ticks_distribution: hist,
        onsets_per_quarter: span.map(|s| (ticks.len() - 1) as f64 * f64::from(doc.ppq) / s as f64),
        repeated_patterns: repeated,
        repeated_patterns_truncated: truncated,
        modal_interval_fraction: fraction,
        stability: match fraction {
            None => "unknown",
            Some(v) if v >= 0.8 => "regular_onset_intervals",
            Some(_) => "variable_onset_intervals",
        },
        near_nominal_downbeat_onsets: near,
        downbeat_method: "unique onsets within PPQ/16 of nominal bar boundary; initial default 4/4; reset only at changed numerator/denominator; denominator exponent >31 is unknown",
        beat_candidates: candidates,
        evidence,
    }
}
