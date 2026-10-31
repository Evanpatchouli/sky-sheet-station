use crate::{
    oracle::{self, Exact, MAX_BITS},
    samples,
};
use num_bigint::BigUint;
use num_traits::One;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Case {
    pub id: String,
    pub raw: String,
    pub schema_valid: Option<bool>,
    pub score_valid: bool,
    pub numeric_valid: Option<bool>,
    pub bits: Option<String>,
}

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?)
        .map_err(|e| e.to_string())
}

pub fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

pub fn generated_midpoints(root: &Path) -> Result<Vec<Case>, String> {
    let example = fs::read_to_string(root.join("docs/examples/sky-15-example.json"))
        .map_err(|e| e.to_string())?
        .replace("\r\n", "\n");
    let mut cases = Vec::new();
    for left in [
        0,
        1,
        2,
        (1 << 52) - 1,
        1 << 52,
        0x3fefffffffffffff,
        0x3ff0000000000000,
        0x3ff0000000000001,
        0x3fb999999999999a,
        MAX_BITS - 1,
        MAX_BITS,
    ] {
        let a = oracle::dyadic(left);
        let b = if left == MAX_BITS {
            Exact {
                n: BigUint::one() << 1024usize,
                d: BigUint::one(),
            }
        } else {
            oracle::dyadic(left + 1)
        };
        let midpoint = a.midpoint(&b);
        for (label, value, expected) in [
            ("below", a.midpoint(&midpoint), left),
            (
                "tie",
                midpoint.clone(),
                if left % 2 == 0 { left } else { left + 1 },
            ),
            ("above", midpoint.midpoint(&b), left + 1),
        ] {
            let valid = expected != 0 && expected <= MAX_BITS;
            cases.push(Case {
                id: format!("midpoint/{left:016x}/{label}"),
                raw: example.replacen(
                    "\"tempo\": 120",
                    &format!("\"tempo\": {}", value.decimal_text()),
                    1,
                ),
                schema_valid: Some(true),
                score_valid: valid,
                numeric_valid: Some(valid),
                bits: valid.then(|| format!("{expected:016x}")),
            });
        }
    }
    Ok(cases)
}

pub fn freeze(root: &Path) -> Result<(), String> {
    let folder = root.join("tools/numeric-validation/fixtures");
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let samples: Vec<String> = samples::historical()
        .iter()
        .map(|b| format!("{b:016x}"))
        .collect();
    let midpoints = generated_midpoints(root)?;
    // Existing evidence is a cross-check, never a source of oracle expectations.
    let old = read_json(
        &root.join("harness/verification/numeric-before-rust/numeric-conformance-report.json"),
    )?;
    let set: HashSet<_> = samples.iter().map(String::as_str).collect();
    for fail in old["failures"].as_array().ok_or("old failures missing")? {
        let id = fail["id"].as_str().ok_or("old failure id missing")?;
        if let Some(bits) = id.strip_prefix("sample/") {
            if !set.contains(bits) {
                return Err(format!("historical sample missing: {id}"));
            }
        } else if id.starts_with("midpoint/") {
            let case = midpoints
                .iter()
                .find(|c| c.id == id)
                .ok_or("historical midpoint missing")?;
            if case.raw != fail["raw"].as_str().ok_or("old raw missing")? {
                return Err(format!("midpoint text drift: {id}"));
            }
        }
    }
    let mut records = Vec::new();
    for (file, value) in [
        ("samples.json", json!(samples)),
        ("midpoints.json", json!(midpoints)),
    ] {
        let bytes =
            (serde_json::to_string_pretty(&value).map_err(|e| e.to_string())? + "\n").into_bytes();
        let destination = folder.join(file);
        if destination.exists() {
            return Err(format!(
                "refusing to overwrite frozen corpus {}",
                destination.display()
            ));
        }
        fs::write(&destination, &bytes).map_err(|e| e.to_string())?;
        records.push(json!({"file":file,"sha256":digest(&bytes)}));
    }
    let manifest = json!({"seed":samples::SEED,"sample_count":10000,"midpoint_count":33,
        "generator":"Rust recovery of legacy MT19937 integer seed/getrandbits/rejection stream; no interpreter", "files":records});
    fs::write(
        folder.join("manifest.json"),
        serde_json::to_string_pretty(&manifest).unwrap() + "\n",
    )
    .map_err(|e| e.to_string())
}

pub fn verify_manifest(root: &Path) -> Result<(), String> {
    let folder = root.join("tools/numeric-validation/fixtures");
    for record in read_json(&folder.join("manifest.json"))?["files"]
        .as_array()
        .ok_or("corpus manifest missing files")?
    {
        let file = record["file"].as_str().ok_or("manifest file missing")?;
        let data = fs::read(folder.join(file)).map_err(|e| e.to_string())?;
        if digest(&data) != record["sha256"].as_str().ok_or("manifest digest missing")? {
            return Err(format!("frozen corpus digest mismatch: {file}"));
        }
    }
    Ok(())
}

pub fn sample_bits(root: &Path) -> Result<Vec<u64>, String> {
    verify_manifest(root)?;
    read_json(&root.join("tools/numeric-validation/fixtures/samples.json"))?
        .as_array()
        .ok_or("samples not an array")?
        .iter()
        .map(|v| {
            u64::from_str_radix(v.as_str().ok_or("sample not hex string")?, 16)
                .map_err(|e| e.to_string())
        })
        .collect()
}

pub fn fixed(root: &Path) -> Result<Vec<Case>, String> {
    verify_manifest(root)?;
    let directory = root.join("crates/skysheet-score/tests/fixtures");
    let example = read_json(&root.join("docs/examples/sky-15-example.json"))?;
    let mut result = Vec::new();
    for file in [
        "contract-boundaries.json",
        "contract-conformance.json",
        "object-shapes.json",
    ] {
        for case in read_json(&directory.join(file))?
            .as_array()
            .ok_or("shared fixture not array")?
        {
            let mut value = example.clone();
            if let Some(removes) = case["removes"].as_array() {
                for p in removes {
                    let (parent, key) = p
                        .as_str()
                        .ok_or("bad removal")?
                        .rsplit_once('/')
                        .ok_or("bad pointer")?;
                    value
                        .pointer_mut(parent)
                        .and_then(Value::as_object_mut)
                        .ok_or("bad removal parent")?
                        .remove(key)
                        .ok_or("missing removed key")?;
                }
            }
            for (pointer, replacement) in case["changes"].as_object().ok_or("changes missing")? {
                if let Some(target) = value.pointer_mut(pointer) {
                    *target = replacement.clone();
                } else {
                    let (parent, key) = pointer.rsplit_once('/').ok_or("bad change pointer")?;
                    value
                        .pointer_mut(parent)
                        .and_then(Value::as_object_mut)
                        .ok_or("bad change parent")?
                        .insert(key.into(), replacement.clone());
                }
            }
            result.push(Case {
                id: format!(
                    "shared/{}",
                    case["name"].as_str().ok_or("case name missing")?
                ),
                raw: serde_json::to_string(&value).unwrap(),
                schema_valid: case["schema_valid"].as_bool(),
                score_valid: case["rust_error"].is_null(),
                numeric_valid: None,
                bits: None,
            });
        }
    }
    for case in read_json(&directory.join("numeric-inputs.json"))?
        .as_array()
        .ok_or("numeric fixtures not array")?
    {
        result.push(Case {
            id: case["id"].as_str().ok_or("id missing")?.into(),
            raw: case["raw"].as_str().ok_or("raw missing")?.into(),
            schema_valid: case["schema_valid"].as_bool(),
            score_valid: case["valid"].as_bool().ok_or("expectation missing")?,
            numeric_valid: case["valid"].as_bool(),
            bits: case["bits"].as_str().map(str::to_owned),
        });
    }
    let more: Vec<Case> = serde_json::from_value(read_json(
        &root.join("tools/numeric-validation/fixtures/midpoints.json"),
    )?)
    .map_err(|e| e.to_string())?;
    result.extend(more);
    if result.len() != 267 || result.iter().map(|c| &c.id).collect::<HashSet<_>>().len() != 267 {
        return Err("fixed corpus size/identity drift".into());
    }
    Ok(result)
}
