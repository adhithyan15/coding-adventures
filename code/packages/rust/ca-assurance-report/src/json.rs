//! # Minimal Bounded JSON Parser & Serializer
//!
//! Zero third-party dependencies: implements recursive descent JSON parsing
//! and canonical indented serialization using standard library Rust only.

#![deny(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

/// Maximum allowed JSON nesting depth to prevent stack overflow DoS.
pub const MAX_JSON_DEPTH: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<JsonValue>),
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            JsonValue::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && *n >= (i64::MIN as f64) && *n <= (i64::MAX as f64) {
                    Some(*n as i64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn as_u32(&self) -> Option<u32> {
        self.as_i64().and_then(|v| u32::try_from(v).ok())
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            JsonValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            JsonValue::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            JsonValue::Object(o) => Some(o),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        self.as_object().and_then(|o| o.get(key))
    }

    pub fn parse(input: &str) -> Result<Self, String> {
        let mut parser = JsonParser::new(input);
        let val = parser.parse_value(0)?;
        parser.skip_whitespace();
        if parser.pos < parser.chars.len() {
            return Err(format!(
                "unexpected trailing character at offset {}",
                parser.pos
            ));
        }
        Ok(val)
    }

    pub fn to_pretty_string(&self, indent: usize) -> String {
        let mut out = String::new();
        self.format_pretty(&mut out, 0, indent);
        out
    }

    fn format_pretty(&self, out: &mut String, depth: usize, indent: usize) {
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 {
                    out.push_str(&format!("{:.0}", n));
                } else {
                    out.push_str(&format!("{}", n));
                }
            }
            JsonValue::String(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        '\n' => out.push_str("\\n"),
                        '\r' => out.push_str("\\r"),
                        '\t' => out.push_str("\\t"),
                        c if (c as u32) <= 0x1f => {
                            out.push_str(&format!("\\u{:04x}", c as u32));
                        }
                        other => out.push(other),
                    }
                }
                out.push('"');
            }
            JsonValue::Array(arr) => {
                if arr.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                let pad = " ".repeat((depth + 1) * indent);
                for (i, item) in arr.iter().enumerate() {
                    out.push_str(&pad);
                    item.format_pretty(out, depth + 1, indent);
                    if i + 1 < arr.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&" ".repeat(depth * indent));
                out.push(']');
            }
            JsonValue::Object(obj) => {
                if obj.is_empty() {
                    out.push_str("{}");
                    return;
                }
                out.push_str("{\n");
                let pad = " ".repeat((depth + 1) * indent);
                let count = obj.len();
                for (i, (k, v)) in obj.iter().enumerate() {
                    out.push_str(&pad);
                    out.push('"');
                    out.push_str(k);
                    out.push_str("\": ");
                    v.format_pretty(out, depth + 1, indent);
                    if i + 1 < count {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push_str(&" ".repeat(depth * indent));
                out.push('}');
            }
        }
    }
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_pretty_string(2))
    }
}

struct JsonParser<'a> {
    chars: Vec<char>,
    pos: usize,
    _phantom: std::marker::PhantomData<&'a str>,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, String> {
        if depth > MAX_JSON_DEPTH {
            return Err(format!("JSON nesting depth exceeded maximum ({MAX_JSON_DEPTH})"));
        }
        self.skip_whitespace();
        let c = self
            .peek()
            .ok_or_else(|| "unexpected end of JSON input".to_string())?;

        match c {
            'n' => self.parse_null(),
            't' | 'f' => self.parse_bool(),
            '"' => self.parse_string().map(JsonValue::String),
            '[' => self.parse_array(depth),
            '{' => self.parse_object(depth),
            '-' | '0'..='9' => self.parse_number(),
            other => Err(format!(
                "unexpected character '{other}' at offset {}",
                self.pos
            )),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        if self.match_keyword("null") {
            Ok(JsonValue::Null)
        } else {
            Err(format!("invalid literal at offset {}", self.pos))
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, String> {
        if self.match_keyword("true") {
            Ok(JsonValue::Bool(true))
        } else if self.match_keyword("false") {
            Ok(JsonValue::Bool(false))
        } else {
            Err(format!("invalid boolean at offset {}", self.pos))
        }
    }

    fn match_keyword(&mut self, kw: &str) -> bool {
        let len = kw.chars().count();
        if self.pos + len <= self.chars.len() {
            let slice: String = self.chars[self.pos..self.pos + len].iter().collect();
            if slice == kw {
                self.pos += len;
                return true;
            }
        }
        false
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.advance() != Some('"') {
            return Err("expected '\"'".into());
        }
        let mut s = String::new();
        while self.pos < self.chars.len() {
            match self.advance().unwrap() {
                '"' => return Ok(s),
                '\\' => {
                    let esc = self
                        .advance()
                        .ok_or_else(|| "unterminated string escape".to_string())?;
                    match esc {
                        '"' => s.push('"'),
                        '\\' => s.push('\\'),
                        '/' => s.push('/'),
                        'b' => s.push('\x08'),
                        'f' => s.push('\x0c'),
                        'n' => s.push('\n'),
                        'r' => s.push('\r'),
                        't' => s.push('\t'),
                        'u' => {
                            if self.pos + 4 > self.chars.len() {
                                return Err("truncated unicode escape".into());
                            }
                            let hex: String = self.chars[self.pos..self.pos + 4].iter().collect();
                            self.pos += 4;
                            let code = u32::from_str_radix(&hex, 16)
                                .map_err(|e| format!("invalid unicode escape: {e}"))?;
                            let ch = char::from_u32(code)
                                .ok_or_else(|| "invalid codepoint".to_string())?;
                            s.push(ch);
                        }
                        other => return Err(format!("unknown escape '\\{other}'")),
                    }
                }
                c => s.push(c),
            }
        }
        Err("unterminated string".into())
    }

    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        if self.peek() == Some('-') {
            self.advance();
        }
        while let Some('0'..='9') = self.peek() {
            self.advance();
        }
        if self.peek() == Some('.') {
            self.advance();
            while let Some('0'..='9') = self.peek() {
                self.advance();
            }
        }
        if let Some('e' | 'E') = self.peek() {
            self.advance();
            if let Some('+' | '-') = self.peek() {
                self.advance();
            }
            while let Some('0'..='9') = self.peek() {
                self.advance();
            }
        }
        let raw: String = self.chars[start..self.pos].iter().collect();
        let num: f64 = raw
            .parse()
            .map_err(|e| format!("invalid number '{raw}': {e}"))?;
        Ok(JsonValue::Number(num))
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, String> {
        if self.advance() != Some('[') {
            return Err("expected '['".into());
        }
        self.skip_whitespace();
        let mut items = Vec::new();
        if self.peek() == Some(']') {
            self.advance();
            return Ok(JsonValue::Array(items));
        }

        loop {
            let item = self.parse_value(depth + 1)?;
            items.push(item);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some(']') => {
                    self.advance();
                    break;
                }
                other => {
                    return Err(format!(
                        "expected ',' or ']' in array, found {:?}",
                        other
                    ))
                }
            }
        }
        Ok(JsonValue::Array(items))
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, String> {
        if self.advance() != Some('{') {
            return Err("expected '{'".into());
        }
        self.skip_whitespace();
        let mut map = BTreeMap::new();
        if self.peek() == Some('}') {
            self.advance();
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            if self.peek() != Some('"') {
                return Err(format!(
                    "expected string key in object at offset {}",
                    self.pos
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.advance() != Some(':') {
                return Err(format!("expected ':' after key '{key}'"));
            }
            let val = self.parse_value(depth + 1)?;
            // A duplicate key is ambiguous (parsers disagree on which value
            // wins), so a document containing one is rejected outright.
            if map.insert(key.clone(), val).is_some() {
                return Err(format!("duplicate key '{key}' in object"));
            }
            self.skip_whitespace();
            match self.peek() {
                Some(',') => {
                    self.advance();
                }
                Some('}') => {
                    self.advance();
                    break;
                }
                other => {
                    return Err(format!(
                        "expected ',' or '}}' in object, found {:?}",
                        other
                    ))
                }
            }
        }
        Ok(JsonValue::Object(map))
    }
}
