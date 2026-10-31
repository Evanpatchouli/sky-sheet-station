use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use skysheet_score::{Instrument, Metadata, Note, Score, TimeSignature, Timeline, Track};
use std::fmt::Debug;

const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");

fn check_object_type<T>(pointer: &str, fields: &[&str])
where
    T: DeserializeOwned + Serialize + PartialEq + Debug,
{
    let example: Value = serde_json::from_str(EXAMPLE).unwrap();
    let object = example.pointer(pointer).unwrap().clone();
    let expected: T = serde_json::from_value(object.clone()).unwrap();

    // A valid named object must remain independent of member order.
    let reversed_members: Vec<String> = fields
        .iter()
        .rev()
        .map(|field| {
            format!(
                "{}:{}",
                serde_json::to_string(field).unwrap(),
                object[*field]
            )
        })
        .collect();
    let reversed = format!("{{{}}}", reversed_members.join(","));
    assert_eq!(serde_json::from_str::<T>(&reversed).unwrap(), expected);

    let encoded = serde_json::to_value(&expected).unwrap();
    assert!(encoded.is_object());
    assert_eq!(encoded.as_object().unwrap().len(), fields.len());
    assert_eq!(serde_json::from_value::<T>(encoded).unwrap(), expected);

    let positional: Vec<Value> = fields.iter().map(|field| object[*field].clone()).collect();
    let mut extra = positional.clone();
    extra.push(Value::Null);
    for invalid in [
        Value::Array(positional.clone()),
        Value::Array(Vec::new()),
        Value::Array(positional[..positional.len() - 1].to_vec()),
        Value::Array(extra),
        Value::Null,
        serde_json::json!("not an object"),
        serde_json::json!(42),
    ] {
        let raw = serde_json::to_string(&invalid).unwrap();
        assert!(
            serde_json::from_str::<T>(&raw).is_err(),
            "{pointer}: accepted direct {raw}"
        );
        assert!(
            serde_json::from_reader::<_, T>(raw.as_bytes()).is_err(),
            "{pointer}: accepted reader {raw}"
        );
        assert!(
            serde_json::from_value::<T>(invalid.clone()).is_err(),
            "{pointer}: accepted Value {raw}"
        );

        // Verify that the same guard is applied when reached through Score.
        let mut nested = example.clone();
        *nested.pointer_mut(pointer).unwrap() = invalid;
        let nested_raw = serde_json::to_string(&nested).unwrap();
        assert!(
            serde_json::from_str::<Score>(&nested_raw).is_err(),
            "{pointer}: accepted nested input"
        );
        assert!(
            serde_json::from_value::<Score>(nested).is_err(),
            "{pointer}: accepted nested Value"
        );
    }
}

#[test]
fn score_requires_object() {
    check_object_type::<Score>(
        "",
        &[
            "format_version",
            "metadata",
            "instrument",
            "timeline",
            "tracks",
            "text",
        ],
    );
}

#[test]
fn metadata_requires_object() {
    check_object_type::<Metadata>("/metadata", &["id", "name"]);
}

#[test]
fn instrument_requires_object() {
    check_object_type::<Instrument>("/instrument", &["id", "name", "playback_mode", "keys"]);
}

#[test]
fn timeline_requires_object() {
    check_object_type::<Timeline>("/timeline", &["ppq", "tempo", "time_signature"]);
}

#[test]
fn time_signature_requires_object() {
    check_object_type::<TimeSignature>("/timeline/time_signature", &["numerator", "denominator"]);
}

#[test]
fn track_requires_object() {
    check_object_type::<Track>("/tracks/0", &["id", "name", "role", "notes"]);
}

#[test]
fn note_requires_object() {
    check_object_type::<Note>(
        "/tracks/0/notes/0",
        &["id", "pitch", "start_tick", "duration_ticks"],
    );
}
