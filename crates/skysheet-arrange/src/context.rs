use crate::{DropReason, NoteMapping, TranspositionSearch};
use serde::Serialize;
use skysheet_music::{MusicDocument, TrackRole};
use skysheet_score::Instrument;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArrangementMode {
    #[default]
    MelodyFirst,
    SimpleAccompaniment,
    Polyphonic,
}
impl ArrangementMode {
    pub fn name(self) -> &'static str {
        match self {
            Self::MelodyFirst => "melody-first",
            Self::SimpleAccompaniment => "simple-accompaniment",
            Self::Polyphonic => "polyphonic",
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ArrangementOptions {
    pub mode: ArrangementMode,
    pub allow_octave_shifts: bool,
    pub melody_priority: u8,
    pub bass_priority: u8,
    pub accompaniment_priority: u8,
    pub maximum_polyphony: Option<usize>,
}
impl Default for ArrangementOptions {
    fn default() -> Self {
        Self {
            mode: ArrangementMode::MelodyFirst,
            allow_octave_shifts: true,
            melody_priority: 3,
            bass_priority: 2,
            accompaniment_priority: 1,
            maximum_polyphony: None,
        }
    }
}
impl ArrangementOptions {
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.melody_priority,
            self.bass_priority,
            self.accompaniment_priority,
        ]
        .iter()
        .any(|v| *v > 5)
        {
            return Err("priorities must be integers in 0..5".into());
        }
        if self
            .maximum_polyphony
            .is_some_and(|v| !(1..=128).contains(&v))
        {
            return Err("maximum_polyphony must be in 1..128 or absent (unlimited)".into());
        }
        Ok(())
    }
}

pub(crate) fn role(search: &TranspositionSearch, track: usize) -> TrackRole {
    if search.melody.track == Some(track) {
        TrackRole::Melody
    } else {
        match search.analysis.track_roles[track].role {
            TrackRole::Melody => TrackRole::Unknown,
            other => other,
        }
    }
}

pub(crate) fn is_accompaniment(r: TrackRole) -> bool {
    matches!(
        r,
        TrackRole::Bass | TrackRole::Harmony | TrackRole::Accompaniment | TrackRole::Rhythm
    )
}

// Global search is a retention upper-bound heuristic, NOT joint optimization of
// every track's DP. Preserve the original global-search API for the 4-A baseline.
pub(crate) fn rerank(
    doc: &MusicDocument,
    search: &mut TranspositionSearch,
    instrument: &Instrument,
) {
    let classes: BTreeSet<_> = instrument
        .keys
        .iter()
        .map(|p| i16::from(p.midi_number()) % 12)
        .collect();
    for c in &mut search.candidates {
        c.selected = false;
        c.total_kept = 0;
        c.accompaniment_kept = 0;
        c.melody_kept = search.melody.track.map(|_| 0);
        for n in &doc.notes {
            let p = i16::from(n.pitch.value()) + c.transpose_semitones;
            let kept = (0..=127).contains(&p) && classes.contains(&(p % 12));
            c.total_kept += usize::from(kept);
            if Some(n.track) == search.melody.track {
                *c.melody_kept.as_mut().unwrap() += usize::from(kept);
            } else if is_accompaniment(search.analysis.track_roles[n.track].role) {
                c.accompaniment_kept += usize::from(kept);
            }
        }
        c.dropped = doc.notes.len() - c.total_kept;
        c.melody_retention = c
            .melody_kept
            .zip(c.melody_total)
            .and_then(|(k, t)| crate::search::ratio(k, t));
        c.accompaniment_retention =
            crate::search::ratio(c.accompaniment_kept, c.accompaniment_total);
        c.preserved_melody_pairs = 0;
        c.retained_melody_leap_sum = 0;
        c.scoring_rule = "octave-feasible retention upper bound: melody count, total count, accompaniment count, smallest absolute then signed global shift; final DP/mode counts appear in report, not these candidates";
    }
    search.selected_index = search
        .candidates
        .iter()
        .enumerate()
        .max_by(|(_, a), (_, b)| crate::search::compare(a, b))
        .unwrap()
        .0;
    search.candidates[search.selected_index].selected = true;
}

#[derive(Clone)]
struct State {
    octave: i16,
    representative: i16,
    lost: usize,
    local: u64,
}

// Exact finite-state shortest path for this additive objective only. Each onset
// group shares one octave, so mapped chord members keep their exact intervals.
// Lost notes dominate cost. Stable ascending octave/predecessor order breaks ties.
fn solve(groups: &[Vec<State>], contour_weight: u64) -> Vec<usize> {
    if groups.is_empty() {
        return vec![];
    }
    let mut costs: Vec<(usize, u64)> = groups[0].iter().map(|s| (s.lost, s.local)).collect();
    let mut parents = vec![vec![0; groups[0].len()]];
    for g in 1..groups.len() {
        let mut next = Vec::new();
        let mut row = Vec::new();
        for s in &groups[g] {
            let (parent, cost) = groups[g - 1]
                .iter()
                .enumerate()
                .map(|(j, p)| {
                    let original = s.representative - p.representative;
                    let mapped = original + 12 * (s.octave - p.octave);
                    let transition = u64::from((mapped - original).unsigned_abs()) * contour_weight
                        + u64::from(original.signum() != mapped.signum()) * contour_weight * 6
                        + u64::from(mapped.unsigned_abs().saturating_sub(12)) * 4
                        + u64::from(s.octave.abs_diff(p.octave)) * 8;
                    (j, (costs[j].0 + s.lost, costs[j].1 + s.local + transition))
                })
                .min_by_key(|(j, c)| (*c, *j))
                .unwrap();
            next.push(cost);
            row.push(parent);
        }
        costs = next;
        parents.push(row);
    }
    let mut selected = costs
        .iter()
        .enumerate()
        .min_by_key(|(i, c)| (**c, *i))
        .unwrap()
        .0;
    let mut result = vec![0; groups.len()];
    for g in (0..groups.len()).rev() {
        result[g] = selected;
        selected = parents[g][selected];
    }
    result
}

pub(crate) fn octaves(
    doc: &MusicDocument,
    search: &TranspositionSearch,
    instrument: &Instrument,
    options: &ArrangementOptions,
) -> Vec<i16> {
    let mut result = vec![0; doc.notes.len()];
    if !options.allow_octave_shifts {
        return result;
    }
    let shift = search.candidates[search.selected_index].transpose_semitones;
    let keys: BTreeSet<_> = instrument
        .keys
        .iter()
        .map(|p| i16::from(p.midi_number()))
        .collect();
    let mut tracks = vec![BTreeMap::<u64, Vec<usize>>::new(); doc.tracks.len()];
    for (i, n) in doc.notes.iter().enumerate() {
        tracks[n.track].entry(n.start_tick).or_default().push(i);
    }
    let mut order: Vec<_> = (0..tracks.len()).collect();
    order.sort_by_key(|t| (Some(*t) != search.melody.track, *t));
    let mut melody_floor = BTreeMap::<u64, i16>::new();
    for t in order {
        let onsets = &tracks[t];
        let r = role(search, t);
        let center = match r {
            TrackRole::Melody => 76,
            TrackRole::Bass => 64,
            TrackRole::Harmony | TrackRole::Accompaniment => {
                if options.mode == ArrangementMode::Polyphonic {
                    69
                } else {
                    67
                }
            }
            _ => 72,
        };
        let groups: Vec<Vec<State>> = onsets
            .values()
            .map(|ids| {
                let representative = ids
                    .iter()
                    .map(|i| i16::from(doc.notes[*i].pitch.value()) + shift)
                    .max()
                    .unwrap();
                (-10i16..=10)
                    .map(|octave| {
                        let mut lost = 0;
                        let mut local = 0;
                        for i in ids {
                            let p = i16::from(doc.notes[*i].pitch.value()) + shift;
                            let target = p + 12 * octave;
                            if !(0..=127).contains(&p) || !keys.contains(&target) {
                                lost += 1;
                            } else {
                                local += u64::from(target.abs_diff(center));
                                if r == TrackRole::Bass {
                                    // Sequential conditional optimization: melody is fixed first.
                                    // Context is actual sounding melody at this source onset.
                                    if melody_floor
                                        .get(&doc.notes[*i].start_tick)
                                        .is_some_and(|p| target >= *p)
                                    {
                                        // Bass function below the selected melody also matters
                                        // in polyphonic mode. The first candidate's weaker
                                        // penalty produced audible register confusion in Ode.
                                        local += 480;
                                    }
                                }
                            }
                        }
                        // Prefer no gratuitous displacement; role register is a soft preference.
                        local += u64::from(octave.unsigned_abs()) * 3;
                        State {
                            octave,
                            representative,
                            lost,
                            local,
                        }
                    })
                    .collect()
            })
            .collect();
        let weight = if r == TrackRole::Melody {
            4 * (u64::from(options.melody_priority) + 1)
        } else {
            16
        };
        for ((ids, states), selected) in onsets.values().zip(&groups).zip(solve(&groups, weight)) {
            for i in ids {
                result[*i] = states[selected].octave;
            }
        }
        if Some(t) == search.melody.track {
            let mut events = BTreeMap::<u64, (Vec<usize>, Vec<usize>)>::new();
            for n in &doc.notes {
                events.entry(n.start_tick).or_default();
            }
            for ids in onsets.values() {
                for i in ids {
                    let n = &doc.notes[*i];
                    let p = i16::from(n.pitch.value()) + shift + 12 * result[*i];
                    if keys.contains(&p) {
                        events.entry(n.start_tick).or_default().0.push(*i);
                        events.entry(n.end_tick).or_default().1.push(*i);
                    }
                }
            }
            let mut active = BTreeSet::new();
            for (tick, (starts, ends)) in events {
                for i in ends {
                    active.remove(&(
                        i16::from(doc.notes[i].pitch.value()) + shift + 12 * result[i],
                        i,
                    ));
                }
                for i in starts {
                    active.insert((
                        i16::from(doc.notes[i].pitch.value()) + shift + 12 * result[i],
                        i,
                    ));
                }
                if let Some((pitch, _)) = active.first() {
                    melody_floor.insert(tick, *pitch);
                }
            }
        }
    }
    result
}

pub(crate) fn importance(
    doc: &MusicDocument,
    search: &TranspositionSearch,
    options: &ArrangementOptions,
    index: usize,
) -> (u32, Vec<&'static str>) {
    let n = &doc.notes[index];
    let r = role(search, n.track);
    let (score, reason) = match r {
        TrackRole::Melody => (
            1000 + u32::from(options.melody_priority) * 100,
            "provisional melody; protected tier",
        ),
        TrackRole::Bass => (
            300 + u32::from(options.bass_priority) * 100,
            "provisional bass support",
        ),
        TrackRole::Harmony | TrackRole::Accompaniment | TrackRole::Rhythm => (
            100 + u32::from(options.accompaniment_priority) * 100,
            "provisional accompaniment support",
        ),
        _ => (200, "unknown role; no accompaniment assumption"),
    };
    let beat = n.start_tick.is_multiple_of(u64::from(doc.ppq));
    let mut reasons = vec![reason];
    if beat {
        reasons.push("source quarter-note onset (+20), not a meter/tonic claim");
    }
    (score + if beat { 20 } else { 0 }, reasons)
}

pub(crate) fn reduce(
    mappings: &mut [NoteMapping],
    search: &TranspositionSearch,
    options: &ArrangementOptions,
) {
    let mut onsets = BTreeMap::<u64, Vec<usize>>::new();
    for (i, n) in mappings.iter().enumerate() {
        if n.mapped {
            onsets.entry(n.projected_start_tick).or_default().push(i);
        }
    }
    let mut active = BTreeSet::<(u64, usize)>::new();
    let mut priorities = BTreeSet::<(u32, usize)>::new();
    for (tick, mut ids) in onsets {
        while active.first().is_some_and(|(end, _)| *end <= tick) {
            let (_, i) = active.pop_first().unwrap();
            priorities.remove(&(mappings[i].importance, i));
        }
        ids.sort_by_key(|i| {
            (
                std::cmp::Reverse(mappings[*i].importance),
                mappings[*i].source_midi,
                *i,
            )
        });
        // Simple mode keeps at most two DISTINCT accompaniment pitches per onset;
        // repeated events of an admitted pitch remain separate. Never filter melody/Unknown.
        let mut support = BTreeSet::new();
        for i in ids {
            let r = role(search, mappings[i].source_track);
            let pitch = mappings[i].target_midi;
            let simplify = options.mode == ArrangementMode::SimpleAccompaniment
                && is_accompaniment(r)
                && !support.contains(&pitch)
                && support.len() >= 2;
            let mut limit = options
                .maximum_polyphony
                .is_some_and(|max| active.len() >= max);
            if limit && !simplify {
                let (priority, j) = *priorities.first().unwrap();
                if mappings[i].importance > priority {
                    priorities.remove(&(priority, j));
                    let n = &mut mappings[j];
                    active.remove(&(n.projected_start_tick + n.projected_duration_ticks, j));
                    n.mapped = false;
                    n.drop_reason = Some(DropReason::MaximumPolyphony);
                    n.key_index = None;
                    n.target_pitch = None;
                    n.mapping_reason.push_str("; omitted whole event for a later overlapping event with higher importance; never shortened");
                    limit = false;
                }
            }
            if simplify || limit {
                let n = &mut mappings[i];
                n.mapped = false;
                n.drop_reason = Some(if simplify {
                    DropReason::AccompanimentSimplified
                } else {
                    DropReason::MaximumPolyphony
                });
                n.mapping_reason.push_str(if simplify { "; omitted: simple mode two distinct support tones per onset, importance then lower source pitch" } else { "; omitted: user overlap limit; higher importance wins, ties retain ongoing events then source pitch/event order" });
                n.key_index = None;
                n.target_pitch = None;
            } else {
                if is_accompaniment(r) {
                    support.insert(pitch);
                }
                active.insert((tick + mappings[i].projected_duration_ticks, i));
                priorities.insert((mappings[i].importance, i));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dp_matches_independent_enumeration_and_stable_tie() {
        let groups: Vec<Vec<State>> = [60, 62, 79]
            .iter()
            .map(|p| {
                [-1, 0, 1]
                    .iter()
                    .map(|o| State {
                        octave: *o,
                        representative: *p,
                        lost: 0,
                        local: u64::from((*p + 12 * o - 72).unsigned_abs()),
                    })
                    .collect()
            })
            .collect();
        // Independent complete enumeration, including the documented objective.
        let mut best = (u64::MAX, vec![]);
        for a in 0..3 {
            for b in 0..3 {
                for c in 0..3 {
                    let path = vec![a, b, c];
                    let mut cost = 0;
                    for i in 0..3 {
                        cost += groups[i][path[i]].local;
                    }
                    for i in 1..3 {
                        let x = &groups[i - 1][path[i - 1]];
                        let y = &groups[i][path[i]];
                        let s = y.representative - x.representative;
                        let d = s + 12 * (y.octave - x.octave);
                        cost += ((d - s).abs() * 16
                            + if s.signum() != d.signum() { 96 } else { 0 }
                            + (d.abs() - 12).max(0) * 4
                            + (y.octave - x.octave).abs() * 8)
                            as u64;
                    }
                    if cost < best.0 {
                        best = (cost, path);
                    }
                }
            }
        }
        // +1 octave: local 0+2+19, original 17-semitone leap costs 20.
        assert_eq!(best.0, 41);
        assert_eq!(solve(&groups, 16), best.1);
        let tie = vec![
            vec![
                State {
                    octave: 0,
                    representative: 60,
                    lost: 0,
                    local: 0
                };
                2
            ];
            3
        ];
        assert_eq!(solve(&tie, 16), vec![0, 0, 0]);
    }
}
