//! Former audit inputs promoted to direct-input regressions after D2-D4 approval.
//! The printed Value-first comparison remains diagnostic only: pre-parsing loses
//! information and cannot certify that the original file satisfies the contract.
use serde::Deserialize;
use serde_json::{Value, json};
use skysheet_score::Score;
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Probe {
    name: String,
    find: Option<String>,
    replace: Option<String>,
    #[serde(default)]
    changes: BTreeMap<String, Value>,
    rust_error: Option<String>,
}

fn outcome(result: Result<Score, serde_json::Error>) -> String {
    match result {
        Err(error) => format!("parse error: {error}"),
        Ok(score) => match score.validate() {
            Err(error) => format!("validation error: {error}"),
            Ok(()) => format!(
                "accepted; tempo={:e}; text={:?}",
                score.timeline.tempo, score.text
            ),
        },
    }
}

#[test]
fn approved_raw_input_cases() {
    let example = include_str!("../../../docs/examples/sky-15-example.json");
    let probes: Vec<Probe> =
        serde_json::from_str(include_str!("fixtures/raw-input-audit.json")).unwrap();
    for probe in probes {
        let raw = if let Some(find) = &probe.find {
            assert!(
                probe.changes.is_empty(),
                "mixed audit modes: {}",
                probe.name
            );
            assert!(
                example.contains(find),
                "missing audit token: {}",
                probe.name
            );
            example.replacen(find, probe.replace.as_ref().expect("replacement text"), 1)
        } else {
            assert!(!probe.changes.is_empty() && probe.replace.is_none());
            let mut value: Value = serde_json::from_str(example).unwrap();
            for (path, replacement) in &probe.changes {
                *value.pointer_mut(path).expect("audit path exists") = replacement.clone();
            }
            serde_json::to_string(&value).unwrap()
        };
        let parsed = serde_json::from_str::<Score>(&raw);
        let error = match &parsed {
            Err(error) => Some(error.to_string()),
            Ok(score) => score.validate().err().map(|error| error.to_string()),
        };
        match (&probe.rust_error, error) {
            (Some(expected), Some(actual)) => {
                assert!(actual.contains(expected), "{}: {actual}", probe.name)
            }
            (None, None) => {}
            (expected, actual) => panic!("{}: expected {expected:?}, got {actual:?}", probe.name),
        }
        let direct = outcome(parsed);
        let via_value =
            outcome(serde_json::from_str::<Value>(&raw).and_then(serde_json::from_value::<Score>));
        println!(
            "{}",
            json!({"name": probe.name, "direct": direct, "via_value": via_value})
        );
    }
}
