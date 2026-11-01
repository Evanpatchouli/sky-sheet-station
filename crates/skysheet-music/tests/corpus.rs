use serde::Deserialize;
use skysheet_music::*;

#[derive(Deserialize)]
struct Corpus {
    cases: Vec<Case>,
}
#[derive(Deserialize)]
struct Case {
    id: String,
    notes: Vec<Vec<[u64; 5]>>,
    count: usize,
    polyphony: usize,
    roles: Vec<String>,
    melody_tracks: Vec<usize>,
    unique_melody: bool,
    tempos: Vec<[u64; 2]>,
    meters: Vec<[u64; 3]>,
}
fn cases() -> Vec<Case> {
    serde_json::from_str::<Corpus>(include_str!("fixtures/cases.json"))
        .unwrap()
        .cases
}
fn document(case: &Case) -> MusicDocument {
    MusicDocument {
        id: case.id.clone(),
        source_name: Some(case.id.clone()),
        source_format: "synthetic-owned-events".into(),
        ppq: 480,
        tracks: (0..case.notes.len())
            .map(|index| MusicTrack { index, name: None })
            .collect(),
        notes: case
            .notes
            .iter()
            .enumerate()
            .flat_map(|(track, notes)| {
                notes.iter().enumerate().map(move |(i, n)| MusicNote {
                    id: format!("{track}-{i}"),
                    start_tick: n[0],
                    end_tick: n[1],
                    pitch: MidiPitch::new(n[2] as u8).unwrap(),
                    track,
                    midi: Some(MidiNoteSource {
                        velocity: n[3] as u8,
                        channel: n[4] as u8,
                        on_event: 2 * i,
                        off_event: 2 * i + 1,
                    }),
                    diagnostics: Vec::new(),
                })
            })
            .collect(),
        tempo_map: case
            .tempos
            .iter()
            .enumerate()
            .map(|(i, t)| TempoEvent {
                location: EventLocation {
                    track: 0,
                    event: 100 + i,
                    tick: t[0],
                },
                microseconds_per_quarter: t[1] as u32,
            })
            .collect(),
        time_signature_map: case
            .meters
            .iter()
            .enumerate()
            .map(|(i, m)| MeterEvent {
                location: EventLocation {
                    track: 0,
                    event: 200 + i,
                    tick: m[0],
                },
                numerator: m[1] as u8,
                denominator_power: m[2] as u8,
                clocks_per_click: 24,
                thirty_seconds_per_quarter: 8,
            })
            .collect(),
        metadata: Vec::new(),
        annotations: Vec::new(),
        analysis_refs: Vec::new(),
        diagnostics: Vec::new(),
        original_midi: None,
    }
}

#[test]
fn fixed_corpus_statistics_preservation_and_traceability() {
    for case in cases() {
        let doc = document(&case);
        let before = doc.clone();
        let r = analyze(&doc).unwrap();
        assert_eq!(doc, before, "{} input mutation", case.id);
        assert_eq!(r.source, before, "{} report facts", case.id);
        assert_eq!(r.note_statistics.total_notes, case.count, "{}", case.id);
        assert_eq!(
            r.note_statistics.max_polyphony, case.polyphony,
            "{}",
            case.id
        );
        assert_eq!(r.track_roles.len(), case.notes.len());
        assert!(r.confidence.probability.is_none());
        for c in &r.melody_candidates {
            let extracted = extract_melody(&doc, c).unwrap();
            assert!(
                extracted
                    .windows(2)
                    .all(|w| w[0].end_tick <= w[1].start_tick)
            );
            assert!(
                extracted
                    .iter()
                    .all(|n| std::ptr::eq(*n, doc.notes.iter().find(|x| x.id == n.id).unwrap()))
            );
            assert_eq!(extracted.len(), c.features.note_count);
            assert!(c.confidence.probability.is_none());
        }
        let first = serde_json::to_string(&r).unwrap();
        assert_eq!(
            first,
            serde_json::to_string(&analyze(&doc).unwrap()).unwrap()
        );
    }
}

#[test]
fn independent_hand_calculated_statistics() {
    let corpus = cases();
    let mono = analyze(&document(&corpus[0])).unwrap();
    assert_eq!(
        mono.note_statistics.duration_ticks_distribution,
        [(480, 8)].into()
    );
    assert_eq!(
        mono.note_statistics.polyphony_ticks_distribution,
        [(1, 3840)].into()
    );
    assert_eq!(mono.note_statistics.overlapping_note_pairs, 0);
    assert_eq!(
        mono.rhythm_analysis.inter_onset_ticks_distribution,
        [(480, 7)].into()
    );
    assert_eq!(mono.rhythm_analysis.near_nominal_downbeat_onsets, Some(2));
    assert_eq!(mono.rhythm_analysis.repeated_patterns[0].occurrences, 5);
    assert_eq!(
        mono.pitch_statistics.pitch_class_counts,
        [2, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0, 1]
    );
    assert_eq!(
        mono.tempo_analysis.effective_map[0].origin,
        Origin::ProtocolDefault
    );
    let chord = analyze(&document(&corpus[1])).unwrap();
    assert_eq!(chord.note_statistics.overlapping_note_pairs, 3);
    assert_eq!(
        chord.note_statistics.polyphony_ticks_distribution,
        [(3, 960)].into()
    );
    assert_eq!(chord.rhythm_analysis.unique_onsets, 1);
    assert!(chord.rhythm_analysis.beat_candidates.is_empty());
    let tempo = analyze(&document(&corpus[5])).unwrap();
    assert_eq!(tempo.tempo_analysis.change_positions, [960, 1440]);
    assert_eq!(
        tempo.tempo_analysis.principal_microseconds_per_quarter,
        Some(500000)
    );
    assert!(!tempo.tempo_analysis.uses_protocol_default);
    let meter = analyze(&document(&corpus[6])).unwrap();
    assert_eq!(meter.rhythm_analysis.near_nominal_downbeat_onsets, Some(3));
    let wide = analyze(&document(&corpus[8])).unwrap();
    assert_eq!(wide.note_statistics.lowest_pitch, Some(0));
    assert_eq!(wide.note_statistics.highest_pitch, Some(127));
    let ambiguous = analyze(&document(&corpus[11])).unwrap();
    assert_eq!(ambiguous.key_candidates.status, "uncertain");
    assert!(
        ambiguous
            .key_candidates
            .candidates
            .iter()
            .any(|c| c.tonic_pitch_class == 0 && c.mode == "major")
    );
    assert!(
        ambiguous
            .key_candidates
            .candidates
            .iter()
            .any(|c| c.tonic_pitch_class == 9 && c.mode == "natural_minor")
    );
    assert!(ambiguous.key_candidates.selected_key.is_none());
}

#[test]
fn evaluation_against_fixed_design_intent_references() {
    // Report measurements honestly: inference quality is not an invariant and
    // these agent-authored labels are not independently human-validated truth.
    let (mut roles, mut correct, mut unique, mut top1, mut top3) = (0, 0, 0, 0, 0);
    let started = std::time::Instant::now();
    for case in cases() {
        let r = analyze(&document(&case)).unwrap();
        let actual: Vec<_> = r
            .track_roles
            .iter()
            .map(|r| {
                serde_json::to_value(r.role)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        roles += case.roles.len();
        correct += actual
            .iter()
            .zip(&case.roles)
            .filter(|(a, b)| a == b)
            .count();
        let hit = |k| {
            r.melody_candidates
                .iter()
                .take(k)
                .any(|c| c.score >= 0.65 && case.melody_tracks.contains(&c.track))
        };
        if case.unique_melody {
            unique += 1;
            top1 += usize::from(hit(1));
            top3 += usize::from(hit(3));
        }
        println!(
            "{}: roles {:?}, references {:?}, top1 {}, top3 {}, unique_reference {}, high_score_tracks {:?}",
            case.id,
            actual,
            case.roles,
            hit(1),
            hit(3),
            case.unique_melody,
            r.melody_candidates
                .iter()
                .filter(|c| c.score >= 0.65)
                .map(|c| c.track)
                .collect::<Vec<_>>()
        );
    }
    println!(
        "EVALUATION role_accuracy={correct}/{roles}; unique_melody_top1={top1}/{unique}; unique_melody_top3={top3}/{unique}; corpus_elapsed_us={}; human_review=pending",
        started.elapsed().as_micros()
    );
}

#[test]
fn empty_sparse_extreme_ticks_and_invalid_documents() {
    let mut doc = document(&cases()[0]);
    doc.notes.clear();
    let r = analyze(&doc).unwrap();
    assert_eq!(r.key_candidates.status, "unknown");
    assert!(r.note_statistics.lowest_pitch.is_none());
    assert!(r.tempo_analysis.principal_bpm.is_none());
    assert_eq!(r.track_roles[0].role, TrackRole::Unknown);
    doc = document(&cases()[1]);
    for n in &mut doc.notes {
        n.start_tick = 0;
        n.end_tick = u64::MAX;
    }
    let r = analyze(&doc).unwrap();
    assert_eq!(r.note_statistics.polyphony_ticks_distribution[&3], u64::MAX);
    serde_json::to_value(&r).unwrap();
    doc.notes[1].pitch = doc.notes[0].pitch;
    let r = analyze(&doc).unwrap();
    assert_eq!(
        r.pitch_statistics.pitch_class_duration_ticks[0],
        2 * u128::from(u64::MAX)
    );
    assert_eq!(
        serde_json::to_value(&r).unwrap()["pitch_statistics"]["pitch_class_duration_ticks"][0],
        "36893488147419103230"
    );
    doc.notes[0].end_tick = 0;
    assert!(analyze(&doc).is_err());
    doc = document(&cases()[0]);
    doc.ppq = 0;
    assert!(analyze(&doc).is_err());
    assert!(MidiPitch::new(128).is_err());
    doc = document(&cases()[0]);
    doc.notes[1].id = doc.notes[0].id.clone();
    assert!(analyze(&doc).is_err());
}

#[test]
fn map_conflicts_and_voice_budget_are_explicit() {
    let mut doc = document(&cases()[5]);
    doc.tempo_map[1].location.tick = 0;
    assert!(
        analyze(&doc)
            .unwrap_err()
            .to_string()
            .contains("tempo conflict")
    );
    doc = document(&cases()[6]);
    doc.time_signature_map[1].location.tick = 0;
    assert!(analyze(&doc).is_err());
    doc = document(&cases()[1]);
    let template = doc.notes[0].clone();
    doc.notes = (0..40)
        .map(|i| MusicNote {
            id: format!("n-{i}"),
            midi: Some(MidiNoteSource {
                velocity: 80,
                channel: 0,
                on_event: i,
                off_event: 40 + i,
            }),
            ..template.clone()
        })
        .collect();
    let r = analyze(&doc).unwrap();
    assert_eq!(r.note_statistics.total_notes, 40);
    assert_eq!(r.melody_candidates.len(), 32);
    assert!(r.diagnostics.iter().any(|d| d.code == "voice_search_limit"));
    let mut candidate = r.melody_candidates[0].clone();
    candidate.evidence.source_document_id = "another".into();
    assert!(extract_melody(&doc, &candidate).is_err());
}

#[test]
fn polyphony_sweep_matches_independent_bounded_tick_and_pair_oracles() {
    let mut doc = document(&cases()[0]);
    // Fixed combinatorial intervals, checked by exhaustive sampling of every
    // integer cell and every unordered pair, independent of the sweep algorithm.
    let template = doc.notes[0].clone();
    doc.notes = (0..64)
        .map(|i| MusicNote {
            id: format!("oracle-{i}"),
            midi: None,
            start_tick: (i * 17 % 101) as u64,
            end_tick: (i * 17 % 101 + 1 + i % 19) as u64,
            ..template.clone()
        })
        .collect();
    let horizon = doc.notes.iter().map(|n| n.end_tick).max().unwrap();
    let mut distribution = std::collections::BTreeMap::new();
    for tick in 0..horizon {
        let active = doc
            .notes
            .iter()
            .filter(|n| n.start_tick <= tick && tick < n.end_tick)
            .count();
        *distribution.entry(active).or_insert(0_u64) += 1;
    }
    let mut pairs = 0;
    for (i, a) in doc.notes.iter().enumerate() {
        for b in &doc.notes[i + 1..] {
            pairs += u64::from(a.start_tick < b.end_tick && b.start_tick < a.end_tick);
        }
    }
    let report = analyze(&doc).unwrap();
    assert_eq!(
        report.note_statistics.polyphony_ticks_distribution,
        distribution
    );
    assert_eq!(report.note_statistics.overlapping_note_pairs, pairs);
}

#[test]
fn source_maps_remain_raw_while_defaults_and_duplicate_events_are_derived() {
    let mut doc = document(&cases()[0]);
    doc.tempo_map = vec![TempoEvent {
        location: EventLocation {
            track: 0,
            event: 100,
            tick: 960,
        },
        microseconds_per_quarter: 1_000_000,
    }];
    let mut duplicate = doc.tempo_map[0].clone();
    duplicate.location.event = 101;
    doc.tempo_map.push(duplicate);
    let report = analyze(&doc).unwrap();
    assert_eq!(report.source.tempo_map, doc.tempo_map);
    assert_eq!(report.tempo_analysis.change_positions, [960]);
    assert_eq!(report.tempo_analysis.effective_map.len(), 2);
    assert_eq!(
        report.tempo_analysis.effective_map[0].origin,
        Origin::ProtocolDefault
    );
    assert_eq!(
        report.tempo_analysis.effective_map[1].source_events.len(),
        2
    );
    assert_eq!(
        report.tempo_analysis.principal_microseconds_per_quarter,
        Some(1_000_000)
    );
    doc.time_signature_map = vec![MeterEvent {
        location: EventLocation {
            track: 0,
            event: 102,
            tick: 0,
        },
        numerator: 3,
        denominator_power: 128,
        clocks_per_click: 24,
        thirty_seconds_per_quarter: 8,
    }];
    let report = analyze(&doc).unwrap();
    assert!(
        report
            .rhythm_analysis
            .near_nominal_downbeat_onsets
            .is_none()
    );
    assert_eq!(report.source.time_signature_map, doc.time_signature_map);
}

#[test]
fn analysis_budget_rejects_without_silent_sampling() {
    let mut doc = document(&cases()[0]);
    doc.notes = vec![doc.notes[0].clone(); 100_001];
    assert!(
        analyze(&doc)
            .unwrap_err()
            .to_string()
            .contains("analysis limit")
    );
    assert_eq!(doc.notes.len(), 100_001);
}

#[test]
fn generic_notes_need_no_midi_provenance_and_pitch_helpers_are_exact() {
    let mut doc = document(&cases()[0]);
    for n in &mut doc.notes {
        n.midi = None;
    }
    let r = analyze(&doc).unwrap();
    assert!(r.source.notes.iter().all(|n| n.midi.is_none()));
    assert_eq!(r.note_statistics.total_notes, 8);
    for (pitch, name, octave, class) in [
        (0, "C-1", -1, 0),
        (60, "C4", 4, 0),
        (61, "C#4", 4, 1),
        (127, "G9", 9, 7),
    ] {
        let p = MidiPitch::new(pitch).unwrap();
        assert_eq!(p.name(), name);
        assert_eq!(p.octave(), octave);
        assert_eq!(p.pitch_class(), class);
    }
    assert_eq!(
        MidiPitch::new(60)
            .unwrap()
            .semitones_to(MidiPitch::new(43).unwrap()),
        -17
    );
}

#[test]
fn component_arithmetic_track_ranking_ties_and_unknown_are_asserted() {
    let doc = document(&cases()[0]);
    let r = analyze(&doc).unwrap();
    let score = &r.track_ranking[0].scoring.as_ref().unwrap();
    // Ascending C scale: mean leap 12/7; continuity 35/47; all other
    // positive components 1, repetition 0. Total = 44/47, independent arithmetic.
    assert!((score.components[0].raw - 35.0 / 47.0).abs() < 1e-12);
    let weights = [0.25, 0.15, 0.15, 0.15, 0.2, 0.1, -0.2];
    for (c, w) in score.components.iter().zip(weights) {
        assert_eq!(c.weight, w);
        assert!((c.contribution - c.raw * w).abs() < 1e-12);
    }
    assert!((score.total - 44.0 / 47.0).abs() < 1e-12);
    assert!((score.weighted_sum - score.total).abs() < 1e-12);
    let ties = analyze(&document(&cases()[4])).unwrap();
    assert_eq!(
        ties.track_ranking
            .iter()
            .map(|r| r.track)
            .collect::<Vec<_>>(),
        [0, 1]
    );
    assert_eq!(ties.track_ranking[0].score, ties.track_ranking[1].score);
    assert!(
        ties.diagnostics
            .iter()
            .any(|d| d.code == "ambiguous_melody")
    );
    for index in [2, 8] {
        assert_eq!(
            analyze(&document(&cases()[index])).unwrap().track_roles[0].role,
            TrackRole::Unknown
        );
    }
    let mut sparse = doc.clone();
    sparse.notes.truncate(2);
    let sparse = analyze(&sparse).unwrap();
    assert_eq!(sparse.track_ranking[0].score, 0.0);
    assert_eq!(sparse.track_roles[0].role, TrackRole::Unknown);
    assert!(!sparse.track_ranking[0].scoring.as_ref().unwrap().eligible);
    assert!(sparse.diagnostics.iter().any(|d| d.code == "sparse_track"));
}

#[test]
fn hand_calculated_registers_track_activity_groups_overlap_and_positions() {
    let case = Case {
        id: "structural".into(),
        notes: vec![
            vec![
                [0, 960, 48, 80, 0],
                [480, 1440, 60, 80, 0],
                [1920, 2400, 60, 80, 0],
            ],
            vec![[480, 960, 55, 80, 1], [960, 1920, 67, 80, 1]],
        ],
        count: 5,
        polyphony: 3,
        roles: vec![],
        melody_tracks: vec![],
        unique_melody: false,
        tempos: vec![],
        meters: vec![],
    };
    let r = analyze(&document(&case)).unwrap();
    let s = &r.structure_analysis;
    assert_eq!(
        (s.registers.low, s.registers.middle, s.registers.high),
        (1, 4, 0)
    );
    assert_eq!(s.midi_pitch_distribution[60], 2);
    assert_eq!(s.tracks[0].active_ticks, 1920);
    assert_eq!(
        s.tracks[0]
            .activity_intervals
            .iter()
            .map(|i| (i.start_tick, i.end_tick))
            .collect::<Vec<_>>(),
        [(0, 1440), (1920, 2400)]
    );
    assert_eq!(s.track_pairs[0].overlap_ticks, 960);
    assert_eq!(s.track_pairs[0].simultaneous_onsets, 1);
    assert_eq!(s.track_pairs[0].onset_jaccard, Some(0.25));
    let range = s.track_pairs[0].pitch_range_intersection.as_ref().unwrap();
    assert_eq!((range.lowest, range.highest), (55, 60));
    assert_eq!(s.onset_groups[1].note_ids.len(), 2);
    assert_eq!(s.repeated_pitches.len(), 1);
    assert_eq!(s.repeated_pitches[0].pitch, 60);
    assert!(
        s.sounding_intervals
            .iter()
            .any(|i| i.start_tick == 480 && i.end_tick == 960 && i.active_notes == 3)
    );
    assert_eq!(
        s.onset_groups.last().unwrap().position.bar.as_deref(),
        Some("2")
    );
    let mut doc = document(&cases()[0]);
    doc.notes.truncate(4);
    doc.notes[1].start_tick = 500;
    doc.time_signature_map = vec![MeterEvent {
        location: EventLocation {
            track: 0,
            event: 100,
            tick: 960,
        },
        numerator: 3,
        denominator_power: 3,
        clocks_per_click: 24,
        thirty_seconds_per_quarter: 8,
    }];
    let r = analyze(&doc).unwrap();
    let g = &r.structure_analysis.onset_groups;
    assert_eq!(
        (
            g[1].position.beat,
            g[1].position.beat_fraction_numerator,
            g[1].position.beat_fraction_denominator
        ),
        (Some(2), Some(80), Some(1920))
    );
    assert_eq!(
        (g[2].position.bar.as_deref(), g[2].position.beat),
        (Some("2"), Some(1))
    );
    assert_eq!(
        (g[3].position.bar.as_deref(), g[3].position.beat),
        (Some("2"), Some(3))
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "partial_bar_at_meter_change")
    );
}

#[test]
fn repeated_meter_does_not_shift_nominal_bars_or_downbeats() {
    let mut doc = document(&cases()[0]);
    doc.time_signature_map = vec![MeterEvent {
        location: EventLocation {
            track: 0,
            event: 100,
            tick: 500,
        },
        numerator: 4,
        denominator_power: 2,
        clocks_per_click: 36,
        thirty_seconds_per_quarter: 8,
    }];
    let r = analyze(&doc).unwrap();
    assert_eq!(r.structure_analysis.meter_segments.len(), 1);
    assert_eq!(r.source.time_signature_map, doc.time_signature_map);
    let at1920 = r
        .structure_analysis
        .onset_groups
        .iter()
        .find(|g| g.tick == 1920)
        .unwrap();
    assert_eq!(
        (at1920.position.bar.as_deref(), at1920.position.beat),
        (Some("2"), Some(1))
    );
    assert_eq!(r.rhythm_analysis.near_nominal_downbeat_onsets, Some(2));
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.code == "partial_bar_at_meter_change")
    );
}
