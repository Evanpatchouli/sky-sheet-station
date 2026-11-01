use skysheet_midi::{ImportOptions, export, import, inspect};

// Independently encode tiny SMF fixtures, rather than trusting the production
// encoder to manufacture the input that its own parser will accept.
fn vlq(mut n: u32) -> Vec<u8> {
    let mut b = vec![(n & 127) as u8];
    n >>= 7;
    while n != 0 {
        b.push((n as u8 & 127) | 128);
        n >>= 7;
    }
    b.reverse();
    b
}
fn smf(format: u16, ppq: u16, tracks: Vec<Vec<(u32, Vec<u8>)>>) -> Vec<u8> {
    let mut b = b"MThd\0\0\0\x06".to_vec();
    b.extend(format.to_be_bytes());
    b.extend((tracks.len() as u16).to_be_bytes());
    b.extend(ppq.to_be_bytes());
    for events in tracks {
        let mut track = Vec::new();
        for (delta, data) in events {
            track.extend(vlq(delta));
            track.extend(data);
        }
        track.extend([0, 255, 47, 0]);
        b.extend(b"MTrk");
        b.extend((track.len() as u32).to_be_bytes());
        b.extend(track);
    }
    b
}
fn on(ch: u8, key: u8) -> Vec<u8> {
    vec![0x90 + ch, key, 80]
}
fn off(ch: u8, key: u8) -> Vec<u8> {
    vec![0x80 + ch, key, 0]
}
fn tempo(us: u32) -> Vec<u8> {
    let mut b = vec![255, 81, 3];
    b.extend(&us.to_be_bytes()[1..]);
    b
}
fn meter(n: u8, d: u8, q: u8) -> Vec<u8> {
    vec![255, 88, 4, n, d, 24, q]
}
fn note(start: u32, length: u32) -> Vec<(u32, Vec<u8>)> {
    vec![(start, on(0, 60)), (length, off(0, 60))]
}
fn opts() -> ImportOptions {
    ImportOptions::default()
}
fn failure(bytes: &[u8], code: &str) {
    let error = import(bytes, &opts()).unwrap_err();
    assert!(
        error.report.diagnostics.iter().any(|d| d.code == code),
        "expected {code}, got {error:?}"
    );
}

#[test]
fn formats_defaults_leading_silence_and_source_provenance() {
    for format in [0, 1] {
        let (score, r) = import(&smf(format, 480, vec![note(480, 480)]), &opts()).unwrap();
        score.validate().unwrap();
        assert_eq!(score.timeline.tempo, 120.0);
        let n = &score.tracks[0].notes[0];
        assert_eq!((n.start_tick, n.duration_ticks), (960, 960));
        assert_eq!(r.source_notes[0].start_tick, 480);
        assert_eq!(r.source_notes[0].velocity, 80);
        for code in ["default_tempo", "default_meter", "velocity_omitted"] {
            assert!(r.diagnostics.iter().any(|d| d.code == code));
        }
    }
}

#[test]
fn cross_tempo_notes_and_initial_reference_selection() {
    let bytes = smf(1, 960, vec![vec![(960, tempo(1_000_000))], note(480, 960)]);
    let (s, r) = import(&bytes, &opts()).unwrap();
    assert_eq!(
        (
            s.tracks[1].notes[0].start_tick,
            s.tracks[1].notes[0].duration_ticks
        ),
        (480, 1440)
    );
    assert_eq!(s.timeline.tempo, 120.0);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "tempo_map_flattened")
    );
    let (s, _) = import(
        &bytes,
        &ImportOptions {
            reference_tempo: Some(60.0),
            name: None,
        },
    )
    .unwrap();
    assert_eq!(
        (
            s.tracks[1].notes[0].start_tick,
            s.tracks[1].notes[0].duration_ticks
        ),
        (240, 720)
    );
    let (s, r) = import(
        &smf(1, 480, vec![vec![(0, tempo(1_000_000))], note(0, 480)]),
        &opts(),
    )
    .unwrap();
    assert_eq!(s.timeline.tempo, 60.0);
    assert_eq!(s.tracks[1].notes[0].duration_ticks, 960);
    assert!(!r.diagnostics.iter().any(|d| d.code == "default_tempo"));
}

#[test]
fn global_tempo_conflicts_and_track_order_independence() {
    let a = vec![(0, tempo(500000)), (960, tempo(1000000))];
    for tracks in [
        vec![a.clone(), note(960, 960)],
        vec![note(960, 960), a.clone()],
    ] {
        let (s, r) = import(&smf(1, 960, tracks), &opts()).unwrap();
        let n = s.tracks.iter().flat_map(|t| &t.notes).next().unwrap();
        assert_eq!((n.start_tick, n.duration_ticks), (960, 1920));
        if r.tempos[0].track == 1 {
            assert!(
                r.diagnostics
                    .iter()
                    .any(|d| d.code == "tempo_outside_first_track")
            );
        }
    }
    for tracks in [
        vec![vec![(0, tempo(500000)), (0, tempo(1000000))]],
        vec![vec![(0, tempo(500000))], vec![(0, tempo(1000000))]],
    ] {
        failure(&smf(1, 960, tracks), "tempo_conflict");
    }
    let (_, r) = import(
        &smf(
            1,
            960,
            vec![vec![(0, tempo(500000))], vec![(0, tempo(500000))]],
        ),
        &opts(),
    )
    .unwrap();
    assert!(r.diagnostics.iter().any(|d| d.code == "tempo_merged"));
    failure(&smf(0, 960, vec![vec![(0, tempo(0))]]), "zero_tempo");
}

#[test]
fn meter_defaults_conflicts_later_out_of_range_and_notation_units() {
    let (s, r) = import(
        &smf(
            1,
            960,
            vec![
                vec![(0, meter(3, 2, 8)), (960, meter(3, 7, 8))],
                note(0, 1920),
            ],
        ),
        &opts(),
    )
    .unwrap();
    assert_eq!(s.timeline.time_signature.numerator, 3);
    assert_eq!(s.tracks[1].notes[0].duration_ticks, 1920);
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "meter_change_discarded")
    );
    let (s, _) = import(&smf(0, 960, vec![vec![(960, meter(3, 2, 8))]]), &opts()).unwrap();
    assert_eq!(s.timeline.time_signature.numerator, 4);
    for (events, code) in [
        (vec![(0, meter(3, 7, 8))], "nominal_meter_range"),
        (vec![(0, meter(0, 2, 8))], "invalid_meter"),
        (vec![(0, meter(4, 2, 7))], "unsupported_notation_unit"),
        (
            vec![(0, meter(3, 2, 8)), (0, meter(4, 2, 8))],
            "meter_conflict",
        ),
    ] {
        failure(&smf(0, 960, vec![events]), code);
    }
    failure(
        &smf(
            1,
            960,
            vec![vec![(0, meter(3, 2, 8))], vec![(0, meter(4, 2, 8))]],
        ),
        "meter_conflict",
    );
    let (_, r) = import(
        &smf(0, 960, vec![vec![(0, meter(3, 2, 8)), (0, meter(3, 2, 8))]]),
        &opts(),
    )
    .unwrap();
    assert!(r.diagnostics.iter().any(|d| d.code == "meter_merged"));
}

#[test]
fn fifo_duplicates_chords_channels_and_running_status_are_not_merged() {
    let events = vec![
        (0, on(0, 60)),
        (0, vec![64, 80]),
        (0, on(1, 60)),
        (10, on(0, 60)),
        (10, off(0, 60)),
        (10, vec![0x90, 60, 0]),
        (0, off(0, 64)),
        (0, off(1, 60)),
    ];
    let (s, r) = import(&smf(1, 960, vec![events, note(0, 30)]), &opts()).unwrap();
    assert_eq!(s.tracks[0].notes.len(), 4);
    assert_eq!(s.tracks[1].notes.len(), 1);
    assert_eq!(
        (
            s.tracks[0].notes[0].start_tick,
            s.tracks[0].notes[0].duration_ticks
        ),
        (0, 20)
    );
    assert_eq!(
        (
            s.tracks[0].notes[3].start_tick,
            s.tracks[0].notes[3].duration_ticks
        ),
        (10, 20)
    );
    assert!(
        r.diagnostics
            .iter()
            .any(|d| d.code == "overlapping_note_on")
    );
}

#[test]
fn pairing_and_pitch_failures_retain_source_notes() {
    for (events, code) in [
        (vec![(0, off(0, 60))], "unmatched_note_off"),
        (vec![(0, on(0, 60))], "missing_note_off"),
        (
            vec![(0, on(0, 60)), (0, off(0, 60))],
            "nonpositive_source_duration",
        ),
        (vec![(0, on(0, 61)), (10, off(0, 61))], "unsupported_pitch"),
        (vec![(0, on(0, 48)), (10, off(0, 48))], "unsupported_pitch"),
    ] {
        failure(&smf(0, 960, vec![events]), code);
    }
    failure(
        &smf(1, 960, vec![vec![(0, on(0, 60))], vec![(10, off(0, 60))]]),
        "missing_note_off",
    );
    let e = import(
        &smf(0, 960, vec![vec![(0, on(0, 127)), (10, off(0, 127))]]),
        &opts(),
    )
    .unwrap_err();
    assert_eq!(e.report.source_notes[0].pitch, "G9");
}

#[test]
fn quantization_uses_endpoints_not_duration_or_rounded_segments() {
    let events = [note(0, 1000), note(0, 1000), note(0, 1000)].concat();
    let (s, _) = import(&smf(0, 14400, vec![events]), &opts()).unwrap();
    assert_eq!(
        s.tracks[0]
            .notes
            .iter()
            .map(|n| n.duration_ticks)
            .collect::<Vec<_>>(),
        vec![67, 66, 67]
    );
    assert_eq!(s.tracks[0].notes[0].duration_ticks, 67);
    // 960/1920 = 1/2: exact .5 endpoints round upward independently.
    let (s, _) = import(&smf(0, 1920, vec![note(1, 2)]), &opts()).unwrap();
    assert_eq!(
        (
            s.tracks[0].notes[0].start_tick,
            s.tracks[0].notes[0].duration_ticks
        ),
        (1, 1)
    );
    // x = source tick / 100: 100.10..100.40 collapses; 100.49..100.51 survives.
    let op = ImportOptions {
        reference_tempo: Some(1.2),
        name: None,
    };
    assert!(
        import(&smf(0, 960, vec![note(10010, 30)]), &op)
            .unwrap_err()
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "quantized_zero_duration")
    );
    let (s, _) = import(&smf(0, 960, vec![note(10049, 2)]), &op).unwrap();
    assert_eq!(
        (
            s.tracks[0].notes[0].start_tick,
            s.tracks[0].notes[0].duration_ticks
        ),
        (100, 1)
    );
    for bpm in [f64::from_bits(1), f64::MAX, f64::INFINITY, 0.0] {
        assert!(
            import(
                &smf(0, 960, vec![note(0, 960)]),
                &ImportOptions {
                    reference_tempo: Some(bpm),
                    name: None
                }
            )
            .is_err()
        );
    }
}

#[test]
fn malformed_structures_fail_before_permissive_library_recovery() {
    for bytes in [
        smf(2, 960, vec![note(0, 1)]),
        smf(0, 0xe728, vec![note(0, 1)]),
        smf(0, 0, vec![note(0, 1)]),
        smf(0, 960, vec![]),
        smf(0, 960, vec![note(0, 1), note(0, 1)]),
        smf(0, 960, vec![vec![(0, vec![255, 81, 2, 1, 2])]]),
        smf(0, 960, vec![vec![(0, vec![0x90, 60, 200])]]),
        smf(0, 960, vec![vec![(0, vec![60, 20])]]),
    ] {
        failure(&bytes, "midi_structure");
    }
    let bytes = smf(0, 960, vec![note(0, 960)]);
    for len in 0..bytes.len() {
        assert!(import(&bytes[..len], &opts()).is_err(), "prefix {len}");
    }
    let mut extra = bytes.clone();
    extra.push(0);
    failure(&extra, "midi_structure");
    let mut missing = bytes.clone();
    missing.truncate(missing.len() - 4);
    let len = (missing.len() - 22) as u32;
    missing[18..22].copy_from_slice(&len.to_be_bytes());
    failure(&missing, "midi_structure");
    failure(
        &smf(0, 960, vec![vec![(0, vec![255, 47, 0]), (0, on(0, 60))]]),
        "midi_structure",
    );
}

#[test]
fn unsupported_interpretations_and_track_names_are_explicit() {
    for bytes in [
        vec![0xb0, 64, 127],
        vec![0xe0, 0, 64],
        vec![0xf0, 1, 0xf7],
        vec![255, 0x7f, 0],
    ] {
        assert!(import(&smf(0, 960, vec![vec![(0, bytes)]]), &opts()).is_err());
    }
    let (_, r) = import(
        &smf(
            0,
            960,
            vec![vec![(0, vec![255, 3, 1, 0xff]), (0, vec![0xc0, 5])]],
        ),
        &opts(),
    )
    .unwrap();
    assert!(r.diagnostics.iter().any(|d| d.code == "metadata_encoding"));
    assert_eq!(r.metadata[0].data, vec![255]);
}

#[test]
fn export_roundtrip_nested_unisons_tracks_and_ordering() {
    let (mut s, _) = import(&smf(1, 960, vec![note(0, 960), note(0, 960)]), &opts()).unwrap();
    let mut n = s.tracks[0].notes[0].clone();
    n.id = "nested".into();
    n.start_tick = 100;
    n.duration_ticks = 100;
    s.tracks[0].notes.push(n);
    let mut n = s.tracks[0].notes[0].clone();
    n.id = "retrigger".into();
    n.start_tick = 960;
    s.tracks[0].notes.push(n);
    let (bytes, r) = export(&s).unwrap();
    assert_eq!(r.output_microseconds_per_quarter, Some(500000));
    assert_eq!(&bytes[0..4], b"MThd");
    let (back, report) = import(&bytes, &opts()).unwrap();
    assert_eq!(back.tracks.len(), s.tracks.len() + 1);
    for (a, b) in s.tracks.iter().zip(&back.tracks[1..]) {
        let mut original: Vec<_> = a
            .notes
            .iter()
            .map(|n| (n.pitch.as_str(), n.start_tick, n.duration_ticks))
            .collect();
        let mut decoded: Vec<_> = b
            .notes
            .iter()
            .map(|n| (n.pitch.as_str(), n.start_tick, n.duration_ticks))
            .collect();
        original.sort();
        decoded.sort();
        assert_eq!(original, decoded);
    }
    assert!(report.source_notes.iter().all(|n| n.velocity == 64));
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| d.code == "overlapping_note_on")
    );
    let parsed = midly::Smf::parse(&bytes).unwrap();
    let mut tick = 0;
    let mut at_boundary = Vec::new();
    for e in &parsed.tracks[1] {
        tick += e.delta.as_int();
        if tick == 960 {
            at_boundary.push(e.kind);
        }
    }
    assert!(matches!(
        at_boundary[0],
        midly::TrackEventKind::Midi {
            message: midly::MidiMessage::NoteOff { .. },
            ..
        }
    ));
}

#[test]
fn export_rejects_protocol_limits_and_diagnoses_approximation() {
    let (s, _) = import(&smf(0, 960, vec![note(0, 960)]), &opts()).unwrap();
    for bpm in [f64::from_bits(1), f64::MAX] {
        let mut s = s.clone();
        s.timeline.tempo = bpm;
        assert!(export(&s).is_err());
    }
    let mut meter = s.clone();
    meter.timeline.time_signature.numerator = 256;
    assert!(export(&meter).is_err());
    let mut gap = s.clone();
    gap.tracks[0].notes[0].start_tick = 0x10000000;
    assert!(export(&gap).is_err());
    let mut voices = s.clone();
    for i in 1..16 {
        let mut n = voices.tracks[0].notes[0].clone();
        n.id = format!("extra-{i}");
        voices.tracks[0].notes.push(n);
    }
    assert!(
        export(&voices)
            .unwrap_err()
            .report
            .diagnostics
            .iter()
            .any(|d| d.code == "midi_channel_capacity")
    );
    let mut fractional = s.clone();
    fractional.timeline.tempo = 123.0;
    assert!(
        export(&fractional)
            .unwrap()
            .1
            .diagnostics
            .iter()
            .any(|d| d.code == "tempo_approximated")
    );
    let mut invalid = s;
    invalid.tracks[0].notes[0].duration_ticks = 0;
    assert!(export(&invalid).is_err());
}

#[test]
fn inspection_retains_unrestricted_pitches_without_claiming_score_validity() {
    let b = smf(0, 960, vec![vec![(0, on(0, 0)), (960, off(0, 0))]]);
    let r = inspect(&b).unwrap();
    assert_eq!(r.source_notes[0].pitch, "C-1");
    failure(&b, "unsupported_pitch");
}

#[test]
fn export_cross_track_boundary_cannot_release_another_tracks_new_note() {
    let (s, _) = import(&smf(1, 960, vec![note(960, 960), note(0, 960)]), &opts()).unwrap();
    let (bytes, _) = export(&s).unwrap();
    let r = inspect(&bytes).unwrap();
    assert_ne!(r.source_notes[0].channel, r.source_notes[1].channel);
}
