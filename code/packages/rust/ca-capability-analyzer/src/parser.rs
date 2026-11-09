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
    classify_resolved, is_local_head, is_std_root, Classification, FOREIGN_LINKAGE_ATTRS,
    FS_LIST_METHODS, SHORTHAND_HEADS,
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

/// Scan a single Rust source file string.
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

struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
}

fn is_ident_start(c: char) -> bool {
    c == '_' || c.is_alphabetic()
}

fn is_ident_continue(c: char) -> bool {
    c == '_' || c.is_alphanumeric()
}

impl Lexer {
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
                self.bump();
                while self.peek(0).is_some_and(is_ident_continue) {
                    self.bump();
                }
                Tok::Lifetime
            }
            _ => {
                self.bump();
                Tok::Punct('\'')
            }
        }
    }
}

fn lex(content: &str) -> Result<Vec<Token>, String> {
    let mut lx = Lexer {
        chars: content.chars().collect(),
        pos: 0,
        line: 1,
        col: 0,
    };
    let mut toks = Vec::new();

    while let Some(c) = lx.peek(0) {
        if c.is_whitespace() || c == '\u{feff}' {
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
        } else if (c == 'b' || c == 'c') && lx.peek(1) == Some('"') {
            lx.bump();
            Tok::Str(lx.string()?)
        } else if let (true, Some(h)) = (c == 'b' || c == 'c', lx.raw_string_hashes(1)) {
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
                Tok::Ident { name, raw: true }
            } else {
                let mut name = String::new();
                while let Some(ch) = lx.peek(0).filter(|&ch| is_ident_continue(ch)) {
                    name.push(ch);
                    lx.bump();
                }
                Tok::Ident { name, raw: false }
            }
        } else if c.is_ascii_digit() {
            while lx.peek(0).is_some_and(is_ident_continue) {
                lx.bump();
            }
            if lx.peek(0) == Some('.') && lx.peek(1).is_some_and(|d| d.is_ascii_digit()) {
                lx.bump();
                while lx.peek(0).is_some_and(is_ident_continue) {
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
    Ok(toks)
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

/// Everything stage 1 learns about one file.
#[derive(Default)]
struct FileFacts {
    /// Token ranges `[start, end)` consumed by `use` / `extern crate` items.
    skip: Vec<(usize, usize)>,
    leaves: Vec<UseLeaf>,
    /// `(token index, message)` for `use` items that could not be parsed.
    use_errors: Vec<usize>,
}

fn stage1_file(toks: &[Token]) -> FileFacts {
    let mut f = FileFacts::default();
    let mut i = 0;
    while i < toks.len() {
        let prev_is_dot_or_dollar = i > 0 && (is_punct(toks, i - 1, '.') || is_punct(toks, i - 1, '$'));
        if let (Some(("use", false)), false) = (ident_at(toks, i), prev_is_dot_or_dollar) {
            let starts_tree = ident_at(toks, i + 1).is_some()
                || is_pathsep(toks, i + 1)
                || is_punct(toks, i + 1, '{');
            if starts_tree {
                let start = i;
                let mut j = i + 1;
                let ok = parse_use_tree(toks, &mut j, &[], &mut f.leaves, 0) && is_punct(toks, j, ';');
                if !ok {
                    f.use_errors.push(start);
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
        if let (Some(("extern", false)), Some(("crate", false))) =
            (ident_at(toks, i), ident_at(toks, i + 1))
        {
            // extern crate NAME [as ALIAS];
            if let Some((name, _)) = ident_at(toks, i + 2) {
                let t = &toks[i + 2];
                let mut j = i + 3;
                let mut alias = None;
                if let Some(("as", false)) = ident_at(toks, j) {
                    if let Some((a, _)) = ident_at(toks, j + 1) {
                        alias = Some(a.to_string());
                        j += 2;
                    }
                }
                f.leaves.push(UseLeaf {
                    path: vec![name.to_string()],
                    alias,
                    glob: false,
                    line: t.line,
                    col: t.col,
                });
                while j < toks.len() && !is_punct(toks, j, ';') {
                    j += 1;
                }
                let end = (j + 1).min(toks.len());
                f.skip.push((i, end));
                i = end;
                continue;
            }
        }
        i += 1;
    }
    f
}

const MAX_ALIAS_ROUNDS: usize = 16;
const MAX_ALIAS_PATH: usize = 64;
const MAX_ALIAS_ENTRIES: usize = 20_000;

/// Package-wide binding table.
#[derive(Default)]
struct Resolver {
    bindings: HashMap<String, Vec<Vec<String>>>,
    globs: Vec<Vec<String>>,
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

        // Expand binding heads through other bindings to a fixed point.
        let mut converged = false;
        for _ in 0..MAX_ALIAS_ROUNDS {
            let mut changed = false;
            let mut next: Vec<(String, Vec<String>)> = Vec::with_capacity(entries.len());
            // name -> indices of entries that can serve as substitution targets
            // (an entry whose own head is its own name cannot: see below).
            let mut by_name: HashMap<&str, Vec<usize>> = HashMap::new();
            for (j, (n, p)) in entries.iter().enumerate() {
                if p[0] != *n {
                    by_name.entry(n.as_str()).or_default().push(j);
                }
            }
            for (k, (name, path)) in entries.iter().enumerate() {
                // An entry whose own head is its own name (`pub use sha256::sha256;`:
                // a module and a function sharing a name) cannot be expanded
                // further and must not serve as a substitution target either,
                // or every path through it would grow forever.
                let targets: Vec<&Vec<String>> = by_name
                    .get(path[0].as_str())
                    .map(|v| v.iter().filter(|&&j| j != k).map(|&j| &entries[j].1).collect())
                    .unwrap_or_default();
                if targets.is_empty() {
                    next.push((name.clone(), path.clone()));
                } else {
                    changed = true;
                    for t in targets {
                        let mut np = t.clone();
                        np.extend_from_slice(&path[1..]);
                        next.push((name.clone(), np));
                    }
                }
            }
            if next.len() > MAX_ALIAS_ENTRIES || next.iter().any(|(_, p)| p.len() > MAX_ALIAS_PATH)
            {
                return Err("alias expansion grew without bound".into());
            }
            next.sort();
            next.dedup();
            entries = next;
            if !changed {
                converged = true;
                break;
            }
        }
        if !converged {
            return Err("alias cycle: use-bindings did not converge".into());
        }

        let mut bindings: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for (n, p) in entries {
            bindings.entry(n).or_default().push(p);
        }

        // Normalise glob heads the same way; keep only globs that can name std surfaces.
        let mut norm_globs = Vec::new();
        for g in globs {
            let expanded: Vec<Vec<String>> = match g.first().and_then(|h| bindings.get(h)) {
                Some(ts) => ts
                    .iter()
                    .map(|t| {
                        let mut np = t.clone();
                        np.extend_from_slice(&g[1..]);
                        np
                    })
                    .collect(),
                None => vec![g],
            };
            for e in expanded {
                if let Some(h) = e.first() {
                    let relevant = is_std_root(h)
                        || SHORTHAND_HEADS.contains(&h.as_str())
                        || !matches!(
                            classify_resolved(h, false),
                            Classification::AllowedPure
                        );
                    if relevant && !is_local_head(h) {
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
        })
    }

    /// All spellings of `segs` worth classifying; the bool marks glob-derived ones.
    fn candidates(&self, segs: &[String]) -> Vec<(Vec<String>, bool)> {
        let mut out: Vec<(Vec<String>, bool)> = vec![(segs.to_vec(), false)];
        for (i, s) in segs.iter().enumerate() {
            if let Some(targets) = self.bindings.get(s) {
                for t in targets {
                    let mut c = t.clone();
                    c.extend_from_slice(&segs[i + 1..]);
                    out.push((c, false));
                }
            }
        }
        let rooted = is_std_root(&segs[0]) || is_local_head(&segs[0]);
        if !rooted {
            for g in &self.globs {
                let mut c = g.clone();
                c.extend_from_slice(segs);
                out.push((c, true));
            }
        }
        out.sort();
        out.dedup();
        out
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
        for (cand, from_glob) in res.candidates(segs) {
            let refs: Vec<&str> = cand.iter().map(String::as_str).collect();
            let cls = crate::rules::classify_segments(&refs, is_macro);
            let evidence = if cand.as_slice() == segs {
                written.clone()
            } else {
                format!("{written} => {}", cand.join("::"))
            };
            match cls {
                Classification::Capability { category, action } => {
                    self.detection(line, col, category, action, evidence)
                }
                Classification::BannedConstruct { construct } => {
                    self.banned(line, col, construct, evidence)
                }
                Classification::Unanalyzable { reason } if !from_glob => {
                    self.unanalyzable(line, col, reason, evidence)
                }
                _ => {}
            }
        }
    }
}

/// Index of the token matching the opener at `open` (any of `([{`), or `toks.len()`.
fn matching_close(toks: &[Token], open: usize) -> usize {
    let mut depth = 0usize;
    for (k, t) in toks.iter().enumerate().skip(open) {
        if let Tok::Punct(c) = t.tok {
            match c {
                '(' | '[' | '{' => depth += 1,
                ')' | ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return k;
                    }
                }
                _ => {}
            }
        }
    }
    toks.len()
}

/// Flag metavariables spliced next to `::` inside a `macro_rules!` body.
fn check_macro_body(sink: &mut Sink, toks: &[Token], open: usize, close: usize) {
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
                    let c = matching_close(toks, o).min(toks.len());
                    let spliced_name = is_punct(toks, o + 1, '$');
                    let cfg_attr_splice = (o + 1..c).any(|x| ident_at(toks, x) == Some(("cfg_attr", false)))
                        && (o + 1..c).any(|x| is_punct(toks, x, '$'));
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
            Tok::PathSep => {
                let next_is_meta = is_punct(toks, k + 1, '$') && ident_at(toks, k + 2) != Some(("crate", false));
                let prev_is_meta = k >= 2
                    && ident_at(toks, k - 1).is_some()
                    && is_punct(toks, k - 2, '$')
                    && ident_at(toks, k - 1) != Some(("crate", false));
                let prev_is_repeat = k >= 1 && closed_at == k - 1 && last_closed_repeat;
                if next_is_meta || prev_is_meta || prev_is_repeat {
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
fn check_attribute(sink: &mut Sink, toks: &[Token], hash: usize) {
    let mut open = hash + 1;
    if is_punct(toks, open, '!') {
        open += 1;
    }
    if !is_punct(toks, open, '[') {
        return;
    }
    let close = matching_close(toks, open);
    let (line, col) = (toks[hash].line, toks[hash].col);
    let mut seen_attr: HashSet<String> = HashSet::new();
    let end = close.min(toks.len());
    for k in open + 1..end {
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
    }
}

fn stage2_file(
    toks: &[Token],
    facts: &FileFacts,
    res: &Resolver,
    sink: &mut Sink,
) {
    // Use leaves (declaration sites).
    for l in &facts.leaves {
        if l.path.is_empty() {
            continue;
        }
        sink.path(res, &l.path, false, l.line, l.col);
    }
    for &s in &facts.use_errors {
        sink.unanalyzable(
            toks[s].line,
            toks[s].col,
            "unparseable use declaration",
            "use ...".into(),
        );
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
                check_attribute(sink, toks, i);
                i += 1;
            }
            Tok::Ident { name, raw } => {
                let after_dot = i > 0
                    && is_punct(toks, i - 1, '.')
                    && !(i > 1 && is_punct(toks, i - 2, '.'));
                if after_dot {
                    if is_punct(toks, i + 1, '(') && FS_LIST_METHODS.contains(&name.as_str()) {
                        sink.detection(t.line, t.col, "fs", "list", format!(".{name}()"));
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
                                let close = matching_close(toks, open);
                                check_macro_body(sink, toks, open, close);
                            }
                            i += 1;
                            continue;
                        }
                        _ => {}
                    }
                }
                // Read a whole path: seg (:: seg)*
                let mut segs = vec![name.clone()];
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
                sink.path(res, &segs, is_macro, t.line, t.col);
                i = if is_macro { j + 1 } else { j };
            }
            _ => i += 1,
        }
    }
}

/// `extern` at token `i`: FFI detections (VER08a §3.4).
fn extern_item(sink: &mut Sink, toks: &[Token], i: usize) {
    let (line, col) = (toks[i].line, toks[i].col);
    match toks.get(i + 1).map(|t| &t.tok) {
        Some(Tok::Str(abi)) => {
            if abi != "Rust" {
                sink.detection(line, col, "ffi", "call", format!("extern \"{abi}\""));
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
pub fn scan_sources(files: &[(&str, &str)]) -> Result<ScanResult, String> {
    let mut lexed: Vec<(&str, Vec<Token>)> = Vec::with_capacity(files.len());
    for (name, content) in files {
        let toks = lex(content).map_err(|e| format!("{name}: {e}"))?;
        lexed.push((name, toks));
    }

    let facts: Vec<FileFacts> = lexed.iter().map(|(_, t)| stage1_file(t)).collect();
    let all_leaves: Vec<UseLeaf> = facts.iter().flat_map(|f| f.leaves.iter().cloned()).collect();

    let mut result = ScanResult::default();
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

    for ((name, toks), f) in lexed.iter().zip(facts.iter()) {
        let mut sink = Sink {
            file: name,
            result: &mut result,
            seen: HashSet::new(),
        };
        stage2_file(toks, f, &resolver, &mut sink);
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
    let toks = lex(content)?;
    let chars: Vec<char> = content.chars().collect();
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
