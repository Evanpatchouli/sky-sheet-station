use crate::search_tests::document;
use crate::{ArrangementMode, ArrangementOptions, TransposeMode, arrange_with_options};
use skysheet_midi::project_music;

#[test]
fn contour_octaves_boundaries_and_no_pitch_class_snapping() {
    for pitches in [
        vec![36, 38, 40, 43],
        vec![103, 100, 98, 96],
        vec![60, 62, 64, 67],
        vec![72, 72, 84, 72],
        vec![0, 60, 127],
    ] {
        let doc = document(&pitches);
        let before = doc.clone();
        let p = project_music(&doc).unwrap();
        let r = arrange_with_options(
            &doc,
            &p,
            TransposeMode::Fixed(0),
            &ArrangementOptions::default(),
        )
        .unwrap();
        assert_eq!(r.report.kept, pitches.len());
        for (m, t) in r.report.note_mappings.iter().zip(p.notes()) {
            assert_eq!((m.target_midi - i16::from(m.source_midi)) % 12, 0);
            assert_eq!(
                (m.projected_start_tick, m.projected_duration_ticks),
                (t.start_tick, t.duration_ticks)
            );
            assert!(
                r.score
                    .instrument
                    .keys
                    .iter()
                    .any(|k| i16::from(k.midi_number()) == m.target_midi)
            );
        }
        if pitches[0] == 36 || pitches[0] == 103 || pitches[0] == 60 {
            assert!(
                r.report
                    .note_mappings
                    .iter()
                    .all(|m| m.previous_interval_change.is_none_or(|c| c == 0))
            );
        }
        assert_eq!(r.score.timeline, *p.timeline());
        assert_eq!(doc, before);
    }
    let doc = document(&[60, 61, 63, 66]);
    let p = project_music(&doc).unwrap();
    let r = arrange_with_options(
        &doc,
        &p,
        TransposeMode::Fixed(0),
        &ArrangementOptions::default(),
    )
    .unwrap();
    assert_eq!(r.report.kept, 1);
    assert_eq!(r.report.dropped, 3);
    assert!(
        arrange_with_options(
            &doc,
            &p,
            TransposeMode::Fixed(127),
            &ArrangementOptions::default()
        )
        .is_err()
    );
}

#[test]
fn triads_inversions_repeats_and_interval_overlaps() {
    for pitches in [vec![48, 52, 55], vec![52, 55, 60], vec![55, 60, 64]] {
        let mut doc = document(&pitches);
        for n in &mut doc.notes {
            n.start_tick = 0;
            n.end_tick = 960;
        }
        let r = arrange_with_options(
            &doc,
            &project_music(&doc).unwrap(),
            TransposeMode::Fixed(0),
            &ArrangementOptions::default(),
        )
        .unwrap();
        assert_eq!(r.report.kept, 3);
        assert_eq!(r.report.playability.maximum_simultaneous_notes, 3);
        assert!(
            r.report.onset_groups[0]
                .chord_candidate
                .as_ref()
                .unwrap()
                .contains("major")
        );
        assert!(!r.report.onset_groups[0].exact_intervals_changed);
    }
    let mut doc = document(&[60, 60, 60]);
    doc.notes[0].end_tick = 1920;
    doc.notes[1].start_tick = 0;
    doc.notes[1].end_tick = 960;
    doc.notes[2].start_tick = 480;
    doc.notes[2].end_tick = 1440;
    let r = arrange_with_options(
        &doc,
        &project_music(&doc).unwrap(),
        TransposeMode::Fixed(0),
        &ArrangementOptions::default(),
    )
    .unwrap();
    assert_eq!(r.report.kept, 3);
    assert_eq!(r.report.onset_groups.len(), 2);
    assert_eq!(r.report.playability.same_key_overlap_events, 2);
    assert_eq!(r.report.playability.maximum_simultaneous_notes, 3);
    let o = ArrangementOptions {
        maximum_polyphony: Some(1),
        ..Default::default()
    };
    let limited = arrange_with_options(
        &doc,
        &project_music(&doc).unwrap(),
        TransposeMode::Fixed(0),
        &o,
    )
    .unwrap();
    assert_eq!(limited.report.kept, 1);
    assert_eq!(limited.report.dropped, 2);
    assert_eq!(limited.score.tracks[0].notes[0].duration_ticks, 1920);
}

#[test]
fn real_ode_all_modes_and_legacy_octave_disable() {
    let raw = include_bytes!("../../../audios/ode-to-joy.mid");
    let doc = skysheet_midi::read_music(raw, Some("ode".into())).unwrap();
    let before = doc.clone();
    let p = project_music(&doc).unwrap();
    for mode in [
        ArrangementMode::MelodyFirst,
        ArrangementMode::SimpleAccompaniment,
        ArrangementMode::Polyphonic,
    ] {
        let o = ArrangementOptions {
            mode,
            ..Default::default()
        };
        let a = arrange_with_options(&doc, &p, TransposeMode::Auto, &o).unwrap();
        let b = arrange_with_options(&doc, &p, TransposeMode::Auto, &o).unwrap();
        assert_eq!(
            serde_json::to_vec(&a.report).unwrap(),
            serde_json::to_vec(&b.report).unwrap()
        );
        assert_eq!(a.report.original_notes, 52);
        assert_eq!(a.report.note_mappings.len(), 52);
        assert_eq!(a.report.role_retention[0].kept, 30);
        assert_eq!(a.report.playability.melody_interval_changes, 0);
        assert_eq!(a.report.playability.cross_track_key_conflicts, 0);
        assert_eq!(a.report.playability.bass_melody_crossing_events, 0);
        assert_eq!(a.report.tracks[2].final_lowest_midi, Some(60));
        assert_eq!(a.report.tracks[2].final_highest_midi, Some(67));
        assert_eq!(a.report.tracks[1].adjusted, 0);
        assert_eq!(a.report.tracks[2].adjusted, 22);
        assert_eq!(a.report.kept, 52);
        assert_eq!(a.report.dropped, 0);
        a.score.validate().unwrap();
        assert_eq!(a.score.timeline, *p.timeline());
    }
    let o = ArrangementOptions {
        allow_octave_shifts: false,
        ..Default::default()
    };
    assert_eq!(
        arrange_with_options(&doc, &p, TransposeMode::Auto, &o)
            .unwrap()
            .report
            .kept,
        30
    );
    assert_eq!(doc, before);
}

#[test]
fn parameters_reject_invalid_values() {
    for o in [
        ArrangementOptions {
            melody_priority: 6,
            ..Default::default()
        },
        ArrangementOptions {
            bass_priority: 6,
            ..Default::default()
        },
        ArrangementOptions {
            accompaniment_priority: 6,
            ..Default::default()
        },
        ArrangementOptions {
            maximum_polyphony: Some(0),
            ..Default::default()
        },
        ArrangementOptions {
            maximum_polyphony: Some(129),
            ..Default::default()
        },
    ] {
        assert!(o.validate().is_err());
    }
    for priority in [0, 5] {
        assert!(
            ArrangementOptions {
                melody_priority: priority,
                bass_priority: priority,
                accompaniment_priority: priority,
                maximum_polyphony: Some(128),
                ..Default::default()
            }
            .validate()
            .is_ok()
        );
    }
}

#[test]
fn simple_support_selection_preserves_admitted_repeats_and_unknown() {
    let mut doc = document(&[60, 60, 64, 67]);
    for n in &mut doc.notes {
        n.start_tick = 0;
        n.end_tick = 960;
    }
    let p = project_music(&doc).unwrap();
    let base = arrange_with_options(
        &doc,
        &p,
        TransposeMode::Fixed(0),
        &ArrangementOptions::default(),
    )
    .unwrap();
    let mut search = crate::search_transposition(&doc, TransposeMode::Fixed(0)).unwrap();
    search.melody.track = None;
    search.analysis.track_roles[0].role = skysheet_music::TrackRole::Harmony;
    let mut mappings = base.report.note_mappings.clone();
    let options = ArrangementOptions {
        mode: ArrangementMode::SimpleAccompaniment,
        ..Default::default()
    };
    crate::context::reduce(&mut mappings, &search, &options);
    assert_eq!(
        mappings.iter().map(|m| m.mapped).collect::<Vec<_>>(),
        vec![true, true, true, false]
    );
    assert_eq!(
        mappings[3].drop_reason,
        Some(crate::DropReason::AccompanimentSimplified)
    );
    search.analysis.track_roles[0].role = skysheet_music::TrackRole::Unknown;
    let mut unknown = base.report.note_mappings;
    crate::context::reduce(&mut unknown, &search, &options);
    assert!(unknown.iter().all(|m| m.mapped));
}

#[test]
fn later_melody_can_displace_whole_overlapping_support_without_changing_time() {
    let mut doc = document(&[60, 72]);
    doc.notes[0].end_tick = 2880;
    let p = project_music(&doc).unwrap();
    let mut mappings = arrange_with_options(
        &doc,
        &p,
        TransposeMode::Fixed(0),
        &ArrangementOptions::default(),
    )
    .unwrap()
    .report
    .note_mappings;
    mappings[0].importance = 300;
    mappings[1].importance = 1300;
    let search = crate::search_transposition(&doc, TransposeMode::Fixed(0)).unwrap();
    crate::context::reduce(
        &mut mappings,
        &search,
        &ArrangementOptions {
            maximum_polyphony: Some(1),
            ..Default::default()
        },
    );
    assert!(!mappings[0].mapped);
    assert!(mappings[1].mapped);
    assert_eq!(mappings[1].projected_start_tick, doc.notes[1].start_tick);
    assert_eq!(
        mappings[1].projected_duration_ticks,
        doc.notes[1].end_tick - doc.notes[1].start_tick
    );
}
