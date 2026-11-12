//! # Manifest Parser & Validator (VER08a / Spec 13 / required_capabilities.json)
//!
//! Loads and validates `required_capabilities.json` according to the Spec 13 taxonomy.
//! Zero third-party dependencies: includes a self-contained, bounded JSON parser.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::path::Path;

/// Maximum allowed size for `required_capabilities.json` (1 MiB).
pub const MAX_MANIFEST_SIZE_BYTES: u64 = 1024 * 1024;

/// A parsed capability declaration.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Capability {
    pub category: String,
    pub action: String,
    pub target: String,
    pub justification: String,
}

impl Capability {
    /// Return the canonical wildcard string, e.g. `"fs:read:*"`.
    pub fn wildcard_string(&self) -> String {
        format!("{}:{}:*", self.category, self.action)
    }

    /// Return the full exact capability string, e.g. `"fs:read:foo.txt"`.
    pub fn exact_string(&self) -> String {
        format!("{}:{}:{}", self.category, self.action, self.target)
    }
}

/// A parsed banned construct exception.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BannedConstructException {
    pub construct: String,
    pub language: String,
    pub justification: String,
}

/// Validated manifest information.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ManifestData {
    /// Crate name declared in the manifest, or `"pure-computation"` if absent.
    pub package: String,
    /// Overall package justification.
    pub justification: String,
    /// Exact capabilities declared in the manifest.
    pub capabilities: Vec<Capability>,
    /// Set of wildcard capabilities (`"category:action:*"`).
    pub declared_wildcards: HashSet<String>,
    /// Banned construct exceptions indexed by `<language>:<construct>`.
    pub banned_exceptions: HashMap<String, BannedConstructException>,
    /// Whether the manifest file was present on disk.
    pub is_explicit: bool,
}

impl ManifestData {
    /// Check whether a detected capability (`"category:action:*"`) is declared.
    pub fn covers_capability(&self, category: &str, action: &str) -> bool {
        let wildcard = format!("{category}:{action}:*");
        self.declared_wildcards.contains(&wildcard)
    }

    /// Check whether a banned construct is exempt for Rust.
    pub fn is_construct_exempt(&self, construct: &str) -> bool {
        let key = format!("rust:{construct}");
        self.banned_exceptions.contains_key(&key)
    }
}

/// Errors occurring during manifest discovery, loading, parsing, or validation.
#[derive(Debug)]
pub enum ManifestError {
    Io(String),
    SizeLimitExceeded(u64),
    JsonParse(String),
    Validation(String),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(msg) => write!(f, "manifest I/O error: {msg}"),
            Self::SizeLimitExceeded(sz) => {
                write!(f, "manifest exceeds 1 MiB limit: {sz} bytes")
            }
            Self::JsonParse(msg) => write!(f, "manifest JSON parse error: {msg}"),
            Self::Validation(msg) => write!(f, "manifest validation failed: {msg}"),
        }
    }
}

impl std::error::Error for ManifestError {}

/// Check if a (category, action) pair is part of the closed Spec 13 taxonomy.
pub fn is_valid_capability_pair(category: &str, action: &str) -> bool {
    match category {
        "fs" => matches!(action, "read" | "write" | "create" | "delete" | "list"),
        "net" => matches!(action, "connect" | "listen" | "dns"),
        "proc" => matches!(action, "exec" | "fork" | "signal"),
        "env" => matches!(action, "read" | "write"),
        "ffi" => matches!(action, "call" | "load"),
        "time" => matches!(action, "read" | "sleep"),
        "stdin" => action == "read",
        "stdout" => action == "write",
        "rand" => action == "read",
        "thread" => matches!(action, "spawn" | "message"),
        "ui" => matches!(action, "read" | "write" | "event"),
        _ => false,
    }
}

/// Load and validate `required_capabilities.json` from `package_dir`.
///
/// If the file does not exist, returns a default `ManifestData` (pure computation).
/// If the file exists, enforces strict schema version 1 and Spec 13 taxonomy rules.
pub fn load_manifest(package_dir: &Path) -> Result<ManifestData, ManifestError> {
    let manifest_path = package_dir.join("required_capabilities.json");
    if !manifest_path.exists() {
        return Ok(ManifestData {
            package: "pure-computation".into(),
            justification: "Implicit pure computation profile; no manifest present.".into(),
            capabilities: Vec::new(),
            declared_wildcards: HashSet::new(),
            banned_exceptions: HashMap::new(),
            is_explicit: false,
        });
    }

    let metadata = fs::symlink_metadata(&manifest_path)
        .map_err(|e| ManifestError::Io(format!("{}: {e}", manifest_path.display())))?;
    if metadata.is_symlink() {
        return Err(ManifestError::Validation(
            "required_capabilities.json must not be a symlink".into(),
        ));
    }
    if metadata.len() > MAX_MANIFEST_SIZE_BYTES {
        return Err(ManifestError::SizeLimitExceeded(metadata.len()));
    }

    let content = fs::read_to_string(&manifest_path)
        .map_err(|e| ManifestError::Io(format!("{}: {e}", manifest_path.display())))?;

    let json_val = JsonValue::parse(&content).map_err(ManifestError::JsonParse)?;
    validate_and_convert_manifest(json_val)
}

fn validate_and_convert_manifest(val: JsonValue) -> Result<ManifestData, ManifestError> {
    let obj = match val {
        JsonValue::Object(m) => m,
        _ => {
            return Err(ManifestError::Validation(
                "root must be a JSON object".into(),
            ))
        }
    };

    // Allowed top-level fields
    for key in obj.keys() {
        if !matches!(
            key.as_str(),
            "$schema"
                | "version"
                | "package"
                | "capabilities"
                | "justification"
                | "banned_construct_exceptions"
        ) {
            return Err(ManifestError::Validation(format!(
                "unknown top-level property '{key}'"
            )));
        }
    }

    // version
    let version = obj
        .get("version")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| ManifestError::Validation("missing or non-integer 'version'".into()))?;
    if version != 1 {
        return Err(ManifestError::Validation(format!(
            "unsupported version {version}, expected 1"
        )));
    }

    // package
    let package = obj
        .get("package")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ManifestError::Validation("missing or non-string 'package'".into()))?;
    if !package.starts_with("rust/") {
        return Err(ManifestError::Validation(format!(
            "invalid package '{package}', expected rust/<name>"
        )));
    }
    let pkg_suffix = &package["rust/".len()..];
    if pkg_suffix.is_empty()
        || !pkg_suffix
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
    {
        return Err(ManifestError::Validation(format!(
            "invalid package '{package}', expected rust/<name> containing only [a-z0-9-_]"
        )));
    }

    // justification
    let justification = obj
        .get("justification")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ManifestError::Validation("missing or non-string 'justification'".into()))?;
    if justification.trim().len() < 10 {
        return Err(ManifestError::Validation(
            "package justification must be at least 10 characters".into(),
        ));
    }

    // capabilities
    let caps_raw = obj
        .get("capabilities")
        .and_then(|v| v.as_array())
        .ok_or_else(|| ManifestError::Validation("missing or non-array 'capabilities'".into()))?;

    let mut capabilities = Vec::new();
    let mut declared_wildcards = HashSet::new();
    let mut seen_exact = HashSet::new();

    for (i, c_val) in caps_raw.iter().enumerate() {
        let c_obj = c_val.as_object().ok_or_else(|| {
            ManifestError::Validation(format!("capability[{i}] must be an object"))
        })?;

        for k in c_obj.keys() {
            if !matches!(
                k.as_str(),
                "category" | "action" | "target" | "justification"
            ) {
                return Err(ManifestError::Validation(format!(
                    "unknown capability[{i}] property '{k}'"
                )));
            }
        }

        let category = c_obj
            .get("category")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ManifestError::Validation(format!("capability[{i}].category must be string"))
            })?;
        let action = c_obj
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ManifestError::Validation(format!("capability[{i}].action must be string"))
            })?;
        let target = c_obj
            .get("target")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ManifestError::Validation(format!("capability[{i}].target must be string"))
            })?;
        let just = c_obj
            .get("justification")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ManifestError::Validation(format!("capability[{i}].justification must be string"))
            })?;

        if !is_valid_capability_pair(category, action) {
            return Err(ManifestError::Validation(format!(
                "invalid capability pair '{category}:{action}'"
            )));
        }
        if target.trim().is_empty() {
            return Err(ManifestError::Validation(format!(
                "capability[{i}].target must not be empty"
            )));
        }
        if !target
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.' || c == '/' || c == '*')
        {
            return Err(ManifestError::Validation(format!(
                "capability[{i}].target '{target}' contains disallowed characters (only [a-zA-Z0-9_\\-./*] permitted)"
            )));
        }
        if just.trim().len() < 10 {
            return Err(ManifestError::Validation(format!(
                "capability[{i}].justification must be at least 10 characters"
            )));
        }

        let cap = Capability {
            category: category.into(),
            action: action.into(),
            target: target.into(),
            justification: just.into(),
        };

        let exact_key = cap.exact_string();
        if !seen_exact.insert(exact_key.clone()) {
            return Err(ManifestError::Validation(format!(
                "duplicate capability '{exact_key}'"
            )));
        }

        declared_wildcards.insert(cap.wildcard_string());
        capabilities.push(cap);
    }

    // banned_construct_exceptions
    let mut banned_exceptions = HashMap::new();
    if let Some(banned_val) = obj.get("banned_construct_exceptions") {
        let banned_arr = banned_val.as_array().ok_or_else(|| {
            ManifestError::Validation("'banned_construct_exceptions' must be an array".into())
        })?;

        for (i, b_val) in banned_arr.iter().enumerate() {
            let b_obj = b_val.as_object().ok_or_else(|| {
                ManifestError::Validation(format!(
                    "banned_construct_exceptions[{i}] must be object"
                ))
            })?;

            for k in b_obj.keys() {
                if !matches!(k.as_str(), "construct" | "language" | "justification") {
                    return Err(ManifestError::Validation(format!(
                        "unknown property '{k}' in banned_construct_exceptions[{i}]"
                    )));
                }
            }

            let construct = b_obj
                .get("construct")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ManifestError::Validation(format!(
                        "banned_construct_exceptions[{i}].construct must be string"
                    ))
                })?;
            let language = b_obj
                .get("language")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ManifestError::Validation(format!(
                        "banned_construct_exceptions[{i}].language must be string"
                    ))
                })?;
            let just = b_obj
                .get("justification")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    ManifestError::Validation(format!(
                        "banned_construct_exceptions[{i}].justification must be string"
                    ))
                })?;

            if language.to_lowercase() != "rust" {
                return Err(ManifestError::Validation(format!(
                    "banned_construct_exceptions[{i}].language must be 'rust'"
                )));
            }
            if just.trim().len() < 10 {
                return Err(ManifestError::Validation(format!(
                    "banned_construct_exceptions[{i}].justification must be at least 10 characters"
                )));
            }

            let norm = construct.to_lowercase();
            let norm = norm.trim();
            if !matches!(norm, "unsafe_code" | "inline_assembly" | "foreign_linkage") {
                return Err(ManifestError::Validation(format!(
                    "banned_construct_exceptions[{i}].construct '{construct}' is not a Rust construct id (unsafe_code, inline_assembly, foreign_linkage)"
                )));
            }
            let key = format!("rust:{norm}");
            if banned_exceptions.contains_key(&key) {
                return Err(ManifestError::Validation(format!(
                    "duplicate banned_construct_exceptions entry '{norm}'"
                )));
            }
            banned_exceptions.insert(
                key,
                BannedConstructException {
                    construct: construct.into(),
                    language: language.into(),
                    justification: just.into(),
                },
            );
        }
    }

    Ok(ManifestData {
        package: package.into(),
        justification: justification.into(),
        capabilities,
        declared_wildcards,
        banned_exceptions,
        is_explicit: true,
    })
}

// ---------------------------------------------------------------------------
// Zero-Dependency Bounded JSON Parser
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(f64, Option<i64>),
    String(String),
    Array(Vec<JsonValue>),
    Object(HashMap<String, JsonValue>),
}

impl JsonValue {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Number(_, Some(i)) => Some(*i),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        match self {
            Self::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        match self {
            Self::Object(m) => Some(m),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&JsonValue> {
        match self {
            Self::Object(m) => m.get(key),
            _ => None,
        }
    }

    pub fn parse(input: &str) -> Result<JsonValue, String> {
        let mut p = JsonParser::new(input);
        let val = p.parse_value(0)?;
        p.skip_whitespace();
        if p.pos < p.chars.len() {
            return Err(format!("unexpected trailing content at char {}", p.pos));
        }
        Ok(val)
    }
}

struct JsonParser<'a> {
    chars: Vec<char>,
    pos: usize,
    _phantom: std::marker::PhantomData<&'a str>,
}

impl<'a> JsonParser<'a> {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            // RFC 8259 whitespace only (not Unicode whitespace).
            if matches!(c, ' ' | '\t' | '\n' | '\r') {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, String> {
        if depth > 100 {
            return Err("JSON recursion depth exceeded 100".into());
        }
        self.skip_whitespace();
        match self.peek() {
            Some('{') => self.parse_object(depth + 1),
            Some('[') => self.parse_array(depth + 1),
            Some('"') => self.parse_string().map(JsonValue::String),
            Some('t') | Some('f') => self.parse_bool().map(JsonValue::Bool),
            Some('n') => self.parse_null(),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(other) => Err(format!("unexpected character '{other}' at {}", self.pos)),
            None => Err("unexpected end of input".into()),
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, String> {
        self.next_char(); // consume '{'
        let mut map = HashMap::new();
        self.skip_whitespace();
        if let Some('}') = self.peek() {
            self.next_char();
            return Ok(JsonValue::Object(map));
        }

        loop {
            self.skip_whitespace();
            let key = self.parse_string()?;
            self.skip_whitespace();
            if self.next_char() != Some(':') {
                return Err(format!("expected ':' after key at {}", self.pos));
            }
            let val = self.parse_value(depth)?;
            // Duplicate keys are a parser differential: refuse them (VER08a §2).
            if map.insert(key.clone(), val).is_some() {
                return Err(format!("duplicate object key '{key}'"));
            }

            self.skip_whitespace();
            match self.next_char() {
                Some(',') => continue,
                Some('}') => break,
                Some(c) => return Err(format!("expected ',' or '}}' at {}, got '{c}'", self.pos)),
                None => return Err("unterminated object".into()),
            }
        }
        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, String> {
        self.next_char(); // consume '['
        let mut arr = Vec::new();
        self.skip_whitespace();
        if let Some(']') = self.peek() {
            self.next_char();
            return Ok(JsonValue::Array(arr));
        }

        loop {
            let val = self.parse_value(depth)?;
            arr.push(val);
            self.skip_whitespace();
            match self.next_char() {
                Some(',') => continue,
                Some(']') => break,
                Some(c) => return Err(format!("expected ',' or ']' at {}, got '{c}'", self.pos)),
                None => return Err("unterminated array".into()),
            }
        }
        Ok(JsonValue::Array(arr))
    }

    fn parse_string(&mut self) -> Result<String, String> {
        if self.next_char() != Some('"') {
            return Err(format!("expected '\"' at {}", self.pos));
        }
        let mut out = String::new();
        while let Some(c) = self.next_char() {
            match c {
                '"' => return Ok(out),
                '\\' => {
                    let esc = self.next_char().ok_or("unterminated escape sequence")?;
                    match esc {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\x08'),
                        'f' => out.push('\x0c'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            let mut hex = String::with_capacity(4);
                            for _ in 0..4 {
                                hex.push(self.next_char().ok_or("truncated \\u escape")?);
                            }
                            let cp = u32::from_str_radix(&hex, 16)
                                .map_err(|_| "invalid \\u hex code")?;
                            let ch = char::from_u32(cp).ok_or("invalid unicode codepoint")?;
                            out.push(ch);
                        }
                        other => return Err(format!("invalid escape '\\{other}'")),
                    }
                }
                control if (control as u32) < 0x20 => {
                    return Err("raw control character in string".into());
                }
                normal => out.push(normal),
            }
        }
        Err("unterminated string".into())
    }

    fn parse_bool(&mut self) -> Result<bool, String> {
        let rest: String = self.chars[self.pos..].iter().take(5).collect();
        if rest.starts_with("true") {
            self.pos += 4;
            Ok(true)
        } else if rest.starts_with("false") {
            self.pos += 5;
            Ok(false)
        } else {
            Err(format!("invalid boolean at {}", self.pos))
        }
    }

    fn parse_null(&mut self) -> Result<JsonValue, String> {
        let rest: String = self.chars[self.pos..].iter().take(4).collect();
        if rest == "null" {
            self.pos += 4;
            Ok(JsonValue::Null)
        } else {
            Err(format!("invalid null literal at {}", self.pos))
        }
    }

    /// Strict RFC 8259 number: `-?(0|[1-9][0-9]*)(\.[0-9]+)?([eE][+-]?[0-9]+)?`.
    fn parse_number(&mut self) -> Result<JsonValue, String> {
        let start = self.pos;
        let digits = |p: &mut Self| -> usize {
            let s0 = p.pos;
            while p.peek().is_some_and(|c| c.is_ascii_digit()) {
                p.pos += 1;
            }
            p.pos - s0
        };
        if self.peek() == Some('-') {
            self.pos += 1;
        }
        match self.peek() {
            Some('0') => {
                self.pos += 1;
                if self.peek().is_some_and(|c| c.is_ascii_digit()) {
                    return Err(format!("leading zero in number at {start}"));
                }
            }
            Some(c) if c.is_ascii_digit() => {
                digits(self);
            }
            _ => return Err(format!("invalid number at {start}")),
        }
        let mut is_int = true;
        if self.peek() == Some('.') {
            is_int = false;
            self.pos += 1;
            if digits(self) == 0 {
                return Err(format!("invalid fraction at {start}"));
            }
        }
        if matches!(self.peek(), Some('e') | Some('E')) {
            is_int = false;
            self.pos += 1;
            if matches!(self.peek(), Some('+') | Some('-')) {
                self.pos += 1;
            }
            if digits(self) == 0 {
                return Err(format!("invalid exponent at {start}"));
            }
        }
        let raw: String = self.chars[start..self.pos].iter().collect();
        let float_val: f64 = raw
            .parse()
            .map_err(|_| format!("invalid number '{raw}' at {start}"))?;
        let int_val: Option<i64> = if is_int { raw.parse().ok() } else { None };
        Ok(JsonValue::Number(float_val, int_val))
    }
}
