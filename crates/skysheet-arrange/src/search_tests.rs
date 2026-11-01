use super::search::*;
use skysheet_music::{MidiPitch, MusicDocument, MusicNote, MusicTrack};
use skysheet_score::{Instrument, Pitch, PlaybackMode, SKY_15_INSTRUMENT_ID, SKY_15_KEYS};

pub(crate) fn document(pitches: &[u8]) -> MusicDocument {
    MusicDocument {
        id: "authored-test".into(),
        source_name: None,
        source_format: "authored".into(),
        ppq: 960,
        tracks: vec![MusicTrack {
            index: 0,
            name: Some("Test".into()),
        }],
        notes: pitches
            .iter()
            .enumerate()
            .map(|(i, p)| MusicNote {
                id: format!("note-{i}"),
                start_tick: i as u64 * 960,
                end_tick: i as u64 * 960 + 480,
                pitch: MidiPitch::new(*p).unwrap(),
                track: 0,
                midi: None,
                diagnostics: vec![],
            })
            .collect(),
        tempo_map: vec![],
        time_signature_map: vec![],
        metadata: vec![],
        annotations: vec![],
        analysis_refs: vec![],
        diagnostics: vec![],
        original_midi: None,
    }
}

pub(crate) fn instrument() -> Instrument {
    Instrument {
        id: SKY_15_INSTRUMENT_ID.into(),
        name: "Sky 15-key instrument".into(),
        playback_mode: PlaybackMode::Trigger,
        keys: SKY_15_KEYS
            .iter()
            .map(|p| Pitch::parse(*p).unwrap())
            .collect(),
    }
}

#[test]
fn actual_keys_not_range_and_signed_octaves() {
    let inst = instrument();
    for (pitches, shift, kept) in [
        (vec![60, 62, 64, 84], 0, 4),
        (vec![48, 50, 52], 12, 3),
        (vec![84, 86, 88], -12, 3),
        (vec![61, 63, 65], -1, 3),
        (vec![59, 61, 63], 1, 3),
        (vec![60, 61, 63, 64], 0, 2),
        (vec![0, 36, 60, 84, 127], 0, 2),
        (vec![0, 127], -127, 0),
        (vec![0, 127], 127, 0),
    ] {
        let doc = document(&pitches);
        let before = doc.clone();
        let c = evaluate(&doc, &inst, None, &[false], shift);
        assert_eq!(c.total_kept, kept, "{pitches:?}, {shift}");
        assert_eq!(doc, before);
    }
}

#[test]
fn all_semitones_search_prefers_smallest_shift_and_deterministic_signed_tie() {
    let inst = instrument();
    let doc = document(&[61]);
    let best = (-127..=127)
        .map(|s| evaluate(&doc, &inst, None, &[false], s))
        .max_by(compare)
        .unwrap();
    assert_eq!(best.transpose_semitones, -1);
    assert_eq!(best.total_kept, 1);
    assert_eq!(key_index(&inst, -1), None);
    assert_eq!(key_index(&inst, 128), None);
    assert_eq!(key_index(&inst, 60), Some(0));
    assert_eq!(key_index(&inst, 84), Some(14));
}

#[test]
fn melody_retention_precedes_total_and_continuity_breaks_equal_shift_ties() {
    let inst = instrument();
    let doc = document(&[60, 61, 62, 63]);
    let mut a = evaluate(&doc, &inst, Some(0), &[false], 0);
    let mut b = a.clone();
    a.total_kept = 2;
    a.melody_kept = Some(2);
    b.total_kept = 100;
    b.melody_kept = Some(1);
    assert!(compare(&a, &b).is_gt());
    b = a.clone();
    b.preserved_melody_pairs += 1;
    assert!(compare(&b, &a).is_gt());
    b = a.clone();
    b.retained_melody_leap_sum += 1;
    assert!(compare(&a, &b).is_gt());
}

#[test]
fn ambiguous_sparse_and_bass_material_do_not_invent_a_melody() {
    for pitches in [&[60, 62][..], &[36, 38, 40, 41][..]] {
        let analysis = skysheet_music::analyze(&document(pitches)).unwrap();
        assert_eq!(select_melody(&analysis).track, None);
    }
    let mut doc = document(&[72, 74, 76, 77, 79]);
    doc.tracks.push(MusicTrack {
        index: 1,
        name: None,
    });
    let duplicate = doc
        .notes
        .iter()
        .cloned()
        .map(|mut n| {
            n.id.push_str("-other");
            n.track = 1;
            n
        })
        .collect::<Vec<_>>();
    doc.notes.extend(duplicate);
    let selected = select_melody(&skysheet_music::analyze(&doc).unwrap());
    assert_eq!(selected.status, "unknown");
    assert!(selected.reason.contains("Competing"));
}

#[test]
fn real_ode_search_has_full_melody_and_explicit_accompaniment_loss() {
    let raw = include_bytes!("../../../audios/ode-to-joy.mid");
    let doc = skysheet_midi::read_music(raw, Some("ode-to-joy.mid".into())).unwrap();
    let before = doc.clone();
    let found = search_transposition(&doc, TransposeMode::Auto).unwrap();
    let best = &found.candidates[found.selected_index];
    assert_eq!(found.melody.track, Some(1));
    assert_eq!(best.melody_kept, Some(30));
    assert_eq!(best.melody_retention, Some(1.0));
    assert_eq!(best.accompaniment_total, 22);
    assert_eq!(best.transpose_semitones, 0);
    assert_eq!(best.total_kept, 30);
    assert_eq!(best.accompaniment_kept, 0);
    assert_eq!(best.dropped, 22);
    assert_eq!(found.candidates.len(), 255);
    assert_eq!(doc, before);
    eprintln!(
        "ode-to-joy: shift {}, kept {}, accompaniment {}",
        best.transpose_semitones, best.total_kept, best.accompaniment_kept
    );
    let fixed = search_transposition(&doc, TransposeMode::Fixed(12)).unwrap();
    assert_eq!(fixed.candidates.len(), 1);
    assert_eq!(fixed.candidates[0].transpose_semitones, 12);
    assert!(fixed.candidates[0].dropped > 0);
    assert!(search_transposition(&doc, TransposeMode::Fixed(i16::MAX)).is_err());
}
