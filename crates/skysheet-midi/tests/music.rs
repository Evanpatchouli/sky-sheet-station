use skysheet_midi::{inspect, read_music};
use skysheet_music::{Origin, analyze, extract_melody};

#[test]
fn ode_to_joy_matches_byte_derived_reference_and_is_unchanged() {
    let bytes = include_bytes!("../../../audios/ode-to-joy.mid");
    assert_eq!(bytes.len(), 575);
    let doc = read_music(bytes, Some("ode-to-joy.mid".into())).unwrap();
    let before = doc.clone();
    let r = analyze(&doc).unwrap();
    assert_eq!(doc, before);
    assert_eq!(r.source, before);
    assert_eq!(doc.original_midi.as_deref(), Some(bytes.as_slice()));
    assert_eq!(doc.ppq, 480);
    assert_eq!(doc.tracks.len(), 3);
    assert_eq!(doc.notes.len(), 52);
    assert_eq!(doc.tracks[1].name.as_deref(), Some("Mano destra"));
    assert_eq!(doc.tracks[2].name.as_deref(), Some("Mano sinistra"));
    // Reference transcribed directly from the 575-byte SMF hex dump, not from
    // inspect/read_music/analyze. VLQ 83 3a=442, 85 16=662, 81 5d=221,
    // 86 73=883; following gaps 26=38, 3a=58, 13=19, 4d=77.
    let melody = [
        76, 76, 77, 79, 79, 77, 76, 74, 72, 72, 74, 76, 76, 74, 74, 76, 76, 77, 79, 79, 77, 76, 74,
        72, 72, 74, 76, 74, 72, 72,
    ];
    let bass = [
        48, 55, 48, 55, 43, 50, 48, 55, 43, 50, 48, 55, 48, 55, 43, 50, 48, 55, 43, 50, 48, 55,
    ];
    let bass_lengths = [
        960, 960, 480, 480, 480, 480, 960, 960, 960, 960, 960, 960, 480, 480, 480, 480, 960, 960,
        480, 480, 480, 480,
    ];
    let mut expected = Vec::new();
    for (track, pitches) in [(1, melody.as_slice()), (2, bass.as_slice())] {
        let mut start = 0;
        for (i, &pitch) in pitches.iter().enumerate() {
            let step = if track == 2 {
                bass_lengths[i]
            } else {
                match i % 15 {
                    12 => 720,
                    13 => 240,
                    14 => 960,
                    _ => 480,
                }
            };
            let held = match step {
                240 => 221,
                480 => 442,
                720 => 662,
                960 => 883,
                _ => unreachable!(),
            };
            expected.push((
                track,
                pitch,
                start,
                start + held,
                if track == 1 { 107 } else { 76 },
                0,
                2 + 2 * i,
                3 + 2 * i,
            ));
            start += step;
        }
    }
    let actual: Vec<_> = doc
        .notes
        .iter()
        .map(|n| {
            (
                n.track,
                n.pitch.value(),
                n.start_tick,
                n.end_tick,
                n.midi.as_ref().unwrap().velocity,
                n.midi.as_ref().unwrap().channel,
                n.midi.as_ref().unwrap().on_event,
                n.midi.as_ref().unwrap().off_event,
            )
        })
        .collect();
    assert_eq!(actual, expected);
    assert_eq!(doc.tempo_map.len(), 1);
    let t = &doc.tempo_map[0];
    assert_eq!(
        (
            t.location.track,
            t.location.event,
            t.location.tick,
            t.microseconds_per_quarter
        ),
        (0, 1, 0, 600000)
    );
    let m = &doc.time_signature_map[0];
    assert_eq!(
        (
            m.location.track,
            m.location.event,
            m.location.tick,
            m.numerator,
            m.denominator_power,
            m.clocks_per_click,
            m.thirty_seconds_per_quarter
        ),
        (0, 2, 0, 4, 2, 24, 8)
    );
    let sky = [60, 62, 64, 65, 67, 69, 71, 72, 74, 76, 77, 79, 81, 83, 84];
    assert_eq!(
        doc.notes
            .iter()
            .filter(|n| !sky.contains(&n.pitch.value()))
            .count(),
        22
    );
    assert_eq!(r.note_statistics.per_track_notes, [0, 30, 22]);
    assert_eq!(r.tempo_analysis.effective_map[0].origin, Origin::Source);
    assert_eq!(r.tempo_analysis.principal_bpm, Some(100.0));
    // Pairing is also identical to the existing inspection path; the adapter
    // never passes through the constrained import or D6 score conversion.
    assert_eq!(inspect(bytes).unwrap().source_notes.len(), 52);
    for c in &r.melody_candidates {
        assert_eq!(
            extract_melody(&doc, c).unwrap().len(),
            c.features.note_count
        );
    }
    println!(
        "ode-to-joy top track {:?}; roles {:?}; human musical reference pending",
        r.melody_candidates.first().map(|c| c.track),
        r.track_roles.iter().map(|t| t.role).collect::<Vec<_>>()
    );
}

fn smf(tracks: Vec<Vec<u8>>) -> Vec<u8> {
    let mut bytes = b"MThd\0\0\0\x06\0\x01".to_vec();
    bytes.extend((tracks.len() as u16).to_be_bytes());
    bytes.extend([1, 224]);
    for mut track in tracks {
        track.extend([0, 255, 47, 0]);
        bytes.extend(b"MTrk");
        bytes.extend((track.len() as u32).to_be_bytes());
        bytes.extend(track);
    }
    bytes
}

#[test]
fn unrestricted_unisons_source_channels_maps_and_defaults() {
    // Literal bytes, independent of production MIDI writer. Two equal-pitch
    // overlapping FIFO notes + another channel and another synchronous track.
    let data = smf(vec![
        vec![
            0, 0x90, 0, 1, 0, 0x90, 0, 127, 0, 0x91, 127, 64, 0x83, 0x60, 0x80, 0, 0, 0, 0x80, 0,
            0, 0, 0x81, 127, 0,
        ],
        vec![0, 0x90, 0, 80, 0x83, 0x60, 0x80, 0, 0],
    ]);
    let doc = read_music(&data, None).unwrap();
    let r = analyze(&doc).unwrap();
    assert_eq!(doc.notes.len(), 4);
    assert!(doc.tempo_map.is_empty());
    assert_eq!(r.note_statistics.max_polyphony, 4);
    assert_eq!(r.note_statistics.overlapping_note_pairs, 6);
    assert!(r.tempo_analysis.uses_protocol_default);
    assert_eq!(r.tempo_analysis.principal_bpm, Some(120.0));
    assert_eq!(
        doc.notes
            .iter()
            .map(|n| n.pitch.value())
            .collect::<Vec<_>>(),
        [0, 0, 127, 0]
    );
    assert!(doc.notes.iter().any(|n| !n.diagnostics.is_empty()));
    let maps = smf(vec![
        vec![
            0, 255, 81, 3, 7, 161, 32, 0, 255, 88, 4, 3, 2, 24, 8, 0x83, 0x60, 255, 81, 3, 15, 66,
            64, 0, 255, 88, 4, 5, 3, 24, 8,
        ],
        vec![0, 0x90, 36, 80, 0x87, 0x40, 0x80, 36, 0],
    ]);
    let doc = read_music(&maps, None).unwrap();
    let r = analyze(&doc).unwrap();
    assert_eq!(doc.tempo_map.len(), 2);
    assert_eq!(doc.time_signature_map.len(), 2);
    assert_eq!(doc.notes[0].end_tick, 960);
    assert_eq!(r.tempo_analysis.change_positions, [480]);
    assert_eq!(doc, r.source);
}

#[test]
fn malformed_pairing_conflicts_and_unsupported_sounding_events_fail_explicitly() {
    for (track, code) in [
        (vec![0, 0x90, 60, 80], "missing_note_off"),
        (vec![0, 0xb0, 64, 127], "unsupported_midi_event"),
        (vec![0, 255, 81, 3, 0, 0, 0], "zero_tempo"),
        (
            vec![0, 255, 81, 3, 7, 161, 32, 0, 255, 81, 3, 15, 66, 64],
            "music_document",
        ),
        (
            vec![0, 255, 88, 4, 3, 2, 24, 8, 0, 255, 88, 4, 4, 2, 24, 8],
            "music_document",
        ),
    ] {
        let error = read_music(&smf(vec![track]), None).unwrap_err();
        assert!(
            error.report.diagnostics.iter().any(|d| d.code == code),
            "{code}: {error}"
        );
    }
    assert!(read_music(b"broken", None).is_err());
}

fn encode_delta(mut tick: u64) -> Vec<u8> {
    let mut bytes = vec![(tick & 127) as u8];
    tick >>= 7;
    while tick > 0 {
        bytes.push((tick as u8 & 127) | 128);
        tick >>= 7;
    }
    bytes.reverse();
    bytes
}
#[test]
fn all_fixed_reference_cases_also_cross_the_midi_analysis_boundary() {
    #[derive(serde::Deserialize)]
    struct Cases {
        cases: Vec<Case>,
    }
    #[derive(serde::Deserialize)]
    struct Case {
        id: String,
        notes: Vec<Vec<[u64; 5]>>,
        count: usize,
        polyphony: usize,
        tempos: Vec<[u64; 2]>,
        meters: Vec<[u64; 3]>,
    }
    let cases: Cases = serde_json::from_str(include_str!(
        "../../skysheet-music/tests/fixtures/cases.json"
    ))
    .unwrap();
    for case in cases.cases {
        let tracks = case
            .notes
            .iter()
            .enumerate()
            .map(|(track, notes)| {
                let mut events = Vec::new();
                for n in notes {
                    events.push((n[0], 2, vec![0x90 + n[4] as u8, n[2] as u8, n[3] as u8]));
                    events.push((n[1], 1, vec![0x80 + n[4] as u8, n[2] as u8, 0]));
                }
                if track == 0 {
                    for t in &case.tempos {
                        let mut bytes = vec![255, 81, 3];
                        bytes.extend(&(t[1] as u32).to_be_bytes()[1..]);
                        events.push((t[0], 0, bytes));
                    }
                    for m in &case.meters {
                        events.push((m[0], 0, vec![255, 88, 4, m[1] as u8, m[2] as u8, 24, 8]));
                    }
                }
                events.sort_by_key(|e| (e.0, e.1));
                let mut previous = 0;
                let mut bytes = Vec::new();
                for (tick, _, event) in events {
                    bytes.extend(encode_delta(tick - previous));
                    bytes.extend(event);
                    previous = tick;
                }
                bytes
            })
            .collect();
        let bytes = smf(tracks);
        let doc = read_music(&bytes, None).unwrap();
        let r = analyze(&doc).unwrap();
        assert_eq!(doc.notes.len(), case.count, "{}", case.id);
        assert_eq!(
            r.note_statistics.max_polyphony, case.polyphony,
            "{}",
            case.id
        );
        assert_eq!(r.source, doc);
        assert_eq!(doc.tempo_map.len(), case.tempos.len());
        assert_eq!(doc.time_signature_map.len(), case.meters.len());
        let mut expected: Vec<_> = case
            .notes
            .iter()
            .enumerate()
            .flat_map(|(t, ns)| {
                ns.iter()
                    .map(move |n| (t, n[0], n[1], n[2] as u8, n[3] as u8, n[4] as u8))
            })
            .collect();
        let mut actual: Vec<_> = doc
            .notes
            .iter()
            .map(|n| {
                let m = n.midi.as_ref().unwrap();
                (
                    n.track,
                    n.start_tick,
                    n.end_tick,
                    n.pitch.value(),
                    m.velocity,
                    m.channel,
                )
            })
            .collect();
        expected.sort();
        actual.sort();
        assert_eq!(actual, expected, "{}", case.id);
    }
}

#[test]
fn large_midi_empty_track_and_maximum_delta_preserve_all_events() {
    let mut track = Vec::new();
    for i in 0..4096 {
        track.extend([0, 0x90, (i % 128) as u8, 127, 1, 0x80, (i % 128) as u8, 0]);
    }
    let bytes = smf(vec![Vec::new(), track]);
    let doc = read_music(&bytes, None).unwrap();
    let report = analyze(&doc).unwrap();
    assert_eq!(report.note_statistics.per_track_notes, [0, 4096]);
    assert_eq!(report.note_statistics.lowest_pitch, Some(0));
    assert_eq!(report.note_statistics.highest_pitch, Some(127));
    let bytes = smf(vec![vec![
        0xff, 0xff, 0xff, 0x7f, 0x90, 127, 127, 0xff, 0xff, 0xff, 0x7f, 0x80, 127, 0,
    ]]);
    let doc = read_music(&bytes, None).unwrap();
    assert_eq!(
        (doc.notes[0].start_tick, doc.notes[0].end_tick),
        (268435455, 536870910)
    );
    assert_eq!(analyze(&doc).unwrap().source, doc);
}
