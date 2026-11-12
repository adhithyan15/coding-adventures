//! # Minimal `Cargo.toml` Facts (VER08a §3.5, §3.6)
//!
//! Rust source is not the only thing that decides what code runs: `Cargo.toml`
//! can mark the crate as a procedural macro, point the library or a build script
//! at a file the scanner skips, and pull in third-party dependencies.  This module
//! extracts exactly those facts with a deliberately small TOML-subset reader
//! (zero dependencies).
//!
//! ```text
//!   proc-macro = true            anywhere outside a dependency table -> CAP003
//!   path = "tests/x.rs"          anywhere outside a dependency table -> CAP003
//!                                (if it escapes the scanned source set)
//!   build = "../b.rs"            same rule as `path`
//!   [dependencies] / [build-dependencies] / [target.*.dependencies]
//!                  names         -> reported as unanalyzed
//! ```
//!
//! ## Why not a line reader
//!
//! The first version read one line at a time.  A line reader loses every bit of
//! state TOML carries *between* lines, and each loss was an evasion route:
//! `lib = { path = ".." }` (inline table at the top level), `bin = [{ path = .. }]`
//! (array of inline tables), `package = { build = ".." }`, quoted table names
//! (`["lib"]`), multi-line `'''..'''` strings whose body *looks like* a header, and
//! multi-line arrays containing `[` lines that reset the "current section".
//!
//! This reader tokenises the whole file, so strings, arrays and inline tables
//! are consumed as units.  It then flattens every value into a full key path
//! (`bin`, `[]`, `path`) and applies one rule to that path, however the author
//! spelled it.
//!
//! ## Fail closed
//!
//! * any syntax the reader does not understand -> one finding (CAP003);
//! * the keys `path`, `build`, `proc-macro`, `proc_macro` must have a value the
//!   reader can verify (a plain single-line string without escapes that stays
//!   inside the scanned set; `false` for `build`/`proc-macro`).  A multi-line
//!   string, an escape sequence (`"t\u0065sts/x.rs"`), a table or an array there
//!   is a finding, because the analyzer cannot tell where it points;
//! * the exemptions are exactly the dependency tables (a dependency's `path`
//!   names *another* crate), `[features]`, and `path` inside `[[test]]` /
//!   `[[bench]]` / `[[example]]` (not shipped, VER08a G-5).

use std::fs;
use std::path::Path;

use crate::parser::Edition;

/// A fail-closed finding inside `Cargo.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CargoFinding {
    pub line: usize,
    pub reason: String,
    pub evidence: String,
}

/// Facts extracted from `Cargo.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CargoFacts {
    pub findings: Vec<CargoFinding>,
    /// Sorted, de-duplicated names under dependency / build-dependency tables.
    pub dependencies: Vec<String>,
    /// The edition every target is compiled with, or `Unknown` (see [`EditionState`]).
    pub edition: Edition,
    /// `[lib] path` when it is a plain string (the certificate requires the
    /// library root to be `src/lib.rs`, see `certificate.rs`).
    pub lib_path: Option<String>,
}

/// Target kinds whose `path` names a crate root (`[lib]`, `[[bin]]`, ...).
const TARGET_TABLES: &[&str] = &["lib", "bin", "example", "test", "bench"];

/// Why a target root or build script that is not a `.rs` file is CAP003.
///
/// Cargo compiles whatever file `path` / `build` names, whatever its
/// extension, but the scanner (`scanner.rs`) reads only `*.rs` files.  So
/// `[lib] path = "src/lib.txt"` compiled `lib.txt` while the analyzer scanned
/// a decoy `src/lib.rs` and printed PASS - and the certificate said "proved"
/// (security review round 4, H5, cases `cargo-lib-nonrs`, `cargo-bin-nonrs`,
/// `cargo-build-nonrs`).  The extension is compared exactly (`.RS` is not
/// scanned either).
const NON_RS_REASON: &str =
    "does not name a `.rs` file; the scanner reads only `.rs` files, so this target root would not be analysed";

/// What the reader has learned about the edition so far.
///
/// Only one spelling is trusted: `[package] edition = "20xx"` as a plain
/// single-line string.  Everything else that *could* set an edition makes it
/// `Unknown`, because the lexer would otherwise read some file with the wrong
/// rules (HIGH-7):
///
/// * an inherited `edition.workspace = true` (the value lives in another file);
/// * a per-target `edition` key (`[lib] edition = ..`, `[[bin]] edition = ..`):
///   targets may then disagree;
/// * any other form of the value (multi-line, escaped, table, scalar).
///
/// `[workspace.package] edition` only feeds members that opt in, so it is ignored.
#[derive(Default)]
struct EditionState {
    package: Option<String>,
    conflict: bool,
}

use crate::scanner::SKIPPED_ROOT_DIRS as UNSCANNED_ROOTS;

/// True if a Cargo `path`/`build` value would point outside the scanned set.
pub fn path_escapes_scan(raw: &str) -> bool {
    let p = raw.replace('\\', "/");
    if p.starts_with('/') || p.contains(':') {
        return true;
    }
    let mut comps = p.split('/').filter(|c| !c.is_empty() && *c != ".");
    let comp_vec: Vec<&str> = comps.by_ref().collect();
    if comp_vec.contains(&"..") {
        return true;
    }
    if comp_vec.iter().any(|c| c.starts_with('.')) {
        return true;
    }
    // Case-insensitive filesystems resolve `Tests/` to `tests/`.  Only the first component
    // is a root: the scanner skips the unscanned names at the package root only (D7), so
    // `src/tests/a.rs` is scanned.
    match comp_vec.first() {
        Some(first) => UNSCANNED_ROOTS.iter().any(|r| first.eq_ignore_ascii_case(r)),
        None => false,
    }
}

/// Keys whose value decides which code is compiled.
const WATCHED_KEYS: &[&str] = &["path", "build", "proc-macro", "proc_macro"];

/// Maximum nesting of arrays / inline tables before the reader gives up.
const MAX_VALUE_DEPTH: usize = 32;

/// A parsed TOML value (only as much structure as the rules need).
#[derive(Debug)]
enum Val {
    /// A string; `multi` is true for `"""` / `'''`, `escaped` if a basic string held a `\`.
    Str { text: String, multi: bool, escaped: bool },
    /// Any other scalar, as written (`true`, `1`, a date ...).
    Scalar(String),
    Array(Vec<(usize, Val)>),
    Table(Vec<(Vec<String>, usize, Val)>),
}

struct Reader {
    c: Vec<char>,
    i: usize,
    line: usize,
}

type R<T> = Result<T, String>;

impl Reader {
    fn peek(&self, off: usize) -> Option<char> {
        self.c.get(self.i + off).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.c.get(self.i).copied()?;
        self.i += 1;
        if ch == '\n' {
            self.line += 1;
        }
        Some(ch)
    }

    fn starts_with3(&self, q: char) -> bool {
        self.peek(0) == Some(q) && self.peek(1) == Some(q) && self.peek(2) == Some(q)
    }

    /// Spaces and tabs only.
    fn skip_blank(&mut self) {
        while matches!(self.peek(0), Some(' ' | '\t')) {
            self.bump();
        }
    }

    fn skip_comment(&mut self) {
        while !matches!(self.peek(0), None | Some('\n')) {
            self.bump();
        }
    }

    /// Blanks, newlines (incl. CR) and `#` comments.
    fn skip_all(&mut self) {
        loop {
            match self.peek(0) {
                Some(' ' | '\t' | '\n' | '\r') => {
                    self.bump();
                }
                Some('#') => self.skip_comment(),
                _ => return,
            }
        }
    }

    /// After a statement: only blanks, an optional comment, then newline / EOF.
    fn end_of_statement(&mut self) -> R<()> {
        self.skip_blank();
        if self.peek(0) == Some('#') {
            self.skip_comment();
        }
        match self.peek(0) {
            None => Ok(()),
            Some('\n') => {
                self.bump();
                Ok(())
            }
            Some('\r') if self.peek(1) == Some('\n') => {
                self.bump();
                self.bump();
                Ok(())
            }
            Some(ch) => Err(format!("unexpected `{ch}` after a value")),
        }
    }

    fn string(&mut self) -> R<Val> {
        let quote = self.peek(0).ok_or("unexpected end of file")?;
        let multi = self.starts_with3(quote);
        let mut text = String::new();
        let mut escaped = false;
        if multi {
            for _ in 0..3 {
                self.bump();
            }
            loop {
                if self.starts_with3(quote) {
                    for _ in 0..3 {
                        self.bump();
                    }
                    // Up to two further quotes belong to the body.
                    for _ in 0..2 {
                        if self.peek(0) == Some(quote) {
                            text.push(quote);
                            self.bump();
                        }
                    }
                    return Ok(Val::Str { text, multi, escaped });
                }
                match self.bump() {
                    None => return Err("unterminated multi-line string".into()),
                    Some('\\') if quote == '"' => {
                        escaped = true;
                        text.push('\\');
                        if let Some(n) = self.bump() {
                            text.push(n);
                        }
                    }
                    Some(ch) => text.push(ch),
                }
            }
        }
        self.bump();
        loop {
            match self.bump() {
                None | Some('\n') => return Err("unterminated string".into()),
                Some(ch) if ch == quote => return Ok(Val::Str { text, multi, escaped }),
                Some('\\') if quote == '"' => {
                    escaped = true;
                    text.push('\\');
                    match self.bump() {
                        None | Some('\n') => return Err("unterminated string".into()),
                        Some(n) => text.push(n),
                    }
                }
                Some(ch) => text.push(ch),
            }
        }
    }

    /// A dotted key: `a`, `"a"`, `a . 'b'`.
    fn key(&mut self) -> R<Vec<String>> {
        let is_bare = |ch: char| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-';
        let mut parts = Vec::new();
        loop {
            self.skip_blank();
            match self.peek(0) {
                Some('"') | Some('\'') => match self.string()? {
                    Val::Str { text, multi: false, escaped: false } => parts.push(text),
                    _ => return Err("quoted key with an escape or multi-line form".into()),
                },
                Some(ch) if is_bare(ch) => {
                    let mut s = String::new();
                    while let Some(ch) = self.peek(0).filter(|&ch| is_bare(ch)) {
                        s.push(ch);
                        self.bump();
                    }
                    parts.push(s);
                }
                _ => return Err("expected a key".into()),
            }
            self.skip_blank();
            if self.peek(0) == Some('.') {
                self.bump();
            } else {
                return Ok(parts);
            }
        }
    }

    fn value(&mut self, depth: usize) -> R<Val> {
        if depth > MAX_VALUE_DEPTH {
            return Err("values nested too deeply".into());
        }
        self.skip_blank();
        match self.peek(0) {
            Some('"') | Some('\'') => self.string(),
            Some('[') => {
                self.bump();
                let mut items = Vec::new();
                loop {
                    self.skip_all();
                    if self.peek(0) == Some(']') {
                        self.bump();
                        return Ok(Val::Array(items));
                    }
                    let line = self.line;
                    let v = self.value(depth + 1)?;
                    items.push((line, v));
                    self.skip_all();
                    match self.peek(0) {
                        Some(',') => {
                            self.bump();
                        }
                        Some(']') => {}
                        _ => return Err("expected `,` or `]` in an array".into()),
                    }
                }
            }
            Some('{') => {
                self.bump();
                let mut entries = Vec::new();
                loop {
                    self.skip_all();
                    if self.peek(0) == Some('}') {
                        self.bump();
                        return Ok(Val::Table(entries));
                    }
                    let line = self.line;
                    let k = self.key()?;
                    self.skip_blank();
                    if self.peek(0) != Some('=') {
                        return Err("expected `=` in an inline table".into());
                    }
                    self.bump();
                    let v = self.value(depth + 1)?;
                    entries.push((k, line, v));
                    self.skip_all();
                    match self.peek(0) {
                        Some(',') => {
                            self.bump();
                        }
                        Some('}') => {}
                        _ => return Err("expected `,` or `}` in an inline table".into()),
                    }
                }
            }
            _ => {
                let mut s = String::new();
                while let Some(ch) = self.peek(0) {
                    if ch.is_whitespace() || matches!(ch, ',' | ']' | '}' | '#' | '"' | '\'' | '=' | '[' | '{') {
                        break;
                    }
                    s.push(ch);
                    self.bump();
                }
                if s.is_empty() {
                    Err("expected a value".into())
                } else {
                    Ok(Val::Scalar(s))
                }
            }
        }
    }
}

/// Dependency-like tables whose entries name *other* crates.
fn is_dep_table_name(s: &str) -> bool {
    matches!(
        s,
        "dependencies" | "dev-dependencies" | "build-dependencies" | "dev_dependencies" | "build_dependencies"
    )
}

/// `Some(index of the dependency name within the path)` for a dependency table path.
fn dependency_name_index(path: &[String]) -> Option<usize> {
    if path.first().map(String::as_str) == Some("target") && path.len() >= 3 && is_dep_table_name(&path[2]) {
        return Some(3);
    }
    if path.first().is_some_and(|p| is_dep_table_name(p)) {
        return Some(1);
    }
    None
}

/// Tables exempt from the watched-key rule that are not reported as dependencies.
fn is_exempt_table(path: &[String]) -> bool {
    let first = path.first().map(String::as_str);
    first == Some("features")
        || first == Some("patch")
        || first == Some("replace")
        || (first == Some("workspace") && path.get(1).map(String::as_str) == Some("dependencies"))
}

struct Checker<'a> {
    facts: &'a mut CargoFacts,
    edition: &'a mut EditionState,
}

impl Checker<'_> {
    fn finding(&mut self, line: usize, reason: String, evidence: String) {
        self.facts.findings.push(CargoFinding { line, reason, evidence });
    }

    /// Apply the rules to one full key path (and its value, if it has one).
    fn check(&mut self, path: &[String], val: Option<&Val>, line: usize) {
        if let Some(idx) = dependency_name_index(path) {
            let is_dev = path[0].starts_with("dev")
                || (path[0] == "target" && path[2].starts_with("dev"));
            if !is_dev {
                if let Some(name) = path.get(idx) {
                    self.facts.dependencies.push(name.clone());
                }
            }
            return;
        }
        if path.is_empty() || is_exempt_table(path) {
            return;
        }
        if path.iter().any(|s| s == "edition") && path[0] != "workspace" {
            match (path, val) {
                ([p, e], Some(Val::Str { text, multi: false, escaped: false })) if p == "package" && e == "edition" => {
                    if self.edition.package.replace(text.clone()).is_some() {
                        self.edition.conflict = true;
                    }
                }
                _ => self.edition.conflict = true,
            }
        }
        let Some(k) = path.iter().position(|s| WATCHED_KEYS.contains(&s.as_str())) else {
            return;
        };
        let key = path[k].clone();
        let is_target_root = key == "path" && TARGET_TABLES.contains(&path[0].as_str());
        if matches!(path[0].as_str(), "test" | "bench" | "example") && key == "path" {
            // Not shipped (G-5), so where it points is not examined - but it
            // must still be a `.rs` file (H5).
            if let Some(Val::Str { text, multi: false, escaped: false }) = val {
                if k + 1 == path.len() && !text.ends_with(".rs") {
                    self.finding(line, format!("{key} {NON_RS_REASON}"), format!("{key} = \"{text}\""));
                }
            }
            return;
        }
        let evidence = format!("{} ...", path.join("."));
        let ambiguous = |this: &mut Self, why: &str| {
            this.finding(
                line,
                format!("Cargo.toml key `{key}` has {why}; the analyzer cannot verify what it selects"),
                evidence.clone(),
            );
        };
        if k + 1 != path.len() {
            ambiguous(self, "a nested structure under it");
            return;
        }
        let Some(val) = val else {
            ambiguous(self, "a table header form");
            return;
        };
        match (key.as_str(), val) {
            ("path" | "build", Val::Str { text, multi: false, escaped: false }) => {
                if path_escapes_scan(text) {
                    self.finding(
                        line,
                        format!("{key} points outside the scanned source set"),
                        format!("{key} = \"{text}\""),
                    );
                }
                if (is_target_root || key == "build") && !text.ends_with(".rs") {
                    self.finding(line, format!("{key} {NON_RS_REASON}"), format!("{key} = \"{text}\""));
                }
                if path.len() == 2 && path[0] == "lib" && key == "path" {
                    self.facts.lib_path = Some(text.clone());
                }
            }
            ("build", Val::Scalar(s)) if s == "false" => {}
            ("proc-macro" | "proc_macro", Val::Scalar(s)) if s == "false" => {}
            ("proc-macro" | "proc_macro", Val::Scalar(s)) if s == "true" => {
                self.finding(
                    line,
                    "proc-macro crate runs arbitrary code at compile time".into(),
                    "proc-macro = true".into(),
                );
            }
            (_, Val::Str { multi: true, .. }) => ambiguous(self, "a multi-line string value"),
            (_, Val::Str { escaped: true, .. }) => ambiguous(self, "an escape sequence in its value"),
            (_, Val::Array(_)) | (_, Val::Table(_)) => ambiguous(self, "an array or table value"),
            _ => ambiguous(self, "a value of an unexpected form"),
        }
    }

    fn visit(&mut self, path: &[String], val: &Val, line: usize) {
        self.check(path, Some(val), line);
        match val {
            Val::Table(entries) => {
                for (k, l, v) in entries {
                    let mut p = path.to_vec();
                    p.extend(k.iter().cloned());
                    self.visit(&p, v, *l);
                }
            }
            Val::Array(items) => {
                for (l, v) in items {
                    let mut p = path.to_vec();
                    p.push("[]".into());
                    self.visit(&p, v, *l);
                }
            }
            _ => {}
        }
    }
}

/// Parse one statement (table header or `key = value`).  `Ok(false)` at end of file.
fn statement(
    rd: &mut Reader,
    section: &mut Vec<String>,
    facts: &mut CargoFacts,
    edition: &mut EditionState,
) -> R<bool> {
    rd.skip_all();
    let line = rd.line;
    if rd.peek(0).is_none() {
        return Ok(false);
    }
    if rd.peek(0) == Some('[') {
        let array = rd.peek(1) == Some('[');
        rd.bump();
        if array {
            rd.bump();
        }
        let key = rd.key()?;
        rd.skip_blank();
        for _ in 0..(if array { 2 } else { 1 }) {
            if rd.peek(0) != Some(']') {
                return Err("unterminated table header".into());
            }
            rd.bump();
        }
        rd.end_of_statement()?;
        *section = key;
        Checker { facts, edition }.check(section, None, line);
        return Ok(true);
    }
    let key = rd.key()?;
    rd.skip_blank();
    if rd.peek(0) != Some('=') {
        return Err("expected `=` after a key".into());
    }
    rd.bump();
    let v = rd.value(0)?;
    rd.end_of_statement()?;
    let mut full = section.clone();
    full.extend(key);
    Checker { facts, edition }.visit(&full, &v, line);
    Ok(true)
}

/// Analyse the text of a `Cargo.toml`.
pub fn analyze_cargo_text(text: &str) -> CargoFacts {
    let mut facts = CargoFacts::default();
    let mut rd = Reader {
        c: text.strip_prefix('\u{feff}').unwrap_or(text).chars().collect(),
        i: 0,
        line: 1,
    };
    let mut section: Vec<String> = Vec::new();
    let mut edition = EditionState::default();
    loop {
        match statement(&mut rd, &mut section, &mut facts, &mut edition) {
            Ok(true) => {}
            Ok(false) => break,
            Err(msg) => {
                facts.findings.push(CargoFinding {
                    line: rd.line,
                    reason: "Cargo.toml could not be parsed; the analyzer fails closed".into(),
                    evidence: msg,
                });
                break;
            }
        }
    }
    facts.dependencies.sort();
    facts.dependencies.dedup();
    // A file the reader could not finish may hold an edition key it never saw.
    let complete = facts.findings.iter().all(|f| !f.reason.contains("could not be parsed"));
    facts.edition = match (edition.package, edition.conflict, complete) {
        (Some(e), false, true) => Edition::parse(&e),
        _ => Edition::Unknown,
    };
    facts
}

/// Read and analyse `<package_dir>/Cargo.toml` (absent file: no facts).
pub fn analyze_cargo_toml(package_dir: &Path) -> Result<CargoFacts, String> {
    let path = package_dir.join("Cargo.toml");
    let meta = match fs::symlink_metadata(&path) {
        Ok(m) => m,
        Err(_) => return Ok(CargoFacts::default()),
    };
    if meta.is_symlink() {
        return Err("Cargo.toml must not be a symlink".into());
    }
    if meta.len() > 1024 * 1024 {
        return Err("Cargo.toml exceeds 1 MiB".into());
    }
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(analyze_cargo_text(&text))
}
