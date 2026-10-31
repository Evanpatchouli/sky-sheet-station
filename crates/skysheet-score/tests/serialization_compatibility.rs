//! Approved D1/D5/D7 behavior. These tests do not implement future format versions
//! or MIDI conversion; shared fixtures also cover D8's closed score shape.

use serde_json::{Value, json};
use skysheet_score::{Pitch, PlaybackMode, Score};
use std::collections::HashSet;

const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");

#[test]
fn decoded_pitch_strings_survive_without_canonicalization() {
    for (spelling, midi) in [
        ("C04", 60),
        ("B#3", 60),
        ("E#4", 65),
        ("C-0", 12),
        ("B#-2", 0),
    ] {
        let encoded = serde_json::to_string(spelling).unwrap();
        let pitch: Pitch = serde_json::from_reader(encoded.as_bytes()).unwrap();
        assert_eq!(pitch.midi_number(), midi, "{spelling}");
        assert_eq!(pitch.as_str(), spelling);
        assert_eq!(serde_json::to_string(&pitch).unwrap(), encoded);
    }

    // JSON escapes do not constitute separate key identities; decoded spellings do.
    let escaped: Pitch = serde_json::from_str(r#""B\u00233""#).unwrap();
    assert_eq!(escaped, Pitch::parse("B#3").unwrap());
    assert_ne!(escaped, Pitch::parse("C4").unwrap());
    assert_eq!(serde_json::to_string(&escaped).unwrap(), r#""B#3""#);
}

#[test]
fn custom_enharmonic_keys_keep_identity_order_and_note_references() {
    // Deliberately neither sorted by sound nor written only in canonical form.
    let spellings = ["F4", "B#3", "E#4", "C04", "C4"];
    let pitches: Vec<Pitch> = spellings
        .iter()
        .map(|spelling| Pitch::parse(*spelling).unwrap())
        .collect();
    assert_eq!(pitches.iter().collect::<HashSet<_>>().len(), 5);
    assert_eq!(
        pitches
            .iter()
            .map(Pitch::midi_number)
            .collect::<HashSet<_>>()
            .len(),
        2
    );

    for mode in [PlaybackMode::Trigger, PlaybackMode::Sustain] {
        let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
        score.instrument.id = "custom-enharmonic".into();
        score.instrument.keys = pitches.clone();
        score.instrument.playback_mode = mode;
        // Retain deliberately reversed track/note arrays too.
        score.tracks.reverse();
        for track in &mut score.tracks {
            track.notes.reverse();
        }
        for (index, note) in score
            .tracks
            .iter_mut()
            .flat_map(|track| &mut track.notes)
            .enumerate()
        {
            note.pitch = pitches[index % pitches.len()].clone();
        }
        score.validate().unwrap();
        let encoded = serde_json::to_string(&score).unwrap();
        let decoded: Score = serde_json::from_reader(encoded.as_bytes()).unwrap();
        decoded.validate().unwrap();
        assert_eq!(decoded, score);
        let value: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(value["instrument"]["keys"], json!(spellings));
        for (index, pitch) in pitches.iter().enumerate() {
            assert_eq!(decoded.key_number_for(pitch), Some(index + 1));
        }
        let other_alias = Pitch::parse("C004").unwrap();
        assert_eq!(other_alias.midi_number(), 60);
        assert_eq!(decoded.key_number_for(&other_alias), None);
        let mut invalid = decoded;
        invalid.tracks[0].notes[0].pitch = other_alias;
        assert!(
            invalid
                .validate()
                .unwrap_err()
                .to_string()
                .contains("not playable")
        );
    }
}

#[test]
fn supported_format_version_is_an_exact_string_checked_by_validation() {
    let supported: Score = serde_json::from_str(EXAMPLE).unwrap();
    assert_eq!(supported.format_version, "1.0");
    supported.validate().unwrap();
    for version in ["1", "1.00", "1.0.0", "01.0", "1.1", "2.0", "1.0 ", ""] {
        let raw = EXAMPLE.replacen(r#""1.0""#, &serde_json::to_string(version).unwrap(), 1);
        // Parsing alone is not a claim of version support or score validity.
        let score: Score = serde_json::from_str(&raw).unwrap();
        assert!(
            score
                .validate()
                .unwrap_err()
                .to_string()
                .contains("unsupported format_version"),
            "{version:?}"
        );
    }
}

#[test]
fn absent_and_null_text_share_a_model_but_empty_text_remains_present() {
    let mut value: Value = serde_json::from_str(EXAMPLE).unwrap();
    value.as_object_mut().unwrap().remove("text");
    let absent: Score = serde_json::from_str(&value.to_string()).unwrap();
    value["text"] = Value::Null;
    let null: Score = serde_json::from_str(&value.to_string()).unwrap();
    assert_eq!(absent, null);
    for score in [absent, null] {
        score.validate().unwrap();
        let encoded = serde_json::to_string(&score).unwrap();
        let serialized: Value = serde_json::from_str(&encoded).unwrap();
        assert_eq!(serialized.get("text"), Some(&Value::Null));
        assert_eq!(serde_json::from_str::<Score>(&encoded).unwrap(), score);
    }
    value["text"] = json!("");
    let empty: Score = serde_json::from_str(&value.to_string()).unwrap();
    empty.validate().unwrap();
    assert_eq!(empty.text.as_deref(), Some(""));
    assert_eq!(serde_json::to_value(&empty).unwrap()["text"], json!(""));
}
