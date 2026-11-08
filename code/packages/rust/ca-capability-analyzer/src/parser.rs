//! # Rust Source Scanner & Token Visitor (VER08a)
//!
//! Scans Rust source files for ambient capability paths, standard library surfaces,
//! and banned constructs (unsafe blocks, inline assembly, foreign linkage).
//! Zero third-party dependencies.

use crate::rules::{classify_path, Classification};

/// A detected capability access within a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceDetection {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub category: &'static str,
    pub action: &'static str,
    pub evidence: String,
}

/// A detected banned construct within a source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceBannedConstruct {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub construct: &'static str,
    pub evidence: String,
}

/// Results of scanning one or more source files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanResult {
    pub detections: Vec<SourceDetection>,
    pub banned: Vec<SourceBannedConstruct>,
}

/// Scan a single Rust source file string.
pub fn scan_source(file_rel_path: &str, content: &str) -> Result<ScanResult, String> {
    let mut scanner = SourceScanner::new(file_rel_path, content);
    scanner.scan()?;
    Ok(scanner.into_result())
}

struct SourceScanner<'a> {
    file: &'a str,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    detections: Vec<SourceDetection>,
    banned: Vec<SourceBannedConstruct>,
}

impl<'a> SourceScanner<'a> {
    fn new(file: &'a str, content: &str) -> Self {
        Self {
            file,
            chars: content.chars().collect(),
            pos: 0,
            line: 1,
            col: 0,
            detections: Vec::new(),
            banned: Vec::new(),
        }
    }

    fn into_result(self) -> ScanResult {
        ScanResult {
            detections: self.detections,
            banned: self.banned,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_ahead(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if let Some(c) = ch {
            self.pos += 1;
            if c == '\n' {
                self.line += 1;
                self.col = 0;
            } else {
                self.col += 1;
            }
        }
        ch
    }

    fn is_raw_string_start(&self) -> bool {
        if self.peek() != Some('r') {
            return false;
        }
        let mut offset = 1;
        while let Some(c) = self.peek_ahead(offset) {
            if c == '#' {
                offset += 1;
            } else {
                return c == '"';
            }
        }
        false
    }

    fn is_raw_byte_string_start(&self) -> bool {
        if self.peek() != Some('b') || self.peek_ahead(1) != Some('r') {
            return false;
        }
        let mut offset = 2;
        while let Some(c) = self.peek_ahead(offset) {
            if c == '#' {
                offset += 1;
            } else {
                return c == '"';
            }
        }
        false
    }

    fn scan(&mut self) -> Result<(), String> {
        while let Some(ch) = self.peek() {
            // 1. Comments
            if ch == '/' {
                if self.peek_ahead(1) == Some('/') {
                    self.skip_line_comment();
                    continue;
                } else if self.peek_ahead(1) == Some('*') {
                    self.skip_block_comment()?;
                    continue;
                }
            }

            // 2. String literals & Raw string literals
            if ch == '"' {
                self.skip_string()?;
                continue;
            }
            if self.is_raw_string_start() {
                self.skip_raw_string()?;
                continue;
            }
            if ch == 'b' && self.peek_ahead(1) == Some('"') {
                self.advance(); // consume 'b'
                self.skip_string()?;
                continue;
            }
            if self.is_raw_byte_string_start() {
                self.advance(); // consume 'b'
                self.skip_raw_string()?;
                continue;
            }

            // 3. Character literals
            if ch == '\'' {
                self.skip_char_literal();
                continue;
            }

            // 4. Attributes: #[no_mangle], #[link_name = "..."]
            if ch == '#' {
                let start_line = self.line;
                let start_col = self.col;
                let attr_text = self.read_attribute();
                if attr_text.contains("no_mangle") {
                    self.banned.push(SourceBannedConstruct {
                        file: self.file.to_string(),
                        line: start_line,
                        col: start_col,
                        construct: "foreign_linkage",
                        evidence: "#[no_mangle]".into(),
                    });
                }
                if attr_text.contains("link_name") {
                    self.banned.push(SourceBannedConstruct {
                        file: self.file.to_string(),
                        line: start_line,
                        col: start_col,
                        construct: "foreign_linkage",
                        evidence: "#[link_name]".into(),
                    });
                }
                if attr_text.contains("export_name") {
                    self.banned.push(SourceBannedConstruct {
                        file: self.file.to_string(),
                        line: start_line,
                        col: start_col,
                        construct: "foreign_linkage",
                        evidence: "#[export_name]".into(),
                    });
                }
                continue;
            }

            // 5. Identifiers and Path expressions
            if ch.is_ascii_alphabetic() || ch == '_' || (ch == ':' && self.peek_ahead(1) == Some(':')) {
                let start_line = self.line;
                let start_col = self.col;
                let path = self.read_path();

                // Check for banned construct keywords
                if path == "unsafe" {
                    self.banned.push(SourceBannedConstruct {
                        file: self.file.to_string(),
                        line: start_line,
                        col: start_col,
                        construct: "unsafe_code",
                        evidence: "unsafe".into(),
                    });
                    continue;
                }

                // Check for extern "C" / extern "system" / extern fn / extern { ... }
                if path == "extern" {
                    let _ = self.skip_whitespace_and_comments();
                    if self.peek() == Some('"') {
                        let abi = self.read_string_content();
                        if matches!(
                            abi.as_str(),
                            "C" | "system" | "cdecl" | "stdcall" | "win64" | "sysv64"
                        ) {
                            self.detections.push(SourceDetection {
                                file: self.file.to_string(),
                                line: start_line,
                                col: start_col,
                                category: "ffi",
                                action: "call",
                                evidence: format!("extern \"{abi}\""),
                            });
                        }
                    } else if self.peek() == Some('{') {
                        self.detections.push(SourceDetection {
                            file: self.file.to_string(),
                            line: start_line,
                            col: start_col,
                            category: "ffi",
                            action: "call",
                            evidence: "extern { ... }".into(),
                        });
                    } else {
                        let saved = (self.pos, self.line, self.col);
                        let next_word = self.read_path();
                        if matches!(next_word.as_str(), "fn" | "unsafe") {
                            self.detections.push(SourceDetection {
                                file: self.file.to_string(),
                                line: start_line,
                                col: start_col,
                                category: "ffi",
                                action: "call",
                                evidence: format!("extern {next_word}"),
                            });
                        } else {
                            self.pos = saved.0;
                            self.line = saved.1;
                            self.col = saved.2;
                        }
                    }
                    continue;
                }

                // Check if macro invocation (e.g. asm!, println!)
                let is_macro = if self.peek() == Some('!') && self.peek_ahead(1) != Some('=') {
                    self.advance(); // consume '!'
                    true
                } else {
                    false
                };

                // Classify the symbol/path
                match classify_path(&path) {
                    Classification::Capability { category, action } => {
                        let ev = if is_macro { format!("{path}!") } else { path };
                        self.detections.push(SourceDetection {
                            file: self.file.to_string(),
                            line: start_line,
                            col: start_col,
                            category,
                            action,
                            evidence: ev,
                        });
                    }
                    Classification::BannedConstruct { construct } => {
                        let ev = if is_macro { format!("{path}!") } else { path };
                        self.banned.push(SourceBannedConstruct {
                            file: self.file.to_string(),
                            line: start_line,
                            col: start_col,
                            construct,
                            evidence: ev,
                        });
                    }
                    Classification::AllowedPure => {}
                }
                continue;
            }

            self.advance();
        }
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_line_comment(&mut self) {
        while let Some(c) = self.advance() {
            if c == '\n' {
                break;
            }
        }
    }

    fn skip_block_comment(&mut self) -> Result<(), String> {
        self.advance(); // '/'
        self.advance(); // '*'
        let mut depth = 1;
        while let Some(c) = self.advance() {
            if c == '/' && self.peek() == Some('*') {
                self.advance();
                depth += 1;
            } else if c == '*' && self.peek() == Some('/') {
                self.advance();
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
        }
        Err("unterminated block comment".into())
    }

    fn skip_string(&mut self) -> Result<(), String> {
        self.advance(); // consume '"'
        while let Some(c) = self.advance() {
            if c == '"' {
                return Ok(());
            }
            if c == '\\' {
                self.advance(); // skip escaped char
            }
        }
        Err("unterminated string literal".into())
    }

    fn read_string_content(&mut self) -> String {
        let mut out = String::new();
        if self.advance() != Some('"') {
            return out;
        }
        while let Some(c) = self.advance() {
            if c == '"' {
                break;
            }
            if c == '\\' {
                if let Some(esc) = self.advance() {
                    out.push(esc);
                }
            } else {
                out.push(c);
            }
        }
        out
    }

    fn skip_raw_string(&mut self) -> Result<(), String> {
        self.advance(); // consume 'r'
        let mut hashes = 0;
        while self.peek() == Some('#') {
            self.advance();
            hashes += 1;
        }
        if self.advance() != Some('"') {
            return Err(format!(
                "malformed raw string at line {}, col {}",
                self.line, self.col
            ));
        }

        loop {
            match self.advance() {
                Some('"') => {
                    let mut matched = 0;
                    while matched < hashes && self.peek() == Some('#') {
                        self.advance();
                        matched += 1;
                    }
                    if matched == hashes {
                        return Ok(());
                    }
                }
                Some(_) => continue,
                None => return Err("unterminated raw string".into()),
            }
        }
    }

    fn skip_char_literal(&mut self) {
        self.advance(); // consume '\''
        if self.peek() == Some('\\') {
            self.advance();
            self.advance();
        } else {
            self.advance();
        }
        if self.peek() == Some('\'') {
            self.advance();
        }
    }

    fn read_attribute(&mut self) -> String {
        let mut out = String::new();
        out.push(self.advance().unwrap()); // '#'
        if self.peek() == Some('!') {
            out.push(self.advance().unwrap());
        }
        if self.peek() != Some('[') {
            return out;
        }
        out.push(self.advance().unwrap()); // '['
        let mut bracket_depth = 1;
        while let Some(c) = self.advance() {
            out.push(c);
            if c == '[' {
                bracket_depth += 1;
            } else if c == ']' {
                bracket_depth -= 1;
                if bracket_depth == 0 {
                    break;
                }
            }
        }
        out
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), String> {
        loop {
            self.skip_whitespace();
            if self.peek() == Some('/') && self.peek_ahead(1) == Some('/') {
                self.skip_line_comment();
            } else if self.peek() == Some('/') && self.peek_ahead(1) == Some('*') {
                self.skip_block_comment()?;
            } else {
                break;
            }
        }
        Ok(())
    }

    fn read_path(&mut self) -> String {
        let mut path = String::new();

        // Check if path starts with leading `::`
        if self.peek() == Some(':') && self.peek_ahead(1) == Some(':') {
            path.push(self.advance().unwrap());
            path.push(self.advance().unwrap());
            let _ = self.skip_whitespace_and_comments();
        }

        loop {
            let start_len = path.len();
            while let Some(c) = self.peek() {
                if c.is_ascii_alphanumeric() || c == '_' {
                    path.push(self.advance().unwrap());
                } else {
                    break;
                }
            }
            if path.len() == start_len {
                break;
            }

            // Check if next token is `::` allowing whitespace/comments
            let saved = (self.pos, self.line, self.col);
            let _ = self.skip_whitespace_and_comments();
            if self.peek() == Some(':') && self.peek_ahead(1) == Some(':') {
                self.advance();
                self.advance();
                let _ = self.skip_whitespace_and_comments();
                if let Some(next_c) = self.peek() {
                    if next_c.is_ascii_alphabetic() || next_c == '_' {
                        path.push_str("::");
                        continue;
                    }
                }
            }
            // Not a continuing path segment; restore cursor
            self.pos = saved.0;
            self.line = saved.1;
            self.col = saved.2;
            break;
        }

        path
    }
}
