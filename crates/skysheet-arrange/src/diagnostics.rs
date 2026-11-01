use crate::{ArrangementReport, Conflict, OnsetGroup, RoleRetention};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn complete(r: &mut ArrangementReport) {
    r.kept = r.note_mappings.iter().filter(|n| n.mapped).count();
    r.dropped = r.original_notes - r.kept;
    r.retention = crate::search::ratio(r.kept, r.original_notes);
    let role_names: Vec<_> = r
        .tracks
        .iter()
        .map(|t| {
            if Some(t.source_track) == r.melody.track {
                "melody".into()
            } else {
                if t.phase3_role.role == skysheet_music::TrackRole::Melody {
                    "unknown".into()
                } else {
                    format!("{:?}", t.phase3_role.role).to_lowercase()
                }
            }
        })
        .collect();
    for role in [
        "melody",
        "bass",
        "harmony",
        "accompaniment",
        "rhythm",
        "unknown",
    ] {
        let notes: Vec<_> = r
            .note_mappings
            .iter()
            .filter(|n| role_names[n.source_track] == role)
            .collect();
        let kept = notes.iter().filter(|n| n.mapped).count();
        r.role_retention.push(RoleRetention {
            role: role.into(),
            kept,
            total: notes.len(),
            retention: crate::search::ratio(kept, notes.len()),
        });
    }
    r.melody_retention = r.role_retention[0].retention;
    let support = |t: usize| {
        matches!(
            role_names[t].as_str(),
            "bass" | "harmony" | "accompaniment" | "rhythm"
        )
    };
    let support_total: usize = r.role_retention[1..5].iter().map(|x| x.total).sum();
    let support_kept: usize = r.role_retention[1..5].iter().map(|x| x.kept).sum();
    r.accompaniment_retention = crate::search::ratio(support_kept, support_total);
    let mut tracks = vec![Vec::new(); r.tracks.len()];
    let mut groups = BTreeMap::<u64, Vec<usize>>::new();
    let mut source_support = BTreeSet::new();
    let mut kept_support = BTreeSet::new();
    for (i, n) in r.note_mappings.iter().enumerate() {
        tracks[n.source_track].push(i);
        groups.entry(n.projected_start_tick).or_default().push(i);
        if support(n.source_track) {
            source_support.insert(n.projected_start_tick);
            if n.mapped {
                kept_support.insert(n.projected_start_tick);
            }
        }
    }
    r.playability.source_accompaniment_onsets = source_support.len();
    r.playability.retained_accompaniment_onsets = kept_support.len();
    for (t, ids) in tracks.iter_mut().enumerate() {
        let kept: Vec<_> = ids
            .iter()
            .map(|i| &r.note_mappings[*i])
            .filter(|n| n.mapped)
            .collect();
        let tr = &mut r.tracks[t];
        tr.kept = kept.len();
        tr.dropped = tr.original_notes - tr.kept;
        tr.retention = crate::search::ratio(tr.kept, tr.original_notes);
        tr.final_lowest_midi = kept.iter().map(|n| n.target_midi as u8).min();
        tr.final_highest_midi = kept.iter().map(|n| n.target_midi as u8).max();
        tr.adjusted = kept.iter().filter(|n| n.voice_octave_shift != 0).count();
        r.playability.octave_adjusted_notes += tr.adjusted;
        r.per_track_octave_shift.push(
            kept.iter()
                .map(|n| n.voice_octave_shift)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        );
        ids.sort_by_key(|i| (r.note_mappings[*i].projected_start_tick, *i));
        for pair in ids.windows(2) {
            let a = &r.note_mappings[pair[0]];
            let b = &r.note_mappings[pair[1]];
            if !a.mapped || !b.mapped || a.projected_start_tick == b.projected_start_tick {
                continue;
            }
            let original = i16::from(b.source_midi) - i16::from(a.source_midi);
            let final_interval = b.target_midi - a.target_midi;
            let change = final_interval - original;
            if Some(t) == r.melody.track {
                r.playability.melody_interval_pairs += 1;
                r.playability.melody_interval_changes += usize::from(change != 0);
                r.playability.melody_direction_changes +=
                    usize::from(original.signum() != final_interval.signum());
                r.playability.unnecessary_large_leaps +=
                    usize::from(final_interval.abs() > 12 && final_interval.abs() > original.abs());
            }
            r.note_mappings[pair[0]].next_interval_change = Some(change);
            r.note_mappings[pair[1]].previous_interval_change = Some(change);
        }
    }
    let mut active = BTreeSet::<(u64, usize)>::new();
    let mut by_pitch = BTreeMap::<i16, BTreeSet<usize>>::new();
    let mut pitch_tracks = BTreeMap::<(i16, usize), usize>::new();
    let mut melody_pitches = BTreeMap::<i16, usize>::new();
    let mut bass_pitches = BTreeMap::<i16, usize>::new();
    for (tick, ids) in groups {
        let notes: Vec<_> = ids.iter().map(|i| &r.note_mappings[*i]).collect();
        let classes: BTreeSet<_> = notes.iter().map(|n| n.source_midi % 12).collect();
        let chord = classes.iter().find_map(|root| {
            let intervals: BTreeSet<_> = classes.iter().map(|p| (p + 12 - root) % 12).collect();
            match intervals.iter().copied().collect::<Vec<_>>().as_slice() {
                [0, 4, 7] => Some(format!(
                    "major triad pitch-class root {root}; inversion not tonal certainty"
                )),
                [0, 3, 7] => Some(format!(
                    "minor triad pitch-class root {root}; inversion not tonal certainty"
                )),
                [0, 3, 6] => Some(format!("diminished triad pitch-class root {root}")),
                _ => None,
            }
        });
        let kept: Vec<_> = notes.iter().filter(|n| n.mapped).collect();
        let changed = kept.first().is_some_and(|first| {
            kept.iter().any(|n| {
                n.target_midi - first.target_midi
                    != i16::from(n.source_midi) - i16::from(first.source_midi)
            })
        });
        r.onset_groups.push(OnsetGroup {
            start_tick: tick,
            source_note_ids: notes.iter().map(|n| n.source_note_id.clone()).collect(),
            kept: kept.len(),
            chord_candidate: chord,
            pitch_class_intervals_preserved: kept.windows(2).all(|pair| {
                (pair[1].target_midi - pair[0].target_midi).rem_euclid(12)
                    == (i16::from(pair[1].source_midi) - i16::from(pair[0].source_midi))
                        .rem_euclid(12)
            }),
            exact_intervals_changed: changed,
        });
        if notes.iter().any(|n| {
            matches!(
                n.drop_reason,
                Some(
                    crate::DropReason::MaximumPolyphony
                        | crate::DropReason::AccompanimentSimplified
                )
            )
        }) {
            r.conflicts.push(Conflict {
                start_tick: tick,
                source_note_ids: notes.iter().map(|n| n.source_note_id.clone()).collect(),
                source_count: notes.len(),
                kept: kept.len(),
                reason: "mode_or_user_constraint",
                resolution: "see per-note importance and drop reason; no time changes",
            });
        }
        while let Some((end, i)) = active.first().copied() {
            if end > tick {
                break;
            }
            active.pop_first();
            let n = &r.note_mappings[i];
            by_pitch.get_mut(&n.target_midi).unwrap().remove(&i);
            *pitch_tracks
                .get_mut(&(n.target_midi, n.source_track))
                .unwrap() -= 1;
            let map = match role_names[n.source_track].as_str() {
                "melody" => Some(&mut melody_pitches),
                "bass" => Some(&mut bass_pitches),
                _ => None,
            };
            if let Some(map) = map {
                let count = map.get_mut(&n.target_midi).unwrap();
                *count -= 1;
                if *count == 0 {
                    map.remove(&n.target_midi);
                }
            }
        }
        for i in ids {
            let n = &r.note_mappings[i];
            if !n.mapped {
                continue;
            }
            let same = by_pitch.entry(n.target_midi).or_default();
            if let Some(other) = same.first() {
                let p = &r.note_mappings[*other];
                r.playability.same_key_overlap_events += 1;
                let cross = same.len()
                    > pitch_tracks
                        .get(&(n.target_midi, n.source_track))
                        .copied()
                        .unwrap_or(0);
                r.playability.cross_track_key_conflicts += usize::from(cross);
                r.conflicts.push(Conflict {start_tick:tick,source_note_ids:vec![p.source_note_id.clone(),n.source_note_id.clone()],source_count:same.len()+1,kept:same.len()+1,reason:if p.projected_start_tick==tick {"same_key_simultaneous_onset"}else{"same_key_interval_overlap"},resolution:"retain independent events; IDs show current event and one representative, source_count includes all active same-key events"});
            }
            same.insert(i);
            *pitch_tracks
                .entry((n.target_midi, n.source_track))
                .or_default() += 1;
            let crossing = match role_names[n.source_track].as_str() {
                "melody" => bass_pitches
                    .last_key_value()
                    .is_some_and(|(p, _)| *p >= n.target_midi),
                "bass" => melody_pitches
                    .first_key_value()
                    .is_some_and(|(p, _)| *p <= n.target_midi),
                _ => false,
            };
            r.playability.bass_melody_crossing_events += usize::from(crossing);
            match role_names[n.source_track].as_str() {
                "melody" => *melody_pitches.entry(n.target_midi).or_default() += 1,
                "bass" => *bass_pitches.entry(n.target_midi).or_default() += 1,
                _ => {}
            }
            active.insert((tick + n.projected_duration_ticks, i));
        }
        r.playability.maximum_simultaneous_notes =
            r.playability.maximum_simultaneous_notes.max(active.len());
    }
    r.playability.arrangement_constraints_satisfied = r
        .maximum_polyphony
        .is_none_or(|max| r.playability.maximum_simultaneous_notes <= max);
    r.playability.human_audition =
        "not evaluated; structural checks cannot establish musical quality or ease of performance"
            .into();
}
