//! # Rust Lexer, Name Resolver and Token Visitor (VER08a §3)
//!
//! A *lexical* analyzer: it never builds a syntax tree.  Three stages:
//!
//! ```text
//!  source text ──lex──► tokens ──┬─ stage 1: collect `use` / `extern crate` bindings
//!   (per file)                   │           (package-wide table, VER08a §3.2)
//!                                └─ stage 2: walk tokens, resolve every path through
//!                                            the table, classify (rules.rs)
//! ```
//!
//! ## Why a real lexer
//!
//! The previous scanner skipped literals with ad-hoc code.  Its character-literal
//! skipper mis-handled `'\u{41}'` and could swallow the character after it; its
//! `use` handling stopped at the first `{`, so `use std::{fs, process};` looked
//! like a bare `std` (pure).  Both were evasion routes.  Here:
//!
//! * comments, strings (plain/raw/byte/C), chars and lifetimes are separate token
//!   kinds, so text inside them can never be mistaken for code;
//! * `use` trees are expanded leaf by leaf (groups, `self`, `as`, globs);
//! * aliases are looked up for *every* segment of a path, so a re-export in
//!   another file (`pub use std as s;`) is still resolved.
//!
//! ## What a token walk still cannot see
//!
//! See VER08a §8.  The walker fails *closed* (CAP003) on the constructs it can
//! see but not analyse: `include!`, `#[path]`, proc-macros, and paths spliced
//! together by a `macro_rules!` metavariable.

use std::collections::{HashMap, HashSet};

use crate::rules::{
    classify_resolved, is_local_head, is_std_root, method_call_rule, Classification, FOREIGN_LINKAGE_ATTRS,
    SHORTHAND_HEADS, UNKNOWN_TYPE_SEGMENT,
};

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

/// A construct the analyzer can see but cannot analyse (CAP003).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceUnanalyzable {
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub reason: String,
    pub evidence: String,
}

/// Results of scanning one or more source files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScanResult {
    pub detections: Vec<SourceDetection>,
    pub banned: Vec<SourceBannedConstruct>,
    pub unanalyzable: Vec<SourceUnanalyzable>,
}

/// The Rust edition the scanned files are compiled with (from `Cargo.toml`).
///
/// The edition changes how text is *lexed*.  Edition 2021 added C-string
/// literals: `cr#"..."#` is one raw C string there, but in 2015/2018 it is the
/// identifier `cr`, a `#` punctuation token and an ordinary string.  Reading
/// a 2018 crate with 2021 rules turned real code into "string contents" and
/// hid it (security review round 3, HIGH-7, PoC t8).
///
/// `Unknown` (no `Cargo.toml`, no `package.edition`, an inherited
/// `edition.workspace = true`, a per-target `edition` key, or a future edition)
/// makes every construct whose reading depends on the edition `CAP003`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Edition {
    E2015,
    E2018,
    E2021,
    E2024,
    #[default]
    Unknown,
}

impl Edition {
    /// Parse a `package.edition` value; anything unrecognised is `Unknown`.
    pub fn parse(s: &str) -> Self {
        match s {
            "2015" => Self::E2015,
            "2018" => Self::E2018,
            "2021" => Self::E2021,
            "2024" => Self::E2024,
            _ => Self::Unknown,
        }
    }

    /// `Some(true)` if `c"`/`cr"` start C strings, `Some(false)` if they are an
    /// identifier followed by a string, `None` if the edition is unknown.
    fn has_c_strings(self) -> Option<bool> {
        match self {
            Self::E2015 | Self::E2018 => Some(false),
            Self::E2021 | Self::E2024 => Some(true),
            Self::Unknown => None,
        }
    }
}

/// Scan a single Rust source file string (edition unknown: see [`Edition`]).
pub fn scan_source(file_rel_path: &str, content: &str) -> Result<ScanResult, String> {
    scan_sources(&[(file_rel_path, content)]).map_err(|e| {
        // Strip the "<file>: " prefix added for multi-file scans.
        let prefix = format!("{file_rel_path}: ");
        e.strip_prefix(&prefix).map(str::to_string).unwrap_or(e)
    })
}

// ---------------------------------------------------------------------------
// Lexer
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
enum Tok {
    /// Identifier (raw identifiers are normalised; `raw` records `r#`).
    Ident { name: String, raw: bool },
    Lifetime,
    /// String-like literal; payload is the *source text* between the quotes.
    Str(String),
    Char,
    Num,
    Punct(char),
    /// `::`
    PathSep,
}

#[derive(Clone, Debug)]
struct Token {
    tok: Tok,
    line: usize,
    col: usize,
    /// Character offsets `[start, end)` into the file.
    start: usize,
    end: usize,
}

/// Something the lexer saw but cannot lex the way rustc would (CAP003).
#[derive(Clone, Debug)]
struct LexNote {
    line: usize,
    col: usize,
    reason: &'static str,
    evidence: String,
}

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    notes: Vec<LexNote>,
    /// Do `c"` / `cr"` start C strings?  `None`: edition unknown.
    c_strings: Option<bool>,
}

/// Why non-ASCII text outside a literal or comment is CAP003 (HIGH-6).
///
/// rustc accepts identifiers made of Unicode `XID_Start`/`XID_Continue`
/// characters and compares them after NFC normalisation.  This crate has no
/// dependencies and no Unicode tables, and `char::is_alphanumeric` is a
/// *different* set: U+00B7 MIDDLE DOT is `XID_Continue` but not alphanumeric,
/// so `extern crate std as s·x;` gave the analyzer the alias `s` while rustc
/// bound `s·x` (PoC t4).  Rather than guess, any non-ASCII character that is
/// not Rust whitespace and sits outside a string, character literal or comment
/// is lexed as part of an identifier and reported.  rustc either accepts it as
/// an identifier we cannot compare soundly or rejects the file; both are
/// "not a PASS".
const NON_ASCII_REASON: &str =
    "non-ASCII character outside a literal or comment (identifier rules are not implemented; fail closed)";

/// ASCII identifier characters, plus every non-ASCII character that is not
/// whitespace (reported through [`NON_ASCII_REASON`]).
fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_ascii_alphabetic() || (!c.is_ascii() && !is_rust_whitespace(c))
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric() || (!c.is_ascii() && !is_rust_whitespace(c))
}

/// Numeric literal body and suffix: ASCII only, so a non-ASCII suffix starts
/// an identifier token of its own and is reported.
fn is_ascii_ident_continue(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

impl Lexer {
    /// Report a lexeme (identifier or lifetime) that contains non-ASCII text.
    fn note_if_non_ascii(&mut self, text: &str, line: usize, col: usize) {
        if !text.is_ascii() {
            self.notes.push(LexNote {
                line,
                col,
                reason: NON_ASCII_REASON,
                evidence: text.chars().take(32).collect(),
            });
        }
    }

    fn peek(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    /// If a raw-string opener (`r"`, `r#"`, ...) starts at `off`, return the number of `#`.
    fn raw_string_hashes(&self, off: usize) -> Option<usize> {
        if self.peek(off) != Some('r') {
            return None;
        }
        let mut n = 0;
        while self.peek(off + 1 + n) == Some('#') {
            n += 1;
        }
        if self.peek(off + 1 + n) == Some('"') {
            Some(n)
        } else {
            None
        }
    }

    /// Consume a raw string whose `r` is at the cursor; returns its inner text.
    fn raw_string(&mut self, hashes: usize) -> Result<String, String> {
        self.bump(); // r
        for _ in 0..hashes {
            self.bump();
        }
        self.bump(); // opening quote
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("unterminated raw string".into()),
                Some('"') => {
                    let mut matched = 0;
                    while matched < hashes && self.peek(0) == Some('#') {
                        self.bump();
                        matched += 1;
                    }
                    if matched == hashes {
                        return Ok(out);
                    }
                    out.push('"');
                    for _ in 0..matched {
                        out.push('#');
                    }
                }
                Some(c) => out.push(c),
            }
        }
    }

    /// Consume a cooked string whose opening quote is at the cursor.
    fn string(&mut self) -> Result<String, String> {
        self.bump(); // opening quote
        let mut out = String::new();
        loop {
            match self.bump() {
                None => return Err("unterminated string literal".into()),
                Some('"') => return Ok(out),
                Some('\\') => {
                    out.push('\\');
                    if let Some(c) = self.bump() {
                        out.push(c);
                    }
                }
                Some(c) => out.push(c),
            }
        }
    }

    fn block_comment(&mut self) -> Result<(), String> {
        self.bump();
        self.bump();
        let mut depth = 1usize;
        while let Some(c) = self.bump() {
            if c == '/' && self.peek(0) == Some('*') {
                self.bump();
                depth += 1;
            } else if c == '*' && self.peek(0) == Some('/') {
                self.bump();
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
        }
        Err("unterminated block comment".into())
    }

    /// Cursor is on a `'`.  Decide lifetime vs character literal.
    fn quote(&mut self) -> Tok {
        match (self.peek(1), self.peek(2)) {
            (Some('\\'), _) => {
                self.bump(); // '
                self.bump(); // backslash
                self.bump(); // escaped char (handles \' and \\)
                while let Some(c) = self.peek(0) {
                    self.bump();
                    if c == '\'' {
                        break;
                    }
                }
                Tok::Char
            }
            (Some(c1), Some('\'')) if c1 != '\'' => {
                self.bump();
                self.bump();
                self.bump();
                Tok::Char
            }
            (Some(c1), _) if is_ident_start(c1) => {
                let (line, col) = (self.line, self.col);
                let mut text = String::from("'");
                self.bump();
                while let Some(c) = self.peek(0).filter(|&c| is_ident_continue(c)) {
                    text.push(c);
                    self.bump();
                }
                self.note_if_non_ascii(&text, line, col);
                Tok::Lifetime
            }
            _ => {
                self.bump();
                Tok::Punct('\'')
            }
        }
    }
}

/// Length (in chars) of the shebang line that rustc would skip, or 0.
///
/// The analyzer must read the same tokens rustc reads.  Its old lexer treated
/// `#!/x"` as `#`, `!`, `/`, `x` and the *start of a string*, which then
/// swallowed the real code that followed (security review round 3, HIGH-3):
///
/// ```text
///   #!/x"                                   rustc: shebang line, ignored
///   pub fn f() { std::fs::write(..) }       rustc: code      old analyzer: "string"
///   // "                                    rustc: comment   old analyzer: string ends
/// ```
///
/// This mirrors `rustc_lexer::strip_shebang` exactly:
///
/// 1. the file (after a leading byte-order mark) must start with `#!`;
/// 2. skip whitespace and *non-doc* comments (`//`, `////..`, `/* */`, `/***/`);
/// 3. if the next token is `[`, the text is an inner attribute (`#![no_std]`)
///    and nothing is skipped; anything else (including a doc comment, which is
///    a token) makes the whole first line a shebang.
///
/// The skipped length is the first line without its `\n`; a `\r` before the
/// `\n` is left behind as whitespace, exactly as `str::lines` trims it there.
fn shebang_len(chars: &[char]) -> usize {
    if chars.first() != Some(&'#') || chars.get(1) != Some(&'!') {
        return 0;
    }
    let at = |i: usize| chars.get(i).copied();
    let mut i = 2;
    loop {
        match at(i) {
            Some(c) if is_rust_whitespace(c) => i += 1,
            // `//` is a plain comment unless it is `//!` or exactly `///`.
            Some('/') if at(i + 1) == Some('/') => {
                let doc = at(i + 2) == Some('!') || (at(i + 2) == Some('/') && at(i + 3) != Some('/'));
                if doc {
                    break;
                }
                while at(i).is_some_and(|c| c != '\n') {
                    i += 1;
                }
            }
            // `/*` is a plain comment unless it is `/*!` or `/**x` (`/**/` and
            // `/***` are plain).  An unterminated one is still a plain comment
            // token to `strip_shebang`, after which there is no `[`.
            Some('/') if at(i + 1) == Some('*') => {
                let doc = at(i + 2) == Some('!')
                    || (at(i + 2) == Some('*') && !matches!(at(i + 3), Some('*') | Some('/')));
                if doc {
                    break;
                }
                let mut depth = 1usize;
                i += 2;
                while depth > 0 {
                    match (at(i), at(i + 1)) {
                        (None, _) => break,
                        (Some('/'), Some('*')) => {
                            depth += 1;
                            i += 2;
                        }
                        (Some('*'), Some('/')) => {
                            depth -= 1;
                            i += 2;
                        }
                        _ => i += 1,
                    }
                }
            }
            _ => break,
        }
    }
    if at(i) == Some('[') {
        return 0;
    }
    chars.iter().position(|&c| c == '\n').unwrap_or(chars.len())
}

/// Rust's whitespace: exactly the Unicode `Pattern_White_Space` set (the
/// Reference, "Whitespace"), which is what `rustc_lexer::is_whitespace` uses.
fn is_rust_whitespace(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{0B}' | '\u{0C}' | '\r' | ' ' | '\u{85}' | '\u{200E}' | '\u{200F}' | '\u{2028}' | '\u{2029}'
    )
}

fn lex(content: &str, edition: Edition) -> Result<(Vec<Token>, Vec<LexNote>), String> {
    // rustc removes one leading byte-order mark, then a shebang line.
    let content = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut lx = Lexer {
        chars: content.chars().collect(),
        pos: 0,
        line: 1,
        col: 0,
        notes: Vec::new(),
        c_strings: edition.has_c_strings(),
    };
    // Skipping with `bump` keeps line/column numbers true to the file.
    for _ in 0..shebang_len(&lx.chars) {
        lx.bump();
    }
    let mut toks = Vec::new();

    while let Some(c) = lx.peek(0) {
        // Exactly Pattern_White_Space: `char::is_whitespace` is a different set
        // (it lacks U+200E/U+200F, which rustc skips, so `std::fs::<U+200E>write`
        // read as `std::fs` plus a stray character - PoC t9).  A byte-order
        // mark is only skipped at the start of the file (above).
        if is_rust_whitespace(c) {
            lx.bump();
            continue;
        }
        if c == '/' && lx.peek(1) == Some('/') {
            while let Some(ch) = lx.peek(0) {
                if ch == '\n' {
                    break;
                }
                lx.bump();
            }
            continue;
        }
        if c == '/' && lx.peek(1) == Some('*') {
            lx.block_comment()?;
            continue;
        }

        let (line, col, start) = (lx.line, lx.col, lx.pos);
        let tok = if c == '"' {
            Tok::Str(lx.string()?)
        } else if let Some(h) = lx.raw_string_hashes(0) {
            Tok::Str(lx.raw_string(h)?)
        } else if c == 'b' && lx.peek(1) == Some('"') {
            lx.bump();
            Tok::Str(lx.string()?)
        } else if c == 'c' && lx.peek(1) == Some('"') && lx.c_strings != Some(false) {
            // A cooked C string has the same boundaries as `c` followed by an
            // ordinary string (same escapes), so an unknown edition is harmless
            // here; only before 2021 is `c` lexed as an identifier.
            lx.bump();
            Tok::Str(lx.string()?)
        } else if let (true, Some(h)) = (c == 'b', lx.raw_string_hashes(1)) {
            lx.bump();
            Tok::Str(lx.raw_string(h)?)
        } else if let (true, Some(h)) = (c == 'c' && lx.c_strings != Some(false), lx.raw_string_hashes(1)) {
            // A raw C string `cr#"..."#` (2021+) and `cr` `#` `"..."` (2015/2018)
            // end at different quotes, so with an unknown edition one of the two
            // readings may hide code: fail closed.  The raw reading is then
            // used only to keep lexing the rest of the file.
            if lx.c_strings.is_none() {
                lx.notes.push(LexNote {
                    line,
                    col,
                    reason: "raw C-string prefix `cr` whose meaning depends on the edition, which is unknown (Cargo.toml `package.edition`)",
                    evidence: "cr\"..\"".into(),
                });
            }
            lx.bump();
            Tok::Str(lx.raw_string(h)?)
        } else if c == 'b' && lx.peek(1) == Some('\'') {
            lx.bump();
            lx.quote()
        } else if c == '\'' {
            lx.quote()
        } else if is_ident_start(c) {
            // Raw identifier `r#name` (the raw-string case was handled above).
            if c == 'r' && lx.peek(1) == Some('#') && lx.peek(2).is_some_and(is_ident_start) {
                lx.bump();
                lx.bump();
                let mut name = String::new();
                while let Some(ch) = lx.peek(0).filter(|&ch| is_ident_continue(ch)) {
                    name.push(ch);
                    lx.bump();
                }
                lx.note_if_non_ascii(&name, line, col);
                Tok::Ident { name, raw: true }
            } else {
                let mut name = String::new();
                while let Some(ch) = lx.peek(0).filter(|&ch| is_ident_continue(ch)) {
                    name.push(ch);
                    lx.bump();
                }
                lx.note_if_non_ascii(&name, line, col);
                Tok::Ident { name, raw: false }
            }
        } else if c.is_ascii_digit() {
            while lx.peek(0).is_some_and(is_ascii_ident_continue) {
                lx.bump();
            }
            if lx.peek(0) == Some('.') && lx.peek(1).is_some_and(|d| d.is_ascii_digit()) {
                lx.bump();
                while lx.peek(0).is_some_and(is_ascii_ident_continue) {
                    lx.bump();
                }
            }
            Tok::Num
        } else if c == ':' && lx.peek(1) == Some(':') {
            lx.bump();
            lx.bump();
            Tok::PathSep
        } else {
            lx.bump();
            Tok::Punct(c)
        };
        toks.push(Token {
            tok,
            line,
            col,
            start,
            end: lx.pos,
        });
    }
    Ok((toks, lx.notes))
}

// ---------------------------------------------------------------------------
// Stage 1: `use` trees, `extern crate`, binding table
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
struct UseLeaf {
    path: Vec<String>,
    /// `Some("_")` means anonymous.
    alias: Option<String>,
    glob: bool,
    line: usize,
    col: usize,
}

fn ident_at(toks: &[Token], i: usize) -> Option<(&str, bool)> {
    match toks.get(i).map(|t| &t.tok) {
        Some(Tok::Ident { name, raw }) => Some((name.as_str(), *raw)),
        _ => None,
    }
}

fn is_punct(toks: &[Token], i: usize, c: char) -> bool {
    matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Punct(p)) if *p == c)
}

fn is_pathsep(toks: &[Token], i: usize) -> bool {
    matches!(toks.get(i).map(|t| &t.tok), Some(Tok::PathSep))
}

/// Parse one use-tree starting at `*i`.  Returns false on a syntax surprise.
fn parse_use_tree(
    toks: &[Token],
    i: &mut usize,
    prefix: &[String],
    out: &mut Vec<UseLeaf>,
    depth: usize,
) -> bool {
    if depth > 32 {
        return false;
    }
    let mut path: Vec<String> = prefix.to_vec();
    if is_pathsep(toks, *i) {
        *i += 1;
    }
    let (mut line, mut col);
    loop {
        if let Some((name, raw)) = ident_at(toks, *i) {
            let t = &toks[*i];
            line = t.line;
            col = t.col;
            let leaf_self = name == "self" && !raw && !prefix.is_empty() && !is_pathsep(toks, *i + 1);
            *i += 1;
            if !leaf_self {
                path.push(name.to_string());
            }
            if is_pathsep(toks, *i) {
                *i += 1;
                continue;
            }
            break;
        } else if is_punct(toks, *i, '{') {
            *i += 1;
            loop {
                if is_punct(toks, *i, '}') {
                    *i += 1;
                    return true;
                }
                if !parse_use_tree(toks, i, &path, out, depth + 1) {
                    return false;
                }
                if is_punct(toks, *i, ',') {
                    *i += 1;
                } else if is_punct(toks, *i, '}') {
                    *i += 1;
                    return true;
                } else {
                    return false;
                }
            }
        } else if is_punct(toks, *i, '*') {
            let t = &toks[*i];
            *i += 1;
            out.push(UseLeaf {
                path,
                alias: None,
                glob: true,
                line: t.line,
                col: t.col,
            });
            return true;
        } else {
            return false;
        }
    }
    let mut alias = None;
    if let Some(("as", false)) = ident_at(toks, *i) {
        *i += 1;
        match ident_at(toks, *i) {
            Some((a, _)) => {
                alias = Some(a.to_string());
                *i += 1;
            }
            None => return false,
        }
    }
    out.push(UseLeaf {
        path,
        alias,
        glob: false,
        line,
        col,
    });
    true
}

/// Index just past the `>` that closes the `<` at `open`, or `None` (also past 256 tokens).
fn skip_angle(toks: &[Token], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for k in open..toks.len().min(open + 256) {
        if is_punct(toks, k, '<') {
            depth += 1;
        } else if is_punct(toks, k, '>') && !(k > 0 && is_punct(toks, k - 1, '-')) {
            depth = depth.checked_sub(1)?;
            if depth == 0 {
                return Some(k + 1);
            }
        }
    }
    None
}

/// Like [`skip_angle`], but a `(..)`, `[..]` or `{..}` group is skipped
/// whole, so a `>` inside one (`<const B: bool = { 2 > 1 }, D>`,
/// `<T = [u8; (2 > 1) as usize]>`) does not close the list.  For generic
/// *parameter* lists, whose brackets are always balanced in valid Rust.
fn skip_angle_bracketed(toks: &[Token], br: &Brackets, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut k = open;
    while k < toks.len().min(open + 256) {
        match &toks[k].tok {
            Tok::Punct('(' | '[' | '{') => {
                k = br.close(k).saturating_add(1).max(k + 1);
                continue;
            }
            Tok::Punct(')' | ']' | '}') => return None,
            Tok::Punct('<') => depth += 1,
            Tok::Punct('>') if !(k > 0 && is_punct(toks, k - 1, '-')) => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(k + 1);
                }
            }
            _ => {}
        }
        k += 1;
    }
    None
}

/// The names a generic parameter list declares: `open` is its `<`, `end` is
/// just past its `>` ([`skip_angle_bracketed`]).  A name is an identifier at
/// the list's top level that starts a parameter: right after `<` or `,`, or
/// after the `]` of an attribute (`<#[allow(unused)] T>`, follow-up review
/// F2); `const N` declares `N`.  Lifetimes are not identifiers, and nothing
/// inside a bound (`T: AsRef<File>`) or a bracketed default is a name.
///
/// ```text
///   < 'a , #[cfg(x)] #[allow(y)] const N : usize , D : Tr<File> = Vec<u8> >
///                                      ^ N             ^ D   (File, Vec: not names)
/// ```
fn generic_list_names<'t>(toks: &'t [Token], br: &Brackets, open: usize, end: usize) -> Vec<&'t str> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut k = open + 1;
    while k + 1 < end {
        match &toks[k].tok {
            Tok::Punct('(' | '[' | '{') => {
                k = br.close(k).saturating_add(1).max(k + 1);
                continue;
            }
            Tok::Punct('<') => depth += 1,
            Tok::Punct('>') if !is_punct(toks, k - 1, '-') => depth = depth.saturating_sub(1),
            Tok::Ident { name, .. }
                if depth == 0 && (k == open + 1 || is_punct(toks, k - 1, ',') || closes_attribute(toks, br, k - 1)) =>
            {
                let mut n = name.as_str();
                if n == "const" {
                    if let Some((c, _)) = ident_at(toks, k + 1) {
                        n = c;
                    }
                }
                out.push(n);
            }
            _ => {}
        }
        k += 1;
    }
    out
}

/// Why a `type` item is CAP003.
const TYPE_ALIAS_META_NAME: &str =
    "type alias whose name is a macro_rules! metavariable; the name it binds is not known";
const TYPE_ALIAS_UNPARSEABLE: &str = "unparseable type alias; the names it binds are not known";

/// Longest `type` item (in tokens) read before it is refused.
const TYPE_ALIAS_LIMIT: usize = 4096;

/// Keywords that occur in types but are never a type path.
fn is_type_keyword(name: &str) -> bool {
    matches!(name, "dyn" | "impl" | "mut" | "const" | "for" | "where" | "as" | "fn" | "unsafe" | "extern")
}

/// The bindings of the `type` item at token `i` (security review round 4, H3).
///
/// `type F = File;` binds `F` exactly like `use .. File as F;` (VER08a L1).
/// The old reader took the *first* path after `=` and returned nothing for any
/// other shape, so three shapes bound nothing at all and `P::exists(..)`
/// passed:
///
/// ```text
///   type $n = std::path::Path;                              name is a metavariable
///   type P = (std::path::Path);                             parenthesised
///   type P = <std::path::PathBuf as std::ops::Deref>::Target;   qualified
/// ```
///
/// Now:
///
/// * a metavariable name is `Err` (CAP003): the analyzer cannot know which
///   name the expansion binds;
/// * the alias is bound to **every** path on the right-hand side, up to the
///   `;` that ends the item (generic arguments, parentheses, qualified
///   projections, trait objects included); the alias's own generic
///   parameters are skipped;
/// * a metavariable on the right-hand side (`type P = $t;`) also binds the
///   alias to [`UNKNOWN_TYPE_SEGMENT`], a segment no identifier can spell, so
///   no binding substitutes it and `P::name` gets the method-call verdict of
///   `.name(` (`rules::classify_segments`);
/// * so does a macro invocation (`type P = ty!();`, security review round 5,
///   R5-2): its name is not the type it expands to;
/// * so does a projection (`type X<D> = <D as Deref>::Target;`,
///   `type X<D: Deref> = D::Target;`, follow-up review F3): the type it names
///   is not evaluated;
/// * `type Item;` / `type Item: Bound;` (no `=`) binds nothing;
/// * anything the reader cannot delimit (no `;` before a closing bracket or
///   the end of the file, unbalanced generics, a name that is not an
///   identifier) is `Err` (CAP003).
///
/// The tokens stay in the walk, so the right-hand side is classified too.
fn type_alias_leaves(toks: &[Token], br: &Brackets, i: usize) -> Result<Vec<UseLeaf>, &'static str> {
    if is_punct(toks, i + 1, '$') {
        return Err(TYPE_ALIAS_META_NAME);
    }
    let Some((name, _)) = ident_at(toks, i + 1) else {
        return Err(TYPE_ALIAS_UNPARSEABLE);
    };
    let name = name.to_string();
    let stop = (i + TYPE_ALIAS_LIMIT).min(toks.len());
    let mut j = i + 2;
    // Generic parameters: `type P<'a, T: Bound, const N: usize>`.
    // Only top-level names count (`type P<D: AsRef<File>>` declares `D`, not
    // `File`), attributed ones included (`<#[allow(unused)] D>`, F2).
    let mut params: HashSet<&str> = HashSet::new();
    if is_punct(toks, j, '<') {
        let end = skip_angle_bracketed(toks, br, j).ok_or(TYPE_ALIAS_UNPARSEABLE)?;
        params.extend(generic_list_names(toks, br, j, end));
        j = end;
    }
    // Bounds and a `where` clause may precede `=`; `=` inside `<..>`
    // (`Iterator<Item = u8>`) does not count.
    let mut angle = 0usize;
    let eq = loop {
        if j >= stop {
            return Err(TYPE_ALIAS_UNPARSEABLE);
        }
        match &toks[j].tok {
            Tok::Punct('<') => angle += 1,
            Tok::Punct('>') if !is_punct(toks, j - 1, '-') => angle = angle.saturating_sub(1),
            Tok::Punct('(' | '[' | '{') => {
                j = br.close(j).saturating_add(1).max(j + 1);
                continue;
            }
            Tok::Punct(')' | ']' | '}') => return Err(TYPE_ALIAS_UNPARSEABLE),
            Tok::Punct(';') if angle == 0 => return Ok(Vec::new()),
            Tok::Punct('=') if angle == 0 && !is_punct(toks, j + 1, '>') => break j,
            _ => {}
        }
        j += 1;
    };
    // The right-hand side ends at the first `;` outside brackets.
    let mut k = eq + 1;
    let end = loop {
        if k >= stop {
            return Err(TYPE_ALIAS_UNPARSEABLE);
        }
        match &toks[k].tok {
            Tok::Punct('(' | '[' | '{') => {
                let c = br.close(k);
                if c >= toks.len() {
                    return Err(TYPE_ALIAS_UNPARSEABLE);
                }
                k = c + 1;
                continue;
            }
            Tok::Punct(')' | ']' | '}') => return Err(TYPE_ALIAS_UNPARSEABLE),
            Tok::Punct(';') => break k,
            _ => {}
        }
        k += 1;
    };
    // A projection on the right-hand side names a type the analyzer does not
    // evaluate (follow-up review F3):
    //
    //   type X<D> = <D as std::ops::Deref>::Target;    qualified group `<..>::`
    //   type X<D: Deref> = D::Target;                  own parameter, then `::`
    //
    // `X::<PathBuf>::exists(p)` is `Path::exists`, but the paths on the
    // right-hand side are only `D`, `Deref` and `Target`.  Such an alias is
    // also bound to [`UNKNOWN_TYPE_SEGMENT`], so `X::name` gets the
    // method-call verdict of `.name(`, as for a metavariable.  Any `>` `::`
    // counts, with or without `as`: in type position a turbofish-qualified
    // `Vec::<u8>::Assoc` is a projection too.
    let mut projection = (eq + 1..end).any(|k| {
        is_punct(toks, k, '>') && is_pathsep(toks, k + 1) && !(is_punct(toks, k - 1, '-') || is_punct(toks, k - 1, '='))
    });
    let mut leaves = Vec::new();
    let mut k = eq + 1;
    while k < end {
        if is_punct(toks, k, '$') {
            if ident_at(toks, k + 1) == Some(("crate", false)) {
                // `$crate::x::T` is the defining crate: read it as `crate::x::T`.
                k += 1;
            } else {
                leaves.push(UseLeaf {
                    path: vec![UNKNOWN_TYPE_SEGMENT.to_string()],
                    alias: Some(name.clone()),
                    glob: false,
                    line: toks[k].line,
                    col: toks[k].col,
                });
                k += 2;
                continue;
            }
        }
        let starts_path = match ident_at(toks, k) {
            Some((n, raw)) => raw || !(is_type_keyword(n) || is_non_path_keyword(n)),
            None => is_pathsep(toks, k) && ident_at(toks, k + 1).is_some(),
        } && !(k > 0 && is_pathsep(toks, k - 1));
        if !starts_path {
            k += 1;
            continue;
        }
        let (line, col) = (toks[k].line, toks[k].col);
        if is_pathsep(toks, k) {
            k += 1;
        }
        let mut path: Vec<String> = Vec::new();
        while let Some((seg, _)) = ident_at(toks, k) {
            path.push(seg.to_string());
            if path.len() > MAX_ALIAS_PATH {
                return Err(TYPE_ALIAS_UNPARSEABLE);
            }
            k += 1;
            // `Foo::<T>::Bar`: skip the turbofish, keep reading the path.
            if is_pathsep(toks, k) && is_punct(toks, k + 1, '<') {
                k = skip_angle(toks, k + 1).ok_or(TYPE_ALIAS_UNPARSEABLE)?;
            }
            if is_pathsep(toks, k) && ident_at(toks, k + 1).is_some() {
                k += 1;
            } else {
                break;
            }
        }
        // `type F = ty!();`: the alias names whatever the macro expands to,
        // not a type called `ty` (R5-2, case `type-macro-alias`).  Bind it to
        // the unknown type, as for a metavariable right-hand side.
        if !path.is_empty() && is_punct(toks, k, '!') {
            leaves.push(UseLeaf {
                path: vec![UNKNOWN_TYPE_SEGMENT.to_string()],
                alias: Some(name.clone()),
                glob: false,
                line,
                col,
            });
            continue;
        }
        if path.len() >= 2 && params.contains(path[0].as_str()) {
            projection = true;
        }
        let trivial = path.len() == 1 && (path[0] == name || params.contains(path[0].as_str()));
        if !path.is_empty() && !trivial {
            leaves.push(UseLeaf { path, alias: Some(name.clone()), glob: false, line, col });
        }
    }
    if projection {
        leaves.push(UseLeaf {
            path: vec![UNKNOWN_TYPE_SEGMENT.to_string()],
            alias: Some(name),
            glob: false,
            line: toks[eq].line,
            col: toks[eq].col,
        });
    }
    Ok(leaves)
}

/// Token ranges of `macro_rules!` matchers: patterns, not code, so a `type $n`
/// or `use $p` there is not an item (H3, H4).
fn matcher_mask(toks: &[Token], br: &Brackets) -> Vec<bool> {
    let mut mask = vec![false; toks.len()];
    for i in 0..toks.len() {
        if ident_at(toks, i) == Some(("macro_rules", false))
            && is_punct(toks, i + 1, '!')
            && matches!(toks.get(i + 3).map(|t| &t.tok), Some(Tok::Punct('(' | '[' | '{')))
        {
            let open = i + 3;
            if let Some(arms) = macro_arms(toks, br, open, br.close(open)) {
                for a in arms {
                    for m in mask.iter_mut().take(a.m_close + 1).skip(a.m_open) {
                        *m = true;
                    }
                }
            }
        }
    }
    mask
}

/// Everything stage 1 learns about one file.
#[derive(Default)]
struct FileFacts {
    /// Token ranges `[start, end)` consumed by `use` / `extern crate` items.
    skip: Vec<(usize, usize)>,
    leaves: Vec<UseLeaf>,
    /// `(token index, message)` for `use` / `extern crate` items that could not be parsed.
    use_errors: Vec<(usize, &'static str)>,
}

fn stage1_file(toks: &[Token], br: &Brackets) -> FileFacts {
    let mut f = FileFacts::default();
    let in_matcher = matcher_mask(toks, br);
    let mut i = 0;
    while i < toks.len() {
        if in_matcher[i] {
            i += 1;
            continue;
        }
        let prev_is_dot_or_dollar = i > 0 && (is_punct(toks, i - 1, '.') || is_punct(toks, i - 1, '$'));
        if let (Some(("use", false)), false) = (ident_at(toks, i), prev_is_dot_or_dollar) {
            // `use $crate::..` names the defining crate: read it as `crate::..`.
            // Any other metavariable after `use` chooses the imported path
            // (`($i:ident) => { use $i as out; }` with `u!(std)`), so the
            // binding is unknown: CAP003 (H4).
            let dollar_crate = is_punct(toks, i + 1, '$') && ident_at(toks, i + 2) == Some(("crate", false));
            if is_punct(toks, i + 1, '$') && !dollar_crate {
                f.use_errors.push((i, "use declaration whose path is a macro_rules! metavariable; the names it binds are not known"));
                i += 1;
                continue;
            }
            let starts_tree = ident_at(toks, i + 1).is_some()
                || is_pathsep(toks, i + 1)
                || is_punct(toks, i + 1, '{')
                || dollar_crate;
            if starts_tree {
                let start = i;
                let mut j = if dollar_crate { i + 2 } else { i + 1 };
                let ok = parse_use_tree(toks, &mut j, &[], &mut f.leaves, 0) && is_punct(toks, j, ';');
                if !ok {
                    f.use_errors.push((start, "unparseable use declaration"));
                    while j < toks.len() && !is_punct(toks, j, ';') {
                        j += 1;
                    }
                }
                let end = (j + 1).min(toks.len());
                f.skip.push((start, end));
                i = end;
                continue;
            }
        }
        if let (Some(("type", false)), false) = (ident_at(toks, i), prev_is_dot_or_dollar) {
            // `type F = File;` binds `F` exactly like `use ... as F` (VER08a L1).
            match type_alias_leaves(toks, br, i) {
                Ok(leaves) => f.leaves.extend(leaves),
                Err(reason) => f.use_errors.push((i, reason)),
            }
        }
        if let (Some(("extern", false)), Some(("crate", false))) =
            (ident_at(toks, i), ident_at(toks, i + 1))
        {
            // `extern crate NAME [as ALIAS];` and nothing else.  The old reader
            // took the first identifier after `as` and skipped to the next `;`,
            // so whatever rustc saw as the rest of the alias (`s·x`, PoC t4)
            // was silently dropped.  Any other shape - including a name spliced
            // in by a `macro_rules!` metavariable - binds a name the analyzer
            // cannot see, so it is CAP003.
            let mut j = i + 2;
            let mut leaf = None;
            if let Some((name, _)) = ident_at(toks, j) {
                let t = &toks[j];
                j += 1;
                let mut alias = None;
                let mut well_formed = true;
                if let Some(("as", false)) = ident_at(toks, j) {
                    match ident_at(toks, j + 1) {
                        Some((a, _)) => {
                            alias = Some(a.to_string());
                            j += 2;
                        }
                        None => well_formed = false,
                    }
                }
                if well_formed && is_punct(toks, j, ';') {
                    leaf = Some(UseLeaf {
                        path: vec![name.to_string()],
                        alias,
                        glob: false,
                        line: t.line,
                        col: t.col,
                    });
                }
            }
            match leaf {
                Some(l) => f.leaves.push(l),
                None => f.use_errors.push((i, "unparseable extern crate declaration")),
            }
            while j < toks.len() && !is_punct(toks, j, ';') {
                j += 1;
            }
            let end = (j + 1).min(toks.len());
            f.skip.push((i, end));
            i = end;
            continue;
        }
        i += 1;
    }
    f
}

const MAX_ALIAS_ROUNDS: usize = 16;
const MAX_ALIAS_PATH: usize = 64;
const MAX_ALIAS_ENTRIES: usize = 20_000;
/// Distinct glob imports kept (after normalisation); more is treated as an attack.
const MAX_GLOBS: usize = 256;
/// Spellings produced while normalising glob imports, *pure ones included*;
/// more is CAP003.  The old cap counted only the globs that were kept, after
/// materialising (globs x binding targets), so pure globs through a heavily
/// rebound name grew without bound (security review round 3, LOW).
const MAX_GLOB_SPELLINGS: usize = 4096;
/// Longest path (in segments) the resolver will expand; longer is CAP003.
const MAX_PATH_SEGS: usize = 64;
/// Spellings tried for one written path; more is CAP003 (ambiguity explosion).
const MAX_CANDIDATES: usize = 64;
/// Extra spellings (beyond the written one) classified per file before
/// resolution is abandoned (CAP003).
const CANDIDATE_BUDGET: usize = 200_000;

/// Package-wide binding table.
#[derive(Default)]
struct Resolver {
    bindings: HashMap<String, Vec<Vec<String>>>,
    globs: Vec<Vec<String>>,
    /// Glob paths, as written, that resolve to a bare `std`/`core`/`alloc`
    /// root (`use std::*;`, `use s::*;` after `use std as s;`).  See
    /// [`Resolver::glob_is_bare_root`].
    root_globs: HashSet<Vec<String>>,
    /// Some glob import is headed by something other than `crate`/`self`/
    /// `super`/`Self` after normalisation (`use std::path::*;`, `use dep::*;`):
    /// a bare type name may then be an item from outside this crate.
    foreign_glob: bool,
}

/// Every spelling of the glob path `g` after alias substitution: `g` itself
/// and, for each segment that is a binding name, `target + rest`.  This is the
/// same per-segment rule [`Resolver::candidates`] applies to ordinary paths, so
/// `use crate::s::*;` with `pub use std as s;` elsewhere is seen as `std::*`.
/// (The old code substituted only the *first* segment, so a local head such
/// as `crate::` hid the alias.)  Every spelling counts against `budget`.
fn glob_spellings(
    bindings: &HashMap<String, Vec<Vec<String>>>,
    g: &[String],
    budget: &mut usize,
) -> Result<Vec<Vec<String>>, String> {
    let mut out = vec![g.to_vec()];
    for (i, seg) in g.iter().enumerate() {
        for t in bindings.get(seg).map(Vec::as_slice).unwrap_or(&[]) {
            if out.len() >= *budget {
                return Err("too many glob import spellings".into());
            }
            let mut np = t.clone();
            np.extend_from_slice(&g[i + 1..]);
            out.push(np);
        }
    }
    if out.len() > *budget {
        return Err("too many glob import spellings".into());
    }
    *budget -= out.len();
    Ok(out)
}

/// Upper bound on substitution attempts while closing the binding table;
/// more is CAP003 (an attacker-sized table must not take minutes).
const MAX_ALIAS_WORK: usize = 4_000_000;

/// `a` and `b` (sorted, de-duplicated) share an element.
fn sorted_intersects(a: &[u32], b: &[u32]) -> bool {
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].cmp(&b[j]) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => return true,
        }
    }
    false
}

/// Union of two sorted, de-duplicated sets.
fn sorted_union(a: &[u32], b: &[u32]) -> Vec<u32> {
    let mut out: Vec<u32> = a.iter().chain(b.iter()).copied().collect();
    out.sort_unstable();
    out.dedup();
    out
}

/// Intersection of two sorted, de-duplicated sets.
fn sorted_intersection(a: &[u32], b: &[u32]) -> Vec<u32> {
    a.iter().copied().filter(|x| b.binary_search(x).is_ok()).collect()
}

/// Close the binding table under substitution at every segment.
///
/// ## Why every segment
///
/// rustc resolves the *first* segment of a path in the current scope and every
/// later segment inside the module the prefix names.  This table is not
/// scoped, so it cannot tell which module a later segment lives in; what it can
/// do is try every binding of that name.  The old code expanded a binding only
/// through its *first* segment, and expanded a written path only once.  So
///
/// ```text
///   mod a { pub use std as s; }      s -> std
///   use a::s as t;                   t -> a::s      (head `a` is no binding)
///   t::env::var("HOME")              t::env::var -> a::s::env::var   (pure!)
/// ```
///
/// never reached `std::env::var` (review cases `alias2-*`,
/// `extern-crate-nested-2level`).  Now a binding `n -> p` gains `n -> t + rest`
/// for every segment `p[i]` that is itself a binding with spelling `t`
/// (`rest = p[i+1..]`), repeated until nothing changes: `t -> a::s` gains
/// `t -> std`.  This is the rule [`glob_spellings`] and
/// [`Resolver::candidates`] apply to globs and written paths; with a closed
/// table, one substitution at any segment of a written path reaches every
/// spelling (a substitution discards the prefix before it, so a second one
/// further right is the same as doing only that one).
///
/// ## Why provenance
///
/// Names repeat, so a binding can be substituted into itself.  With
/// `mod sha256 { pub fn sha256() }`, `pub use self::sha256::sha256;` and
/// `use crate::sha256;` in another module:
///
/// ```text
///   self::sha256::sha256 -> crate::sha256::sha256 -> crate::sha256::sha256::sha256 -> ...
/// ```
///
/// without end.  Each spelling therefore records the set of original
/// bindings it was derived from, and a substitution that would use one of them
/// twice is skipped.  rustc never resolves a path through the same import
/// twice (that is an import cycle), so nothing it can resolve is lost.  When
/// two derivations reach the same spelling, the *intersection* of their sets is
/// kept: a smaller set blocks fewer later substitutions, so the result only
/// grows (sound), and the process is monotone, so it terminates.
///
/// ## Caps
///
/// Every cap is checked inside the loops, before anything is materialised:
/// [`MAX_ALIAS_PATH`] segments per spelling, [`MAX_ALIAS_ENTRIES`] spellings,
/// [`MAX_ALIAS_WORK`] substitution attempts and [`MAX_ALIAS_ROUNDS`] rounds.
/// Exceeding any of them is an `Err`, which the caller reports as CAP003.
fn close_bindings(entries: Vec<(String, Vec<String>)>) -> Result<Vec<(String, Vec<String>)>, String> {
    struct Spelling {
        name: String,
        path: Vec<String>,
        prov: Vec<u32>,
    }
    let grew = || "alias expansion grew without bound".to_string();
    let mut table: Vec<Spelling> = entries
        .into_iter()
        .enumerate()
        .map(|(k, (name, path))| Spelling { name, path, prov: vec![k as u32] })
        .collect();
    let mut index: HashMap<(String, Vec<String>), usize> =
        table.iter().enumerate().map(|(k, e)| ((e.name.clone(), e.path.clone()), k)).collect();
    let mut work = 0usize;
    for _ in 0..MAX_ALIAS_ROUNDS {
        // New spellings (and shrunk provenance) found this round, applied after it.
        let mut staged: HashMap<(String, Vec<String>), Vec<u32>> = HashMap::new();
        // name -> spellings of that name, as of the start of this round.
        let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
        for (k, e) in table.iter().enumerate() {
            by_name.entry(e.name.as_str()).or_default().push(k);
        }
        for e in &table {
            for (i, seg) in e.path.iter().enumerate() {
                let Some(targets) = by_name.get(seg.as_str()) else { continue };
                for &f in targets {
                    let t = &table[f];
                    work += 1;
                    if work > MAX_ALIAS_WORK {
                        return Err(grew());
                    }
                    if sorted_intersects(&e.prov, &t.prov) {
                        continue;
                    }
                    let len = t.path.len() + e.path.len() - i - 1;
                    if len > MAX_ALIAS_PATH {
                        return Err(grew());
                    }
                    let mut np = Vec::with_capacity(len);
                    np.extend_from_slice(&t.path);
                    np.extend_from_slice(&e.path[i + 1..]);
                    let prov = sorted_union(&e.prov, &t.prov);
                    let key = (e.name.clone(), np);
                    match staged.get_mut(&key) {
                        Some(p) => *p = sorted_intersection(p, &prov),
                        None => {
                            if !index.contains_key(&key) && table.len() + staged.len() >= MAX_ALIAS_ENTRIES {
                                return Err(grew());
                            }
                            staged.insert(key, prov);
                        }
                    }
                }
            }
        }
        drop(by_name);
        let mut changed = false;
        for (key, prov) in staged {
            match index.get(&key) {
                Some(&k) => {
                    let narrowed = sorted_intersection(&table[k].prov, &prov);
                    if narrowed != table[k].prov {
                        table[k].prov = narrowed;
                        changed = true;
                    }
                }
                None => {
                    index.insert(key.clone(), table.len());
                    table.push(Spelling { name: key.0, path: key.1, prov });
                    changed = true;
                }
            }
        }
        if !changed {
            let mut out: Vec<(String, Vec<String>)> = table.into_iter().map(|e| (e.name, e.path)).collect();
            out.sort();
            return Ok(out);
        }
    }
    Err("alias cycle: use-bindings did not converge".into())
}

impl Resolver {
    fn build(leaves: &[UseLeaf]) -> Result<Self, String> {
        let mut entries: Vec<(String, Vec<String>)> = Vec::new();
        let mut globs: Vec<Vec<String>> = Vec::new();
        for l in leaves {
            if l.glob {
                globs.push(l.path.clone());
                continue;
            }
            let name = match (&l.alias, l.path.last()) {
                (Some(a), _) => a.clone(),
                (None, Some(last)) => last.clone(),
                _ => continue,
            };
            if name == "_" || l.path.is_empty() {
                continue;
            }
            // `use std;` binds `std -> std`: trivial, skip.
            if l.path.len() == 1 && l.path[0] == name {
                continue;
            }
            entries.push((name, l.path.clone()));
        }

        // Dedup *before* expanding: expansion is (entries x targets), so a
        // package with thousands of copies of one binding must not multiply.
        entries.sort();
        entries.dedup();
        globs.sort();
        globs.dedup();
        if entries.len() > MAX_ALIAS_ENTRIES || globs.len() > MAX_ALIAS_ENTRIES {
            return Err("too many use bindings".into());
        }

        // Expand every binding through every other binding, at *every*
        // segment, to a fixed point (security review round 4, H1).
        let entries = close_bindings(entries)?;

        let mut bindings: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for (n, p) in entries {
            bindings.entry(n).or_default().push(p);
        }

        // Normalise globs through the bindings; keep only globs that can name
        // std surfaces.  Every spelling, kept or not, is counted (LOW).
        let mut norm_globs: Vec<Vec<String>> = Vec::new();
        let mut root_globs: HashSet<Vec<String>> = HashSet::new();
        let mut foreign_glob = false;
        let mut budget = MAX_GLOB_SPELLINGS;
        for g in globs {
            let expanded = glob_spellings(&bindings, &g, &mut budget)?;
            if expanded.iter().any(|e| e.len() == 1 && is_std_root(&e[0])) {
                root_globs.insert(g.clone());
            }
            for e in expanded {
                if e.first().is_some_and(|h| !is_local_head(h)) {
                    foreign_glob = true;
                }
                if let Some(h) = e.first() {
                    let relevant = is_std_root(h)
                        || SHORTHAND_HEADS.contains(&h.as_str())
                        || !matches!(
                            classify_resolved(h, false),
                            Classification::AllowedPure
                        );
                    if relevant && !is_local_head(h) {
                        if norm_globs.len() >= MAX_GLOBS {
                            return Err("too many glob imports".into());
                        }
                        norm_globs.push(e);
                    }
                }
            }
        }
        norm_globs.sort();
        norm_globs.dedup();
        Ok(Self {
            bindings,
            globs: norm_globs,
            root_globs,
            foreign_glob,
        })
    }

    fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// Is the glob import of `path` (as written) a glob over a whole standard
    /// crate (`use std::*;`)?  Such a glob makes every name of `std`'s root -
    /// `fs`, `process`, `net`, ... - nameable through any module that holds it,
    /// so a path can reach authority without one `std` segment in sight
    /// (security review round 3, HIGH-1, PoC t12).  It is CAP003.
    fn glob_is_bare_root(&self, path: &[String]) -> bool {
        self.root_globs.contains(path)
    }

    /// All alias spellings of `segs` worth classifying (glob spellings are
    /// generated by [`Sink::path`], see there).  The second result is true if
    /// the list was truncated at [`MAX_CANDIDATES`] (the caller reports
    /// CAP003).  `segs` must be at most [`MAX_PATH_SEGS`] long, so the work
    /// here is bounded by `MAX_CANDIDATES x (MAX_ALIAS_PATH + MAX_PATH_SEGS)`.
    fn candidates(&self, segs: &[String]) -> (Vec<Vec<String>>, bool) {
        let mut out: Vec<Vec<String>> = vec![segs.to_vec()];
        let mut truncated = false;
        'outer: for (i, s) in segs.iter().enumerate() {
            if let Some(targets) = self.bindings.get(s) {
                for t in targets {
                    if out.len() >= MAX_CANDIDATES {
                        truncated = true;
                        break 'outer;
                    }
                    let mut c = t.clone();
                    c.extend_from_slice(&segs[i + 1..]);
                    out.push(c);
                }
            }
        }
        out.sort();
        out.dedup();
        (out, truncated)
    }
}

// ---------------------------------------------------------------------------
// Stage 2: classification walk
// ---------------------------------------------------------------------------

struct Sink<'a> {
    file: &'a str,
    result: &'a mut ScanResult,
    /// De-duplication of identical findings at one position.
    seen: HashSet<(usize, usize, String)>,
    /// Spellings still allowed to be classified in this file ([`CANDIDATE_BUDGET`]).
    cand_budget: usize,
    /// Tokens of `macro_rules!` body scanning still allowed in this file.
    macro_budget: usize,
}

impl Sink<'_> {
    fn detection(&mut self, line: usize, col: usize, category: &'static str, action: &'static str, evidence: String) {
        if self.seen.insert((line, col, format!("c:{category}:{action}"))) {
            self.result.detections.push(SourceDetection {
                file: self.file.to_string(),
                line,
                col,
                category,
                action,
                evidence,
            });
        }
    }

    fn banned(&mut self, line: usize, col: usize, construct: &'static str, evidence: String) {
        if self.seen.insert((line, col, format!("b:{construct}:{evidence}"))) {
            self.result.banned.push(SourceBannedConstruct {
                file: self.file.to_string(),
                line,
                col,
                construct,
                evidence,
            });
        }
    }

    fn unanalyzable(&mut self, line: usize, col: usize, reason: &str, evidence: String) {
        if self.seen.insert((line, col, format!("u:{reason}"))) {
            self.result.unanalyzable.push(SourceUnanalyzable {
                file: self.file.to_string(),
                line,
                col,
                reason: reason.to_string(),
                evidence,
            });
        }
    }

    /// Resolve and classify one path occurrence.
    fn path(&mut self, res: &Resolver, segs: &[String], is_macro: bool, line: usize, col: usize) {
        let written = {
            let mut w = segs.join("::");
            if is_macro {
                w.push('!');
            }
            w
        };
        // Bounded work per path (VER08a §3.2): a pathological path is refused,
        // not resolved quadratically.
        if segs.len() > MAX_PATH_SEGS {
            self.unanalyzable(
                line,
                col,
                "path has too many segments to resolve",
                format!("{}...", segs[..4].join("::")),
            );
            return;
        }
        // Once the per-file budget is spent, paths are classified as written only
        // (never silently skipped) and the file carries one CAP003.
        let mut cands = vec![segs.to_vec()];
        if self.cand_budget > 0 {
            let (all, truncated) = res.candidates(segs);
            if truncated {
                self.unanalyzable(
                    line,
                    col,
                    "path has too many alias spellings to resolve",
                    written.clone(),
                );
            }
            // Only the spellings beyond the written one cost budget, so ordinary
            // files (one or two spellings per path) never come near the limit.
            let extra = all.len().saturating_sub(1);
            if extra > self.cand_budget {
                self.spend_all_budget(line, col, &written);
            } else {
                self.cand_budget -= extra;
                cands = all;
            }
        }
        for cand in &cands {
            self.classify(cand, segs, &written, is_macro, line, col, true);
        }

        // Glob spellings.  A glob `use G::*;` in module `m` makes `m::x` (and,
        // inside `m`, plain `x`) name `G::x`.  Which module holds the glob is
        // not tracked, so *every suffix* of every spelling is tried after every
        // glob prefix: `m::process::Command` with `use std::*;` in `m` is
        // `std::process::Command`.  The old code tried only `G + <whole path>`
        // (`std::m::process::Command`, an unknown std module), so a glob
        // re-export through a local module hid everything behind it (HIGH-1,
        // PoCs t12/t13).  A suffix starting at `crate`/`self`/`super`/`Self` is
        // skipped (it cannot follow a glob prefix), and spellings headed by a
        // standard crate are absolute and never go through a local glob.
        //
        // CAP003 from a glob spelling is reported only when the glob prefix is
        // a bare standard crate (`std::*`, itself CAP003): under `use std::fs::*`
        // most suffixes are local names that only *look* like `std::fs::x`,
        // and they are classified as `fs` access, not refused.
        if res.globs.is_empty() || self.cand_budget == 0 {
            return;
        }
        for cand in &cands {
            if is_std_root(&cand[0]) {
                continue;
            }
            for k in 0..cand.len() {
                if is_local_head(&cand[k]) {
                    continue;
                }
                for g in &res.globs {
                    if self.cand_budget == 0 {
                        self.spend_all_budget(line, col, &written);
                        return;
                    }
                    self.cand_budget -= 1;
                    let mut gc = g.clone();
                    gc.extend_from_slice(&cand[k..]);
                    self.classify(&gc, segs, &written, is_macro, line, col, g.len() == 1);
                }
            }
        }
    }

    /// The per-file spelling budget is exhausted: say so once (CAP003).
    fn spend_all_budget(&mut self, line: usize, col: usize, written: &str) {
        self.cand_budget = 0;
        self.unanalyzable(
            line,
            col,
            "alias resolution exceeded the per-file work budget",
            written.to_string(),
        );
    }

    /// Classify one spelling `cand` of the written path `segs`.
    /// `report_cap003` is false for glob spellings under a module glob (above).
    #[allow(clippy::too_many_arguments)]
    fn classify(
        &mut self,
        cand: &[String],
        segs: &[String],
        written: &str,
        is_macro: bool,
        line: usize,
        col: usize,
        report_cap003: bool,
    ) {
        let refs: Vec<&str> = cand.iter().map(String::as_str).collect();
        let cls = crate::rules::classify_segments(&refs, is_macro);
        let evidence = || {
            if cand == segs {
                written.to_string()
            } else {
                format!("{written} => {}", cand.join("::"))
            }
        };
        match cls {
            Classification::Capability { category, action } => {
                self.detection(line, col, category, action, evidence())
            }
            Classification::BannedConstruct { construct } => self.banned(line, col, construct, evidence()),
            Classification::Unanalyzable { reason } if report_cap003 => {
                self.unanalyzable(line, col, reason, evidence())
            }
            _ => {}
        }
    }
}

/// Bracket pairing, computed once per file in a single pass.
///
/// The old `matching_close` rescanned from every `#[` and every `macro_rules!`,
/// which is O(n^2) on `#[#[#[...` (VER08a §3.2, DoS).  Any closer closes the
/// innermost opener, exactly as the old depth counter did.
struct Brackets {
    /// `close_of[open]` = index of the matching closer, or `len` if unmatched.
    close_of: Vec<usize>,
    /// `open_of[close]` = index of the matching opener, or `usize::MAX`.
    open_of: Vec<usize>,
    len: usize,
}

impl Brackets {
    fn build(toks: &[Token]) -> Self {
        let mut close_of = vec![toks.len(); toks.len()];
        let mut open_of = vec![usize::MAX; toks.len()];
        let mut stack: Vec<usize> = Vec::new();
        for (k, t) in toks.iter().enumerate() {
            match t.tok {
                Tok::Punct('(' | '[' | '{') => stack.push(k),
                Tok::Punct(')' | ']' | '}') => {
                    if let Some(o) = stack.pop() {
                        close_of[o] = k;
                        open_of[k] = o;
                    }
                }
                _ => {}
            }
        }
        Self {
            close_of,
            open_of,
            len: toks.len(),
        }
    }

    /// Index of the token matching the opener at `open`, or `toks.len()`.
    fn close(&self, open: usize) -> usize {
        self.close_of.get(open).copied().unwrap_or(self.len)
    }
}

/// `(has cfg_attr, has $)` over the tokens of an attribute, not descending into
/// nested attributes (each of those is inspected on its own, so the total work
/// stays linear even for `#[#[#[...`).
fn scan_attr_span(toks: &[Token], br: &Brackets, from: usize, to: usize) -> (bool, bool) {
    let (mut cfg_attr, mut dollar) = (false, false);
    let mut k = from;
    while k < to {
        match &toks[k].tok {
            Tok::Punct('#') => {
                let mut o = k + 1;
                if is_punct(toks, o, '!') {
                    o += 1;
                }
                if is_punct(toks, o, '[') {
                    k = br.close(o).saturating_add(1).max(k + 1);
                    continue;
                }
            }
            Tok::Punct('$') => dollar = true,
            Tok::Ident { name, .. } if name == "cfg_attr" => cfg_attr = true,
            _ => {}
        }
        k += 1;
    }
    (cfg_attr, dollar)
}

/// True for the include-family macro names (code or data pulled from a file).
fn is_include_name(name: &str) -> bool {
    matches!(name, "include" | "include_str" | "include_bytes")
}

/// Longest `<..>` group (in tokens) searched backwards by `angle_group_has_meta`.
const ANGLE_SCAN_LIMIT: usize = 512;

/// Is the `>` at `gt` the end of a `<..>` group that contains a `$` metavariable
/// (or is too long / unbalanced to tell)?  Fails closed.
fn angle_group_has_meta(toks: &[Token], gt: usize) -> bool {
    let mut depth = 0usize;
    let mut saw_dollar = false;
    let lo = gt.saturating_sub(ANGLE_SCAN_LIMIT);
    let mut i = gt + 1;
    while i > lo {
        i -= 1;
        match &toks[i].tok {
            Tok::Punct('>') => depth += 1,
            Tok::Punct('<') => {
                depth -= 1;
                if depth == 0 {
                    return saw_dollar;
                }
            }
            Tok::Punct('$') => saw_dollar = true,
            _ => {}
        }
    }
    // No matching `<` within the limit: could be `->`/`>>` noise or a huge group.
    saw_dollar
}

/// One `(matcher) => {transcriber}` arm of a `macro_rules!` body: the token
/// indices of the matcher's and the transcriber's opening and closing delimiters.
struct MacroArm {
    m_open: usize,
    m_close: usize,
    t_open: usize,
    t_close: usize,
}

/// Split the body `toks[open..=close]` into arms.  `None` if it is not a
/// sequence of `group => group` separated by `;` (rustc rejects such a body).
fn macro_arms(toks: &[Token], br: &Brackets, open: usize, close: usize) -> Option<Vec<MacroArm>> {
    let is_open = |i: usize| matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Punct('(' | '[' | '{')));
    if close >= toks.len() {
        return None;
    }
    let mut arms = Vec::new();
    let mut k = open + 1;
    while k < close {
        if !is_open(k) {
            return None;
        }
        let m_close = br.close(k);
        if !(m_close < close && is_punct(toks, m_close + 1, '=') && is_punct(toks, m_close + 2, '>') && is_open(m_close + 3)) {
            return None;
        }
        let t_close = br.close(m_close + 3);
        if t_close >= close {
            return None;
        }
        arms.push(MacroArm { m_open: k, m_close, t_open: m_close + 3, t_close });
        k = t_close + 1;
        if is_punct(toks, k, ';') {
            k += 1;
        }
    }
    Some(arms)
}

/// `$name:frag` declarations of one matcher.  A name declared twice with
/// different fragments (only possible across nested repetitions of a
/// malformed macro) is recorded as `tt`, the most permissive.
fn matcher_fragments(toks: &[Token], from: usize, to: usize) -> HashMap<String, String> {
    let mut frags: HashMap<String, String> = HashMap::new();
    for k in from..to {
        if !is_punct(toks, k, '$') || !is_punct(toks, k + 2, ':') {
            continue;
        }
        if let (Some((name, _)), Some((frag, _))) = (ident_at(toks, k + 1), ident_at(toks, k + 3)) {
            let e = frags.entry(name.to_string()).or_insert_with(|| frag.to_string());
            if e != frag {
                *e = "tt".to_string();
            }
        }
    }
    frags
}

/// Strict keywords that can never be a path segment.  `self`, `super`,
/// `crate` and `Self` *are* path segments, and edition-dependent or weak
/// keywords (`dyn`, `try`, `gen`, `union`, `macro_rules`, ...) can be plain
/// identifiers, so none of those is listed: they count as identifiers.
fn is_non_path_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "static"
            | "struct"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
    )
}

/// What one transcriber unit can turn into when the macro is expanded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum UnitKind {
    /// An identifier that may be a path segment (`std`, `fs`, `self`, `$crate`).
    PathIdent,
    /// A keyword that is never a path segment (`let`, `fn`, `struct`, ...).
    Keyword,
    /// A `$x` whose fragment is `tt` or unknown: the caller may supply *any*
    /// single token, including `::`, `#` and `!`.
    MetaTt,
    /// A `$x` whose fragment is `ident`: one identifier - **including any
    /// keyword** (`use`, `extern`, `type`, `impl`, ...).  See [`check_keyword_splices`].
    MetaIdent,
    /// A `$x` with any other fragment (`expr`, `ty`, `path`, ...): an opaque,
    /// already-parsed node.
    MetaOther,
    /// A `$( .. ) sep? op` repetition; `tt` is true if it contains a `MetaTt`.
    Repeat { tt: bool },
    /// `[`
    OpenBracket,
    /// `!` not followed by `=`
    Bang,
    Other,
}

struct Unit {
    kind: UnitKind,
    /// First token of the unit (for the diagnostic position).
    at: usize,
    /// Repetition contents, separator token and whether the repetition is a
    /// verbatim `tt` pass-through of a matcher repetition (see below).
    inner: Vec<Unit>,
    sep: Option<usize>,
    passthrough: bool,
}

impl Unit {
    fn is_meta(&self) -> bool {
        matches!(self.kind, UnitKind::MetaTt | UnitKind::MetaIdent | UnitKind::MetaOther | UnitKind::Repeat { .. })
    }

    /// Can this unit expand to a keyword?  An `ident` or `tt` metavariable can
    /// (an `ident` fragment matches keywords), and so can a repetition whose
    /// last unit can.
    fn may_be_keyword(&self) -> bool {
        match self.kind {
            UnitKind::MetaTt | UnitKind::MetaIdent => true,
            UnitKind::Repeat { .. } => self.inner.last().is_some_and(Unit::may_be_keyword),
            _ => false,
        }
    }

    fn is_tt(&self) -> bool {
        matches!(self.kind, UnitKind::MetaTt | UnitKind::Repeat { tt: true })
    }
}

/// Repetitions nested deeper than this are refused (CAP003): the unit tree
/// is built recursively.
const MAX_REPEAT_DEPTH: usize = 64;

/// Classify a single (non-`$`) token as a unit kind.
fn token_unit_kind(toks: &[Token], k: usize) -> UnitKind {
    match &toks[k].tok {
        Tok::Ident { name, raw } => {
            if !*raw && is_non_path_keyword(name) {
                UnitKind::Keyword
            } else {
                UnitKind::PathIdent
            }
        }
        Tok::Punct('[') => UnitKind::OpenBracket,
        Tok::Punct('!') if !(is_punct(toks, k + 1, '=') && toks[k].end == toks[k + 1].start) => UnitKind::Bang,
        _ => UnitKind::Other,
    }
}

/// The `$( $name:tt ) sep? op` repetitions of one matcher, keyed by `name`,
/// as `(separator, operator)` pairs.
///
/// A transcriber `$( $name ) sep? op` with the same separator and operator
/// emits the caller's tokens in the caller's own order, contiguous exactly as
/// the walker already saw them inside the invocation's arguments.  The old
/// `matcher_has_tt_repeat` rescanned the whole matcher for every transcriber
/// repetition, so a long matcher with many repetitions was quadratic (40,000
/// of each took about 30 s; security review round 4, LOW, cases
/// `perf-matcher-rescan-*`, `perf-valid-40000`).  The set is built once per arm
/// in one pass, and each repetition is a hash lookup.
type TtRepeats = HashMap<String, Vec<(Option<Tok>, Tok)>>;

fn matcher_tt_repeats(toks: &[Token], arm: &MacroArm) -> TtRepeats {
    let mut out: TtRepeats = HashMap::new();
    for k in arm.m_open + 1..arm.m_close {
        if !(is_punct(toks, k, '$')
            && is_punct(toks, k + 1, '(')
            && is_punct(toks, k + 2, '$')
            && is_punct(toks, k + 4, ':')
            && ident_at(toks, k + 5) == Some(("tt", false))
            && is_punct(toks, k + 6, ')'))
        {
            continue;
        }
        let Some((name, false)) = ident_at(toks, k + 3) else { continue };
        let (Some(a), b) = (toks.get(k + 7).map(|t| &t.tok), toks.get(k + 8).map(|t| &t.tok)) else { continue };
        let entry = out.entry(name.to_string()).or_default();
        // `$($t:tt)*`: no separator, operator `a`.
        entry.push((None, a.clone()));
        // `$($t:tt),*`: separator `a`, operator `b`.
        if let Some(b) = b {
            entry.push((Some(a.clone()), b.clone()));
        }
    }
    out
}

/// Build the units of `toks[from..to]` (a transcriber or a repetition body).
/// `Err(at)` if repetitions nest deeper than [`MAX_REPEAT_DEPTH`].
fn transcriber_units(
    toks: &[Token],
    br: &Brackets,
    tt_repeats: &TtRepeats,
    frags: &HashMap<String, String>,
    from: usize,
    to: usize,
    depth: usize,
) -> Result<Vec<Unit>, usize> {
    if depth > MAX_REPEAT_DEPTH {
        return Err(from);
    }
    let is_kleene = |i: usize| is_punct(toks, i, '*') || is_punct(toks, i, '+') || is_punct(toks, i, '?');
    let mut units = Vec::new();
    let mut k = from;
    while k < to {
        let unit = if is_punct(toks, k, '$') && is_punct(toks, k + 1, '(') {
            let c = br.close(k + 1).min(to);
            let inner = transcriber_units(toks, br, tt_repeats, frags, k + 2, c, depth + 1)?;
            // `$( .. ) op` or `$( .. ) sep op`; anything else is malformed and
            // treated as a separator-less repetition (more adjacency, not less).
            let (sep, end) = if is_kleene(c + 1) {
                (None, c + 1)
            } else if c + 2 < to && is_kleene(c + 2) {
                (Some(c + 1), c + 2)
            } else {
                (None, c)
            };
            let tt = inner.iter().any(Unit::is_tt);
            let passthrough = match (inner.as_slice(), toks.get(end).map(|t| &t.tok)) {
                ([only], Some(op)) if only.kind == UnitKind::MetaTt && end > c => ident_at(toks, only.at + 1)
                    .is_some_and(|(n, _)| {
                        let sep = sep.map(|s| &toks[s].tok);
                        tt_repeats.get(n).is_some_and(|v| v.iter().any(|(s, o)| s.as_ref() == sep && o == op))
                    }),
                _ => false,
            };
            let u = Unit { kind: UnitKind::Repeat { tt }, at: k, inner, sep, passthrough };
            k = end + 1;
            u
        } else if is_punct(toks, k, '$') && ident_at(toks, k + 1).is_some() {
            let (name, raw) = ident_at(toks, k + 1).unwrap_or(("", false));
            let kind = if name == "crate" && !raw {
                UnitKind::PathIdent
            } else {
                match frags.get(name).map(String::as_str) {
                    Some("tt") | None => UnitKind::MetaTt,
                    Some("ident") => UnitKind::MetaIdent,
                    Some(_) => UnitKind::MetaOther,
                }
            };
            let u = Unit { kind, at: k, inner: Vec::new(), sep: None, passthrough: false };
            k += 2;
            u
        } else {
            let u = Unit { kind: token_unit_kind(toks, k), at: k, inner: Vec::new(), sep: None, passthrough: false };
            k += 1;
            u
        };
        units.push(unit);
    }
    Ok(units)
}

/// Can the expansion of `a` immediately followed by `b` glue tokens into a
/// path, an attribute or a macro call that neither the transcriber nor the
/// call site shows?  A `tt` metavariable may be *any* token - `::`, `#`, `!`,
/// `std` - so it is unsafe next to an identifier, another metavariable, `[`
/// (a `#` makes it an attribute) or `!` (an identifier makes it a macro call).
fn units_glue(a: &Unit, b: &Unit) -> bool {
    (a.is_tt() && (b.kind == UnitKind::PathIdent || b.is_meta() || matches!(b.kind, UnitKind::OpenBracket | UnitKind::Bang)))
        || (b.is_tt() && (a.kind == UnitKind::PathIdent || a.is_meta()))
}

/// Is the `{` at `open` the start of something that could be a `use` group
/// (`{std::env as out}`, `{a::{b, c}}`, `{$p}`) rather than a block or a struct
/// literal?  A use group holds only identifiers, `::`, `,`, `*`, nested braces
/// and metavariables, and binds a name other than its own only through `::`,
/// `as`, `*` or a non-`ident` metavariable.  A struct literal has `field:` or
/// `..`; a block has calls, operators or literals; `{ $a, $b }` with `ident`
/// fragments binds each name to itself.
fn group_may_be_use_tree(toks: &[Token], br: &Brackets, frags: &HashMap<String, String>, open: usize) -> bool {
    let close = br.close(open).min(toks.len());
    let mut renames = false;
    let mut k = open + 1;
    while k < close {
        match &toks[k].tok {
            Tok::PathSep | Tok::Punct('*') => renames = true,
            Tok::Ident { name, raw: false } if name == "as" => renames = true,
            Tok::Ident { .. } | Tok::Punct(',' | '{' | '}') => {}
            Tok::Punct('$') => match ident_at(toks, k + 1) {
                Some((n, _)) if frags.get(n).map(String::as_str) == Some("ident") || n == "crate" => k += 1,
                _ => renames = true,
            },
            _ => return false,
        }
        k += 1;
    }
    renames
}

/// Keyword splices (security review round 4, H4).
///
/// An `ident` fragment matches *any* identifier, keywords included, and a `tt`
/// matches any token.  So the caller can supply the keyword of an item whose
/// body the transcriber spells:
///
/// ```text
///   ($k:ident) => { $k std::io as out; }        kw!(use)      -> use std::io as out;
///   ($k:ident) => { $k crate std as s; }        kw!(extern)   -> extern crate std as s;
///   ($k:tt)    => { $k {std::env as out}; }     kw!(use)      -> use {std::env as out};
/// ```
///
/// The walker sees `use`/`extern` only as a lone argument and the binding
/// never reaches the table, so `out::stdout()` was pure (cases
/// `ident-keyword-use`, `ident-keyword-extern-narrow`).
///
/// Two places could catch this: the *call site* (a bare keyword as a macro
/// argument) or the *transcriber* (a metavariable where a keyword would start
/// an item).  The call site is not sound: `kw!(use std;)` against
/// `($k:ident $s:ident ;) => { $k $s as out; }` passes a complete-looking
/// `use std;` while the expansion binds `out`.  So the rule is on the
/// transcriber side, keyed on what follows the metavariable.  Every keyword
/// that introduces a binding the analyzer tracks is followed by a name, a path
/// or a group: `use <path|{..}|$x>`, `extern crate`, `type <name>`,
/// `impl <path|<..>|$x>`, `mod <name>`.  So an `ident`/`tt` metavariable (or a
/// repetition ending in one) is CAP003 when the next unit is
///
/// * an identifier that can be a path segment (`std`, `crate`, a name) or a
///   string literal (`extern "C"`),
/// * another metavariable or repetition,
/// * a `{ .. }` group that can be a use tree ([`group_may_be_use_tree`]), or
/// * `<` with the metavariable at the start of an item (`$k<T> Tr for T {}`).
///
/// A strict keyword after it (`$x as u8`, `for $i in`, `$t where`) is not a
/// trigger: no tracked binding keyword is followed by one.  `unsafe` and
/// `extern` reach the walk as tokens wherever they are written, call sites
/// included, so `kw!(unsafe)` stays `CAP002` (control `ident-keyword-unsafe`).
/// Accepted false positive: `$a < b` at the start of a statement.
fn keyword_follows(toks: &[Token], br: &Brackets, frags: &HashMap<String, String>, before: Option<&Unit>, b: &Unit) -> bool {
    if b.kind == UnitKind::PathIdent || b.is_meta() {
        return true;
    }
    match &toks[b.at].tok {
        Tok::Str(_) => true,
        Tok::Punct('{') => group_may_be_use_tree(toks, br, frags, b.at),
        Tok::Punct('<') => {
            before.is_none() || before.is_some_and(|u| matches!(toks[u.at].tok, Tok::Punct(';' | '{' | '}' | ']')))
        }
        _ => false,
    }
}

/// Report every gluing adjacency in `units`, recursing into repetitions.
fn check_unit_seq(sink: &mut Sink, toks: &[Token], units: &[Unit]) {
    let report = |sink: &mut Sink, at: usize| {
        sink.unanalyzable(
            toks[at].line,
            toks[at].col,
            "macro_rules! splices a token-tree metavariable next to an identifier or another metavariable; the expansion is not analysed",
            "$t ident / $a $b".into(),
        );
    };
    for w in units.windows(2) {
        if units_glue(&w[0], &w[1]) {
            report(sink, w[0].at);
        }
    }
    for u in units {
        if !matches!(u.kind, UnitKind::Repeat { .. }) || u.inner.is_empty() {
            continue;
        }
        check_unit_seq(sink, toks, &u.inner);
        // Iterations are emitted back to back: the last unit of one iteration
        // touches the separator (if any), which touches the first unit of the
        // next iteration.
        let (first, last) = (&u.inner[0], &u.inner[u.inner.len() - 1]);
        match u.sep {
            None if !u.passthrough && units_glue(last, first) => report(sink, u.at),
            Some(s) => {
                let sep = Unit { kind: token_unit_kind(toks, s), at: s, inner: Vec::new(), sep: None, passthrough: false };
                if units_glue(last, &sep) || units_glue(&sep, first) {
                    report(sink, u.at);
                }
            }
            None => {}
        }
    }
}

/// Apply [`keyword_follows`] to every adjacency in `units`, recursing into
/// repetitions (also across consecutive iterations and around a separator).
fn check_keyword_splices(sink: &mut Sink, toks: &[Token], br: &Brackets, frags: &HashMap<String, String>, units: &[Unit]) {
    let report = |sink: &mut Sink, at: usize| {
        sink.unanalyzable(
            toks[at].line,
            toks[at].col,
            "macro_rules! places an `ident`/`tt` metavariable where a keyword would start an item (`$k std::io as out;` with `m!(use)`); the expansion is not analysed",
            "$kw ..".into(),
        );
    };
    for (n, w) in units.windows(2).enumerate() {
        let before = if n == 0 { None } else { units.get(n - 1) };
        if w[0].may_be_keyword() && keyword_follows(toks, br, frags, before, &w[1]) {
            report(sink, w[0].at);
        }
    }
    for u in units {
        if !matches!(u.kind, UnitKind::Repeat { .. }) || u.inner.is_empty() {
            continue;
        }
        check_keyword_splices(sink, toks, br, frags, &u.inner);
        let (first, last) = (&u.inner[0], &u.inner[u.inner.len() - 1]);
        match u.sep {
            None if !u.passthrough && last.may_be_keyword() && keyword_follows(toks, br, frags, None, first) => {
                report(sink, u.at)
            }
            Some(s) => {
                let sep = Unit { kind: token_unit_kind(toks, s), at: s, inner: Vec::new(), sep: None, passthrough: false };
                if (last.may_be_keyword() && keyword_follows(toks, br, frags, None, &sep))
                    || (sep.may_be_keyword() && keyword_follows(toks, br, frags, None, first))
                {
                    report(sink, u.at);
                }
            }
            None => {}
        }
    }
}

/// Splice checks for one transcriber `toks[t_open..=t_close]` (HIGH-2).
///
/// The walker sees a transcriber's literal tokens and, separately, each call
/// site's argument tokens.  A path or an attribute that exists only in the
/// *expansion* - pieces from both glued together - is invisible to it:
///
/// ```text
///   ($s:tt) => { std $s fs $s write(..) }     m!(::)              PoC t2
///   ($a:tt) => { # $a pub mod hidden; }       m!([path = ".."])   PoC t6
/// ```
///
/// Two rules, both fail closed:
///
/// 1. a `#` must be followed by `[` or `![`, i.e. start a complete attribute the
///    walker inspects.  `# $a`, `#! $a`, a `#` before a delimiter or used as a
///    repetition separator, are CAP003;
/// 2. a `tt` metavariable (or a repetition containing one) directly next to an
///    identifier that can be a path segment, to another metavariable, or
///    followed by `[` / `!`, is CAP003 ([`units_glue`]).  Other fragments
///    (`ident`, `expr`, `ty`, `path`, ...) cannot produce `::` or `#`, so
///    `$vis struct $name` or `fn $name(x: $t)` stay clean.  One exception: a
///    verbatim pass-through `$($t)*` of a matcher `$($t:tt)*` (same separator
///    and operator) re-emits the caller's tokens in the caller's order.
fn check_transcriber_splices(sink: &mut Sink, toks: &[Token], br: &Brackets, arm: &MacroArm) {
    let (from, to) = (arm.t_open + 1, arm.t_close.min(toks.len()));
    for k in from..to {
        if is_punct(toks, k, '#') {
            let whole_attr = is_punct(toks, k + 1, '[') || (is_punct(toks, k + 1, '!') && is_punct(toks, k + 2, '['));
            if !whole_attr {
                sink.unanalyzable(
                    toks[k].line,
                    toks[k].col,
                    "macro_rules! emits a `#` that does not start a complete attribute; the expansion is not analysed",
                    "# $..".into(),
                );
            }
        }
    }
    // `mod $n;` / `mod $n { .. }` with a caller-chosen name: invoked from a
    // root-level file, `mk!(tests)` declares `mod tests;` and compiles a
    // skipped directory (HIGH-5).  The invocation site is unknown here, so an
    // out-of-line declaration is CAP003, and so is an inline one whose body
    // may declare an out-of-line module: it holds `mod`, or a metavariable
    // that can *be* one (`tt`, `item`, `stmt`, an undeclared name, or an
    // `ident` - which matches keywords - followed by a name: `$k x;`).
    let frags = matcher_fragments(toks, arm.m_open + 1, arm.m_close);
    let may_declare_mod = |j: usize| -> bool {
        if ident_at(toks, j) == Some(("mod", false)) {
            return true;
        }
        if !is_punct(toks, j, '$') {
            return false;
        }
        match ident_at(toks, j + 1) {
            Some(("crate", false)) | None => false,
            Some((name, _)) => match frags.get(name).map(String::as_str) {
                Some("expr" | "ty" | "path" | "pat" | "pat_param" | "literal" | "lifetime" | "block" | "meta" | "vis") => false,
                // A module name is never a strict keyword: `$f as alias` is not `mod as`.
                Some("ident") => {
                    matches!(ident_at(toks, j + 2), Some((n, raw)) if raw || !is_non_path_keyword(n))
                        || is_punct(toks, j + 2, '$')
                }
                _ => true,
            },
        }
    };
    for k in from..to {
        if ident_at(toks, k) != Some(("mod", false)) || !is_punct(toks, k + 1, '$') || ident_at(toks, k + 2).is_none() {
            continue;
        }
        let after = k + 3;
        let risky = if is_punct(toks, after, '{') {
            let c = br.close(after).min(to);
            (after + 1..c).any(may_declare_mod)
        } else {
            true
        };
        if risky {
            sink.unanalyzable(
                toks[k].line,
                toks[k].col,
                "macro_rules! declares a module whose name is a metavariable; its file location is not tracked",
                "mod $name".into(),
            );
        }
    }
    let tt_repeats = matcher_tt_repeats(toks, arm);
    match transcriber_units(toks, br, &tt_repeats, &frags, from, to, 0) {
        Ok(units) => {
            check_unit_seq(sink, toks, &units);
            check_keyword_splices(sink, toks, br, &frags, &units);
        }
        Err(at) => sink.unanalyzable(
            toks[at].line,
            toks[at].col,
            "macro_rules! repetitions nest too deeply to analyse",
            "$( $( ..".into(),
        ),
    }
}

/// Flag metavariable splices inside a `macro_rules!` body (VER08a §3.5).
fn check_macro_body(sink: &mut Sink, toks: &[Token], br: &Brackets, open: usize, close: usize) {
    // `macro_rules!` nested inside `macro_rules!` is re-scanned by its outer
    // bodies; the budget keeps the total linear (CAP003 when exceeded).
    let cost = close.min(toks.len()).saturating_sub(open);
    if cost > sink.macro_budget {
        sink.macro_budget = 0;
        sink.unanalyzable(
            toks[open].line,
            toks[open].col,
            "macro_rules! nesting exceeds the analysis budget",
            "macro_rules! ...".into(),
        );
        return;
    }
    sink.macro_budget -= cost;
    // Only transcribers are emitted as code; matchers (`$a:ident $b:tt`,
    // `$(#[$m:meta])*`) are patterns and are not checked.  A body that is not
    // a list of `(matcher) => {transcriber}` arms is rejected by rustc; it is
    // checked as one transcriber and reported.
    match macro_arms(toks, br, open, close) {
        Some(arms) => {
            for arm in &arms {
                check_transcriber_legacy(sink, toks, br, arm.t_open, arm.t_close);
                check_transcriber_splices(sink, toks, br, arm);
            }
        }
        None => {
            sink.unanalyzable(
                toks[open].line,
                toks[open].col,
                "macro_rules! body is not a list of `(matcher) => {transcriber}` arms",
                "macro_rules! ...".into(),
            );
            check_transcriber_legacy(sink, toks, br, open, close);
        }
    }
}

/// The path-splice rules of rounds 1-2 (`$x::`, `::$x`, `$(..)::`, `<$t>::`,
/// `#[$a]`, `$m!`) over the tokens `toks[open..close]`.
fn check_transcriber_legacy(sink: &mut Sink, toks: &[Token], br: &Brackets, open: usize, close: usize) {
    // `repeat[d]` is true if the group opened at depth d began with `$(`.
    let mut stack: Vec<bool> = Vec::new();
    let mut last_closed_repeat = false;
    let mut closed_at = usize::MAX;
    for k in open..close.min(toks.len()) {
        match &toks[k].tok {
            Tok::Punct('(') | Tok::Punct('[') | Tok::Punct('{') => {
                stack.push(k > 0 && is_punct(toks, k - 1, '$'));
            }
            Tok::Punct(')') | Tok::Punct(']') | Tok::Punct('}') => {
                last_closed_repeat = stack.pop().unwrap_or(false);
                closed_at = k;
            }
            Tok::Punct('#') => {
                // `#[$a]` / `#[cfg_attr(x, $a)]`: the attribute's identity is chosen by the
                // caller, so `m!(no_mangle)` would create `#[no_mangle]` unseen.
                let mut o = k + 1;
                if is_punct(toks, o, '!') {
                    o += 1;
                }
                if is_punct(toks, o, '[') {
                    let c = br.close(o).min(toks.len());
                    let spliced_name = is_punct(toks, o + 1, '$');
                    let (has_cfg_attr, has_dollar) = scan_attr_span(toks, br, o + 1, c);
                    let cfg_attr_splice = has_cfg_attr && has_dollar;
                    if spliced_name || cfg_attr_splice {
                        sink.unanalyzable(
                            toks[k].line,
                            toks[k].col,
                            "macro_rules! splices a metavariable into an attribute; the expansion is not analysed",
                            "#[$..]".into(),
                        );
                    }
                }
            }
            Tok::Punct('!') => {
                // `$mac!(..)`: the macro being invoked is chosen by the caller, so
                // `m!(include, "../x.rs")` would pull in unscanned code (M2).
                let is_ne = is_punct(toks, k + 1, '=') && toks[k].end == toks[k + 1].start;
                if !is_ne
                    && k >= 2
                    && ident_at(toks, k - 1).is_some()
                    && is_punct(toks, k - 2, '$')
                {
                    sink.unanalyzable(
                        toks[k].line,
                        toks[k].col,
                        "macro_rules! invokes a macro chosen by a metavariable; the expansion is not analysed",
                        "$name!(..)".into(),
                    );
                }
            }
            Tok::Punct('.') if is_punct(toks, k + 1, '$') && !(k > 0 && is_punct(toks, k - 1, '.')) => {
                // `$e.$m()`: the method is chosen by the caller, so
                // `call!(Path::new(".."), exists)` is `Path::exists` and the
                // method heuristic (§3.3) never sees the name (H7, case
                // `ident-method-splice`).  After `.`, a repetition `$( .. )`, or
                // a metavariable followed by `(` or a turbofish `::`, is
                // CAP003; a spliced *field* (`$e.$f`) is not a call.
                let method_splice = is_punct(toks, k + 2, '(')
                    || (ident_at(toks, k + 2).is_some() && (is_punct(toks, k + 3, '(') || is_pathsep(toks, k + 3)));
                if method_splice {
                    sink.unanalyzable(
                        toks[k].line,
                        toks[k].col,
                        "macro_rules! calls a method whose name is a metavariable; the method is not analysed",
                        ".$m(..)".into(),
                    );
                }
            }
            Tok::PathSep => {
                let next_is_meta = is_punct(toks, k + 1, '$') && ident_at(toks, k + 2) != Some(("crate", false));
                let prev_is_meta = k >= 2
                    && ident_at(toks, k - 1).is_some()
                    && is_punct(toks, k - 2, '$')
                    && ident_at(toks, k - 1) != Some(("crate", false));
                // `$(..)::`, `$(..)*::` and `$(..),*::`: a repetition group (with or without
                // separator / kleene operator) directly before `::` splices unseen path parts.
                let is_kleene = |i: usize| is_punct(toks, i, '*') || is_punct(toks, i, '+') || is_punct(toks, i, '?');
                let prev_is_repeat = last_closed_repeat
                    && k >= 1
                    && (closed_at == k - 1
                        || (k >= 2 && closed_at == k - 2 && is_kleene(k - 1))
                        || (k >= 3 && closed_at == k - 3 && is_kleene(k - 1)));
                // `<$t>::method`: a metavariable inside a `<..>` qualified-path group.
                // Exception: `<$t>::MAX` / `<$t>::INFINITY` (SCREAMING_CASE) is an associated
                // constant; a constant cannot perform I/O, and no `Path` method is spelled so.
                let screaming_const = matches!(ident_at(toks, k + 1), Some((n, false))
                    if n.len() > 1 && n.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_'));
                let angle_splice =
                    k >= 1 && is_punct(toks, k - 1, '>') && !screaming_const && angle_group_has_meta(toks, k - 1);
                if next_is_meta || prev_is_meta || prev_is_repeat || angle_splice {
                    sink.unanalyzable(
                        toks[k].line,
                        toks[k].col,
                        "macro_rules! splices a metavariable into a path; the expansion is not analysed",
                        "$..::..".into(),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Inspect the attribute whose `#` is at `hash`.
fn check_attribute(sink: &mut Sink, toks: &[Token], br: &Brackets, hash: usize) {
    let mut open = hash + 1;
    if is_punct(toks, open, '!') {
        open += 1;
    }
    if !is_punct(toks, open, '[') {
        return;
    }
    let close = br.close(open);
    let (line, col) = (toks[hash].line, toks[hash].col);
    let mut seen_attr: HashSet<String> = HashSet::new();
    let end = close.min(toks.len());
    let mut k = open + 1;
    while k < end {
        // A nested attribute is inspected by its own `#`; skipping it here keeps
        // the total work linear on `#[#[#[...`.
        if is_punct(toks, k, '#') {
            let mut o = k + 1;
            if is_punct(toks, o, '!') {
                o += 1;
            }
            if is_punct(toks, o, '[') {
                k = br.close(o).saturating_add(1).max(k + 1);
                continue;
            }
        }
        if let Some((name, _)) = ident_at(toks, k) {
            if FOREIGN_LINKAGE_ATTRS.contains(&name) && seen_attr.insert(name.to_string()) {
                sink.banned(line, col, "foreign_linkage", format!("#[{name}]"));
            }
            if name == "path" && is_punct(toks, k + 1, '=') {
                sink.unanalyzable(
                    line,
                    col,
                    "#[path] module redirection is not tracked",
                    "#[path = ...]".into(),
                );
            }
            if matches!(name, "proc_macro" | "proc_macro_derive" | "proc_macro_attribute") {
                sink.unanalyzable(
                    line,
                    col,
                    "procedural macro definition runs arbitrary code at compile time",
                    format!("#[{name}]"),
                );
            }
        }
        k += 1;
    }
}

// ---------------------------------------------------------------------------
// Receiver resolution for `.canonicalize()`-style method heuristics (VER08a §3.3, L4)
// ---------------------------------------------------------------------------

/// Inherent methods and constructors of types *defined in this package*.
#[derive(Default)]
struct LocalTypes {
    /// `Type -> methods taking self` declared in `impl Type { .. }`.
    methods: HashMap<String, HashSet<String>>,
    /// `Type -> associated fns` whose return type is `Self`, `Type`,
    /// `Result<Self, _>` or `Option<Self>` (so `Type::ctor(..)` has type `Type`).
    ctors: HashMap<String, HashSet<String>>,
}

/// Is the return type starting at `r` one of `Self`, `T`, `Result<Self|T, ..>`, `Option<Self|T>`?
fn returns_own_type(toks: &[Token], r: usize, ty: &str) -> bool {
    let own = |i: usize| matches!(ident_at(toks, i), Some((n, false)) if n == "Self" || n == ty);
    if own(r) {
        return true;
    }
    matches!(ident_at(toks, r), Some(("Result" | "Option", false)))
        && is_punct(toks, r + 1, '<')
        && own(r + 2)
        && (is_punct(toks, r + 3, ',') || is_punct(toks, r + 3, '>'))
}

fn collect_local_types(toks: &[Token], br: &Brackets, out: &mut LocalTypes) {
    for i in 0..toks.len() {
        if ident_at(toks, i) != Some(("impl", false)) {
            continue;
        }
        // Item position only: `-> impl Trait` and `x: impl Fn()` follow `>` / `:`.
        let item_pos = i == 0
            || is_punct(toks, i - 1, ';')
            || is_punct(toks, i - 1, '}')
            || is_punct(toks, i - 1, '{')
            || is_punct(toks, i - 1, ']')
            || matches!(ident_at(toks, i - 1), Some(("unsafe" | "default", false)));
        if !item_pos {
            continue;
        }
        let mut j = i + 1;
        if is_punct(toks, j, '<') {
            match skip_angle(toks, j) {
                Some(n) => j = n,
                None => continue,
            }
        }
        let mut ty: Option<String> = None;
        while let Some((seg, _)) = ident_at(toks, j) {
            ty = Some(seg.to_string());
            if is_pathsep(toks, j + 1) {
                j += 2;
            } else {
                j += 1;
                break;
            }
        }
        let Some(ty) = ty else { continue };
        if is_punct(toks, j, '<') {
            match skip_angle(toks, j) {
                Some(n) => j = n,
                None => continue,
            }
        }
        if ident_at(toks, j) == Some(("for", false)) {
            continue; // trait impl: not inherent
        }
        if ident_at(toks, j) == Some(("where", false)) {
            let stop = (j + 512).min(toks.len());
            while j < stop && !is_punct(toks, j, '{') {
                j += 1;
            }
        }
        if !is_punct(toks, j, '{') {
            continue;
        }
        let close = br.close(j).min(toks.len());
        let mut k = j + 1;
        while k < close {
            if matches!(toks[k].tok, Tok::Punct('(' | '[' | '{')) {
                k = br.close(k).saturating_add(1).max(k + 1);
                continue;
            }
            if let (Some(("fn", false)), Some((fname, _))) = (ident_at(toks, k), ident_at(toks, k + 1)) {
                let mut m = k + 2;
                if is_punct(toks, m, '<') {
                    match skip_angle(toks, m) {
                        Some(n) => m = n,
                        None => {
                            k += 1;
                            continue;
                        }
                    }
                }
                if is_punct(toks, m, '(') {
                    let pc = br.close(m);
                    let mut a = m + 1;
                    while matches!(toks.get(a).map(|t| &t.tok), Some(Tok::Punct('&') | Tok::Lifetime))
                        || ident_at(toks, a) == Some(("mut", false))
                    {
                        a += 1;
                    }
                    let has_self = ident_at(toks, a) == Some(("self", false));
                    if has_self {
                        out.methods.entry(ty.clone()).or_default().insert(fname.to_string());
                    } else if is_punct(toks, pc + 1, '-')
                        && is_punct(toks, pc + 2, '>')
                        && returns_own_type(toks, pc + 3, &ty)
                    {
                        out.ctors.entry(ty.clone()).or_default().insert(fname.to_string());
                    }
                    k = pc.saturating_add(1).max(k + 1);
                    continue;
                }
            }
            k += 1;
        }
    }
}

/// Is the receiver of the method call whose `.` is at `dot` provably an
/// instance of a package-local type that has its *own inherent* method of that
/// name?  Inherent methods win method resolution, so such a call can never
/// reach `Path::<name>`.  Recognised receivers (and only these):
///
/// ```text
///   Local::ctor(..)                       ctor returns Self / Result<Self,_> / Option<Self>
///   Local::ctor(..).unwrap() / .expect(..) / .clone() / ?
/// ```
///
/// Anything else (a variable, a field, any other adapter) stays unresolved and
/// is flagged, as before.
fn receiver_is_local_inherent(
    toks: &[Token],
    br: &Brackets,
    res: &Resolver,
    local: &LocalTypes,
    dot: usize,
    method: &str,
) -> bool {
    let mut pos = dot;
    for _ in 0..64 {
        if pos == 0 {
            return false;
        }
        let j = pos - 1;
        if is_punct(toks, j, '?') {
            pos = j;
            continue;
        }
        if !is_punct(toks, j, ')') {
            return false;
        }
        let open = br.open_of.get(j).copied().unwrap_or(usize::MAX);
        if open == usize::MAX || open == 0 {
            return false;
        }
        let f = open - 1;
        let Some((fname, _)) = ident_at(toks, f) else {
            return false;
        };
        if f >= 1 && is_punct(toks, f - 1, '.') {
            if matches!(fname, "unwrap" | "expect" | "clone") {
                pos = f - 1;
                continue;
            }
            return false;
        }
        if f >= 2 && is_pathsep(toks, f - 1) {
            let Some((ty, _)) = ident_at(toks, f - 2) else {
                return false;
            };
            // `a::Type::ctor` / `<..>::ctor` / `x.Type::..` are not simple local paths.
            if f >= 3 && (is_pathsep(toks, f - 3) || is_punct(toks, f - 3, '>') || is_punct(toks, f - 3, '.')) {
                return false;
            }
            // The name `ty` must denote the local type.  A `use`/`type` binding
            // of that name rebinds it (L4).  So may a glob import, which no
            // binding records: with `use std::path::*;` in scope, `PathBuf` at
            // the call site is std's `PathBuf` even if another module defines
            // its own `PathBuf` with an inherent `read_dir` (security review
            // round 3, MEDIUM-1, PoC t11).  Bindings and globs are package-wide,
            // so any glob reaching outside the crate refuses the waiver.
            return local.ctors.get(ty).is_some_and(|c| c.contains(fname))
                && local.methods.get(ty).is_some_and(|m| m.contains(method))
                && !res.has_binding(ty)
                && !res.foreign_glob;
        }
        return false;
    }
    false
}

/// Most `<..>` groups [`qualified_self_path`] walks through before giving up
/// (`CAP003`).  Real code chains one or two (`<T as Tr>::A::<>::item`); the
/// cap keeps the work per path small, since every segment after a `>::`
/// starts a new walk and an uncapped chain made each walk as long as the
/// chain.
const MAX_QUALIFIED_HOPS: usize = 8;

/// What [`qualified_self_path`] learns about the `<..>` group before a `>::`.
struct QualifiedSelf {
    /// The path whose associated item is being named, if any.
    prefix: Option<Vec<String>>,
    /// The innermost qualified group names a trait (`<T as Trait>`, an `as`
    /// at its top level).
    via_trait: bool,
    /// How many segments of `prefix` were written *after* that qualified
    /// group, through turbofish segments: 1 for the `Target` of
    /// `<T as Deref>::Target::<>::exists`.  0 when there is no such group.
    assoc_segs: usize,
}

/// For the `>` at `gt` followed by `::` (`<File as T>::create`, `<File>::create`,
/// `Vec::<u8>::new`): the path whose associated item is being named, and
/// whether the group names a trait (`<T as Trait>`, an `as` at its top level).
/// A macro invocation in the self position (`<ty!()>::exists`) yields
/// `[UNKNOWN_TYPE_SEGMENT]`, so the item gets the method-call verdict.
/// `Err(())` if a matching `<` cannot be found within 512 tokens.
///
/// A turbofish group (`Path::<T>::item`) names the path before `::<`.  That
/// path may itself follow a `>::` (follow-up review F1, case `proj_qual_tf`):
///
/// ```text
///   <PathBuf as Deref> :: Target :: <> :: exists
///   '----- group -----'   '--- turbofish ---'
/// ```
///
/// The walk back from the turbofish stopped at the group's `>`, so the item
/// read `Target::exists`, with no qualified group and no projection.  It now
/// continues through that `>` (at most [`MAX_QUALIFIED_HOPS`] times), prepends
/// what it finds, and reports the innermost qualified group's `via_trait`
/// together with the number of segments written after it.
fn qualified_self_path(toks: &[Token], gt: usize) -> Result<QualifiedSelf, ()> {
    // Segments collected from turbofish groups, last first (the walk goes
    // backwards; reversed once at the end, so each segment is moved once).
    let mut rev_tail: Vec<String> = Vec::new();
    let mut gt = gt;
    let mut hops = 0usize;
    loop {
        let open = matching_open_angle(toks, gt).ok_or(())?;
        if !(open >= 1 && is_pathsep(toks, open - 1)) {
            let (prefix, via_trait) = qualified_group(toks, open, gt)?;
            let assoc_segs = rev_tail.len();
            rev_tail.reverse();
            let tail = rev_tail;
            let prefix = match prefix {
                Some(mut p) => {
                    p.extend(tail);
                    Some(p)
                }
                None if !tail.is_empty() => Some(tail),
                None => None,
            };
            if prefix.as_ref().is_some_and(|p| p.len() > MAX_PATH_SEGS) {
                return Err(());
            }
            return Ok(QualifiedSelf { prefix, via_trait, assoc_segs });
        }
        // Turbofish `Path::<T>::item`: the item belongs to the path before `::<`.
        let mut q = open;
        while q >= 2 && is_pathsep(toks, q - 1) {
            match ident_at(toks, q - 2) {
                Some((n, _)) => rev_tail.push(n.to_string()),
                None => break,
            }
            q -= 2;
            if rev_tail.len() > MAX_PATH_SEGS {
                return Err(());
            }
        }
        // `<..> :: Seg :: <..> ::` - the path continues before another group
        // (not `->` / `=>`, whose `>` closes nothing).
        let chained = !rev_tail.is_empty()
            && q >= 2
            && is_pathsep(toks, q - 1)
            && is_punct(toks, q - 2, '>')
            && !(q >= 3 && (is_punct(toks, q - 3, '-') || is_punct(toks, q - 3, '=')));
        if !chained {
            rev_tail.reverse();
            let prefix = if rev_tail.is_empty() { None } else { Some(rev_tail) };
            return Ok(QualifiedSelf { prefix, via_trait: false, assoc_segs: 0 });
        }
        hops += 1;
        if hops > MAX_QUALIFIED_HOPS {
            return Err(());
        }
        gt = q - 2;
    }
}

/// The `<` matching the `>` at `gt`, searching back at most 512 tokens.
fn matching_open_angle(toks: &[Token], gt: usize) -> Option<usize> {
    let mut depth = 1usize;
    let lo = gt.saturating_sub(512);
    let mut k = gt;
    while k > lo {
        k -= 1;
        if is_punct(toks, k, '>') && !(k > 0 && is_punct(toks, k - 1, '-')) {
            depth += 1;
        } else if is_punct(toks, k, '<') {
            depth -= 1;
            if depth == 0 {
                return Some(k);
            }
        }
    }
    None
}

/// The self type path of the qualified group `<..>` spanning `open..=gt`
/// (not a turbofish), and whether the group names a trait.
fn qualified_group(toks: &[Token], open: usize, gt: usize) -> Result<(Option<Vec<String>>, bool), ()> {
    // `<T as Trait>`: an `as` outside any nested `<..>` or bracket.
    let mut via_trait = false;
    let mut nest = 0usize;
    for k in open + 1..gt {
        match &toks[k].tok {
            Tok::Punct('<' | '(' | '[' | '{') => nest += 1,
            Tok::Punct('>') if !is_punct(toks, k - 1, '-') => nest = nest.saturating_sub(1),
            Tok::Punct(')' | ']' | '}') => nest = nest.saturating_sub(1),
            Tok::Ident { name, raw: false } if nest == 0 && name == "as" => via_trait = true,
            _ => {}
        }
    }
    let mut j = open + 1;
    loop {
        match toks.get(j).map(|t| &t.tok) {
            // `(` too: `<(std::path::Path)>::exists` is a parenthesised type,
            // read as the type inside (security review round 5, R5-2).  For a
            // tuple `<(A, B)>::x` this reads `A::x`, which only adds a spelling.
            Some(Tok::Punct('&' | '*' | '<' | '(')) | Some(Tok::Lifetime) => j += 1,
            Some(Tok::Ident { name, raw: false }) if matches!(name.as_str(), "mut" | "dyn" | "const") => j += 1,
            _ => break,
        }
    }
    if is_pathsep(toks, j) {
        j += 1;
    }
    let mut segs: Vec<String> = Vec::new();
    while let Some((n, _)) = ident_at(toks, j) {
        segs.push(n.to_string());
        if segs.len() > MAX_PATH_SEGS {
            return Err(());
        }
        if is_pathsep(toks, j + 1) {
            j += 2;
        } else {
            break;
        }
    }
    // `<ty!()>::exists`: a macro invocation in type position expands to a type
    // the analyzer never sees; its name is not that type (R5-2, case
    // `type-macro-qualified`).  The self type is unknown.
    if !segs.is_empty() && is_punct(toks, j + 1, '!') {
        return Ok((Some(vec![UNKNOWN_TYPE_SEGMENT.to_string()]), via_trait));
    }
    Ok((if segs.is_empty() { None } else { Some(segs) }, via_trait))
}

/// True if the token at `k` is the `]` that closes an attribute `#[..]`
/// (or `#![..]`).
///
/// A generic parameter may carry attributes, `<#[allow(unused)] D: Tr>`, so
/// the token before its name can be the end of an attribute rather than `<`
/// or `,` (follow-up review F2, case `proj_attr_param`).
fn closes_attribute(toks: &[Token], br: &Brackets, k: usize) -> bool {
    if !is_punct(toks, k, ']') {
        return false;
    }
    let o = br.open_of.get(k).copied().unwrap_or(usize::MAX);
    if o == usize::MAX || o == 0 {
        return false;
    }
    is_punct(toks, o - 1, '#') || (o >= 2 && is_punct(toks, o - 1, '!') && is_punct(toks, o - 2, '#'))
}

/// Names declared as generic type (or const) parameters anywhere in the file:
/// the names ([`generic_list_names`]) of the `<..>` right after `impl`, or
/// after the name in `fn`/`struct`/`enum`/`union`/`trait`/`type NAME <..>`.
/// Package-wide scoping is not tracked: a name declared as a parameter
/// anywhere in the file counts everywhere in it (security review round 5,
/// R5-4).  The list is delimited bracket-aware, so a `>` inside a bracketed
/// default does not end it early and hide the parameters after it.
fn generic_param_names(toks: &[Token], br: &Brackets) -> HashSet<String> {
    let mut out = HashSet::new();
    for i in 1..toks.len() {
        if !is_punct(toks, i, '<') {
            continue;
        }
        let opens_params = ident_at(toks, i - 1) == Some(("impl", false))
            || (i >= 2
                && ident_at(toks, i - 1).is_some()
                && matches!(
                    ident_at(toks, i - 2),
                    Some(("fn" | "struct" | "enum" | "union" | "trait" | "type", false))
                ));
        if !opens_params {
            continue;
        }
        let Some(end) = skip_angle_bracketed(toks, br, i) else { continue };
        out.extend(generic_list_names(toks, br, i, end).into_iter().map(str::to_string));
    }
    out
}

/// `mask[i]` is true if token `i` lies inside the delimiters of a macro invocation `name!( .. )`.
fn macro_arg_mask(toks: &[Token], br: &Brackets) -> Vec<bool> {
    let mut diff = vec![0i32; toks.len() + 2];
    for j in 1..toks.len() {
        if is_punct(toks, j, '!')
            && ident_at(toks, j - 1).is_some()
            && matches!(toks.get(j + 1).map(|t| &t.tok), Some(Tok::Punct('(' | '[' | '{')))
        {
            let c = br.close(j + 1).min(toks.len());
            diff[j + 2] += 1;
            diff[c.max(j + 2)] -= 1;
        }
    }
    let mut mask = vec![false; toks.len()];
    let mut acc = 0i32;
    for (k, m) in mask.iter_mut().enumerate() {
        acc += diff[k];
        *m = acc > 0;
    }
    mask
}

// ---------------------------------------------------------------------------
// `Self` (VER08a §3.2, security review round 4, H2)
// ---------------------------------------------------------------------------
//
// `Self` is a keyword, not a binding, so it never reached the binding table:
//
//     impl Probe for std::path::Path {
//         fn probe(&self) -> bool { Self::exists(self) }     // Path::exists
//     }
//
// classified `Self::exists` as a local path (pure).  `Self` is lexically
// scoped to an `impl` block, so it is resolved by *region*: every `impl`
// header names a self type, and inside the block's braces `Self::x` is also
// classified as `<self type>::x`, through the binding table as usual.
//
// Regions are found without a parser, so two things are deliberately loose:
//
// * every non-raw `impl` token starts a candidate region, also `-> impl Trait {`
//   (a return type, whose "region" is a function body).  A bogus region only
//   *adds* spellings, because `Self` is resolved against **every** enclosing
//   region, never just the innermost - so it can never hide the real one;
// * a self type that is not a plain path is `Unknown` when it involves a
//   metavariable, a macro invocation or a qualified projection
//   (`impl T for $t`, `impl T for ty!()`, `impl T for <A as B>::C`), and
//   `NoPath` for slices, tuples, arrays and fn
//   pointers (whose inherent methods are pure).  Inside a `macro_rules!` body
//   `Self` is always `Unknown`: it names whatever impl the macro is expanded
//   in.  For an `Unknown` self type, `Self::name` gets the method-call verdict
//   of `.name(` (`Self::exists` is `fs:list`), exactly as a method call on an
//   unknown receiver does.

/// What `Self` denotes inside one region.
#[derive(Clone, Debug, PartialEq, Eq)]
enum SelfTy {
    /// A path, resolved further through the binding table.
    Known(Vec<String>),
    /// A slice, tuple, array, never or fn-pointer type: no capability-bearing
    /// inherent items.
    NoPath,
    /// Chosen by a macro caller or not a path we can read.
    Unknown,
}

/// A token range `[open, close]` in which `Self` denotes `ty`.
struct SelfRegion {
    open: usize,
    close: usize,
    ty: SelfTy,
}

/// Longest `impl` header (in tokens) searched for the body's `{`.
const IMPL_HEADER_LIMIT: usize = 1024;
/// Enclosing regions considered for one `Self`; deeper nesting is `Unknown`.
const MAX_SELF_REGIONS: usize = 32;

/// Read the self type in `toks[from..to]` (the tokens after `for`, or after
/// `impl<..>` in an inherent impl, up to `where` or the body).
fn parse_self_type(toks: &[Token], br: &Brackets, from: usize, to: usize, depth: usize) -> SelfTy {
    if depth > 16 {
        return SelfTy::Unknown;
    }
    let mut j = from;
    // References, pointers and trait objects: `&'a mut T`, `*const T`, `dyn T`.
    loop {
        match toks.get(j).map(|t| &t.tok) {
            Some(Tok::Punct('&' | '*')) | Some(Tok::Lifetime) if j < to => j += 1,
            Some(Tok::Ident { name, raw: false }) if j < to && matches!(name.as_str(), "mut" | "const" | "dyn") => j += 1,
            _ => break,
        }
    }
    if j >= to {
        return SelfTy::Unknown;
    }
    match &toks[j].tok {
        // `(T)` is `T`; `(A, B)` and `()` are tuples.
        Tok::Punct('(') => {
            let c = br.close(j);
            if c >= to {
                return SelfTy::Unknown;
            }
            let mut k = j + 1;
            let mut top_comma = false;
            while k < c {
                if matches!(toks[k].tok, Tok::Punct('(' | '[' | '{')) {
                    k = br.close(k).saturating_add(1).max(k + 1);
                    continue;
                }
                if is_punct(toks, k, ',') {
                    top_comma = true;
                }
                k += 1;
            }
            if top_comma || c == j + 1 {
                SelfTy::NoPath
            } else {
                parse_self_type(toks, br, j + 1, c, depth + 1)
            }
        }
        Tok::Punct('[') | Tok::Punct('!') => SelfTy::NoPath,
        Tok::Ident { name, raw: false } if matches!(name.as_str(), "fn" | "unsafe" | "extern" | "_") => SelfTy::NoPath,
        Tok::PathSep | Tok::Ident { .. } => {
            if is_pathsep(toks, j) {
                j += 1;
            }
            let mut segs: Vec<String> = Vec::new();
            while let Some((seg, _)) = ident_at(toks, j) {
                if j >= to {
                    break;
                }
                segs.push(seg.to_string());
                if segs.len() > MAX_PATH_SEGS {
                    return SelfTy::Unknown;
                }
                j += 1;
                // Generic arguments `Foo<T>` / `Foo::<T>` belong to this segment.
                if is_pathsep(toks, j) && is_punct(toks, j + 1, '<') {
                    j += 1;
                }
                if is_punct(toks, j, '<') {
                    match skip_angle(toks, j) {
                        Some(n) => j = n,
                        None => return SelfTy::Unknown,
                    }
                }
                if is_pathsep(toks, j) && ident_at(toks, j + 1).is_some() {
                    j += 1;
                } else {
                    break;
                }
            }
            // `impl P for ty!() { .. }`: a macro invocation in type position;
            // its name is not the type it expands to (R5-2, case
            // `type-macro-impl-self`).
            if segs.is_empty() || is_punct(toks, j, '!') {
                SelfTy::Unknown
            } else {
                SelfTy::Known(segs)
            }
        }
        // `<A as B>::C`, a metavariable, anything else.
        _ => SelfTy::Unknown,
    }
}

/// The region of the `impl` at token `i`: its body braces and self type.
/// `None` if no body `{` is found before a `;` or within the header limit.
fn impl_region(toks: &[Token], br: &Brackets, i: usize) -> Option<SelfRegion> {
    // Find the body: the first `{` outside `<..>` (const generic arguments such
    // as `Foo<{ N }>` are blocks inside angle brackets) and outside `(..)`/`[..]`.
    let mut angle = 0usize;
    let mut k = i + 1;
    let stop = (i + 1 + IMPL_HEADER_LIMIT).min(toks.len());
    let open = loop {
        if k >= stop {
            return None;
        }
        match &toks[k].tok {
            Tok::Punct('<') => angle += 1,
            Tok::Punct('>') if !(k > 0 && is_punct(toks, k - 1, '-')) => angle = angle.saturating_sub(1),
            Tok::Punct(';') => return None,
            Tok::Punct('(' | '[') => {
                k = br.close(k).saturating_add(1).max(k + 1);
                continue;
            }
            Tok::Punct('{') if angle == 0 => break k,
            Tok::Punct('{') => {
                k = br.close(k).saturating_add(1).max(k + 1);
                continue;
            }
            _ => {}
        }
        k += 1;
    };
    let close = br.close(open).min(toks.len().saturating_sub(1));
    // A metavariable anywhere in the header: the caller chooses the type.
    if (i + 1..open).any(|k| is_punct(toks, k, '$')) {
        return Some(SelfRegion { open, close, ty: SelfTy::Unknown });
    }
    let mut j = i + 1;
    if is_punct(toks, j, '<') {
        j = match skip_angle(toks, j) {
            Some(n) if n <= open => n,
            _ => return Some(SelfRegion { open, close, ty: SelfTy::Unknown }),
        };
    }
    // `for` at angle depth 0 separates trait and self type; `for<'a>` is a
    // higher-ranked bound, and `where` ends the self type.
    let mut angle = 0usize;
    let mut for_at = None;
    let mut end = open;
    for k in j..open {
        match &toks[k].tok {
            Tok::Punct('<') => angle += 1,
            Tok::Punct('>') if !is_punct(toks, k - 1, '-') => angle = angle.saturating_sub(1),
            Tok::Ident { name, raw: false } if angle == 0 && name == "where" => {
                end = k;
                break;
            }
            Tok::Ident { name, raw: false }
                if angle == 0
                    && name == "for"
                    && for_at.is_none()
                    && !(is_punct(toks, k + 1, '<') && matches!(toks.get(k + 2).map(|t| &t.tok), Some(Tok::Lifetime))) =>
            {
                for_at = Some(k);
            }
            _ => {}
        }
    }
    let from = match for_at {
        Some(f) => f + 1,
        None => {
            // `impl const Tr for` / `impl !Send for` without a `for` is malformed;
            // an inherent impl starts with its type.
            j
        }
    };
    let ty = parse_self_type(toks, br, from, end, 0);
    Some(SelfRegion { open, close, ty })
}

/// Every `impl` region and `macro_rules!` body of a file, sorted by `open`.
fn self_regions(toks: &[Token], br: &Brackets) -> Vec<SelfRegion> {
    let mut out = Vec::new();
    for i in 0..toks.len() {
        if ident_at(toks, i) == Some(("impl", false)) && !(i > 0 && is_punct(toks, i - 1, '$')) {
            if let Some(r) = impl_region(toks, br, i) {
                out.push(r);
            }
        } else if ident_at(toks, i) == Some(("macro_rules", false))
            && is_punct(toks, i + 1, '!')
            && matches!(toks.get(i + 3).map(|t| &t.tok), Some(Tok::Punct('(' | '[' | '{')))
        {
            let open = i + 3;
            out.push(SelfRegion { open, close: br.close(open).min(toks.len().saturating_sub(1)), ty: SelfTy::Unknown });
        }
    }
    out.sort_by_key(|r| r.open);
    out
}

/// Classify the spellings `Self` adds to the path `segs` (`segs[0] == "Self"`)
/// written at a token inside the regions `active` (indices into `regions`).
#[allow(clippy::too_many_arguments)]
fn self_path(
    sink: &mut Sink,
    res: &Resolver,
    regions: &[SelfRegion],
    active: &[usize],
    segs: &[String],
    is_macro: bool,
    line: usize,
    col: usize,
) {
    if segs.len() < 2 {
        return;
    }
    let mut unknown = active.len() > MAX_SELF_REGIONS;
    let mut seen: HashSet<&[String]> = HashSet::new();
    for &r in active.iter().rev().take(MAX_SELF_REGIONS) {
        match &regions[r].ty {
            SelfTy::Known(p) => {
                if seen.insert(p.as_slice()) {
                    let mut full = p.clone();
                    full.extend_from_slice(&segs[1..]);
                    if full.len() <= MAX_PATH_SEGS {
                        sink.path(res, &full, is_macro, line, col);
                    } else {
                        unknown = true;
                    }
                }
            }
            SelfTy::NoPath => {}
            SelfTy::Unknown => unknown = true,
        }
    }
    // The method is the *last* segment: `Self::Target::exists` under an
    // unknown self type is `.exists(` (R5-4), not `.Target(`.
    if unknown {
        if let Some(Classification::Capability { category, action }) = method_call_rule(&segs[segs.len() - 1]) {
            sink.detection(line, col, category, action, format!("{} (Self type not known)", segs.join("::")));
        }
    }
}

fn stage2_file(
    toks: &[Token],
    br: &Brackets,
    facts: &FileFacts,
    res: &Resolver,
    local: &LocalTypes,
    sink: &mut Sink,
) {
    let in_macro_args = macro_arg_mask(toks, br);
    let generics = generic_param_names(toks, br);
    sink.macro_budget = toks.len().saturating_mul(4) + 1024;
    // `Self` regions (H2), entered and left as the walk moves forward.
    let regions = self_regions(toks, br);
    let mut next_region = 0usize;
    let mut active: Vec<usize> = Vec::new();
    // Use leaves (declaration sites).
    for l in &facts.leaves {
        if l.path.is_empty() {
            continue;
        }
        if l.glob && res.glob_is_bare_root(&l.path) {
            sink.unanalyzable(
                l.line,
                l.col,
                "glob import of a whole standard crate (`use std::*`): every std module becomes nameable without `std`",
                format!("{}::*", l.path.join("::")),
            );
        }
        sink.path(res, &l.path, false, l.line, l.col);
    }
    for &(s, reason) in &facts.use_errors {
        let evidence = if reason.contains("extern crate") {
            "extern crate ..."
        } else if reason.contains("type alias") {
            "type ..."
        } else {
            "use ..."
        };
        sink.unanalyzable(toks[s].line, toks[s].col, reason, evidence.into());
    }

    let skip: HashMap<usize, usize> = facts.skip.iter().copied().collect();
    let mut i = 0;
    while i < toks.len() {
        if let Some(&end) = skip.get(&i) {
            i = end;
            continue;
        }
        let t = &toks[i];
        match &t.tok {
            Tok::Punct('#') => {
                check_attribute(sink, toks, br, i);
                i += 1;
            }
            Tok::Ident { name, raw } => {
                let after_dot = i > 0
                    && is_punct(toks, i - 1, '.')
                    && !(i > 1 && is_punct(toks, i - 2, '.'));
                if after_dot {
                    // Method heuristic (VER08a §3.3): `.exists(`, `.set_times(`,
                    // `.recv_timeout(`, `.elapsed(`, ... reach the OS on some std
                    // type whatever the receiver is spelled as.
                    //
                    // A turbofish may sit between the name and `(`:
                    // `p.exists::<>()` is legal for a method with no generic
                    // parameters, and called `Path::exists` unseen (security
                    // review round 5, R5-1, case `turbofish-exists`).
                    //
                    // `.name::<` is *always* a method call: a field takes no
                    // generic arguments, so there is nothing else it can be.
                    // The turbofish is therefore not walked to find the `(`.
                    // Walking it was unsound: `skip_angle` counts `<`/`>`
                    // without regard to brackets, so the comparison in
                    // `c.wait_timeout::<[u8; (2 > 1) as usize]>(g, d)` closed
                    // it early and the call read as a field (follow-up review
                    // F4, case `tf_paren_gt`).
                    //
                    //   .name (          call
                    //   .name ::<        call (turbofish; fields cannot have one)
                    //   .name            field (anything else)
                    let is_call = is_punct(toks, i + 1, '(')
                        || (is_pathsep(toks, i + 1) && is_punct(toks, i + 2, '<'));
                    if is_call {
                        if let Some(Classification::Capability { category, action }) = method_call_rule(name) {
                            if !receiver_is_local_inherent(toks, br, res, local, i - 1, name) {
                                sink.detection(t.line, t.col, category, action, format!(".{name}()"));
                            }
                        }
                    }
                    i += 1;
                    continue;
                }
                if i > 0 && is_punct(toks, i - 1, '$') {
                    i += 1;
                    continue;
                }
                if !*raw {
                    match name.as_str() {
                        "unsafe" => {
                            sink.banned(t.line, t.col, "unsafe_code", "unsafe".into());
                            i += 1;
                            continue;
                        }
                        "extern" => {
                            extern_item(sink, toks, i);
                            i += 1;
                            // Swallow the ABI string so it is not re-read as code.
                            if matches!(toks.get(i).map(|t| &t.tok), Some(Tok::Str(_))) {
                                i += 1;
                            }
                            continue;
                        }
                        "macro_rules" if is_punct(toks, i + 1, '!') => {
                            // macro_rules ! name { ... }
                            let open = i + 3;
                            if matches!(toks.get(open).map(|t| &t.tok), Some(Tok::Punct('(' | '[' | '{'))) {
                                let close = br.close(open);
                                check_macro_body(sink, toks, br, open, close);
                            }
                            i += 1;
                            continue;
                        }
                        _ => {}
                    }
                }
                // Read a whole path: seg (:: seg)*.  After `>::` the item belongs to
                // the qualified self type: `<File>::create`, `<File as T>::create`.
                //
                // `=> ::std::fs::read(..)` and `-> ::std::io::Result` have the same
                // token shape (`>` `::` ident) but no angle bracket to close, and a
                // comparison `a > ::std::x` is ambiguous; so the path as written is
                // *always* classified too, and the qualified reading only adds to it.
                let mut qualified_prefix: Vec<String> = Vec::new();
                let mut via_trait = false;
                let mut assoc_segs = 0usize;
                let closes_angle = i >= 2
                    && is_pathsep(toks, i - 1)
                    && is_punct(toks, i - 2, '>')
                    && !(i >= 3 && (is_punct(toks, i - 3, '=') || is_punct(toks, i - 3, '-')));
                if closes_angle {
                    match qualified_self_path(toks, i - 2) {
                        Ok(q) => {
                            qualified_prefix = q.prefix.unwrap_or_default();
                            via_trait = q.via_trait;
                            assoc_segs = q.assoc_segs;
                        }
                        Err(()) => sink.unanalyzable(
                            t.line,
                            t.col,
                            "qualified path whose self type cannot be located",
                            format!(">::{name}"),
                        ),
                    }
                }
                let mut segs: Vec<String> = vec![name.clone()];
                let mut j = i + 1;
                while is_pathsep(toks, j) {
                    match ident_at(toks, j + 1) {
                        Some((n, _)) => {
                            segs.push(n.to_string());
                            j += 2;
                        }
                        None => break,
                    }
                }
                let is_macro = is_punct(toks, j, '!')
                    && !(is_punct(toks, j + 1, '=') && toks[j].end == toks[j + 1].start);
                // `m!(include, "../x.rs")`: a macro name passed as an argument is
                // invoked by the callee, out of sight of this walk (M2).
                if !is_macro && segs.len() == 1 && is_include_name(&segs[0]) && in_macro_args[i] {
                    sink.unanalyzable(
                        t.line,
                        t.col,
                        "include-family macro name passed as a macro argument; the expansion is not analysed",
                        segs[0].clone(),
                    );
                }
                sink.path(res, &segs, is_macro, t.line, t.col);
                let written_len = segs.len();
                if !qualified_prefix.is_empty() {
                    let mut q = qualified_prefix;
                    q.extend(segs);
                    sink.path(res, &q, is_macro, t.line, t.col);
                    segs = q;
                }
                // Associated-type projections (R5-4): `<T as Tr>::Assoc::item`,
                // `Self::Assoc::item` and `P::Assoc::item` for a generic
                // parameter `P` name a type the analyzer does not evaluate
                // (`<PathBuf as Deref>::Target` is `Path`).  Rust accepts the
                // shorthand `X::Assoc` only for a type parameter or `Self`
                // (E0223), so these are the shapes a projection is *spelled*
                // in at its use.  A type alias can carry one to a use that
                // names only the alias; `type_alias_leaves` binds such an
                // alias to the unknown type (F3).  A turbofish may separate
                // the associated type from the item
                // (`<T as Tr>::Assoc::<>::item`, F1): `assoc_segs` counts the
                // segments written between the group and this path.  The
                // type is unknown: the item gets the method-call verdict of
                // `.item(`.
                let projection = !is_macro
                    && ((via_trait && written_len + assoc_segs >= 2)
                        || (segs.len() >= 3 && (segs[0] == "Self" || generics.contains(segs[0].as_str()))));
                if projection {
                    if let Some(Classification::Capability { category, action }) = method_call_rule(&segs[segs.len() - 1]) {
                        sink.detection(
                            t.line,
                            t.col,
                            category,
                            action,
                            format!("{} (associated type not known)", segs.join("::")),
                        );
                    }
                }
                if segs[0] == "Self" {
                    // Regions are properly nested (they are bracket pairs), so a
                    // stack of the ones containing `i` is maintained lazily.
                    while next_region < regions.len() && regions[next_region].open <= i {
                        while active.last().is_some_and(|&r| regions[r].close < regions[next_region].open) {
                            active.pop();
                        }
                        active.push(next_region);
                        next_region += 1;
                    }
                    while active.last().is_some_and(|&r| regions[r].close < i) {
                        active.pop();
                    }
                    self_path(sink, res, &regions, &active, &segs, is_macro, t.line, t.col);
                }
                i = if is_macro { j + 1 } else { j };
            }
            _ => i += 1,
        }
    }
}

/// Module declarations in a *root-level* file (no `/` in its path: `build.rs`,
/// a `lib.rs`/`main.rs` that `Cargo.toml` places at the root, or a module file
/// beside them).
///
/// The scanner skips `tests/ benches/ examples/ target/ node_modules/ _build/`
/// at the package root (VER08a §5).  A crate root at the root resolves
/// `mod NAME;` to `./NAME.rs` or `./NAME/mod.rs`, so `mod examples;` in
/// `build.rs` (PoC t5), or `mod tests;` in a root `lib.rs` (PoC t10), compiles
/// code from a skipped directory.  In a root-level file this is CAP003:
///
/// * `mod NAME`, inline or out-of-line, at any nesting, when `NAME` is a
///   skipped directory name compared ASCII case-insensitively (a
///   case-insensitive filesystem resolves `Tests/` to `tests/`).  Inline
///   `mod tests { mod x; }` reaches `tests/x.rs` the same way;
/// * any out-of-line `mod NAME;` in a file whose stem is a skipped name
///   (`tests.rs`): as a module file its children live in `tests/`;
/// * a `mod` keyword not followed by a name: it is being handed to a macro
///   (`m!(mod)`), which can complete the declaration out of sight.
///
/// Cargo `path` keys at the root (`[lib] path = "lib.rs"`) stay allowed: the
/// file itself is scanned, and this rule covers what it can reach.
fn check_root_level_mods(sink: &mut Sink, toks: &[Token], file_stem_skipped: bool) {
    for i in 0..toks.len() {
        if ident_at(toks, i) != Some(("mod", false)) || (i > 0 && (is_punct(toks, i - 1, '.') || is_punct(toks, i - 1, '$'))) {
            continue;
        }
        let (line, col) = (toks[i].line, toks[i].col);
        match ident_at(toks, i + 1) {
            Some((name, _)) if is_skipped_root_dir(name) => sink.unanalyzable(
                line,
                col,
                "module declared in a root-level file resolves into a directory the scanner skips",
                format!("mod {name}"),
            ),
            Some((name, _)) if file_stem_skipped && is_punct(toks, i + 2, ';') => sink.unanalyzable(
                line,
                col,
                "out-of-line module in a root-level file named like a skipped directory resolves into that directory",
                format!("mod {name};"),
            ),
            Some(_) => {}
            None => sink.unanalyzable(
                line,
                col,
                "`mod` keyword in a root-level file is not followed by a module name (passed to a macro?)",
                "mod ..".into(),
            ),
        }
    }
}

/// Directory names the scanner skips at the package root (VER08a §5), compared
/// ASCII case-insensitively.
fn is_skipped_root_dir(name: &str) -> bool {
    crate::scanner::SKIPPED_ROOT_DIRS.iter().any(|d| name.eq_ignore_ascii_case(d))
}

/// `extern` at token `i`: FFI detections (VER08a §3.4).
fn extern_item(sink: &mut Sink, toks: &[Token], i: usize) {
    let (line, col) = (toks[i].line, toks[i].col);
    match toks.get(i + 1).map(|t| &t.tok) {
        Some(Tok::Str(abi)) => {
            if abi != "Rust" {
                sink.detection(line, col, "ffi", "call", format!("extern \"{abi}\""));
            } else if is_punct(toks, i + 2, '{') {
                // `extern "Rust" { fn f(); }` imports symbols from outside the
                // crate exactly like `extern "C" { .. }` (L3).
                sink.banned(line, col, "foreign_linkage", "extern \"Rust\" { ... }".into());
                sink.detection(line, col, "ffi", "call", "extern \"Rust\" { ... }".into());
            }
        }
        Some(Tok::Punct('{')) => {
            sink.detection(line, col, "ffi", "call", "extern { ... }".into());
        }
        Some(Tok::Ident { name, .. }) if name == "crate" => {}
        Some(Tok::Ident { name, .. }) if name == "fn" || name == "unsafe" => {
            sink.detection(line, col, "ffi", "call", format!("extern {name}"));
        }
        _ => {
            sink.detection(line, col, "ffi", "call", "extern".into());
        }
    }
}

/// Scan several files as one package: bindings are shared across files.
/// The edition is unknown, so edition-dependent lexemes are CAP003; use
/// [`scan_sources_with_edition`] when `Cargo.toml` names it.
pub fn scan_sources(files: &[(&str, &str)]) -> Result<ScanResult, String> {
    scan_sources_with_edition(files, Edition::Unknown)
}

/// [`scan_sources`] for files compiled with a known edition.
pub fn scan_sources_with_edition(files: &[(&str, &str)], edition: Edition) -> Result<ScanResult, String> {
    let mut lexed: Vec<(&str, Vec<Token>)> = Vec::with_capacity(files.len());
    let mut lex_notes: Vec<(&str, Vec<LexNote>)> = Vec::new();
    for (name, content) in files {
        let (toks, notes) = lex(content, edition).map_err(|e| format!("{name}: {e}"))?;
        lexed.push((name, toks));
        if !notes.is_empty() {
            lex_notes.push((name, notes));
        }
    }

    let brackets: Vec<Brackets> = lexed.iter().map(|(_, t)| Brackets::build(t)).collect();
    let facts: Vec<FileFacts> = lexed.iter().zip(brackets.iter()).map(|((_, t), br)| stage1_file(t, br)).collect();
    let mut local = LocalTypes::default();
    for ((_, toks), br) in lexed.iter().zip(brackets.iter()) {
        collect_local_types(toks, br, &mut local);
    }
    let all_leaves: Vec<UseLeaf> = facts.iter().flat_map(|f| f.leaves.iter().cloned()).collect();

    let mut result = ScanResult::default();
    for (name, notes) in lex_notes {
        for n in notes {
            result.unanalyzable.push(SourceUnanalyzable {
                file: name.to_string(),
                line: n.line,
                col: n.col,
                reason: n.reason.to_string(),
                evidence: n.evidence,
            });
        }
    }
    let resolver = match Resolver::build(&all_leaves) {
        Ok(r) => r,
        Err(reason) => {
            result.unanalyzable.push(SourceUnanalyzable {
                file: files.first().map(|f| f.0).unwrap_or("<package>").to_string(),
                line: 0,
                col: 0,
                reason,
                evidence: "use bindings".into(),
            });
            Resolver::default()
        }
    };

    for (((name, toks), f), br) in lexed.iter().zip(facts.iter()).zip(brackets.iter()) {
        let mut sink = Sink {
            file: name,
            result: &mut result,
            seen: HashSet::new(),
            cand_budget: CANDIDATE_BUDGET,
            macro_budget: 0,
        };
        stage2_file(toks, br, f, &resolver, &local, &mut sink);
        if !name.contains('/') {
            let stem = name.strip_suffix(".rs").unwrap_or(name);
            check_root_level_mods(&mut sink, toks, is_skipped_root_dir(stem));
        }
    }
    Ok(result)
}

// ---------------------------------------------------------------------------
// Public function inventory (used by the certificate checker, VER23a §3.3)
// ---------------------------------------------------------------------------

/// Lexically list every `pub fn` in `content` as `(name, signature)`, in source order.
///
/// A signature is the source text from `pub` up to (excluding) the first `{` or
/// `;` at parenthesis/bracket depth 0, with whitespace collapsed.  Bracket depth
/// matters: `[u8; N]` contains a `;` that must not end the signature (the old
/// word-splitting code truncated there).  `pub(crate)`, `pub(super)` and
/// `pub(in ..)` are not public entry points and are skipped.
pub fn public_fn_signatures(content: &str) -> Result<Vec<(String, String)>, String> {
    let (toks, _) = lex(content, Edition::Unknown)?;
    // `lex` drops a leading byte-order mark; token offsets are relative to the rest.
    let chars: Vec<char> = content.strip_prefix('\u{feff}').unwrap_or(content).chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < toks.len() {
        if let Some(("pub", false)) = ident_at(&toks, i) {
            if is_punct(&toks, i + 1, '(') {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            loop {
                match ident_at(&toks, j) {
                    Some(("const" | "unsafe" | "async", false)) => j += 1,
                    Some(("extern", false)) => {
                        j += 1;
                        if matches!(toks.get(j).map(|t| &t.tok), Some(Tok::Str(_))) {
                            j += 1;
                        }
                    }
                    _ => break,
                }
            }
            if let (Some(("fn", false)), Some((name, _))) =
                (ident_at(&toks, j), ident_at(&toks, j + 1))
            {
                let mut k = j + 2;
                let mut depth = 0i32;
                let mut last = j + 1;
                while k < toks.len() {
                    match &toks[k].tok {
                        Tok::Punct('(') | Tok::Punct('[') => depth += 1,
                        Tok::Punct(')') | Tok::Punct(']') => depth -= 1,
                        Tok::Punct('{') | Tok::Punct(';') if depth <= 0 => break,
                        _ => {}
                    }
                    last = k;
                    k += 1;
                }
                let raw: String = chars[toks[i].start..toks[last].end].iter().collect();
                let sig = raw.split_whitespace().collect::<Vec<_>>().join(" ");
                out.push((name.to_string(), sig));
                i = k;
                continue;
            }
        }
        i += 1;
    }
    Ok(out)
}
