//! Original-token scanner. Only JSON string unescaping and the Schema-only exact
//! Number representation use this workspace's serde_json. Numeric oracle input
//! is the original slice, never Number::as_f64 or the production deserializer.
use serde_json::{Map, Value};
use std::collections::HashMap;

pub struct Document {
    pub value: Value,
    pub numbers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct Error {
    pub stage: &'static str,
    pub path: String,
    pub offset: usize,
    pub reason: String,
}

pub fn pointer(path: &str, key: &str) -> String {
    format!("{path}/{}", key.replace('~', "~0").replace('/', "~1"))
}

pub fn parse(text: &str) -> Result<Document, Error> {
    let mut scanner = Scanner {
        text,
        pos: 0,
        numbers: HashMap::new(),
    };
    let value = scanner.value("", 0)?;
    scanner.space();
    if scanner.pos != text.len() {
        return Err(scanner.error("json", "", "trailing JSON input"));
    }
    Ok(Document {
        value,
        numbers: scanner.numbers,
    })
}

struct Scanner<'a> {
    text: &'a str,
    pos: usize,
    numbers: HashMap<String, String>,
}

impl Scanner<'_> {
    fn error(&self, stage: &'static str, path: &str, reason: impl Into<String>) -> Error {
        Error {
            stage,
            path: path.into(),
            offset: self.pos,
            reason: reason.into(),
        }
    }
    fn peek(&self) -> Option<u8> {
        self.text.as_bytes().get(self.pos).copied()
    }
    fn space(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\r' | b'\t')) {
            self.pos += 1;
        }
    }
    fn eat(&mut self, b: u8) -> bool {
        if self.peek() == Some(b) {
            self.pos += 1;
            true
        } else {
            false
        }
    }
    fn string(&mut self, path: &str) -> Result<String, Error> {
        let start = self.pos;
        if !self.eat(b'"') {
            return Err(self.error("json", path, "expected JSON string"));
        }
        while let Some(b) = self.peek() {
            self.pos += 1;
            if b == b'\\' {
                if self.peek().is_some() {
                    self.pos += 1;
                }
            } else if b == b'"' {
                return serde_json::from_str(&self.text[start..self.pos])
                    .map_err(|e| self.error("json", path, e.to_string()));
            }
        }
        Err(self.error("json", path, "unterminated JSON string"))
    }
    fn digits(&mut self) -> usize {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - start
    }
    fn number(&mut self, path: &str) -> Result<Value, Error> {
        let start = self.pos;
        self.eat(b'-');
        if self.eat(b'0') {
            if matches!(self.peek(), Some(b'0'..=b'9')) {
                return Err(self.error("json", path, "leading zero"));
            }
        } else if self.digits() == 0 {
            return Err(self.error("json", path, "missing number digits"));
        }
        if self.eat(b'.') && self.digits() == 0 {
            return Err(self.error("json", path, "missing fractional digits"));
        }
        if self.eat(b'e') || self.eat(b'E') {
            if !self.eat(b'+') {
                self.eat(b'-');
            }
            if self.digits() == 0 {
                return Err(self.error("json", path, "missing exponent digits"));
            }
        }
        let token = &self.text[start..self.pos];
        self.numbers.insert(path.into(), token.into());
        serde_json::from_str(token).map_err(|e| {
            self.error(
                "tool_error",
                path,
                format!("Schema number representation rejected a lexical JSON number: {e}"),
            )
        })
    }
    fn value(&mut self, path: &str, depth: usize) -> Result<Value, Error> {
        if depth > 512 {
            return Err(self.error(
                "tool_error",
                path,
                "scanner depth capacity (not a format limit)",
            ));
        }
        self.space();
        match self.peek() {
            Some(b'{') => {
                self.pos += 1;
                self.space();
                let mut members = Map::new();
                if self.eat(b'}') {
                    return Ok(Value::Object(members));
                }
                loop {
                    self.space();
                    let key = self.string(path)?;
                    let location = pointer(path, &key);
                    if members.contains_key(&key) {
                        return Err(self.error("raw", &location, "duplicate decoded member"));
                    }
                    self.space();
                    if !self.eat(b':') {
                        return Err(self.error("json", &location, "expected colon"));
                    }
                    members.insert(key, self.value(&location, depth + 1)?);
                    self.space();
                    if self.eat(b'}') {
                        break;
                    }
                    if !self.eat(b',') {
                        return Err(self.error("json", path, "expected object delimiter"));
                    }
                }
                Ok(Value::Object(members))
            }
            Some(b'[') => {
                self.pos += 1;
                self.space();
                let mut items = Vec::new();
                if self.eat(b']') {
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.value(&pointer(path, &items.len().to_string()), depth + 1)?);
                    self.space();
                    if self.eat(b']') {
                        break;
                    }
                    if !self.eat(b',') {
                        return Err(self.error("json", path, "expected array delimiter"));
                    }
                }
                Ok(Value::Array(items))
            }
            Some(b'"') => Ok(Value::String(self.string(path)?)),
            Some(b'-' | b'0'..=b'9') => self.number(path),
            _ => {
                for (word, value) in [
                    ("true", Value::Bool(true)),
                    ("false", Value::Bool(false)),
                    ("null", Value::Null),
                ] {
                    if self.text[self.pos..].starts_with(word) {
                        self.pos += word.len();
                        return Ok(value);
                    }
                }
                Err(self.error("json", path, "expected JSON value"))
            }
        }
    }
}
