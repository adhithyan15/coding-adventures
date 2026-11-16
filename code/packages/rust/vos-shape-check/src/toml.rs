//! # A tiny, fail-closed TOML reader
//!
//! `Cargo.toml` decides things the Rust source never shows: a `build = ".."`
//! key compiles a script before the crate, `proc-macro = true` makes the crate
//! run at compile time, a dependency table pulls in foreign code.  To police
//! those we must *read* the manifest, and a line-by-line `grep` is easy to
//! evade (`package = { build = ".." }`, a quoted table name `["lib"]`, a
//! multi-line array whose body looks like a header).  So this module
//! tokenises the whole document, **flattens every value to a full key path**
//! and lets the rules look at paths, however the author spelled them:
//!
//! ```text
//!   [package]                      path ["package","build"]  = Str("x")
//!   build = "x"
//!
//!   package.build = "x"            same path
//!   package = { build = "x" }      same path
//!   [lib]                          path ["lib","proc-macro"] = Bool(true)
//!   proc-macro = true
//!   [dependencies.foo]             path ["dependencies","foo","path"] = Str(..)
//!   path = "../foo"
//!   bin = [{ path = "x.rs" }]      path ["bin","[]","path"] = Str("x.rs")
//! ```
//!
//! Supported subset: comments, bare and quoted keys, dotted keys, `[table]`
//! and `[[array-of-tables]]` headers, basic and literal strings (single and
//! multi-line), booleans, numbers and dates (kept as opaque text), arrays and
//! inline tables.  Anything else is an error, and an error is a finding
//! (fail closed, VOS03 section 3): we never guess what an unreadable manifest
//! means.

/// A flattened value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    /// A string.
    Str(String),
    /// `true` or `false`.
    Bool(bool),
    /// A number, date or time, kept as text.
    Other(String),
    /// An array (elements that are tables are flattened separately, see
    /// [`Entry`]; they appear here as [`Value::Table`]).
    Array(Vec<Value>),
    /// An inline table or an empty table header; its members are separate
    /// entries with a longer path.
    Table,
}

/// One flattened `path = value` with the 1-based line it was written on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Full key path from the document root.
    pub path: Vec<String>,
    /// The value.
    pub value: Value,
    /// Line of the key.
    pub line: u32,
}

/// A parse failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TomlError {
    /// 1-based line.
    pub line: u32,
    /// Reason.
    pub message: String,
}

struct P {
    c: Vec<char>,
    i: usize,
    line: u32,
    out: Vec<Entry>,
}

type R<T> = Result<T, TomlError>;

impl P {
    fn err<T>(&self, m: &str) -> R<T> {
        Err(TomlError {
            line: self.line,
            message: m.to_string(),
        })
    }

    fn peek(&self) -> Option<char> {
        self.c.get(self.i).copied()
    }

    fn at(&self, off: usize) -> Option<char> {
        self.c.get(self.i + off).copied()
    }

    /// Skip spaces and tabs only.
    fn ws(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.i += 1;
        }
    }

    fn comment(&mut self) {
        if self.peek() == Some('#') {
            while self.peek().is_some_and(|c| c != '\n') {
                self.i += 1;
            }
        }
    }

    /// Skip whitespace, comments and newlines (inside arrays and tables).
    fn skip_all(&mut self) {
        loop {
            self.ws();
            self.comment();
            match self.peek() {
                Some('\n') => {
                    self.line += 1;
                    self.i += 1;
                }
                Some('\r') => self.i += 1,
                _ => return,
            }
        }
    }

    fn key_part(&mut self) -> R<String> {
        match self.peek() {
            Some('"') => self.basic_string(),
            Some('\'') => self.literal_string(),
            Some(c) if c.is_ascii_alphanumeric() || c == '_' || c == '-' => {
                let s = self.i;
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
                {
                    self.i += 1;
                }
                Ok(self.c[s..self.i].iter().collect())
            }
            _ => self.err("expected a key"),
        }
    }

    fn dotted_key(&mut self) -> R<Vec<String>> {
        let mut parts = vec![self.key_part()?];
        loop {
            self.ws();
            if self.peek() == Some('.') {
                self.i += 1;
                self.ws();
                parts.push(self.key_part()?);
            } else {
                return Ok(parts);
            }
        }
    }

    fn escape(&mut self) -> R<char> {
        let Some(c) = self.peek() else {
            return self.err("unterminated string");
        };
        self.i += 1;
        Ok(match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'b' => '\u{8}',
            'f' => '\u{c}',
            '"' => '"',
            '\\' => '\\',
            'u' | 'U' => {
                let n = if c == 'u' { 4 } else { 8 };
                let mut v = 0u32;
                for _ in 0..n {
                    let Some(d) = self.peek().and_then(|c| c.to_digit(16)) else {
                        return self.err("bad unicode escape");
                    };
                    v = v.wrapping_mul(16).wrapping_add(d);
                    self.i += 1;
                }
                match char::from_u32(v) {
                    Some(c) => c,
                    None => return self.err("bad unicode escape"),
                }
            }
            _ => return self.err("unknown escape"),
        })
    }

    fn basic_string(&mut self) -> R<String> {
        if self.at(1) == Some('"') && self.at(2) == Some('"') {
            self.i += 3;
            if self.peek() == Some('\n') {
                self.line += 1;
                self.i += 1;
            }
            let mut s = String::new();
            loop {
                match self.peek() {
                    None => return self.err("unterminated multi-line string"),
                    Some('"') if self.at(1) == Some('"') && self.at(2) == Some('"') => {
                        self.i += 3;
                        return Ok(s);
                    }
                    Some('\\') => {
                        self.i += 1;
                        if matches!(self.peek(), Some('\n' | ' ' | '\t' | '\r')) {
                            // line-ending backslash trims following whitespace
                            while matches!(self.peek(), Some('\n' | ' ' | '\t' | '\r')) {
                                if self.peek() == Some('\n') {
                                    self.line += 1;
                                }
                                self.i += 1;
                            }
                        } else {
                            let c = self.escape()?;
                            s.push(c);
                        }
                    }
                    Some(c) => {
                        if c == '\n' {
                            self.line += 1;
                        }
                        s.push(c);
                        self.i += 1;
                    }
                }
            }
        }
        self.i += 1;
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => return self.err("unterminated string"),
                Some('"') => {
                    self.i += 1;
                    return Ok(s);
                }
                Some('\\') => {
                    self.i += 1;
                    let c = self.escape()?;
                    s.push(c);
                }
                Some(c) => {
                    s.push(c);
                    self.i += 1;
                }
            }
        }
    }

    fn literal_string(&mut self) -> R<String> {
        if self.at(1) == Some('\'') && self.at(2) == Some('\'') {
            self.i += 3;
            if self.peek() == Some('\n') {
                self.line += 1;
                self.i += 1;
            }
            let mut s = String::new();
            loop {
                match self.peek() {
                    None => return self.err("unterminated multi-line string"),
                    Some('\'') if self.at(1) == Some('\'') && self.at(2) == Some('\'') => {
                        self.i += 3;
                        return Ok(s);
                    }
                    Some(c) => {
                        if c == '\n' {
                            self.line += 1;
                        }
                        s.push(c);
                        self.i += 1;
                    }
                }
            }
        }
        self.i += 1;
        let mut s = String::new();
        loop {
            match self.peek() {
                None | Some('\n') => return self.err("unterminated string"),
                Some('\'') => {
                    self.i += 1;
                    return Ok(s);
                }
                Some(c) => {
                    s.push(c);
                    self.i += 1;
                }
            }
        }
    }

    /// Parse a value; `path` is where it lives, for flattening tables.
    fn value(&mut self, path: &[String], depth: usize) -> R<Value> {
        if depth > 32 {
            return self.err("nesting too deep");
        }
        match self.peek() {
            Some('"') => Ok(Value::Str(self.basic_string()?)),
            Some('\'') => Ok(Value::Str(self.literal_string()?)),
            Some('[') => {
                self.i += 1;
                let mut items = Vec::new();
                let mut sub = path.to_vec();
                sub.push("[]".to_string());
                loop {
                    self.skip_all();
                    match self.peek() {
                        Some(']') => {
                            self.i += 1;
                            return Ok(Value::Array(items));
                        }
                        None => return self.err("unterminated array"),
                        _ => {}
                    }
                    items.push(self.value(&sub, depth + 1)?);
                    self.skip_all();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some(']') => {}
                        _ => return self.err("expected `,` or `]` in array"),
                    }
                }
            }
            Some('{') => {
                self.i += 1;
                loop {
                    self.skip_all();
                    match self.peek() {
                        Some('}') => {
                            self.i += 1;
                            return Ok(Value::Table);
                        }
                        None => return self.err("unterminated inline table"),
                        _ => {}
                    }
                    let l = self.line;
                    let key = self.dotted_key()?;
                    self.ws();
                    if self.peek() != Some('=') {
                        return self.err("expected `=` in inline table");
                    }
                    self.i += 1;
                    self.ws();
                    let mut full = path.to_vec();
                    full.extend(key);
                    let v = self.value(&full, depth + 1)?;
                    self.out.push(Entry {
                        path: full,
                        value: v,
                        line: l,
                    });
                    self.skip_all();
                    match self.peek() {
                        Some(',') => self.i += 1,
                        Some('}') => {}
                        _ => return self.err("expected `,` or `}` in inline table"),
                    }
                }
            }
            Some(c) if c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.' | '_' | ':') => {
                let s = self.i;
                while self.peek().is_some_and(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.' | '_' | ':')
                }) {
                    self.i += 1;
                }
                let t: String = self.c[s..self.i].iter().collect();
                match t.as_str() {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    _ => Ok(Value::Other(t)),
                }
            }
            _ => self.err("expected a value"),
        }
    }
}

/// Parse `text` into flattened entries, in document order.
pub fn parse(text: &str) -> Result<Vec<Entry>, TomlError> {
    let mut p = P {
        c: text.chars().collect(),
        i: 0,
        line: 1,
        out: Vec::new(),
    };
    let mut prefix: Vec<String> = Vec::new();
    loop {
        p.skip_all();
        let Some(c) = p.peek() else { break };
        let line = p.line;
        if c == '[' {
            let array = p.at(1) == Some('[');
            p.i += if array { 2 } else { 1 };
            p.ws();
            let key = p.dotted_key()?;
            p.ws();
            let closers = if array { "]]" } else { "]" };
            for want in closers.chars() {
                if p.peek() != Some(want) {
                    return p.err("malformed table header");
                }
                p.i += 1;
            }
            prefix = key;
            p.out.push(Entry {
                path: prefix.clone(),
                value: Value::Table,
                line,
            });
        } else {
            let key = p.dotted_key()?;
            p.ws();
            if p.peek() != Some('=') {
                return p.err("expected `=` after key");
            }
            p.i += 1;
            p.ws();
            let mut full = prefix.clone();
            full.extend(key);
            let v = p.value(&full, 0)?;
            p.out.push(Entry {
                path: full,
                value: v,
                line,
            });
        }
        p.ws();
        p.comment();
        match p.peek() {
            None | Some('\n') | Some('\r') => {}
            _ => return p.err("unexpected text after a value or header"),
        }
    }
    Ok(p.out)
}
