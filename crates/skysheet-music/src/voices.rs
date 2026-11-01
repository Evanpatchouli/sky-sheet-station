use crate::{Confidence, Diagnostic, Evidence, MusicDocument, MusicError, MusicNote};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize)]
pub struct VoiceFeatures {
    pub note_count: usize,
    pub mean_pitch: f64,
    pub mean_leap_semitones: Option<f64>,
    pub stepwise_fraction: Option<f64>,
    pub repeated_pitch_fraction: Option<f64>,
    pub mean_duration_quarters: f64,
    pub onsets_per_quarter: f64,
    pub modal_interval_fraction: Option<f64>,
    pub connected_transition_fraction: Option<f64>,
    pub simultaneous_onset_fraction: f64,
    pub overlapping_transition_fraction: Option<f64>,
    pub onset_with_other_track_fraction: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct MelodyCandidate {
    pub id: String,
    pub track: usize,
    pub channel: Option<u8>,
    pub voice_index: usize,
    pub score: f64,
    pub scoring: MelodyScoring,
    pub confidence: Confidence,
    pub features: VoiceFeatures,
    pub evidence: Evidence,
    pub reasons: Vec<String>,
    pub possible_errors: Vec<&'static str>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TrackRole {
    Melody,
    Bass,
    Harmony,
    Accompaniment,
    Rhythm,
    Unknown,
}
#[derive(Debug, Clone, Serialize)]
pub struct RoleCandidate {
    pub role: TrackRole,
    pub score: f64,
    pub reasons: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackRoleEstimate {
    pub track: usize,
    pub role: TrackRole,
    pub candidates: Vec<RoleCandidate>,
    pub confidence: Confidence,
    pub evidence: Evidence,
    pub features: Option<VoiceFeatures>,
    pub possible_errors: Vec<&'static str>,
}

fn other_track_onsets(doc: &MusicDocument) -> BTreeSet<&str> {
    let mut events: BTreeMap<u64, (Vec<&MusicNote>, Vec<&MusicNote>)> = BTreeMap::new();
    for n in &doc.notes {
        events.entry(n.start_tick).or_default().0.push(n);
        events.entry(n.end_tick).or_default().1.push(n);
    }
    let mut active = BTreeMap::<usize, usize>::new();
    let mut found = BTreeSet::new();
    for (_, (starts, ends)) in events {
        for n in ends {
            let count = active.get_mut(&n.track).expect("paired endpoints");
            *count -= 1;
            if *count == 0 {
                active.remove(&n.track);
            }
        }
        for n in &starts {
            *active.entry(n.track).or_default() += 1;
        }
        if active.len() > 1 {
            for n in starts {
                found.insert(n.id.as_str());
            }
        }
    }
    found
}

fn features(notes: &[&MusicNote], ppq: u16, other: &BTreeSet<&str>) -> VoiceFeatures {
    let count = notes.len();
    let mut intervals = BTreeMap::new();
    let (mut leaps, mut steps, mut repeats, mut connected, mut overlap) = (0_u64, 0, 0, 0, 0);
    let mut onsets = BTreeMap::new();
    for n in notes {
        *onsets.entry(n.start_tick).or_insert(0_usize) += 1;
    }
    for pair in notes.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let leap = a.pitch.value().abs_diff(b.pitch.value());
        leaps += u64::from(leap);
        steps += usize::from(leap <= 2);
        repeats += usize::from(leap == 0);
        connected += usize::from(b.start_tick.saturating_sub(a.end_tick) <= u64::from(ppq));
        overlap += usize::from(b.start_tick < a.end_tick);
        if b.start_tick > a.start_tick {
            *intervals
                .entry(b.start_tick - a.start_tick)
                .or_insert(0_usize) += 1;
        }
    }
    let transitions = count.saturating_sub(1);
    let ratio = |n: usize| (transitions > 0).then(|| n as f64 / transitions as f64);
    let end = notes.iter().map(|n| n.end_tick).max().unwrap();
    let duration: u128 = notes
        .iter()
        .map(|n| u128::from(n.end_tick - n.start_tick))
        .sum();
    VoiceFeatures {
        note_count: count,
        mean_pitch: notes
            .iter()
            .map(|n| f64::from(n.pitch.value()))
            .sum::<f64>()
            / count as f64,
        mean_leap_semitones: (transitions > 0).then(|| leaps as f64 / transitions as f64),
        stepwise_fraction: ratio(steps),
        repeated_pitch_fraction: ratio(repeats),
        mean_duration_quarters: duration as f64 / count as f64 / f64::from(ppq),
        onsets_per_quarter: count as f64 * f64::from(ppq) / (end - notes[0].start_tick) as f64,
        modal_interval_fraction: intervals
            .values()
            .max()
            .map(|v| *v as f64 / intervals.values().sum::<usize>() as f64),
        connected_transition_fraction: ratio(connected),
        simultaneous_onset_fraction: onsets.values().filter(|n| **n > 1).sum::<usize>() as f64
            / count as f64,
        overlapping_transition_fraction: ratio(overlap),
        onset_with_other_track_fraction: notes
            .iter()
            .filter(|n| other.contains(n.id.as_str()))
            .count() as f64
            / count as f64,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ScoreComponent {
    pub name: &'static str,
    pub raw: f64,
    pub weight: f64,
    pub contribution: f64,
}
#[derive(Debug, Clone, Serialize)]
pub struct MelodyScoring {
    pub eligible: bool,
    pub components: Vec<ScoreComponent>,
    pub weighted_sum: f64,
    pub total: f64,
    pub rule: &'static str,
}
#[derive(Debug, Clone, Serialize)]
pub struct TrackRanking {
    pub rank: usize,
    pub track: usize,
    pub score: f64,
    pub best_candidate_id: Option<String>,
    pub scoring: Option<MelodyScoring>,
    pub aggregation: &'static str,
    pub confidence: Confidence,
}
fn melody_score(f: &VoiceFeatures, channel: Option<u8>) -> MelodyScoring {
    let continuity = 1.0 / (1.0 + f.mean_leap_semitones.unwrap_or(12.0) / 5.0);
    let duration = (f.mean_duration_quarters / 0.5).min(1.0);
    let density =
        (f.onsets_per_quarter / 1.0).min(1.0) / (1.0 + (f.onsets_per_quarter - 4.0).max(0.0));
    let components: Vec<_> = [
        ("continuity", continuity, 0.25),
        ("regularity", f.modal_interval_fraction.unwrap_or(0.0), 0.15),
        ("duration", duration, 0.15),
        ("density", density, 0.15),
        (
            "connectivity",
            f.connected_transition_fraction.unwrap_or(0.0),
            0.2,
        ),
        ("stepwise", f.stepwise_fraction.unwrap_or(0.0), 0.1),
        (
            "repetition_penalty",
            f.repeated_pitch_fraction.unwrap_or(0.0),
            -0.2,
        ),
    ]
    .into_iter()
    .map(|(name, raw, weight)| ScoreComponent {
        name,
        raw,
        weight,
        contribution: raw * weight,
    })
    .collect();
    let sum: f64 = components.iter().map(|c| c.contribution).sum();
    let eligible = f.note_count >= 3 && channel != Some(9);
    MelodyScoring {
        eligible,
        components,
        weighted_sum: sum,
        total: if eligible { sum.clamp(0.0, 1.0) } else { 0.0 },
        rule: "clamp weighted sum to [0,1]; fewer than 3 notes or MIDI channel 10 yields 0; uncalibrated",
    }
}

pub(crate) fn rank_tracks(
    doc: &MusicDocument,
    candidates: &[MelodyCandidate],
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<TrackRanking> {
    let mut best = BTreeMap::new();
    for c in candidates {
        best.entry(c.track).or_insert(c);
    }
    let mut result:Vec<_>=doc.tracks.iter().map(|t|{
        let c=best.get(&t.index);
        TrackRanking{rank:0,track:t.index,score:c.map_or(0.0,|c|c.score),best_candidate_id:c.map(|c|c.id.clone()),scoring:c.map(|c|c.scoring.clone()),aggregation:"maximum candidate lane score; empty track = 0; ties use source track index",confidence:Confidence::uncalibrated()}
    }).collect();
    result.sort_by(|a, b| {
        b.score
            .total_cmp(&a.score)
            .then_with(|| a.track.cmp(&b.track))
    });
    for (i, r) in result.iter_mut().enumerate() {
        r.rank = i + 1;
    }
    if result.first().is_none_or(|r| r.score < 0.65)
        || (result.len() > 1 && result[0].score - result[1].score < 0.1)
    {
        diagnostics.push(Diagnostic::analysis("ambiguous_melody","No sufficiently separated track candidate (score >=0.65 and margin >=0.10); review multiple candidates, do not assume a unique melody."));
    }
    result
}

pub(crate) fn estimate(
    doc: &MusicDocument,
    diagnostics: &mut Vec<Diagnostic>,
) -> (Vec<MelodyCandidate>, Vec<TrackRoleEstimate>) {
    let other = other_track_onsets(doc);
    let mut tracks = vec![Vec::new(); doc.tracks.len()];
    for n in &doc.notes {
        tracks[n.track].push(n);
    }
    let mut candidates = Vec::new();
    let mut roles = Vec::new();
    for (track, notes) in tracks.iter_mut().enumerate() {
        notes.sort_by_key(|n| (n.start_tick, n.event_order(), &n.id));
        let mut channels = BTreeMap::<Option<u8>, Vec<&MusicNote>>::new();
        for n in notes.iter() {
            channels.entry(n.channel()).or_default().push(n);
        }
        let mut track_best = 0.0_f64;
        for (channel, channel_notes) in channels {
            // Bounded greedy nonoverlapping voices, by smallest pitch movement,
            // then shortest gap and stable lane index. Does not claim source voices.
            let mut lanes: Vec<Vec<&MusicNote>> = Vec::new();
            let mut skipped = 0;
            for n in channel_notes {
                let lane = lanes
                    .iter()
                    .enumerate()
                    .filter(|(_, v)| v.last().unwrap().end_tick <= n.start_tick)
                    .min_by_key(|(i, v)| {
                        let last = v.last().unwrap();
                        (
                            last.pitch.value().abs_diff(n.pitch.value()),
                            n.start_tick - last.end_tick,
                            *i,
                        )
                    })
                    .map(|(i, _)| i);
                if let Some(i) = lane {
                    lanes[i].push(n);
                } else if lanes.len() < 32 {
                    lanes.push(vec![n]);
                } else {
                    skipped += 1;
                }
            }
            if skipped > 0 {
                diagnostics.push(Diagnostic::analysis("voice_search_limit",&format!("track {track} channel {channel:?}: {skipped} notes outside the 32 simultaneous candidate lanes; all remain in source and statistics")));
            }
            for (voice_index, lane) in lanes.into_iter().enumerate() {
                let f = features(&lane, doc.ppq, &other);
                let scoring = melody_score(&f, channel);
                let score = scoring.total;
                track_best = track_best.max(score);
                let reasons = vec![
                    format!(
                        "{} notes; mean leap {:?} semitones; connected fraction {:?}",
                        f.note_count, f.mean_leap_semitones, f.connected_transition_fraction
                    ),
                    format!(
                        "{:.3} onsets/quarter; mean duration {:.3} quarters; modal interval fraction {:?}",
                        f.onsets_per_quarter, f.mean_duration_quarters, f.modal_interval_fraction
                    ),
                    format!(
                        "{:.3} onsets coincide with another active track; repeated pitch fraction {:?}",
                        f.onset_with_other_track_fraction, f.repeated_pitch_fraction
                    ),
                ];
                candidates.push(MelodyCandidate {id:format!("track-{track}-channel-{channel:?}-voice-{voice_index}"),track,channel,voice_index,score,scoring,
                    confidence:Confidence::uncalibrated(),features:f,evidence:Evidence::inferred(doc,"bounded nearest-continuation voice lanes + weighted continuity/rhythm/duration/activity score v1",&lane),reasons,
                    possible_errors:vec!["Arpeggios, bass ostinatos or inner voices can outrank melody.","Greedy lanes can switch voices at crossings; source voice identity is unknown.","Channel 10 is treated as a percussion hint, not proof; gaps are continuity evidence, not phrase segmentation."]});
            }
        }
        let f = (!notes.is_empty()).then(|| features(notes, doc.ppq, &other));
        let mut hypotheses = vec![RoleCandidate {
            role: TrackRole::Unknown,
            score: 0.0,
            reasons: vec!["A track can contain multiple roles; no calibrated classifier.".into()],
        }];
        if let Some(f) = &f
            && f.note_count >= 3
        {
            if f.mean_pitch < f64::from(crate::LOW_REGISTER_END)
                && f.simultaneous_onset_fraction < 0.25
                && f.connected_transition_fraction.unwrap_or(0.0) > 0.5
            {
                hypotheses.push(RoleCandidate {
                    role: TrackRole::Bass,
                    score: 0.8,
                    reasons: vec![format!(
                        "mean pitch {:.2} below 55; mostly sequential and connected",
                        f.mean_pitch
                    )],
                });
            }
            if f.simultaneous_onset_fraction >= 0.5 {
                hypotheses.push(RoleCandidate {
                    role: TrackRole::Harmony,
                    score: 0.8,
                    reasons: vec![format!(
                        "{:.2} of notes share onsets; chordal behavior",
                        f.simultaneous_onset_fraction
                    )],
                });
            }
            if f.repeated_pitch_fraction.unwrap_or(0.0) >= 0.6
                && f.modal_interval_fraction.unwrap_or(0.0) >= 0.7
            {
                let role = if notes.iter().all(|n| n.channel() == Some(9)) {
                    TrackRole::Rhythm
                } else {
                    TrackRole::Accompaniment
                };
                hypotheses.push(RoleCandidate {role,score:0.8,reasons:vec!["repeated pitch and regular onsets; channel 10 additionally supports rhythm".into()]});
            }
            if track_best >= 0.65
                && f.simultaneous_onset_fraction < 0.25
                && f.mean_pitch >= f64::from(crate::LOW_REGISTER_END)
                && f.mean_leap_semitones.is_some_and(|v| v <= 12.0)
            {
                hypotheses.push(RoleCandidate {role:TrackRole::Melody,score:track_best,reasons:vec![format!("voice score {track_best:.3}; mostly sequential; not in low-register bass heuristic")]});
            }
        }
        hypotheses.sort_by(|a, b| b.score.total_cmp(&a.score));
        let role = if hypotheses[0].score >= 0.65
            && (hypotheses.len() == 1 || hypotheses[0].score - hypotheses[1].score >= 0.1)
        {
            hypotheses[0].role
        } else {
            TrackRole::Unknown
        };
        if role == TrackRole::Unknown {
            diagnostics.push(Diagnostic::analysis("uncertain_track_role",&format!("track {track}: sparse, conflicting or insufficient role evidence; inspect candidate reasons")));
        }
        if hypotheses.iter().any(|c| c.role == TrackRole::Bass)
            && hypotheses
                .iter()
                .any(|c| matches!(c.role, TrackRole::Accompaniment | TrackRole::Rhythm))
        {
            diagnostics.push(Diagnostic::analysis("bass_accompaniment_ambiguity",&format!("track {track}: low register and repeated rhythmic behavior support competing roles")));
        }
        roles.push(TrackRoleEstimate {track,role,candidates:hypotheses,confidence:Confidence::uncalibrated(),evidence:Evidence::inferred(doc,"register, chordal activity, repetition, continuity and voice-score rules v1",notes),features:f,
            possible_errors:vec!["Mixed roles, sparse notes and competing hypotheses remain unknown.","Register and channel conventions can mislead; names and program numbers do not determine roles."]});
    }
    candidates.sort_by(|a, b| b.score.total_cmp(&a.score).then_with(|| a.id.cmp(&b.id)));
    (candidates, roles)
}

/// Return references to a candidate's original events; never creates a Score.
pub fn extract_melody<'a>(
    doc: &'a MusicDocument,
    candidate: &MelodyCandidate,
) -> Result<Vec<&'a MusicNote>, MusicError> {
    if candidate.evidence.source_document_id != doc.id {
        return Err(MusicError("candidate belongs to another document".into()));
    }
    let by_id: BTreeMap<_, _> = doc.notes.iter().map(|n| (n.id.as_str(), n)).collect();
    candidate
        .evidence
        .note_ids
        .iter()
        .map(|id| {
            by_id
                .get(id.as_str())
                .copied()
                .ok_or_else(|| MusicError(format!("candidate source note {id} no longer exists")))
        })
        .collect()
}
