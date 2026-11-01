//! Frozen expectations are independently checked by the isolated numeric tool.
//! Never compute expected bits with the production text decoder.
use serde::Deserialize;
use skysheet_score::Score;

const EXAMPLE: &str = include_str!("../../../docs/examples/sky-15-example.json");

#[derive(Deserialize)]
struct Fixture {
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    token: String,
    decoded_bits: Option<String>,
    valid: bool,
}

fn cases() -> Vec<Case> {
    serde_json::from_str::<Fixture>(include_str!("fixtures/n2-tempo.json"))
        .unwrap()
        .cases
}

fn parse(raw: &str, reader: bool) -> Result<Score, serde_json::Error> {
    if reader {
        serde_json::from_reader(raw.as_bytes())
    } else {
        serde_json::from_str(raw)
    }
}

fn check(case: &Case) {
    let raw = EXAMPLE.replacen("\"tempo\": 120", &format!("\"tempo\": {}", case.token), 1);
    for reader in [false, true] {
        let parsed = parse(&raw, reader);
        let Some(bits) = &case.decoded_bits else {
            let error = parsed.unwrap_err().to_string();
            assert!(error.contains("timeline.tempo"), "{}: {error}", case.id);
            assert!(
                error.contains("number out of range"),
                "{}: {error}",
                case.id
            );
            continue;
        };
        let expected = u64::from_str_radix(bits, 16).unwrap();
        let score = parsed.unwrap_or_else(|e| panic!("{} reader={reader}: {e}", case.id));
        assert_eq!(
            score.timeline.tempo.to_bits(),
            expected,
            "{} reader={reader}",
            case.id
        );
        assert_eq!(score.validate().is_ok(), case.valid, "{}", case.id);
        // Zero and negatives remain decoded f64 values rejected by validation.
        if !case.valid {
            assert!(
                score
                    .validate()
                    .unwrap_err()
                    .to_string()
                    .contains("timeline.tempo")
            );
            continue;
        }
        for saved in [
            serde_json::to_string(&score).unwrap(),
            serde_json::to_string_pretty(&score).unwrap(),
        ] {
            for readback in [false, true] {
                let decoded = parse(&saved, readback).unwrap();
                decoded.validate().unwrap();
                assert_eq!(
                    decoded.timeline.tempo.to_bits(),
                    expected,
                    "{} readback={readback}",
                    case.id
                );
            }
        }
    }
}

#[test]
fn exact_midpoints_and_extreme_binary64_values() {
    let cases = cases();
    assert_eq!(cases.len(), 84);
    for case in cases.iter().filter(|c| !c.id.starts_with("long/")) {
        check(case);
    }
}

#[test]
fn long_integer_midpoints_and_zero_boundary_keep_all_digits() {
    let cases = cases();
    let digits = "100000000000000011102230246251565404236316680908203125";
    let token = format!("{}{}e-768", digits, "0".repeat(715));
    assert!(
        cases
            .iter()
            .any(|c| c.token == token && c.decoded_bits.as_deref() == Some("3ff0000000000000"))
    );
    assert_eq!(
        cases.iter().filter(|c| c.id.starts_with("long/")).count(),
        30
    );
    for case in cases.iter().filter(|c| c.id.starts_with("long/")) {
        check(case);
    }
}

#[test]
fn historical_in_memory_serialization_preserves_bits() {
    // Existing sample/00167d447773ebd6 and Oracle-certified readback token.
    let expected = 0x0016_7d44_7773_ebd6;
    let mut score: Score = serde_json::from_str(EXAMPLE).unwrap();
    score.timeline.tempo = f64::from_bits(expected);
    for _ in 0..3 {
        let saved = serde_json::to_string(&score).unwrap();
        for reader in [false, true] {
            let decoded = parse(&saved, reader).unwrap();
            assert_eq!(decoded.timeline.tempo.to_bits(), expected);
            decoded.validate().unwrap();
        }
        score = parse(&saved, false).unwrap();
    }
}

#[test]
fn raw_values_do_not_coerce_strings_or_non_numbers() {
    for token in [
        "\"120\"",
        "\"NaN\"",
        "true",
        "null",
        "[]",
        "{}",
        "NaN",
        "Infinity",
        "-Infinity",
        "+120",
        "01",
        "1.",
        "1e",
    ] {
        let raw = EXAMPLE.replacen("\"tempo\": 120", &format!("\"tempo\": {token}"), 1);
        for reader in [false, true] {
            assert!(
                parse(&raw, reader).is_err(),
                "accepted {token}, reader={reader}"
            );
        }
    }
}
