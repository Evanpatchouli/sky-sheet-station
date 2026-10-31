use serde::Deserialize;
use serde_json::Value;
use skysheet_score::Score;
use std::collections::{BTreeMap, HashSet};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    changes: BTreeMap<String, Value>,
    #[serde(default)]
    removes: Vec<String>,
    schema_valid: bool,
    rust_error: Option<String>,
}

fn check_fixture(fixture: &str, prefix: &str) {
    let cases: Vec<Case> = serde_json::from_str(fixture).unwrap();
    let mut count = 0;
    for case in cases.iter().filter(|case| case.name.starts_with(prefix)) {
        count += 1;
        let mut value: Value =
            serde_json::from_str(include_str!("../../../docs/examples/sky-15-example.json"))
                .unwrap();
        for pointer in &case.removes {
            let (parent, key) = pointer.rsplit_once('/').expect("fixture pointer");
            assert!(
                value
                    .pointer_mut(parent)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(key)
                    .is_some(),
                "{}: {pointer}",
                case.name
            );
        }
        for (pointer, replacement) in &case.changes {
            if let Some(target) = value.pointer_mut(pointer) {
                *target = replacement.clone();
            } else {
                let (parent, key) = pointer.rsplit_once('/').expect("fixture pointer");
                value
                    .pointer_mut(parent)
                    .expect("fixture parent exists")
                    .as_object_mut()
                    .unwrap()
                    .insert(key.into(), replacement.clone());
            }
        }
        let json = serde_json::to_string(&value).unwrap();
        let result = serde_json::from_str::<Score>(&json)
            .map_err(|error| error.to_string())
            .and_then(|score| {
                score.validate().map_err(|error| error.to_string())?;
                Ok(score)
            });
        if let Some(expected) = &case.rust_error {
            let error = result.expect_err(&case.name);
            assert!(error.contains(expected), "{}: {error}", case.name);
        } else {
            assert!(
                case.schema_valid,
                "{}: valid Rust score must satisfy structural Schema",
                case.name
            );
            let score = result.unwrap_or_else(|error| panic!("{}: {error}", case.name));
            let encoded = serde_json::to_string(&score).unwrap();
            let decoded: Score = serde_json::from_str(&encoded).unwrap();
            decoded.validate().unwrap();
            assert_eq!(score, decoded, "{}: round-trip", case.name);
        }
    }
    assert!(count > 0, "no cases for {prefix}");
}

fn check_cases(prefix: &str) {
    check_fixture(include_str!("fixtures/contract-boundaries.json"), prefix);
}

#[test]
fn tick_boundaries() {
    check_cases("tick:");
}

#[test]
fn numerator_boundaries() {
    check_cases("numerator:");
}

#[test]
fn denominator_boundaries() {
    check_cases("denominator:");
}

#[test]
fn pitch_boundaries() {
    check_cases("pitch:");
}

#[test]
fn required_fields() {
    check_fixture(
        include_str!("fixtures/contract-conformance.json"),
        "required:",
    );
}

#[test]
fn field_types() {
    check_fixture(include_str!("fixtures/contract-conformance.json"), "types:");
}

#[test]
fn unknown_fields() {
    check_fixture(
        include_str!("fixtures/contract-conformance.json"),
        "unknown:",
    );
}

#[test]
fn semantic_constraints() {
    check_fixture(
        include_str!("fixtures/contract-conformance.json"),
        "semantic:",
    );
}

#[test]
fn optional_and_valid_variants() {
    check_fixture(include_str!("fixtures/contract-conformance.json"), "valid:");
}

#[test]
fn object_shapes_match_schema() {
    check_fixture(include_str!("fixtures/object-shapes.json"), "object:");
}

#[test]
fn fixture_catalog_is_fully_assigned() {
    for (fixture, prefixes) in [
        (
            include_str!("fixtures/contract-boundaries.json"),
            &["tick:", "numerator:", "denominator:", "pitch:"][..],
        ),
        (
            include_str!("fixtures/contract-conformance.json"),
            &["required:", "types:", "unknown:", "semantic:", "valid:"][..],
        ),
        (
            include_str!("fixtures/object-shapes.json"),
            &["object:"][..],
        ),
    ] {
        let cases: Vec<Case> = serde_json::from_str(fixture).unwrap();
        let mut names = HashSet::new();
        for case in cases {
            assert!(
                names.insert(case.name.clone()),
                "duplicate case: {}",
                case.name
            );
            assert_eq!(
                prefixes
                    .iter()
                    .filter(|prefix| case.name.starts_with(**prefix))
                    .count(),
                1,
                "unassigned case: {}",
                case.name
            );
        }
    }
}
