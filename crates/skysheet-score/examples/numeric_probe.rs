//! Development-only JSONL observation bridge. No expected results or parser
//! feature overrides live here; the isolated Rust tool evaluates original text.
use serde::Deserialize;
use serde_json::{Value, json};
use skysheet_score::Score;
use std::io::{self, BufRead, Write};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: String,
    raw: Option<String>,
    bits: Option<String>,
}

fn observe(parsed: Result<Score, serde_json::Error>, reader_path: bool) -> Value {
    let score = match parsed {
        Ok(score) => score,
        Err(error) => {
            return json!({"status": "invalid", "stage": "parse", "error": error.to_string()});
        }
    };
    if let Err(error) = score.validate() {
        return json!({"status": "invalid", "stage": "semantic", "error": error.to_string(),
            "bits": format!("{:016x}", score.timeline.tempo.to_bits())});
    }
    let encoded = serde_json::to_string(&score).expect("validated score serializes");
    let decoded = if reader_path {
        serde_json::from_reader::<_, Score>(encoded.as_bytes())
    } else {
        serde_json::from_str::<Score>(&encoded)
    };
    let roundtrip = match decoded {
        Ok(decoded) => match decoded.validate() {
            Ok(()) => {
                json!({"status": "valid", "bits": format!("{:016x}", decoded.timeline.tempo.to_bits())})
            }
            Err(error) => json!({"status": "invalid", "error": error.to_string()}),
        },
        Err(error) => json!({"status": "invalid", "error": error.to_string()}),
    };
    json!({"status": "valid", "bits": format!("{:016x}", score.timeline.tempo.to_bits()),
        "serialized": encoded, "roundtrip": roundtrip})
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut output = io::BufWriter::new(io::stdout().lock());
    for line in io::stdin().lock().lines() {
        let request: Request = serde_json::from_str(&line?)?;
        let raw = match (request.raw, request.bits) {
            (Some(raw), None) => raw,
            (None, Some(bits)) => {
                let mut score: Score = serde_json::from_str(include_str!(
                    "../../../docs/examples/sky-15-example.json"
                ))?;
                score.timeline.tempo = f64::from_bits(u64::from_str_radix(&bits, 16)?);
                score.validate()?;
                serde_json::to_string(&score)?
            }
            _ => return Err("provide exactly one of raw or bits".into()),
        };
        let response = json!({"id": request.id, "raw": raw,
            "from_str": observe(serde_json::from_str(&raw), false),
            "from_reader": observe(serde_json::from_reader(raw.as_bytes()), true)});
        writeln!(output, "{response}")?;
    }
    Ok(())
}
