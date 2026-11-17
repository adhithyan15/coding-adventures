//! Deterministic canonical JSON serialization and deserialization (CWE-674 / CWE-400 / CWE-20).

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use musical_instruments::{AdsrEnvelope, HarmonicPartial};

use crate::error::MusicMachineError;
use crate::limits::*;
use crate::types::*;

/// A minimal JSON value representation without external dependencies.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// null
    Null,
    /// true or false
    Bool(bool),
    /// Finite 64-bit float
    Number(f64),
    /// UTF-8 string
    String(String),
    /// Ordered sequence of values
    Array(Vec<JsonValue>),
    /// Key-value map with deterministic alphabetical key ordering
    Object(BTreeMap<String, JsonValue>),
}

impl JsonValue {
    /// Try to borrow as a string slice.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Try to extract a float number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            _ => None,
        }
    }

    /// Try to extract a non-negative integer tick.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Self::Number(n) => {
                if *n >= 0.0 && n.fract() == 0.0 && *n <= (u64::MAX as f64) {
                    Some(*n as u64)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Try to extract a positive integer.
    pub fn as_u32(&self) -> Option<u32> {
        match self {
            Self::Number(n) => {
                if *n >= 0.0 && n.fract() == 0.0 && *n <= (u32::MAX as f64) {
                    Some(*n as u32)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Try to borrow as an array slice.
    pub fn as_array(&self) -> Option<&[JsonValue]> {
        match self {
            Self::Array(a) => Some(a.as_slice()),
            _ => None,
        }
    }

    /// Try to borrow as an object map.
    pub fn as_object(&self) -> Option<&BTreeMap<String, JsonValue>> {
        match self {
            Self::Object(m) => Some(m),
            _ => None,
        }
    }
}

/// Format a float canonically to match Python's `json.dumps(allow_nan=False)`.
pub fn format_json_float(val: f64) -> String {
    let s = format!("{val}");
    if !s.contains('.') && !s.contains('e') && !s.contains('E') {
        format!("{s}.0")
    } else {
        s
    }
}

/// Format a JSON string with standard escaping.
pub fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\x08' => out.push_str("\\b"),
            '\x0C' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// ============================================================================
// Deterministic Serialization (matching canonical_score.json)
// ============================================================================

impl PortableScore {
    /// Deterministic canonical JSON serialization matching `canonical_score.json`.
    pub fn to_json(&self) -> String {
        let mut out = String::new();
        out.push_str("{\n");

        // 1. "events": [...]
        out.push_str("  \"events\": [\n");
        for (i, ev) in self.events.iter().enumerate() {
            out.push_str("    {\n");
            // Fields in alphabetical order:
            // "duration_tick", ["hit_id"], "kind", "notes", "source_order", "start_tick", "track_id", "velocity"
            out.push_str(&format!("      \"duration_tick\": {},\n", ev.duration_tick));
            if let Some(ref hit) = ev.hit_id {
                out.push_str(&format!("      \"hit_id\": {},\n", escape_json_string(hit)));
            }
            out.push_str(&format!("      \"kind\": \"{}\",\n", ev.kind.as_str()));
            if ev.notes.is_empty() {
                out.push_str("      \"notes\": [],\n");
            } else {
                out.push_str("      \"notes\": [\n");
                for (ni, note) in ev.notes.iter().enumerate() {
                    let comma = if ni + 1 < ev.notes.len() { "," } else { "" };
                    out.push_str(&format!("        {}{}\n", escape_json_string(note), comma));
                }
                out.push_str("      ],\n");
            }
            out.push_str(&format!("      \"source_order\": {},\n", ev.source_order));
            out.push_str(&format!("      \"start_tick\": {},\n", ev.start_tick));
            out.push_str(&format!("      \"track_id\": {},\n", escape_json_string(&ev.track_id)));
            out.push_str(&format!("      \"velocity\": {}\n", format_json_float(ev.velocity)));

            let comma = if i + 1 < self.events.len() { "," } else { "" };
            out.push_str(&format!("    }}{comma}\n"));
        }
        out.push_str("  ],\n");

        // 2. "format_version": ...
        out.push_str(&format!("  \"format_version\": {},\n", escape_json_string(&self.format_version)));

        // 3. "instruments": [...]
        out.push_str("  \"instruments\": [\n");
        for (i, inst) in self.instruments.iter().enumerate() {
            out.push_str("    {\n");
            // Alphabetical order:
            // For preset: "gain", "id", "kind", "profile_id"
            // For drumkit: "gain", "id", "kind", "kit", "profile_id"
            // For additive: ["envelope"], "gain", ["harmonics"], "id", "kind", "profile_id"
            if let Some(ref env) = inst.envelope {
                out.push_str("      \"envelope\": [\n");
                out.push_str(&format!("        {},\n", format_json_float(env.attack_seconds)));
                out.push_str(&format!("        {},\n", format_json_float(env.decay_seconds)));
                out.push_str(&format!("        {},\n", format_json_float(env.sustain_level)));
                out.push_str(&format!("        {}\n", format_json_float(env.release_seconds)));
                out.push_str("      ],\n");
            }
            out.push_str(&format!("      \"gain\": {},\n", format_json_float(inst.gain)));
            if let Some(ref harmonics) = inst.harmonics {
                out.push_str("      \"harmonics\": [\n");
                for (hi, h) in harmonics.iter().enumerate() {
                    out.push_str("        [\n");
                    out.push_str(&format!("          {},\n", format_json_float(h.frequency_multiplier)));
                    out.push_str(&format!("          {},\n", format_json_float(h.amplitude)));
                    out.push_str(&format!("          {}\n", format_json_float(h.phase_cycles)));
                    let hcomma = if hi + 1 < harmonics.len() { "," } else { "" };
                    out.push_str(&format!("        ]{hcomma}\n"));
                }
                out.push_str("      ],\n");
            }
            out.push_str(&format!("      \"id\": {},\n", escape_json_string(&inst.id)));
            out.push_str(&format!("      \"kind\": {},\n", escape_json_string(&inst.kind)));
            if let Some(ref kit) = inst.kit {
                out.push_str(&format!("      \"kit\": {},\n", escape_json_string(kit)));
            }
            out.push_str(&format!("      \"profile_id\": {}\n", escape_json_string(&inst.profile_id)));

            let comma = if i + 1 < self.instruments.len() { "," } else { "" };
            out.push_str(&format!("    }}{comma}\n"));
        }
        out.push_str("  ],\n");

        // 4. "meter_events": [...]
        out.push_str("  \"meter_events\": [\n");
        for (i, m) in self.meter_events.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"meter\": {},\n", escape_json_string(&m.meter)));
            out.push_str(&format!("      \"start_tick\": {}\n", m.start_tick));
            let comma = if i + 1 < self.meter_events.len() { "," } else { "" };
            out.push_str(&format!("    }}{comma}\n"));
        }
        out.push_str("  ],\n");

        // 5. "ppq": ...
        out.push_str(&format!("  \"ppq\": {},\n", self.ppq));

        // 6. "sample_rate_hz": ...
        out.push_str(&format!("  \"sample_rate_hz\": {},\n", self.sample_rate_hz));

        // 7. "tempo_events": [...]
        out.push_str("  \"tempo_events\": [\n");
        for (i, t) in self.tempo_events.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"bpm\": {},\n", format_json_float(t.bpm)));
            out.push_str(&format!("      \"start_tick\": {}\n", t.start_tick));
            let comma = if i + 1 < self.tempo_events.len() { "," } else { "" };
            out.push_str(&format!("    }}{comma}\n"));
        }
        out.push_str("  ],\n");

        // 8. "title": ...
        out.push_str(&format!("  \"title\": {},\n", escape_json_string(&self.title)));

        // 9. "tracks": [...]
        out.push_str("  \"tracks\": [\n");
        for (i, tr) in self.tracks.iter().enumerate() {
            out.push_str("    {\n");
            out.push_str(&format!("      \"id\": {},\n", escape_json_string(&tr.id)));
            out.push_str(&format!("      \"instrument_id\": {}\n", escape_json_string(&tr.instrument_id)));
            let comma = if i + 1 < self.tracks.len() { "," } else { "" };
            out.push_str(&format!("    }}{comma}\n"));
        }
        out.push_str("  ]\n");

        out.push('}');
        out
    }
}

// ============================================================================
// Secure JSON Parser (CWE-674 / CWE-400 / CWE-20)
// ============================================================================

struct JsonParser<'a> {
    input: &'a [u8],
    pos: usize,
    depth: usize,
    max_depth: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a [u8], max_depth: usize) -> Self {
        Self {
            input,
            pos: 0,
            depth: 0,
            max_depth,
        }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len() {
            match self.input[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip_whitespace();
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn parse_value(&mut self) -> Result<JsonValue, MusicMachineError> {
        let ch = self.peek().ok_or_else(|| {
            MusicMachineError::ParseError("unexpected end of JSON input".into())
        })?;

        match ch {
            b'n' => self.parse_null(),
            b't' | b'f' => self.parse_bool(),
            b'"' => self.parse_string().map(JsonValue::String),
            b'[' => self.parse_array(),
            b'{' => self.parse_object(),
            b'-' | b'0'..=b'9' => self.parse_number(),
            _ => Err(MusicMachineError::ParseError(format!(
                "unexpected character in JSON at byte {}: '{}'",
                self.pos, ch as char
            ))),
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, MusicMachineError> {
        if self.input[self.pos..].starts_with(b"null") {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(MusicMachineError::ParseError("expected 'null'".into()))
        }
    }

    fn parse_bool(&mut self) -> Result<JsonValue, MusicMachineError> {
        if self.input[self.pos..].starts_with(b"true") {
            self.pos += 4;
            Ok(JsonValue::Bool(true))
        } else if self.input[self.pos..].starts_with(b"false") {
            self.pos += 5;
            Ok(JsonValue::Bool(false))
        } else {
            Err(MusicMachineError::ParseError("expected boolean".into()))
        }
    }

    fn parse_string(&mut self) -> Result<String, MusicMachineError> {
        if self.pos >= self.input.len() || self.input[self.pos] != b'"' {
            return Err(MusicMachineError::ParseError("expected string starting with '\"'".into()));
        }
        self.pos += 1; // skip opening quote

        let mut s = String::new();
        while self.pos < self.input.len() {
            let b = self.input[self.pos];
            self.pos += 1;
            match b {
                b'"' => return Ok(s),
                b'\\' => {
                    if self.pos >= self.input.len() {
                        return Err(MusicMachineError::ParseError("unfinished escape sequence".into()));
                    }
                    let esc = self.input[self.pos];
                    self.pos += 1;
                    match esc {
                        b'"' => s.push('"'),
                        b'\\' => s.push('\\'),
                        b'/' => s.push('/'),
                        b'b' => s.push('\x08'),
                        b'f' => s.push('\x0C'),
                        b'n' => s.push('\n'),
                        b'r' => s.push('\r'),
                        b't' => s.push('\t'),
                        b'u' => {
                            if self.pos + 4 > self.input.len() {
                                return Err(MusicMachineError::ParseError("unfinished \\u escape".into()));
                            }
                            let hex_str = std::str::from_utf8(&self.input[self.pos..self.pos + 4])
                                .map_err(|_| MusicMachineError::ParseError("invalid hex in \\u".into()))?;
                            self.pos += 4;
                            let cp = u32::from_str_radix(hex_str, 16)
                                .map_err(|_| MusicMachineError::ParseError("invalid hex in \\u".into()))?;
                            let ch = char::from_u32(cp).ok_or_else(|| {
                                MusicMachineError::ParseError(format!("invalid unicode codepoint \\u{hex_str}"))
                            })?;
                            s.push(ch);
                        }
                        _ => {
                            return Err(MusicMachineError::ParseError(format!(
                                "invalid escape character '\\{}'",
                                esc as char
                            )))
                        }
                    }
                }
                c if c < 0x20 => {
                    return Err(MusicMachineError::ParseError(
                        "unescaped control character in JSON string".into(),
                    ));
                }
                _ => {
                    // Check UTF-8 byte
                    self.pos -= 1;
                    let rem = &self.input[self.pos..];
                    let ch = match std::str::from_utf8(rem) {
                        Ok(st) => {
                            let c = st.chars().next().unwrap();
                            self.pos += c.len_utf8();
                            c
                        }
                        Err(e) => {
                            if e.valid_up_to() > 0 {
                                let st = std::str::from_utf8(&rem[..e.valid_up_to()]).unwrap();
                                let c = st.chars().next().unwrap();
                                self.pos += c.len_utf8();
                                c
                            } else {
                                return Err(MusicMachineError::ParseError("invalid UTF-8 in string".into()));
                            }
                        }
                    };
                    s.push(ch);
                }
            }
        }
        Err(MusicMachineError::ParseError("unterminated string".into()))
    }

    fn parse_number(&mut self) -> Result<JsonValue, MusicMachineError> {
        let start = self.pos;
        if self.pos < self.input.len() && self.input[self.pos] == b'-' {
            self.pos += 1;
        }

        let int_start = self.pos;
        while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos == int_start {
            return Err(MusicMachineError::ParseError("invalid number: missing digits".into()));
        }

        // Leading zero check (e.g. 01 is forbidden in JSON)
        let int_part = &self.input[int_start..self.pos];
        if int_part.len() > 1 && int_part[0] == b'0' {
            return Err(MusicMachineError::ParseError("invalid number: leading zero".into()));
        }

        // Fractional part
        if self.pos < self.input.len() && self.input[self.pos] == b'.' {
            self.pos += 1;
            let frac_start = self.pos;
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == frac_start {
                return Err(MusicMachineError::ParseError("invalid number: decimal point without digits".into()));
            }
        }

        // Exponent
        if self.pos < self.input.len() && (self.input[self.pos] == b'e' || self.input[self.pos] == b'E') {
            self.pos += 1;
            if self.pos < self.input.len() && (self.input[self.pos] == b'+' || self.input[self.pos] == b'-') {
                self.pos += 1;
            }
            let exp_start = self.pos;
            while self.pos < self.input.len() && self.input[self.pos].is_ascii_digit() {
                self.pos += 1;
            }
            if self.pos == exp_start {
                return Err(MusicMachineError::ParseError("invalid number: exponent without digits".into()));
            }
        }

        let num_str = std::str::from_utf8(&self.input[start..self.pos])
            .map_err(|_| MusicMachineError::ParseError("invalid UTF-8 in number".into()))?;

        let val: f64 = num_str.parse().map_err(|_| {
            MusicMachineError::ParseError(format!("could not parse float '{num_str}'"))
        })?;

        if !val.is_finite() {
            return Err(MusicMachineError::InvalidArgument(format!(
                "non-finite JSON constants are not allowed: {num_str}"
            )));
        }

        Ok(JsonValue::Number(val))
    }

    fn parse_array(&mut self) -> Result<JsonValue, MusicMachineError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "JSON nesting depth exceeds MAX_JSON_DEPTH={MAX_JSON_DEPTH}"
            )));
        }

        self.pos += 1; // skip '['
        let mut items = Vec::new();

        self.skip_whitespace();
        if self.pos < self.input.len() && self.input[self.pos] == b']' {
            self.pos += 1;
            self.depth -= 1;
            return Ok(JsonValue::Array(items));
        }

        loop {
            let val = self.parse_value()?;
            items.push(val);

            self.skip_whitespace();
            if self.pos >= self.input.len() {
                return Err(MusicMachineError::ParseError("unterminated array".into()));
            }
            let ch = self.input[self.pos];
            if ch == b',' {
                self.pos += 1;
            } else if ch == b']' {
                self.pos += 1;
                break;
            } else {
                return Err(MusicMachineError::ParseError(format!(
                    "expected ',' or ']' in array, got '{}'",
                    ch as char
                )));
            }
        }

        self.depth -= 1;
        Ok(JsonValue::Array(items))
    }

    fn parse_object(&mut self) -> Result<JsonValue, MusicMachineError> {
        self.depth += 1;
        if self.depth > self.max_depth {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "JSON nesting depth exceeds MAX_JSON_DEPTH={MAX_JSON_DEPTH}"
            )));
        }

        self.pos += 1; // skip '{'
        let mut map = BTreeMap::new();

        self.skip_whitespace();
        if self.pos < self.input.len() && self.input[self.pos] == b'}' {
            self.pos += 1;
            self.depth -= 1;
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            if self.pos >= self.input.len() || self.input[self.pos] != b'"' {
                return Err(MusicMachineError::ParseError("expected string key in object".into()));
            }
            let key = self.parse_string()?;

            self.skip_whitespace();
            if self.pos >= self.input.len() || self.input[self.pos] != b':' {
                return Err(MusicMachineError::ParseError("expected ':' after object key".into()));
            }
            self.pos += 1; // skip ':'

            match map.entry(key) {
                Entry::Vacant(entry) => {
                    let val = self.parse_value()?;
                    entry.insert(val);
                }
                Entry::Occupied(_) => {
                    return Err(MusicMachineError::ParseError(
                        "duplicate JSON object member".into(),
                    ));
                }
            }

            self.skip_whitespace();
            if self.pos >= self.input.len() {
                return Err(MusicMachineError::ParseError("unterminated object".into()));
            }
            let ch = self.input[self.pos];
            if ch == b',' {
                self.pos += 1;
            } else if ch == b'}' {
                self.pos += 1;
                break;
            } else {
                return Err(MusicMachineError::ParseError(format!(
                    "expected ',' or '}}' in object, got '{}'",
                    ch as char
                )));
            }
        }

        self.depth -= 1;
        Ok(JsonValue::Object(map))
    }
}

/// Parse a JSON string into a `JsonValue` with explicit size and depth limits (CWE-400/CWE-674).
pub fn parse_json(text: &str, max_bytes: usize, max_depth: usize) -> Result<JsonValue, MusicMachineError> {
    if text.len() > max_bytes {
        return Err(MusicMachineError::BudgetExceeded(format!(
            "JSON text length {} exceeds max_bytes={max_bytes}",
            text.len()
        )));
    }
    let mut parser = JsonParser::new(text.as_bytes(), max_depth);
    let val = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.pos < parser.input.len() {
        return Err(MusicMachineError::ParseError(format!(
            "trailing data after JSON value at byte {}",
            parser.pos
        )));
    }
    Ok(val)
}

// ============================================================================
// Deserialization: JSON to PortableScore
// ============================================================================

impl PortableScore {
    /// Parse canonical JSON into a `PortableScore`.
    pub fn from_json(json_text: &str) -> Result<Self, MusicMachineError> {
        Self::from_json_with_limits(json_text, MAX_SCORE_BYTES, MAX_JSON_DEPTH)
    }

    /// Parse canonical JSON into a `PortableScore` with explicit resource bounds.
    pub fn from_json_with_limits(
        json_text: &str,
        max_bytes: usize,
        max_depth: usize,
    ) -> Result<Self, MusicMachineError> {
        let root = parse_json(json_text, max_bytes, max_depth)?;
        let map = root.as_object().ok_or_else(|| {
            MusicMachineError::InvalidArgument("JSON score root must be an object".into())
        })?;

        // Expected keys validation
        let expected_keys: [&str; 9] = [
            "events",
            "format_version",
            "instruments",
            "meter_events",
            "ppq",
            "sample_rate_hz",
            "tempo_events",
            "title",
            "tracks",
        ];
        let mut unknown_keys = Vec::new();
        for k in map.keys() {
            if !expected_keys.contains(&k.as_str()) {
                unknown_keys.push(k.clone());
            }
        }
        if !unknown_keys.is_empty() {
            unknown_keys.sort();
            return Err(MusicMachineError::InvalidArgument(format!(
                "unknown score keys: {}",
                unknown_keys.join(", ")
            )));
        }

        let format_version = map
            .get("format_version")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MusicMachineError::InvalidArgument("missing 'format_version'".into()))?;

        let title = map
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled");

        let ppq = map
            .get("ppq")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| MusicMachineError::InvalidArgument("missing or invalid 'ppq'".into()))?;

        let sample_rate_hz = map
            .get("sample_rate_hz")
            .and_then(|v| v.as_u32())
            .ok_or_else(|| MusicMachineError::InvalidArgument("missing or invalid 'sample_rate_hz'".into()))?;

        // Parse tempo_events
        let tempo_raw = map
            .get("tempo_events")
            .and_then(|v| v.as_array())
            .ok_or_else(|| MusicMachineError::InvalidArgument("tempo_events must be a list".into()))?;
        if tempo_raw.len() > MAX_EVENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "tempo_events count {} exceeds MAX_EVENTS={MAX_EVENTS}",
                tempo_raw.len()
            )));
        }
        let mut tempo_events = Vec::with_capacity(tempo_raw.len());
        for (idx, item) in tempo_raw.iter().enumerate() {
            let o = item.as_object().ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("tempo_events[{idx}] must be a mapping"))
            })?;
            let start_tick = o.get("start_tick").and_then(|v| v.as_u64()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("tempo_events[{idx}] missing 'start_tick'"))
            })?;
            let bpm = o.get("bpm").and_then(|v| v.as_f64()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("tempo_events[{idx}] missing 'bpm'"))
            })?;
            if bpm <= 0.0 || !bpm.is_finite() {
                return Err(MusicMachineError::InvalidArgument("bpm must be > 0.0".into()));
            }
            tempo_events.push(TempoEvent { start_tick, bpm });
        }

        // Parse meter_events
        let meter_raw = map
            .get("meter_events")
            .and_then(|v| v.as_array())
            .ok_or_else(|| MusicMachineError::InvalidArgument("meter_events must be a list".into()))?;
        if meter_raw.len() > MAX_EVENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "meter_events count {} exceeds MAX_EVENTS={MAX_EVENTS}",
                meter_raw.len()
            )));
        }
        let mut meter_events = Vec::with_capacity(meter_raw.len());
        for (idx, item) in meter_raw.iter().enumerate() {
            let o = item.as_object().ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("meter_events[{idx}] must be a mapping"))
            })?;
            let start_tick = o.get("start_tick").and_then(|v| v.as_u64()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("meter_events[{idx}] missing 'start_tick'"))
            })?;
            let meter = o.get("meter").and_then(|v| v.as_str()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("meter_events[{idx}] missing 'meter'"))
            })?;
            validate_meter(meter)?;
            meter_events.push(MeterEvent {
                start_tick,
                meter: meter.to_string(),
            });
        }

        // Parse instruments
        let inst_raw = map
            .get("instruments")
            .and_then(|v| v.as_array())
            .ok_or_else(|| MusicMachineError::InvalidArgument("instruments must be a list".into()))?;
        if inst_raw.len() > MAX_INSTRUMENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "instruments count {} exceeds MAX_INSTRUMENTS={MAX_INSTRUMENTS}",
                inst_raw.len()
            )));
        }
        let mut instruments = Vec::with_capacity(inst_raw.len());
        for (idx, item) in inst_raw.iter().enumerate() {
            let o = item.as_object().ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("instruments[{idx}] must be a mapping"))
            })?;
            let id = o.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("instruments[{idx}] missing 'id'"))
            })?;
            let profile_id = o.get("profile_id").and_then(|v| v.as_str()).unwrap_or(id);
            let gain = o.get("gain").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let kind = o.get("kind").and_then(|v| v.as_str()).unwrap_or("preset");
            let kit = o.get("kit").and_then(|v| v.as_str()).map(|s| s.to_string());

            let mut harmonics = None;
            if let Some(h_val) = o.get("harmonics") {
                let h_arr = h_val.as_array().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("harmonics must be a list of partials".into())
                })?;
                if h_arr.len() > MAX_PARTIALS {
                    return Err(MusicMachineError::BudgetExceeded(format!(
                        "harmonic partial count {} exceeds MAX_PARTIALS={MAX_PARTIALS}",
                        h_arr.len()
                    )));
                }
                let mut h_vec = Vec::with_capacity(h_arr.len());
                for (pi, p_val) in h_arr.iter().enumerate() {
                    let p_arr = p_val.as_array().ok_or_else(|| {
                        MusicMachineError::InvalidArgument(format!("malformed partial at {pi}"))
                    })?;
                    if p_arr.len() != 2 && p_arr.len() != 3 {
                        return Err(MusicMachineError::InvalidArgument(format!("malformed partial at {pi}")));
                    }
                    let mult = p_arr[0].as_f64().ok_or_else(|| {
                        MusicMachineError::InvalidArgument(format!("malformed partial multiplier at {pi}"))
                    })?;
                    let amp = p_arr[1].as_f64().ok_or_else(|| {
                        MusicMachineError::InvalidArgument(format!("malformed partial amplitude at {pi}"))
                    })?;
                    let phase = if p_arr.len() == 3 {
                        p_arr[2].as_f64().ok_or_else(|| {
                            MusicMachineError::InvalidArgument(format!("malformed partial phase at {pi}"))
                        })?
                    } else {
                        0.0
                    };
                    h_vec.push(HarmonicPartial::with_phase(mult, amp, phase)?);
                }
                harmonics = Some(h_vec);
            }

            let mut envelope = None;
            if let Some(env_val) = o.get("envelope") {
                let env_arr = env_val.as_array().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("envelope must be [atk, dec, sus, rel]".into())
                })?;
                if env_arr.len() != 4 {
                    return Err(MusicMachineError::InvalidArgument(
                        "envelope must be [atk, dec, sus, rel]".into(),
                    ));
                }
                let atk = env_arr[0].as_f64().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("envelope atk must be float".into())
                })?;
                let dec = env_arr[1].as_f64().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("envelope dec must be float".into())
                })?;
                let sus = env_arr[2].as_f64().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("envelope sus must be float".into())
                })?;
                let rel = env_arr[3].as_f64().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("envelope rel must be float".into())
                })?;
                envelope = Some(AdsrEnvelope::new(atk, dec, sus, rel)?);
            }

            instruments.push(InstrumentDeclaration::new(
                id, profile_id, gain, kind, kit, harmonics, envelope,
            )?);
        }

        // Parse tracks
        let track_raw = map
            .get("tracks")
            .and_then(|v| v.as_array())
            .ok_or_else(|| MusicMachineError::InvalidArgument("tracks must be a list".into()))?;
        if track_raw.len() > MAX_TRACKS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "tracks count {} exceeds MAX_TRACKS={MAX_TRACKS}",
                track_raw.len()
            )));
        }
        let mut tracks = Vec::with_capacity(track_raw.len());
        for (idx, item) in track_raw.iter().enumerate() {
            let o = item.as_object().ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("tracks[{idx}] must be a mapping"))
            })?;
            let id = o.get("id").and_then(|v| v.as_str()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("tracks[{idx}] missing 'id'"))
            })?;
            let instrument_id = o.get("instrument_id").and_then(|v| v.as_str()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("tracks[{idx}] missing 'instrument_id'"))
            })?;
            tracks.push(TrackDeclaration::new(id, instrument_id)?);
        }

        // Parse events
        let event_raw = map
            .get("events")
            .and_then(|v| v.as_array())
            .ok_or_else(|| MusicMachineError::InvalidArgument("events must be a list".into()))?;
        if event_raw.len() > MAX_EVENTS {
            return Err(MusicMachineError::BudgetExceeded(format!(
                "events count {} exceeds MAX_EVENTS={MAX_EVENTS}",
                event_raw.len()
            )));
        }
        let mut events = Vec::with_capacity(event_raw.len());
        for (idx, item) in event_raw.iter().enumerate() {
            let o = item.as_object().ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("events[{idx}] must be a mapping"))
            })?;
            let track_id = o.get("track_id").and_then(|v| v.as_str()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("events[{idx}] missing 'track_id'"))
            })?;
            let start_tick = o.get("start_tick").and_then(|v| v.as_u64()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("events[{idx}] missing 'start_tick'"))
            })?;
            let duration_tick = o.get("duration_tick").and_then(|v| v.as_u64()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("events[{idx}] missing 'duration_tick'"))
            })?;
            let kind_str = o.get("kind").and_then(|v| v.as_str()).ok_or_else(|| {
                MusicMachineError::InvalidArgument(format!("events[{idx}] missing 'kind'"))
            })?;
            let kind = ScoreEventKind::from_str_strict(kind_str)?;
            let velocity = o.get("velocity").and_then(|v| v.as_f64()).unwrap_or(1.0);
            let source_order = o.get("source_order").and_then(|v| v.as_u64()).unwrap_or(idx as u64) as usize;
            let hit_id = o.get("hit_id").and_then(|v| v.as_str()).map(|s| s.to_string());

            let mut notes = Vec::new();
            if let Some(n_val) = o.get("notes") {
                let n_arr = n_val.as_array().ok_or_else(|| {
                    MusicMachineError::InvalidArgument("notes must be a list".into())
                })?;
                for (ni, note_item) in n_arr.iter().enumerate() {
                    let note_str = note_item.as_str().ok_or_else(|| {
                        MusicMachineError::InvalidArgument(format!("notes[{ni}] must be a string"))
                    })?;
                    notes.push(note_str.to_string());
                }
            }

            events.push(PortableScoreEvent::new(
                track_id,
                start_tick,
                duration_tick,
                kind,
                notes,
                velocity,
                source_order,
                hit_id,
            )?);
        }

        PortableScore::new(
            format_version,
            title,
            ppq,
            sample_rate_hz,
            tempo_events,
            meter_events,
            instruments,
            tracks,
            events,
        )
    }
}
