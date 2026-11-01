use crate::{Confidence, Evidence, MusicDocument};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PitchStatistics {
    pub pitch_class_counts: [usize; 12],
    /// Decimal strings in JSON: totals can exceed u64 and JSON exact integers.
    #[serde(serialize_with = "serialize_tick_totals")]
    pub pitch_class_duration_ticks: [u128; 12],
    pub mean_midi_pitch: Option<f64>,
    pub duration_weighted_mean_midi_pitch: Option<f64>,
    pub principal_pitch_classes: Vec<u8>,
}
fn serialize_tick_totals<S: serde::Serializer>(
    values: &[u128; 12],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    values.map(|v| v.to_string()).serialize(serializer)
}
pub(crate) fn pitches(doc: &MusicDocument) -> PitchStatistics {
    let mut counts = [0; 12];
    let mut durations = [0_u128; 12];
    let (mut sum, mut weighted, mut total) = (0_u64, 0_u128, 0_u128);
    for n in &doc.notes {
        let pitch = n.pitch.value();
        let duration = u128::from(n.end_tick - n.start_tick);
        counts[usize::from(pitch % 12)] += 1;
        durations[usize::from(pitch % 12)] += duration;
        sum += u64::from(pitch);
        weighted += u128::from(pitch) * duration;
        total += duration;
    }
    let mut ranked: Vec<u8> = (0..12).filter(|p| counts[usize::from(*p)] > 0).collect();
    ranked.sort_by_key(|p| (std::cmp::Reverse(durations[usize::from(*p)]), *p));
    ranked.truncate(3);
    PitchStatistics {
        pitch_class_counts: counts,
        pitch_class_duration_ticks: durations,
        mean_midi_pitch: (!doc.notes.is_empty()).then(|| sum as f64 / doc.notes.len() as f64),
        duration_weighted_mean_midi_pitch: (total > 0).then(|| weighted as f64 / total as f64),
        principal_pitch_classes: ranked,
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct KeyCandidate {
    pub tonic_pitch_class: u8,
    pub mode: &'static str,
    pub in_scale_duration_fraction: f64,
    pub confidence: Confidence,
}
#[derive(Debug, Clone, Serialize)]
pub struct KeyAnalysis {
    pub status: &'static str,
    pub candidates: Vec<KeyCandidate>,
    pub selected_key: Option<String>,
    pub evidence: Evidence,
    pub limitations: Vec<&'static str>,
}
pub(crate) fn keys(doc: &MusicDocument, stats: &PitchStatistics) -> KeyAnalysis {
    let total: u128 = stats.pitch_class_duration_ticks.iter().sum();
    let distinct = stats.pitch_class_counts.iter().filter(|c| **c > 0).count();
    let mut candidates = Vec::new();
    if total > 0 && distinct >= 3 {
        for tonic in 0..12_u8 {
            for (mode, steps) in [
                ("major", [0, 2, 4, 5, 7, 9, 11]),
                ("natural_minor", [0, 2, 3, 5, 7, 8, 10]),
            ] {
                let inside: u128 = steps
                    .iter()
                    .map(|s| stats.pitch_class_duration_ticks[usize::from((tonic + s) % 12)])
                    .sum();
                candidates.push(KeyCandidate {
                    tonic_pitch_class: tonic,
                    mode,
                    in_scale_duration_fraction: inside as f64 / total as f64,
                    confidence: Confidence::uncalibrated(),
                });
            }
        }
        candidates.sort_by(|a, b| {
            b.in_scale_duration_fraction
                .total_cmp(&a.in_scale_duration_fraction)
                .then_with(|| a.tonic_pitch_class.cmp(&b.tonic_pitch_class))
                .then_with(|| a.mode.cmp(b.mode))
        });
        let best = candidates[0].in_scale_duration_fraction;
        candidates.retain(|c| {
            c.in_scale_duration_fraction >= 0.75 && best - c.in_scale_duration_fraction <= 0.05
        });
    }
    KeyAnalysis {
        status: if candidates.is_empty() {
            "unknown"
        } else {
            "uncertain"
        },
        candidates,
        selected_key: None,
        evidence: Evidence::inferred(
            doc,
            "duration-weighted diatonic membership; retain within 0.05 of best and >=0.75 coverage",
            &doc.notes.iter().collect::<Vec<_>>(),
        ),
        limitations: vec![
            "Scale membership cannot identify tonic or distinguish relative major/minor.",
            "No cadence, modulation or modal model; short, chromatic or ambiguous sources may remain unknown.",
            "Duration weighting uses source ticks, not sounding seconds or velocity.",
        ],
    }
}
