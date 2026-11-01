use crate::search_tests::document;
use crate::{DropReason, TransposeMode, arrange};
use skysheet_midi::project_music;
use skysheet_music::{EventLocation, MidiPitch, MusicTrack, TempoEvent};

#[test]
fn fixed_mapping_reports_every_loss_without_folding_or_snapping() {
    for (pitches, shift, expected, reasons) in [
        (vec![60, 62, 64, 84], 0, vec![60, 62, 64, 84], vec![]),
        (vec![48, 50, 52], 12, vec![60, 62, 64], vec![]),
        (vec![84, 86, 88], -12, vec![72, 74, 76], vec![]),
        (vec![59, 61, 63], 1, vec![60, 62, 64], vec![]),
        (vec![61, 63, 65], -1, vec![60, 62, 64], vec![]),
        (
            vec![60, 61, 63, 84],
            0,
            vec![60, 84],
            vec![
                DropReason::NotInInstrumentKeys,
                DropReason::NotInInstrumentKeys,
            ],
        ),
        (
            vec![0, 60, 127],
            0,
            vec![60],
            vec![
                DropReason::BelowInstrumentRange,
                DropReason::AboveInstrumentRange,
            ],
        ),
        (
            vec![0, 61, 127],
            -1,
            vec![60],
            vec![DropReason::MidiOutOfRange, DropReason::AboveInstrumentRange],
        ),
        (
            vec![0, 59, 127],
            1,
            vec![60],
            vec![DropReason::BelowInstrumentRange, DropReason::MidiOutOfRange],
        ),
    ] {
        let doc = document(&pitches);
        let before = doc.clone();
        let r = arrange(
            &doc,
            &project_music(&doc).unwrap(),
            TransposeMode::Fixed(shift),
        )
        .unwrap();
        assert_eq!(
            r.score.tracks[0]
                .notes
                .iter()
                .map(|n| n.pitch.midi_number())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            r.report
                .note_mappings
                .iter()
                .filter_map(|n| n.drop_reason)
                .collect::<Vec<_>>(),
            reasons
        );
        assert_eq!(r.report.note_mappings.len(), pitches.len());
        assert_eq!(doc, before);
        assert!(
            r.report.time_preserved && r.report.pitch_mapping_valid && r.report.score_validation
        );
        for n in &r.score.tracks[0].notes {
            let source = doc.notes.iter().find(|s| s.id == n.id).unwrap();
            assert_eq!(
                (n.start_tick, n.duration_ticks),
                (source.start_tick, source.end_tick - source.start_tick)
            );
        }
    }
}

#[test]
fn raw_480_ppq_and_tempo_map_project_once_then_remain_exact() {
    let mut doc = document(&[60, 64]);
    doc.ppq = 480;
    doc.notes[0].start_tick = 0;
    doc.notes[0].end_tick = 480;
    doc.notes[1].start_tick = 480;
    doc.notes[1].end_tick = 960;
    doc.tempo_map = vec![
        TempoEvent {
            location: EventLocation {
                track: 0,
                event: 0,
                tick: 0,
            },
            microseconds_per_quarter: 500000,
        },
        TempoEvent {
            location: EventLocation {
                track: 0,
                event: 1,
                tick: 480,
            },
            microseconds_per_quarter: 1000000,
        },
    ];
    let before = doc.clone();
    let projection = project_music(&doc).unwrap();
    assert_eq!(
        projection
            .notes()
            .iter()
            .map(|n| (n.start_tick, n.duration_ticks))
            .collect::<Vec<_>>(),
        vec![(0, 960), (960, 1920)]
    );
    let r = arrange(&doc, &projection, TransposeMode::Fixed(0)).unwrap();
    assert_eq!(r.score.timeline.ppq, 960);
    assert_eq!(r.score.timeline.tempo.to_bits(), 120f64.to_bits());
    assert_eq!(r.score.timeline.time_signature.numerator, 4);
    assert_eq!(r.score.timeline.time_signature.denominator, 4);
    assert_eq!(&r.score.timeline, projection.timeline());
    assert_eq!(
        r.score.tracks[0]
            .notes
            .iter()
            .map(|n| (n.start_tick, n.duration_ticks))
            .collect::<Vec<_>>(),
        vec![(0, 960), (960, 1920)]
    );
    assert!(
        r.report
            .timing_report
            .diagnostics
            .iter()
            .any(|d| d.code == "tempo_map_flattened")
    );
    assert_eq!(doc, before);
}

#[test]
fn projection_rejects_zero_duration_overflow_and_changed_source() {
    let mut doc = document(&[60]);
    doc.ppq = 1920;
    doc.notes[0].start_tick = 1;
    doc.notes[0].end_tick = 3;
    let p = project_music(&doc).unwrap();
    assert_eq!(
        (p.notes()[0].start_tick, p.notes()[0].duration_ticks),
        (1, 1)
    );
    doc.notes[0].end_tick = 2;
    assert!(
        project_music(&doc)
            .unwrap_err()
            .to_string()
            .contains("quantized_zero_duration")
    );
    assert!(
        arrange(&doc, &p, TransposeMode::Auto)
            .unwrap_err()
            .message
            .contains("does not match")
    );
    doc.notes[0].start_tick = u64::MAX - 1;
    doc.notes[0].end_tick = u64::MAX;
    assert!(project_music(&doc).is_err());
}

#[test]
fn all_tracks_unisons_repetitions_chords_and_source_order_survive() {
    let mut doc = document(&[64, 60, 60, 67]);
    doc.tracks.push(MusicTrack {
        index: 1,
        name: Some("Other".into()),
    });
    doc.notes[1].track = 1;
    for n in &mut doc.notes {
        n.start_tick = 123;
        n.end_tick = 1111;
    }
    let before = doc.clone();
    let r = arrange(
        &doc,
        &project_music(&doc).unwrap(),
        TransposeMode::Fixed(12),
    )
    .unwrap();
    assert_eq!(r.report.kept, 4);
    assert_eq!(
        r.score.tracks[0]
            .notes
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        vec!["note-0", "note-2", "note-3"]
    );
    assert_eq!(r.score.tracks[1].notes[0].id, "note-1");
    for (source, mapping) in doc.notes.iter().zip(&r.report.note_mappings) {
        assert_eq!(mapping.target_midi, i16::from(source.pitch.value()) + 12);
        assert_eq!(mapping.projected_start_tick, 123);
        assert_eq!(mapping.projected_duration_ticks, 988);
    }
    assert_eq!(doc, before);
}

#[test]
fn no_playable_notes_is_failure_with_complete_loss_report() {
    let doc = document(&[72, 74, 76, 77, 79]);
    let err = arrange(
        &doc,
        &project_music(&doc).unwrap(),
        TransposeMode::Fixed(127),
    )
    .unwrap_err();
    let report = err.report.unwrap();
    assert!(!report.score_generated);
    assert_eq!(report.note_mappings.len(), 5);
    assert_eq!(report.dropped, 5);
    assert_eq!(report.melody_retention, Some(0.0));
    assert!(
        report
            .note_mappings
            .iter()
            .all(|n| n.drop_reason == Some(DropReason::MidiOutOfRange))
    );
    let empty = document(&[]);
    assert!(arrange(&empty, &project_music(&empty).unwrap(), TransposeMode::Auto).is_err());
}

#[test]
fn partial_accompaniment_and_overlapping_registers_are_not_removed_as_a_group() {
    let mut doc = document(&[72, 74, 76, 77, 79]);
    doc.tracks.push(MusicTrack {
        index: 1,
        name: None,
    });
    for (i, p) in [48, 60, 48, 60, 48].iter().enumerate() {
        let mut n = doc.notes[i].clone();
        n.track = 1;
        n.id.push_str("-bass");
        n.pitch = MidiPitch::new(*p).unwrap();
        doc.notes.push(n);
    }
    let r = arrange(&doc, &project_music(&doc).unwrap(), TransposeMode::Fixed(0)).unwrap();
    assert_eq!(r.report.kept, 7);
    assert_eq!(r.report.tracks[1].kept, 2);
    assert_eq!(r.report.tracks[1].dropped, 3);
    // Move accompaniment into the same register: mapped overlaps remain independent.
    for n in doc.notes.iter_mut().filter(|n| n.track == 1) {
        n.pitch = MidiPitch::new(72).unwrap();
    }
    let r = arrange(&doc, &project_music(&doc).unwrap(), TransposeMode::Fixed(0)).unwrap();
    assert_eq!(r.report.kept, 10);
    assert_eq!(r.score.tracks[1].notes.len(), 5);
}

#[test]
fn actual_ode_score_has_exact_projected_time_and_all_22_losses() {
    let raw = include_bytes!("../../../audios/ode-to-joy.mid");
    let doc = skysheet_midi::read_music(raw, Some("ode-to-joy.mid".into())).unwrap();
    let before = doc.clone();
    let p = project_music(&doc).unwrap();
    let r = arrange(&doc, &p, TransposeMode::Auto).unwrap();
    r.score.validate().unwrap();
    assert_eq!(r.report.original_notes, 52);
    assert_eq!(r.report.kept, 30);
    assert_eq!(r.report.dropped, 22);
    assert_eq!(r.report.transpose_semitones, Some(0));
    assert_eq!(r.report.melody.track, Some(1));
    assert_eq!(
        r.report.note_mappings.iter().filter(|n| !n.mapped).count(),
        22
    );
    for n in &r.score.tracks[1].notes {
        let s = doc.notes.iter().find(|s| s.id == n.id).unwrap();
        assert_eq!(n.start_tick, s.start_tick * 2);
        assert_eq!(n.duration_ticks, (s.end_tick - s.start_tick) * 2);
    }
    assert_eq!(r.score.timeline.tempo.to_bits(), 100f64.to_bits());
    let second = arrange(&doc, &p, TransposeMode::Auto).unwrap();
    assert_eq!(
        serde_json::to_vec(&r.report).unwrap(),
        serde_json::to_vec(&second.report).unwrap()
    );
    assert_eq!(doc, before);
    assert_eq!(doc.original_midi.as_deref(), Some(raw.as_slice()));
}

#[test]
fn extreme_pitches_auto_can_map_and_resource_limits_do_not_truncate() {
    for (pitch, shift) in [(0, 60), (127, -43)] {
        let doc = document(&[pitch]);
        let r = arrange(&doc, &project_music(&doc).unwrap(), TransposeMode::Auto).unwrap();
        assert_eq!(r.report.transpose_semitones, Some(shift));
        assert_eq!(r.report.kept, 1);
    }
    let mut doc = document(&[]);
    doc.tracks = (0..4097)
        .map(|index| MusicTrack { index, name: None })
        .collect();
    assert!(
        project_music(&doc)
            .unwrap_err()
            .to_string()
            .contains("projection_resource_limit")
    );
    assert!(crate::search_transposition(&doc, TransposeMode::Auto).is_err());
}
