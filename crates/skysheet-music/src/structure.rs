//! Descriptive source statistics. Register boundaries are analysis heuristics,
//! not target-instrument limits or universal musicological definitions.
use crate::{Diagnostic, MusicDocument, Origin};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const LOW_REGISTER_END: u8 = 55;
pub const HIGH_REGISTER_START: u8 = 79;
const MAX_PAIRWISE_TRACKS: usize = 128;

#[derive(Debug, Clone, Default, Serialize)]
pub struct RegisterCounts {
    pub low: usize,
    pub middle: usize,
    pub high: usize,
}
impl RegisterCounts {
    fn add(&mut self, pitch: u8) {
        if pitch < LOW_REGISTER_END {
            self.low += 1;
        } else if pitch < HIGH_REGISTER_START {
            self.middle += 1;
        } else {
            self.high += 1;
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TickInterval {
    pub start_tick: u64,
    pub end_tick: u64,
}
#[derive(Debug, Clone, Serialize)]
pub struct PitchRange {
    pub lowest: u8,
    pub highest: u8,
    pub lowest_name: String,
    pub highest_name: String,
    pub span_semitones: u8,
}
fn range(low: u8, high: u8) -> PitchRange {
    PitchRange {
        lowest: low,
        highest: high,
        lowest_name: crate::MidiPitch::new(low).unwrap().name(),
        highest_name: crate::MidiPitch::new(high).unwrap().name(),
        span_semitones: high - low,
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackStatistics {
    pub track: usize,
    pub name: Option<String>,
    pub note_count: usize,
    pub pitch_range: Option<PitchRange>,
    pub registers: RegisterCounts,
    pub activity_intervals: Vec<TickInterval>,
    pub active_ticks: u64,
    pub onsets_per_quarter: Option<f64>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackPair {
    pub tracks: [usize; 2],
    pub pitch_range_intersection: Option<PitchRange>,
    pub simultaneous_onsets: usize,
    pub onset_jaccard: Option<f64>,
    pub overlap_intervals: Vec<TickInterval>,
    pub overlap_ticks: u64,
}
#[derive(Debug, Clone, Serialize)]
pub struct OnsetGroup {
    pub tick: u64,
    pub note_ids: Vec<String>,
    pub tracks: Vec<usize>,
    pub position: MusicalPosition,
}
#[derive(Debug, Clone, Serialize)]
pub struct SoundingInterval {
    pub start_tick: u64,
    pub end_tick: u64,
    pub active_notes: usize,
}
#[derive(Debug, Clone, Serialize)]
pub struct RepeatedPitch {
    pub track: usize,
    pub channel: Option<u8>,
    pub pitch: u8,
    pub note_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct MusicalPosition {
    pub meter_start_tick: u64,
    /// One-based, decimal integer string: extreme legal ticks can exceed u64 bars.
    pub bar: Option<String>,
    pub beat: Option<u16>,
    pub beat_fraction_numerator: Option<u64>,
    pub beat_fraction_denominator: Option<u64>,
}
#[derive(Debug, Clone, Serialize)]
pub struct MeterSegment {
    pub start_tick: u64,
    pub end_tick: Option<u64>,
    pub numerator: u8,
    pub denominator_power: u8,
    pub origin: Origin,
    pub bar_at_start: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct StructureAnalysis {
    pub register_policy: &'static str,
    pub low_register_end: u8,
    pub high_register_start: u8,
    pub registers: RegisterCounts,
    pub midi_pitch_distribution: Vec<usize>,
    pub tracks: Vec<TrackStatistics>,
    pub track_pairs: Vec<TrackPair>,
    pub pairwise_complete: bool,
    pub onset_groups: Vec<OnsetGroup>,
    pub sounding_intervals: Vec<SoundingInterval>,
    pub repeated_pitches: Vec<RepeatedPitch>,
    pub meter_segments: Vec<MeterSegment>,
    pub position_policy: &'static str,
}
fn merge(mut intervals: Vec<TickInterval>) -> Vec<TickInterval> {
    intervals.sort_by_key(|i| (i.start_tick, i.end_tick));
    let mut out: Vec<TickInterval> = Vec::new();
    for interval in intervals {
        if let Some(last) = out.last_mut()
            && interval.start_tick <= last.end_tick
        {
            last.end_tick = last.end_tick.max(interval.end_tick);
        } else {
            out.push(interval);
        }
    }
    out
}
fn intersect(a: &[TickInterval], b: &[TickInterval]) -> Vec<TickInterval> {
    let (mut i, mut j) = (0, 0);
    let mut out = Vec::new();
    while i < a.len() && j < b.len() {
        let start = a[i].start_tick.max(b[j].start_tick);
        let end = a[i].end_tick.min(b[j].end_tick);
        if start < end {
            out.push(TickInterval {
                start_tick: start,
                end_tick: end,
            });
        }
        if a[i].end_tick <= b[j].end_tick {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}
pub(crate) fn grid_meters(doc: &MusicDocument) -> BTreeMap<u64, (u8, u8, Origin)> {
    let mut map = BTreeMap::from([(0, (4, 2, Origin::ProtocolDefault))]);
    for m in &doc.time_signature_map {
        map.insert(
            m.location.tick,
            (m.numerator, m.denominator_power, Origin::Source),
        );
    }
    // Repeated declarations of the same nominal signature do not restart a bar.
    // The complete original map (including click parameters) stays in source.
    let mut previous = None;
    map.retain(|_, v| {
        let signature = (v.0, v.1);
        if previous == Some(signature) {
            false
        } else {
            previous = Some(signature);
            true
        }
    });
    map
}
fn meter_segments(doc: &MusicDocument, diagnostics: &mut Vec<Diagnostic>) -> Vec<MeterSegment> {
    let map = grid_meters(doc);
    let mut result: Vec<MeterSegment> = Vec::new();
    let mut bar = Some(1_u128);
    for (tick, (num, pow, origin)) in map {
        if let Some(last) = result.last_mut() {
            last.end_tick = Some(tick);
            if last.denominator_power <= 31 {
                let elapsed =
                    u128::from(tick - last.start_tick) * (1_u128 << last.denominator_power);
                let length = u128::from(last.numerator) * 4 * u128::from(doc.ppq);
                if !elapsed.is_multiple_of(length) {
                    diagnostics.push(Diagnostic::analysis("partial_bar_at_meter_change",&format!("meter change at tick {tick}: start a new nominal bar; this is not recovered musical segmentation")));
                }
                bar = bar.map(|b| b + elapsed.div_ceil(length));
            } else {
                bar = None;
            }
        }
        if pow > 31 {
            diagnostics.push(Diagnostic::analysis("unsupported_meter_grid",&format!("meter denominator exponent {pow} at tick {tick}: beat grid unavailable; original meter retained")));
        }
        result.push(MeterSegment {
            start_tick: tick,
            end_tick: None,
            numerator: num,
            denominator_power: pow,
            origin,
            bar_at_start: bar.map(|v| v.to_string()),
        });
    }
    result
}
fn position(tick: u64, segments: &[MeterSegment], ppq: u16) -> MusicalPosition {
    let i = segments.partition_point(|m| m.start_tick <= tick) - 1;
    let m = &segments[i];
    let mut p = MusicalPosition {
        meter_start_tick: m.start_tick,
        bar: None,
        beat: None,
        beat_fraction_numerator: None,
        beat_fraction_denominator: None,
    };
    if m.denominator_power <= 31 {
        let scaled = u128::from(tick - m.start_tick) * (1_u128 << m.denominator_power);
        let beat_ticks = 4 * u128::from(ppq);
        let beats = scaled / beat_ticks;
        p.bar = m
            .bar_at_start
            .as_ref()
            .map(|b| (b.parse::<u128>().unwrap() + beats / u128::from(m.numerator)).to_string());
        p.beat = Some((beats % u128::from(m.numerator)) as u16 + 1);
        p.beat_fraction_numerator = Some((scaled % beat_ticks) as u64);
        p.beat_fraction_denominator = Some(beat_ticks as u64);
    }
    p
}
pub(crate) fn structure(
    doc: &MusicDocument,
    diagnostics: &mut Vec<Diagnostic>,
) -> StructureAnalysis {
    let meters = meter_segments(doc, diagnostics);
    let mut grouped = vec![Vec::new(); doc.tracks.len()];
    let mut onsets: BTreeMap<u64, Vec<_>> = BTreeMap::new();
    let mut repeated: BTreeMap<_, Vec<String>> = BTreeMap::new();
    let mut sweep: BTreeMap<u64, i64> = BTreeMap::new();
    let mut histogram = vec![0; 128];
    let mut registers = RegisterCounts::default();
    for n in &doc.notes {
        grouped[n.track].push(n);
        onsets.entry(n.start_tick).or_default().push(n);
        repeated
            .entry((n.track, n.channel(), n.pitch.value()))
            .or_default()
            .push(n.id.clone());
        *sweep.entry(n.start_tick).or_default() += 1;
        *sweep.entry(n.end_tick).or_default() -= 1;
        histogram[usize::from(n.pitch.value())] += 1;
        registers.add(n.pitch.value());
    }
    let mut tracks = Vec::new();
    let mut onset_sets = Vec::new();
    for (track, notes) in grouped.iter().enumerate() {
        let low = notes.iter().map(|n| n.pitch.value()).min();
        let high = notes.iter().map(|n| n.pitch.value()).max();
        let mut registers = RegisterCounts::default();
        for n in notes {
            registers.add(n.pitch.value());
        }
        let activity = merge(
            notes
                .iter()
                .map(|n| TickInterval {
                    start_tick: n.start_tick,
                    end_tick: n.end_tick,
                })
                .collect(),
        );
        let density = activity.first().zip(activity.last()).map(|(a, b)| {
            notes.len() as f64 * f64::from(doc.ppq) / (b.end_tick - a.start_tick) as f64
        });
        onset_sets.push(notes.iter().map(|n| n.start_tick).collect::<BTreeSet<_>>());
        tracks.push(TrackStatistics {
            track,
            name: doc.tracks[track].name.clone(),
            note_count: notes.len(),
            pitch_range: low.zip(high).map(|(l, h)| range(l, h)),
            registers,
            active_ticks: activity.iter().map(|v| v.end_tick - v.start_tick).sum(),
            activity_intervals: activity,
            onsets_per_quarter: density,
        });
        if notes.len() < 3 {
            diagnostics.push(Diagnostic::analysis(
                "sparse_track",
                &format!(
                    "track {track}: {} notes; insufficient evidence for melody/role classification",
                    notes.len()
                ),
            ));
        }
        if density.is_some_and(|d| d > 8.0) {
            diagnostics.push(Diagnostic::analysis("dense_track",&format!("track {track}: over 8 note onsets per quarter; inspect chords/retriggers before interpreting as melody")));
        }
    }
    let nonempty: Vec<_> = tracks.iter().filter(|t| t.note_count > 0).collect();
    let complete = nonempty.len() <= MAX_PAIRWISE_TRACKS;
    let mut pairs = Vec::new();
    if complete {
        for (i, a) in nonempty.iter().enumerate() {
            for b in &nonempty[i + 1..] {
                let ar = a.pitch_range.as_ref().unwrap();
                let br = b.pitch_range.as_ref().unwrap();
                let low = ar.lowest.max(br.lowest);
                let high = ar.highest.min(br.highest);
                let overlap = intersect(&a.activity_intervals, &b.activity_intervals);
                let ticks = overlap.iter().map(|v| v.end_tick - v.start_tick).sum();
                let shared = onset_sets[a.track]
                    .intersection(&onset_sets[b.track])
                    .count();
                let union = onset_sets[a.track].len() + onset_sets[b.track].len() - shared;
                let jaccard = (union > 0).then(|| shared as f64 / union as f64);
                if jaccard.is_some_and(|v| v >= 0.8) {
                    diagnostics.push(Diagnostic::analysis("synchronized_tracks",&format!("tracks {} and {} share at least 80% of distinct onset positions; role separation may be ambiguous",a.track,b.track)));
                }
                pairs.push(TrackPair {
                    tracks: [a.track, b.track],
                    pitch_range_intersection: (low <= high).then(|| range(low, high)),
                    simultaneous_onsets: shared,
                    onset_jaccard: jaccard,
                    overlap_intervals: overlap,
                    overlap_ticks: ticks,
                });
            }
        }
    } else {
        diagnostics.push(Diagnostic::analysis("pairwise_track_limit","More than 128 active tracks: pairwise detail omitted; all source notes, individual tracks and global statistics retained."));
    }
    let mut intervals = Vec::new();
    let (mut previous, mut active) = (0, 0_i64);
    for (tick, delta) in sweep {
        if tick > previous && active > 0 {
            intervals.push(SoundingInterval {
                start_tick: previous,
                end_tick: tick,
                active_notes: active as usize,
            });
        }
        active += delta;
        previous = tick;
    }
    StructureAnalysis {
        register_policy: "descriptive heuristic: low <55, middle 55..78, high >=79; no instrument constraint",
        low_register_end: LOW_REGISTER_END,
        high_register_start: HIGH_REGISTER_START,
        registers,
        midi_pitch_distribution: histogram,
        tracks,
        track_pairs: pairs,
        pairwise_complete: complete,
        onset_groups: onsets
            .into_iter()
            .map(|(tick, notes)| OnsetGroup {
                tick,
                position: position(tick, &meters, doc.ppq),
                note_ids: notes.iter().map(|n| n.id.clone()).collect(),
                tracks: notes
                    .iter()
                    .map(|n| n.track)
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            })
            .collect(),
        sounding_intervals: intervals,
        repeated_pitches: repeated
            .into_iter()
            .filter(|(_, v)| v.len() > 1)
            .map(|((track, channel, pitch), note_ids)| RepeatedPitch {
                track,
                channel,
                pitch,
                note_ids,
            })
            .collect(),
        meter_segments: meters,
        position_policy: "one-based nominal bars/beats from tick zero; meter changes start a new bar (partial prior bar rounds up); no phrase detection; beat fractions are exact ratios",
    }
}
