//! Additional N2 evidence, separate from the frozen 267/10,000 corpus.
//! Reproduces the 2026-11-01 research's exact 3,016 token constructions.
//! The DUT remains a separate Cargo process; all expectations use the old Oracle.
use serde_json::{Value, json};
use skysheet_numeric_validation::{corpus, oracle, runner, validation};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::Path,
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn padded(text: &str, pad: usize) -> String {
    let fractional = text.split_once('.').map_or(0, |(_, s)| s.len());
    let digits = text.replace('.', "");
    format!(
        "{}{}e-{}",
        digits.trim_start_matches('0'),
        "0".repeat(pad),
        fractional + pad
    )
}

fn research_tokens(root: &Path) -> Vec<(String, String)> {
    let mut tokens = Vec::new();
    for (i, b) in corpus::sample_bits(root).unwrap().into_iter().enumerate() {
        if i % 10 == 0 && b < oracle::MAX_BITS {
            let a = oracle::dyadic(b);
            let next = oracle::dyadic(b + 1);
            let mid = a.midpoint(&next);
            for (kind, x) in [
                ("below", a.midpoint(&mid)),
                ("tie", mid.clone()),
                ("above", mid.midpoint(&next)),
            ] {
                tokens.push((format!("extra/{b:016x}/{kind}"), x.decimal_text()));
            }
        }
    }
    assert_eq!(tokens.len(), 3000);
    for b in [
        0,
        1,
        2,
        (1 << 52) - 1,
        1 << 52,
        0x3ff0000000000000,
        0x3ff0000000000001,
        oracle::MAX_BITS - 1,
    ] {
        let exact = oracle::dyadic(b)
            .midpoint(&oracle::dyadic(b + 1))
            .decimal_text();
        for pad in [800, 1200] {
            tokens.push((format!("padded/{b:016x}/{pad}"), padded(&exact, pad)));
        }
    }
    assert_eq!(tokens.len(), 3016);
    tokens
}

// Certify zero expectations by constructing the exact zero cell's below/tie
// values. oracle::bits(None) alone also includes overflow and is insufficient.
fn zero_tokens() -> HashSet<String> {
    let zero = oracle::dyadic(0);
    let mid = zero.midpoint(&oracle::dyadic(1));
    let mut tokens = HashSet::from(["0".into(), "0.0".into(), "1e-400".into()]);
    for x in [zero.midpoint(&mid), mid] {
        let text = x.decimal_text();
        tokens.insert(text.clone());
        for pad in [0, 714, 715, 800, 1200] {
            tokens.insert(padded(&text, pad));
        }
    }
    tokens
}

fn check_batch(
    root: &Path,
    directory: &Path,
    label: &str,
    tokens: Vec<(String, String)>,
    decoded: &HashMap<String, Option<u64>>,
) {
    let template = fs::read_to_string(root.join("docs/examples/sky-15-example.json")).unwrap();
    let validator = validation::validator(
        &corpus::read_json(&root.join("docs/schemas/sky-score-v1.schema.json")).unwrap(),
    )
    .unwrap();
    let mut requests = String::new();
    let mut expected = HashMap::new();
    let zero = zero_tokens();
    let token_digest = corpus::digest(&serde_json::to_vec(&tokens).unwrap());
    if label == "research-3016" {
        assert_eq!(
            token_digest, "015273ae1ef01adcb2ef4578b635e9d13d5b014a4f9e9d264d76323b85a87f60",
            "research tokens/order changed"
        );
    }
    for (id, token) in tokens {
        let raw = template.replacen("\"tempo\": 120", &format!("\"tempo\": {token}"), 1);
        let bits = oracle::bits(&token).unwrap();
        let independent = validation::analyse(&raw, &validator).unwrap();
        assert_eq!(independent["status"] == "valid", bits.is_some(), "{id}");
        let case = corpus::Case {
            id: id.clone(),
            raw: raw.clone(),
            schema_valid: independent["schema_valid"].as_bool(),
            score_valid: bits.is_some(),
            numeric_valid: Some(bits.is_some()),
            bits: bits.map(|b| format!("{b:016x}")),
        };
        // Research zero ties also require decoded-zero evidence, not rejection alone.
        let decoded_bits = decoded
            .get(&id)
            .copied()
            .unwrap_or_else(|| bits.or_else(|| zero.contains(&token).then_some(0)));
        requests.push_str(&json!({"id":id,"raw":raw}).to_string());
        requests.push('\n');
        assert!(
            expected
                .insert(id, (case, independent, decoded_bits))
                .is_none()
        );
    }
    let executable = root
        .join("target/formal-numeric-probe/debug/examples")
        .join(format!("numeric_probe{}", std::env::consts::EXE_SUFFIX));
    let mut child = Command::new(executable)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let writer = std::thread::spawn(move || input.write_all(requests.as_bytes()).unwrap());
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    assert!(output.status.success());
    let mut seen = HashSet::new();
    let mut failures = Vec::new();
    let mut valid = 0;
    let mut decoded_count = 0;
    for line in std::str::from_utf8(&output.stdout).unwrap().lines() {
        let observation: Value = serde_json::from_str(line).unwrap();
        let id = observation["id"].as_str().unwrap();
        assert!(seen.insert(id.to_owned()), "duplicate response {id}");
        let (case, independent, decoded_bits) = expected.get(id).expect("unexpected response");
        let mut differences = runner::compare(case, independent, &observation, &validator).unwrap();
        if let Some(bits) = decoded_bits {
            decoded_count += 1;
            for path in ["from_str", "from_reader"] {
                if observation[path]["bits"] != format!("{bits:016x}") {
                    differences.push(json!({"check":format!("{path}/decoded_before_validation"),"expected":format!("{bits:016x}"),"actual":observation[path]}));
                }
            }
        }
        valid += usize::from(case.score_valid);
        if !differences.is_empty() {
            failures.push(json!({"id":id,"raw":case.raw,"differences":differences}));
        }
    }
    assert_eq!(seen.len(), expected.len(), "missing observations");
    let per_path = json!({"acceptance":seen.len(),"decode_bits":decoded_count,"serialization_oracle":valid,"readback_bits":valid});
    let report = json!({"status":if failures.is_empty(){"passed"}else{"differences"},"group":label,"cases":seen.len(),"valid":valid,"tokens_sha256":token_digest,"checks":{"from_str":per_path,"from_reader":per_path},"failures":failures});
    let path = directory.join(format!("{label}.json"));
    fs::write(&path, serde_json::to_string_pretty(&report).unwrap() + "\n").unwrap();
    println!(
        "{label}: {} cases, {valid} valid, {} failures; {}",
        seen.len(),
        failures.len(),
        path.display()
    );
    assert!(failures.is_empty(), "see {}", path.display());
}

#[test]
fn original_research_cases_and_focused_regressions() {
    let root = corpus::root();
    // Each run gets a new directory: neither historical nor earlier new reports
    // are overwritten. The unchanged runner performs the feature isolation check.
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = root.join(format!(
        "target/n2-regressions/{stamp}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&directory).unwrap();
    let report = directory.join("historical-max-repro.json");
    assert_eq!(
        runner::run(
            &root,
            &[
                "--case".into(),
                "tempo 1.7976931348623158e308".into(),
                "--report".into(),
                report.to_string_lossy().into_owned()
            ]
        )
        .unwrap(),
        0
    );

    check_batch(
        &root,
        &directory,
        "research-3016",
        research_tokens(&root),
        &HashMap::new(),
    );
    let fixture =
        corpus::read_json(&root.join("crates/skysheet-score/tests/fixtures/n2-tempo.json"))
            .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 84);
    let zeros = zero_tokens();
    let mut tokens = Vec::new();
    let mut decoded = HashMap::new();
    for case in cases {
        let token = case["token"].as_str().unwrap();
        let id = case["id"].as_str().unwrap();
        let positive = oracle::bits(token).unwrap();
        assert_eq!(case["valid"], positive.is_some(), "{id}");
        let expected = case["decoded_bits"]
            .as_str()
            .map(|s| u64::from_str_radix(s, 16).unwrap());
        let unsigned = token.strip_prefix('-').unwrap_or(token);
        let sign = if token.starts_with('-') { 1 << 63 } else { 0 };
        let magnitude = oracle::bits(unsigned).unwrap();
        let independent_bits = magnitude
            .or_else(|| zeros.contains(unsigned).then_some(0))
            .map(|b| b | sign);
        assert_eq!(expected, independent_bits, "frozen Oracle expectation {id}");
        tokens.push((id.to_owned(), token.to_owned()));
        assert!(decoded.insert(id.to_owned(), expected).is_none());
    }
    check_batch(&root, &directory, "focused-84", tokens, &decoded);
}
