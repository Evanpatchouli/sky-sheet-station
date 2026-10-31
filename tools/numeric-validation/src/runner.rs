use crate::{
    corpus::{self, Case},
    oracle::MAX_BITS,
    validation,
};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

pub fn compare(
    case: &Case,
    independent: &Value,
    observation: &Value,
    validator: &jsonschema::Validator,
) -> Result<Vec<Value>, String> {
    let mut differences = Vec::new();
    for path in ["from_str", "from_reader"] {
        let actual = &observation[path];
        let status = actual["status"].as_str().ok_or("probe status missing")?;
        if (status == "valid") != case.score_valid {
            differences.push(json!({"check":format!("{path}/acceptance"),"expected":case.score_valid,"actual":actual}));
            continue;
        }
        if !case.score_valid {
            continue;
        }
        let expected = case
            .bits
            .as_deref()
            .or(independent["tempo_bits"].as_str())
            .ok_or("expected bits missing")?;
        if actual["bits"] != expected {
            differences.push(json!({"check":format!("{path}/decode_bits"),"expected":expected,"actual":actual["bits"]}));
        }
        let saved = validation::analyse(
            actual["serialized"]
                .as_str()
                .ok_or("serialization missing")?,
            validator,
        )?;
        if saved["status"] != "valid" || saved["tempo_bits"] != actual["bits"] {
            differences.push(json!({"check":format!("{path}/serialization"),"expected":actual["bits"],"actual":saved}));
        }
        if actual["roundtrip"]["status"] != "valid" || actual["roundtrip"]["bits"] != actual["bits"]
        {
            differences.push(json!({"check":format!("{path}/roundtrip"),"expected":actual["bits"],"actual":actual["roundtrip"]}));
        }
    }
    Ok(differences)
}

fn build_probe(root: &Path) -> Result<(PathBuf, String), String> {
    let features = Command::new("cargo")
        .args(["tree", "--locked", "-e", "features", "-p", "skysheet-score"])
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !features.status.success() {
        return Err("cannot inspect root Cargo features".into());
    }
    let tree = String::from_utf8(features.stdout).map_err(|e| e.to_string())?;
    for forbidden in [
        "serde_json feature \"float_roundtrip\"",
        "serde_json feature \"arbitrary_precision\"",
    ] {
        if tree.contains(forbidden) {
            return Err(format!("DUT feature isolation violated: {forbidden}"));
        }
    }
    let target = root.join("target/formal-numeric-probe");
    let status = Command::new("cargo")
        .args([
            "build",
            "--quiet",
            "--locked",
            "-p",
            "skysheet-score",
            "--example",
            "numeric_probe",
            "--target-dir",
        ])
        .arg(&target)
        .current_dir(root)
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("root observation bridge build failed".into());
    }
    Ok((
        target.join("debug/examples").join(if cfg!(windows) {
            "numeric_probe.exe"
        } else {
            "numeric_probe"
        }),
        tree,
    ))
}

fn baseline_equivalence(root: &Path, failures: &[Value]) -> Result<Value, String> {
    let old = corpus::read_json(
        &root.join("harness/verification/numeric-before-rust/numeric-conformance-report.json"),
    )?;
    // Compare semantic findings, not incidental error wording or column offsets.
    // This audit never alters the primary gate's failures or approved expectations.
    fn signatures(failures: &[Value]) -> Result<Vec<Value>, String> {
        failures
            .iter()
            .map(|failure| {
                let differences = failure["differences"]
                    .as_array()
                    .ok_or("differences missing")?
                    .iter()
                    .map(|d| {
                        let actual = if d["check"].as_str().unwrap_or("").ends_with("/acceptance") {
                            d["actual"]["status"].clone()
                        } else {
                            d["actual"].clone()
                        };
                        json!({"check":d["check"],"expected":d["expected"],"actual":actual})
                    })
                    .collect::<Vec<_>>();
                Ok(json!({"id":failure["id"],"differences":differences}))
            })
            .collect()
    }
    let before = signatures(
        old["failures"]
            .as_array()
            .ok_or("old report missing failures")?,
    )?;
    let after = signatures(failures)?;
    Ok(
        json!({"matches":before==after,"historical_count":before.len(),"current_count":after.len(),
        "comparison":"ordered failure IDs/checks/expected bits/actual bits and acceptance status; diagnostics wording excluded"}),
    )
}

pub fn run(root: &Path, args: &[String]) -> Result<i32, String> {
    let mut independent_only = false;
    let mut selected = None;
    let mut count = 10000usize;
    let mut report = root.join("target/numeric-conformance-report.json");
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--independent-only" => independent_only = true,
            "--case" | "--samples" | "--report" => {
                let value = args.get(i + 1).ok_or("missing argument value")?;
                match args[i].as_str() {
                    "--case" => selected = Some(value.clone()),
                    "--samples" => count = value.parse().map_err(|_| "invalid sample count")?,
                    _ => report = PathBuf::from(value),
                }
                i += 1;
            }
            _ => return Err(format!("unknown argument {}", args[i])),
        }
        i += 1;
    }
    if count > 10000 {
        return Err("frozen historical corpus contains exactly 10000 samples".into());
    }
    let result = execute(root, independent_only, selected.as_deref(), count);
    let data = match &result {
        Ok(data) => data.clone(),
        Err(error) => json!({"status":"tool_error","error":error}),
    };
    if let Some(parent) = report.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(&report, serde_json::to_string_pretty(&data).unwrap() + "\n")
        .map_err(|e| e.to_string())?;
    let data = result?;
    println!(
        "{}: {} fixed cases, {} samples, {} failing case records; Rust checked: {}",
        data["status"],
        data["fixed_cases"],
        data["samples"],
        data["failures"].as_array().unwrap().len(),
        data["rust_checked"]
    );
    for fail in data["failures"].as_array().unwrap().iter().take(3) {
        println!("{}: {}", fail["id"], fail["differences"]);
    }
    println!("Report: {}", report.display());
    Ok(if data["status"] == "passed" { 0 } else { 1 })
}

fn execute(
    root: &Path,
    independent_only: bool,
    selected: Option<&str>,
    count: usize,
) -> Result<Value, String> {
    let validator = validation::validator(&corpus::read_json(
        &root.join("docs/schemas/sky-score-v1.schema.json"),
    )?)?;
    let mut fixed = corpus::fixed(root)?;
    let mut samples = if independent_only || selected.is_some() {
        Vec::new()
    } else {
        corpus::sample_bits(root)?.into_iter().take(count).collect()
    };
    if let Some(id) = selected {
        if let Some(hex) = id.strip_prefix("sample/") {
            if independent_only {
                return Err("sample case requires Rust serializer".into());
            }
            let bits = u64::from_str_radix(hex, 16).map_err(|e| e.to_string())?;
            if !(1..=MAX_BITS).contains(&bits) {
                return Err("sample is not finite positive binary64".into());
            }
            fixed.clear();
            samples.push(bits);
        } else {
            fixed.retain(|c| c.id == id);
            if fixed.is_empty() {
                return Err(format!("unknown case: {id}"));
            }
        }
    }
    let mut failures = Vec::new();
    let mut requests = String::new();
    let mut expected = HashMap::new();
    for case in &fixed {
        let value = validation::analyse(&case.raw, &validator)?;
        let mut diff = Vec::new();
        if value["schema_valid"] != json!(case.schema_valid) {
            diff.push(
                json!({"check":"independent/schema","expected":case.schema_valid,"actual":value}),
            );
        }
        if case
            .numeric_valid
            .is_some_and(|v| v != (value["status"] == "valid"))
        {
            diff.push(
                json!({"check":"independent/numeric","expected":case.numeric_valid,"actual":value}),
            );
        }
        if case
            .bits
            .as_deref()
            .is_some_and(|b| value["tempo_bits"] != b)
        {
            diff.push(json!({"check":"independent/bits","expected":case.bits,"actual":value}));
        }
        if !diff.is_empty() {
            failures.push(json!({"id":case.id,"raw":case.raw,"differences":diff}));
        }
        requests.push_str(&serde_json::to_string(&json!({"id":case.id,"raw":case.raw})).unwrap());
        requests.push('\n');
        expected.insert(case.id.clone(), (case.clone(), Some(value)));
    }
    for b in &samples {
        let id = format!("sample/{b:016x}");
        let bits = format!("{b:016x}");
        requests.push_str(&serde_json::to_string(&json!({"id":id,"bits":bits})).unwrap());
        requests.push('\n');
        expected.insert(
            id.clone(),
            (
                Case {
                    id,
                    raw: String::new(),
                    schema_valid: Some(true),
                    score_valid: true,
                    numeric_valid: Some(true),
                    bits: Some(bits),
                },
                None,
            ),
        );
    }
    let mut feature_tree = String::new();
    if !independent_only {
        let (executable, tree) = build_probe(root)?;
        feature_tree = tree;
        let mut child = Command::new(executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;
        let mut input = child.stdin.take().ok_or("no bridge stdin")?;
        // Concurrent write/drain avoids deadlock on large JSONL corpora.
        let writer = std::thread::spawn(move || input.write_all(requests.as_bytes()));
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        writer
            .join()
            .map_err(|_| "bridge input thread failed")?
            .map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err("Rust observation bridge failed".into());
        }
        let text = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
        let mut seen = HashSet::new();
        for line in text.lines() {
            let observed: Value = serde_json::from_str(line).map_err(|e| e.to_string())?;
            let id = observed["id"].as_str().ok_or("response id missing")?;
            if !seen.insert(id.to_owned()) {
                return Err("duplicate bridge response".into());
            }
            let (case, result) = expected.get(id).ok_or("unexpected bridge response")?;
            let result = match result {
                Some(v) => v.clone(),
                None => validation::analyse(
                    observed["raw"].as_str().ok_or("probe raw missing")?,
                    &validator,
                )?,
            };
            if case.raw.is_empty()
                && (result["status"] != "valid" || result["tempo_bits"] != json!(case.bits))
            {
                failures.push(json!({"id":id,"raw":observed["raw"],"differences":[{"check":"in_memory/serialization","expected":case.bits,"actual":result}]}));
            }
            let differences = compare(case, &result, &observed, &validator)?;
            if !differences.is_empty() {
                failures.push(json!({"id":id,"raw":observed["raw"],"token":result.get("token"),"differences":differences}));
            }
        }
        if seen.len() != expected.len() {
            return Err("incomplete bridge response".into());
        }
    }
    let baseline = if !independent_only && selected.is_none() && count == 10000 {
        baseline_equivalence(root, &failures)?
    } else {
        Value::Null
    };
    Ok(
        json!({"status":if failures.is_empty() {"passed"} else {"differences"},
        "rust_checked":!independent_only,"fixed_cases":fixed.len(),"samples":samples.len(),"seed":crate::samples::SEED,
        "scope":"raw input, Schema and numeric oracle; not a full independent score-semantic validator",
        "oracle":"exact rational ordered-bit interval search with rounding-cell certificates",
        "root_feature_tree":feature_tree,"tool_lock_sha256":corpus::digest(&fs::read(root.join("tools/numeric-validation/Cargo.lock")).map_err(|e|e.to_string())?),
        "baseline_equivalence":baseline,"failures":failures}),
    )
}
