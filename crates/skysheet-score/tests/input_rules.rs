//! D2-D4 regressions inspect the original JSON text, not a pre-parsed Value.
use serde_json::{Value, json};
use skysheet_score::Score;

const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");

fn replace_token(field: &str, original: &str, token: &str) -> String {
    let find = format!("\"{field}\": {original}");
    assert!(EXAMPLE.contains(&find));
    EXAMPLE.replacen(&find, &format!("\"{field}\": {token}"), 1)
}

#[test]
fn integer_fields_require_plain_unsigned_tokens() {
    for (field, original) in [
        ("start_tick", "0"),
        ("duration_ticks", "480"),
        ("ppq", "960"),
        ("numerator", "4"),
        ("denominator", "4"),
    ] {
        let plain = replace_token(field, original, original);
        serde_json::from_str::<Score>(&plain)
            .unwrap()
            .validate()
            .unwrap();
        for token in [
            format!("{original}.0"),
            format!("{original}e0"),
            format!("{original}E+0"),
            "-0".into(),
            format!("\"{original}\""),
        ] {
            let raw = replace_token(field, original, &token);
            assert!(
                serde_json::from_str::<Score>(&raw).is_err(),
                "accepted {field}: {token}"
            );
            assert!(
                serde_json::from_reader::<_, Score>(raw.as_bytes()).is_err(),
                "reader accepted {field}: {token}"
            );
        }
    }
}

fn duplicate_object_member(pointer: &str, key: &str, second_key: &str, second: &Value) -> String {
    // The source fixture is known-valid. Introduce duplication only after
    // serialization so no generic object can discard the repeated member.
    let example: Value = serde_json::from_str(EXAMPLE).unwrap();
    let object = example.pointer(pointer).unwrap();
    let old_object = serde_json::to_string(object).unwrap();
    let mut members: Vec<String> = object
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| format!("{}:{value}", serde_json::to_string(name).unwrap()))
        .collect();
    assert!(object.get(key).is_some());
    members.push(format!("{second_key}:{second}"));
    let raw = serde_json::to_string(&example).unwrap();
    assert_eq!(raw.matches(&old_object).count(), 1);
    raw.replacen(&old_object, &format!("{{{}}}", members.join(",")), 1)
}

#[test]
fn repeated_members_rejected_at_every_object_level() {
    let example: Value = serde_json::from_str(EXAMPLE).unwrap();
    for (pointer, key) in [
        ("", "format_version"),
        ("/metadata", "id"),
        ("/instrument", "id"),
        ("/timeline", "ppq"),
        ("/timeline/time_signature", "numerator"),
        ("/tracks/0", "id"),
        ("/tracks/0/notes/0", "id"),
    ] {
        let first = &example.pointer(pointer).unwrap()[key];
        let different = if first.is_string() {
            json!("other")
        } else {
            json!(1)
        };
        let normal_key = serde_json::to_string(key).unwrap();
        let escaped_key = format!("\"\\u{:04x}{}\"", key.as_bytes()[0], &key[1..]);
        for (second_key, value) in [
            (&normal_key, first),
            (&normal_key, &different),
            (&escaped_key, first),
        ] {
            let raw = duplicate_object_member(pointer, key, second_key, value);
            let error = serde_json::from_str::<Score>(&raw).unwrap_err().to_string();
            assert!(
                error.contains(&format!("duplicate field `{key}`")),
                "{pointer}: {error}"
            );
            assert!(serde_json::from_reader::<_, Score>(raw.as_bytes()).is_err());
        }
    }
}

#[test]
fn repeated_optional_text_rejected_even_when_null() {
    let find = "\"text\": \"Optional user note, lyric, or reminder\"";
    for (first, second) in [
        ("null", "null"),
        ("null", "\"second\""),
        ("\"first\"", "null"),
        ("\"same\"", "\"same\""),
    ] {
        let raw = EXAMPLE.replacen(find, &format!("\"text\": {first}, \"text\": {second}"), 1);
        let error = serde_json::from_str::<Score>(&raw).unwrap_err().to_string();
        assert!(error.contains("duplicate field `text`"), "{error}");
    }
}

#[test]
fn tempo_accepts_finite_positive_binary64_values() {
    for (token, expected) in [
        ("120", 120.0),
        ("120.5", 120.5),
        ("1.2e2", 120.0),
        ("0.1", 0.1),
        ("1e-300", 1e-300),
        ("1e300", 1e300),
        ("5e-324", f64::from_bits(1)),
        ("1.7976931348623157e308", f64::MAX),
    ] {
        let raw = replace_token("tempo", "120", token);
        let score: Score =
            serde_json::from_str(&raw).unwrap_or_else(|error| panic!("{token}: {error}"));
        score.validate().unwrap();
        assert_eq!(score.timeline.tempo, expected, "{token}");
        let encoded = serde_json::to_string(&score).unwrap();
        let decoded: Score = serde_json::from_str(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(
            decoded.timeline.tempo, score.timeline.tempo,
            "{token}: round-trip"
        );
    }
}

#[test]
fn tempo_rejects_nonpositive_overflow_and_underflow_to_zero() {
    for token in [
        "0", "0.0", "-0", "-0.0", "-1", "1e400", "1e-400", "-1e-400", "NaN", "Infinity", "\"120\"",
    ] {
        let raw = replace_token("tempo", "120", token);
        let result = serde_json::from_str::<Score>(&raw)
            .map_err(|error| error.to_string())
            .and_then(|score| score.validate().map_err(|error| error.to_string()));
        assert!(result.is_err(), "accepted tempo {token}");
    }
}
