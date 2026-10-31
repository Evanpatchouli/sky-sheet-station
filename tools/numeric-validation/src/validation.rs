use crate::{oracle, raw_json};
use serde_json::{Value, json};

pub const MAX_TICK: u64 = (1 << 53) - 1;

pub fn validator(schema: &Value) -> Result<jsonschema::Validator, String> {
    // The checked-in Schema has only local $refs. Reject any external reference
    // before compilation, independently of library resolver defaults.
    fn check(value: &Value) -> Result<(), String> {
        match value {
            Value::Object(map) => {
                for (key, item) in map {
                    if matches!(key.as_str(), "$ref" | "$dynamicRef" | "$recursiveRef")
                        && !item.as_str().is_some_and(|s| s.starts_with('#'))
                    {
                        return Err("external Schema references disabled".into());
                    }
                    check(item)?;
                }
            }
            Value::Array(items) => {
                for item in items {
                    check(item)?;
                }
            }
            _ => (),
        }
        Ok(())
    }
    check(schema)?;
    jsonschema::draft202012::options()
        .build(schema)
        .map_err(|e| e.to_string())
}

pub fn unsigned_token(token: &str) -> bool {
    token == "0"
        || (matches!(token.as_bytes().first(), Some(b'1'..=b'9'))
            && token.bytes().all(|b| b.is_ascii_digit()))
}

pub fn analyse(raw: &str, validator: &jsonschema::Validator) -> Result<Value, String> {
    let document = match raw_json::parse(raw) {
        Ok(doc) => doc,
        Err(e) if e.stage == "tool_error" => {
            return Err(format!("{} at {}: {}", e.offset, e.path, e.reason));
        }
        Err(e) => {
            return Ok(json!({"status":"invalid", "schema_valid":null,
            "stage":e.stage,"path":e.path,"offset":e.offset,"reason":e.reason}));
        }
    };
    if let Some(error) = validator.iter_errors(&document.value).next() {
        return Ok(
            json!({"status":"invalid","schema_valid":false,"stage":"schema",
            "path":error.instance_path().to_string(),"reason":error.to_string()}),
        );
    }
    let mut fields = vec![
        "/timeline/ppq".to_owned(),
        "/timeline/time_signature/numerator".into(),
        "/timeline/time_signature/denominator".into(),
    ];
    let tracks = document.value["tracks"]
        .as_array()
        .ok_or("validated tracks is not an array")?;
    for (ti, track) in tracks.iter().enumerate() {
        for (ni, _) in track["notes"]
            .as_array()
            .ok_or("validated notes is not an array")?
            .iter()
            .enumerate()
        {
            fields.push(format!("/tracks/{ti}/notes/{ni}/start_tick"));
            fields.push(format!("/tracks/{ti}/notes/{ni}/duration_ticks"));
        }
    }
    for path in fields {
        let token = document
            .numbers
            .get(&path)
            .ok_or("validated integer has no numeric token")?;
        if !unsigned_token(token) {
            return Ok(
                json!({"status":"invalid","schema_valid":true,"stage":"raw","path":path,
                "reason":"plain unsigned integer token required","token":token}),
            );
        }
    }
    for (ti, track) in tracks.iter().enumerate() {
        for (ni, _) in track["notes"].as_array().unwrap().iter().enumerate() {
            let path = format!("/tracks/{ti}/notes/{ni}");
            let start = document.numbers[&(path.clone() + "/start_tick")]
                .parse::<u64>()
                .map_err(|e| e.to_string())?;
            let duration = document.numbers[&(path.clone() + "/duration_ticks")]
                .parse::<u64>()
                .map_err(|e| e.to_string())?;
            if start.checked_add(duration).is_none_or(|end| end > MAX_TICK) {
                return Ok(
                    json!({"status":"invalid","schema_valid":true,"stage":"numeric","path":path,
                    "reason":"derived note end exceeds MAX_TICK","token":format!("{start} + {duration}")}),
                );
            }
        }
    }
    let token = document
        .numbers
        .get("/timeline/tempo")
        .ok_or("validated tempo has no number token")?;
    match oracle::bits(token)? {
        Some(bits) => Ok(
            json!({"status":"valid","schema_valid":true,"tempo_bits":format!("{bits:016x}"),"token":token}),
        ),
        None => Ok(
            json!({"status":"invalid","schema_valid":true,"stage":"numeric","path":"/timeline/tempo",
            "reason":"not finite positive decoded binary64","token":token}),
        ),
    }
}
