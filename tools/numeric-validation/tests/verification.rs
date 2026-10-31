use num_bigint::BigUint;
use num_traits::One;
use serde_json::{Value, json};
use skysheet_numeric_validation::{
    corpus,
    oracle::{self, Exact, MAX_BITS},
    raw_json, runner, samples, validation,
};
use std::{collections::HashSet, fs, process::Command};

fn validator() -> jsonschema::Validator {
    validation::validator(
        &corpus::read_json(&corpus::root().join("docs/schemas/sky-score-v1.schema.json")).unwrap(),
    )
    .unwrap()
}
fn example() -> String {
    fs::read_to_string(corpus::root().join("docs/examples/sky-15-example.json")).unwrap()
}

#[test]
fn original_number_tokens_are_not_coerced() {
    let doc = raw_json::parse("[1,1.0,1e0,-0,true,\"1\"]").unwrap();
    for (i, token) in ["1", "1.0", "1e0", "-0"].iter().enumerate() {
        assert_eq!(doc.numbers[&format!("/{i}")], *token);
    }
    assert!(!doc.numbers.contains_key("/4"));
    assert!(!doc.numbers.contains_key("/5"));
}

#[test]
fn duplicate_names_are_decoded_per_object_without_case_or_unicode_folding() {
    for raw in [
        r#"{"name":null,"\u006eame":null}"#,
        r#"{"a":{"x":1,"x":2}}"#,
    ] {
        assert_eq!(raw_json::parse(raw).err().unwrap().stage, "raw");
    }
    let doc =
        raw_json::parse("{\"Name\":1,\"name\":2,\"é\":3,\"e\\u0301\":4,\"other\":{\"name\":5}}")
            .unwrap();
    assert_eq!(doc.value.as_object().unwrap().len(), 5);
}

#[test]
fn json_grammar_and_string_escapes() {
    for raw in [
        "NaN",
        "Infinity",
        "-Infinity",
        "+1",
        "01",
        "1.",
        "1e",
        "[1,]",
        "{\"x\":1,}",
        r#""\q""#,
        "truex",
    ] {
        assert_eq!(raw_json::parse(raw).err().unwrap().stage, "json", "{raw}");
    }
    let doc = raw_json::parse(r#"{"a/b~c":["\"12\"","\ud83c\udfb5",1e2]}"#).unwrap();
    assert_eq!(doc.value["a/b~c"][1], "🎵");
    assert_eq!(doc.numbers["/a~1b~0c/2"], "1e2");
}

#[test]
fn schema_integer_semantics_are_separate_from_d2() {
    let v = validation::validator(&json!({"type":"integer"})).unwrap();
    for (raw, valid) in [
        ("1", true),
        ("1.0", true),
        ("1e0", true),
        ("-0", true),
        ("0.5", false),
        ("true", false),
        ("\"1\"", false),
    ] {
        assert_eq!(
            v.is_valid(&raw_json::parse(raw).unwrap().value),
            valid,
            "{raw}"
        );
    }
    assert!(validation::unsigned_token("0"));
    assert!(validation::unsigned_token("18446744073709551616"));
    for raw in ["-0", "1.0", "1e0", "01", "+1"] {
        assert!(!validation::unsigned_token(raw));
    }
}

#[test]
fn schema_numbers_are_exact_beyond_f64() {
    let v = validator();
    for token in ["1e-400", "1e400", "2e-324"] {
        let result = validation::analyse(
            &example().replacen("\"tempo\": 120", &format!("\"tempo\": {token}"), 1),
            &v,
        )
        .unwrap();
        assert_eq!(result["schema_valid"], true);
        assert_eq!(result["stage"], "numeric");
    }
    for token in ["1e-300", "1e300", "3e-324", "1.7976931348623158e308"] {
        let result = validation::analyse(
            &example().replacen("\"tempo\": 120", &format!("\"tempo\": {token}"), 1),
            &v,
        )
        .unwrap();
        assert_eq!(result["status"], "valid");
    }
}

#[test]
fn integers_and_note_ends_are_exact() {
    let v = validator();
    for token in [
        "9007199254740992",
        "9007199254740993",
        "18446744073709551616",
    ] {
        let result = validation::analyse(
            &example().replacen("\"start_tick\": 0", &format!("\"start_tick\": {token}"), 1),
            &v,
        )
        .unwrap();
        assert_eq!(result["schema_valid"], false);
    }
    let result = validation::analyse(
        &example().replacen("\"start_tick\": 0", "\"start_tick\": 9007199254740991", 1),
        &v,
    )
    .unwrap();
    assert_eq!(result["schema_valid"], true);
    assert_eq!(result["stage"], "numeric");
}

#[test]
fn known_ieee_bit_patterns_and_decimal_aliases() {
    for (token, bits) in [
        ("1", 0x3ff0000000000000),
        ("0.5", 0x3fe0000000000000),
        ("0.1", 0x3fb999999999999a),
        ("120", 0x405e000000000000),
        ("3e-324", 1),
        ("1.7976931348623158e308", MAX_BITS),
    ] {
        assert_eq!(oracle::bits(token).unwrap(), Some(bits), "{token}");
        assert!(oracle::certify(token, Some(bits)).unwrap());
    }
    for token in ["120.0", "12e1", "1.2e2", "120000e-3"] {
        assert_eq!(oracle::bits(token).unwrap(), Some(0x405e000000000000));
    }
    for token in ["0", "-0", "-0.0", "-1", "1e-400", "1e400"] {
        assert_eq!(oracle::bits(token).unwrap(), None);
    }
}

#[test]
fn exact_midpoint_and_neighbor_certificates() {
    let token = "1.00000000000000011102230246251565404236316680908203125";
    assert_eq!(oracle::bits(token).unwrap(), Some(0x3ff0000000000000));
    assert!(!oracle::certify(token, Some(0x3ff0000000000001)).unwrap());
    assert!(!oracle::certify(token, None).unwrap());
}

#[test]
fn zero_normal_and_overflow_boundaries() {
    let low = Exact {
        n: BigUint::one(),
        d: BigUint::one() << 1075usize,
    };
    assert_eq!(oracle::bits(&low.decimal_text()).unwrap(), None);
    let above = Exact {
        n: BigUint::from(5u8),
        d: BigUint::one() << 1077usize,
    };
    assert_eq!(oracle::bits(&above.decimal_text()).unwrap(), Some(1));
    let high = (BigUint::one() << 1024usize) - (BigUint::one() << 970usize);
    assert_eq!(oracle::bits(&high.to_string()).unwrap(), None);
    assert_eq!(
        oracle::bits(&(high - BigUint::one()).to_string()).unwrap(),
        Some(MAX_BITS)
    );
    let normal = Exact {
        n: BigUint::one(),
        d: BigUint::one() << 1022usize,
    };
    assert_eq!(oracle::bits(&normal.decimal_text()).unwrap(), Some(1 << 52));
    assert_eq!(
        oracle::bits(
            &oracle::dyadic((1 << 52) - 1)
                .midpoint(&normal)
                .decimal_text()
        )
        .unwrap(),
        Some(1 << 52)
    );
}

#[test]
fn no_remote_schema_fetches() {
    for keyword in ["$ref", "$dynamicRef", "$recursiveRef"] {
        assert!(validation::validator(&json!({keyword:"https://example.invalid/schema"})).is_err());
    }
}

#[test]
fn all_fixed_expectations_preserved() {
    let v = validator();
    let cases = corpus::fixed(&corpus::root()).unwrap();
    assert_eq!(cases.len(), 267);
    for case in cases {
        let result = validation::analyse(&case.raw, &v).unwrap();
        assert_eq!(
            result["schema_valid"],
            json!(case.schema_valid),
            "{}",
            case.id
        );
        if let Some(expected) = case.numeric_valid {
            assert_eq!(result["status"] == "valid", expected, "{}", case.id);
        }
        if let Some(bits) = case.bits {
            assert_eq!(result["tempo_bits"], bits, "{}", case.id);
        }
    }
}

#[test]
fn frozen_midpoints_recompute_exactly() {
    let frozen = corpus::fixed(&corpus::root())
        .unwrap()
        .into_iter()
        .filter(|c| c.id.starts_with("midpoint/"))
        .collect::<Vec<_>>();
    let rebuilt = corpus::generated_midpoints(&corpus::root()).unwrap();
    assert_eq!(json!(frozen), json!(rebuilt));
}

#[test]
fn frozen_samples_preserve_seed_order_coverage_and_uniqueness() {
    let values = corpus::sample_bits(&corpus::root()).unwrap();
    assert_eq!(values, samples::historical());
    assert_eq!(values.len(), 10000);
    assert_eq!(values.iter().collect::<HashSet<_>>().len(), 10000);
    assert_eq!(
        values.iter().map(|b| b >> 52).collect::<HashSet<_>>(),
        (0..2047).collect()
    );
    assert!(values.iter().all(|b| *b > 0 && *b <= MAX_BITS));
}

#[test]
fn mt_reference_word_assembly_vector() {
    // Historical runtime's published cross-platform getrandbits(100) vector:
    // https://github.com/python/cpython/blob/v3.13.14/Lib/test/test_random.py
    let mut rng = samples::Mt19937::seeded(&[1234567]);
    let value = BigUint::from(rng.get_bits(32))
        + (BigUint::from(rng.get_bits(32)) << 32usize)
        + (BigUint::from(rng.get_bits(32)) << 64usize)
        + (BigUint::from(rng.get_bits(4)) << 96usize);
    assert_eq!(value.to_string(), "97904845777343510404718956115");
}

#[test]
fn mt_reference_long_stream_vectors() {
    let mut rng = samples::Mt19937::seeded(&[61731, 24903, 614, 42143]);
    let mut final_values = Vec::new();
    for i in 0..2000 {
        let value = ((rng.get_bits(27)) << 26) + rng.get_bits(26);
        if i >= 1990 {
            final_values.push(value);
        }
    }
    assert_eq!(
        final_values,
        [
            0x0eab3258d2231f,
            0x1b89db315277a5,
            0x1db622a5518016,
            0x0b7f9af0d575bf,
            0x029e4c4db82240,
            0x04961892f5d673,
            0x02b291598e4589,
            0x11388382c15694,
            0x02dad977c9e1fe,
            0x191d96d4d334c6
        ]
    );
}

#[test]
fn randomized_exact_values_and_midpoint_cells() {
    // Expectations come from selected bit patterns and interval parity, not any
    // float text decoder. The corpus itself is the original deterministic sample.
    for (i, b) in corpus::sample_bits(&corpus::root())
        .unwrap()
        .into_iter()
        .enumerate()
    {
        let value = oracle::dyadic(b);
        assert_eq!(oracle::bits(&value.decimal_text()).unwrap(), Some(b));
        if i % 10 == 0 && b < MAX_BITS {
            let next = oracle::dyadic(b + 1);
            let mid = value.midpoint(&next);
            let expected = if b % 2 == 0 { b } else { b + 1 };
            assert_eq!(oracle::bits(&mid.decimal_text()).unwrap(), Some(expected));
            assert_eq!(
                oracle::bits(&value.midpoint(&mid).decimal_text()).unwrap(),
                Some(b)
            );
            assert_eq!(
                oracle::bits(&mid.midpoint(&next).decimal_text()).unwrap(),
                Some(b + 1)
            );
            assert!(
                !oracle::certify(
                    &mid.decimal_text(),
                    Some(if expected == b { b + 1 } else { b })
                )
                .unwrap()
            );
        }
    }
}

#[test]
fn archived_evidence_is_byte_identical() {
    let directory = corpus::root().join("harness/verification/numeric-before-rust");
    let manifest = corpus::read_json(&directory.join("manifest.json")).unwrap();
    for file in manifest["files"].as_array().unwrap() {
        assert_eq!(
            corpus::digest(&fs::read(directory.join(file["file"].as_str().unwrap())).unwrap()),
            file["sha256"].as_str().unwrap()
        );
    }
}

#[test]
fn comparator_rejects_deliberately_wrong_observations() {
    let v = validator();
    let raw = example();
    let independent = validation::analyse(&raw, &v).unwrap();
    let bits = independent["tempo_bits"].as_str().unwrap();
    let case = corpus::Case {
        id: "test".into(),
        raw: raw.clone(),
        schema_valid: Some(true),
        score_valid: true,
        numeric_valid: Some(true),
        bits: Some(bits.into()),
    };
    let actual = json!({"status":"valid","bits":bits,"serialized":raw,"roundtrip":{"status":"valid","bits":bits}});
    let correct = json!({"from_str":actual,"from_reader":actual});
    assert!(
        runner::compare(&case, &independent, &correct, &v)
            .unwrap()
            .is_empty()
    );
    for (pointer, value) in [
        ("/from_str/status", json!("invalid")),
        ("/from_str/bits", json!("0000000000000001")),
        ("/from_str/roundtrip/bits", json!("0000000000000001")),
    ] {
        let mut wrong = correct.clone();
        *wrong.pointer_mut(pointer).unwrap() = value;
        assert!(
            !runner::compare(&case, &independent, &wrong, &v)
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn tool_errors_are_not_invalid_data() {
    let temporary =
        std::env::temp_dir().join(format!("skysheet-tool-error-{}.json", std::process::id()));
    let output = Command::new(env!("CARGO_BIN_EXE_skysheet-numeric-validation"))
        .args(["--case", "does-not-exist", "--report"])
        .arg(&temporary)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    let data: Value = serde_json::from_slice(&fs::read(&temporary).unwrap()).unwrap();
    assert_eq!(data["status"], "tool_error");
    fs::remove_file(temporary).unwrap();
}
