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

/// `type NAME[<..>] = [&|*|mut|dyn|impl]* path ...;` as a binding `NAME -> path`.
fn type_alias_leaf(toks: &[Token], i: usize) -> Option<UseLeaf> {
    let (name, _) = ident_at(toks, i + 1)?;
    let mut j = i + 2;
    if is_punct(toks, j, '<') {
        j = skip_angle(toks, j)?;
    }
    if !is_punct(toks, j, '=') {
        return None;
    }
    j += 1;
    loop {
        match toks.get(j).map(|t| &t.tok) {
            Some(Tok::Punct('&' | '*')) | Some(Tok::Lifetime) => j += 1,
            Some(Tok::Ident { name, raw: false })
                if matches!(name.as_str(), "mut" | "const" | "dyn" | "impl") =>
            {
                j += 1
            }
            _ => break,
        }
    }
    if is_pathsep(toks, j) {
        j += 1;
    }
    let (line, col) = (toks.get(j)?.line, toks.get(j)?.col);
    let mut path: Vec<String> = Vec::new();
    while let Some((seg, _)) = ident_at(toks, j) {
        path.push(seg.to_string());
        if path.len() > MAX_ALIAS_PATH {
            return None;
        }
        if is_pathsep(toks, j + 1) {
            j += 2;
        } else {
            break;
        }
    }
    if path.is_empty() || (path.len() == 1 && path[0] == name) {
        return None;
    }
    Some(UseLeaf {
        path,
        alias: Some(name.to_string()),
        glob: false,
        line,
        col,
    })
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
        if let (Some(("type", false)), false) = (ident_at(toks, i), prev_is_dot_or_dollar) {
            // `type F = File;` binds `F` exactly like `use ... as F` (VER08a L1).
            // The tokens stay in the walk, so the right-hand side is classified too.
            if let Some(leaf) = type_alias_leaf(toks, i) {
                f.leaves.push(leaf);
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
/// Distinct glob imports kept (after normalisation); more is treated as an attack.
const MAX_GLOBS: usize = 256;
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
                        // Check the caps *inside* the loop: (entries x targets) can be
                        // quadratic and must never be materialised before it is refused.
                        if np.len() > MAX_ALIAS_PATH || next.len() >= MAX_ALIAS_ENTRIES {
                            return Err("alias expansion grew without bound".into());
                        }
                        next.push((name.clone(), np));
                    }
                }
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
        let mut norm_globs: Vec<Vec<String>> = Vec::new();
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
        })
    }

    fn has_binding(&self, name: &str) -> bool {
        self.bindings.contains_key(name)
    }

    /// All spellings of `segs` worth classifying; the bool marks glob-derived
    /// ones.  The second result is true if the list was truncated at
    /// [`MAX_CANDIDATES`] (the caller reports CAP003).  `segs` must be at most
    /// [`MAX_PATH_SEGS`] long, so the work here is bounded by
    /// `MAX_CANDIDATES x (MAX_ALIAS_PATH + MAX_PATH_SEGS)`.
    fn candidates(&self, segs: &[String]) -> (Vec<(Vec<String>, bool)>, bool) {
        let mut out: Vec<(Vec<String>, bool)> = vec![(segs.to_vec(), false)];
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
                    out.push((c, false));
                }
            }
        }
        let rooted = is_std_root(&segs[0]) || is_local_head(&segs[0]);
        if !rooted && !truncated {
            for g in &self.globs {
                if out.len() >= MAX_CANDIDATES {
                    truncated = true;
                    break;
                }
                let mut c = g.clone();
                c.extend_from_slice(segs);
                out.push((c, true));
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
        let mut cands = vec![(segs.to_vec(), false)];
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
                self.cand_budget = 0;
                self.unanalyzable(
                    line,
                    col,
                    "alias resolution exceeded the per-file work budget",
                    written.clone(),
                );
            } else {
                self.cand_budget -= extra;
                cands = all;
            }
        }
        for (cand, from_glob) in cands {
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

/// Flag metavariables spliced next to `::` inside a `macro_rules!` body.
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
            return local.ctors.get(ty).is_some_and(|c| c.contains(fname))
                && local.methods.get(ty).is_some_and(|m| m.contains(method))
                && !res.has_binding(ty);
        }
        return false;
    }
    false
}

/// For the `>` at `gt` followed by `::` (`<File as T>::create`, `<File>::create`,
/// `Vec::<u8>::new`): the path whose associated item is being named.
/// `Err(())` if the matching `<` cannot be found within 512 tokens.
fn qualified_self_path(toks: &[Token], gt: usize) -> Result<Option<Vec<String>>, ()> {
    let mut depth = 1usize;
    let mut open = None;
    let lo = gt.saturating_sub(512);
    let mut k = gt;
    while k > lo {
        k -= 1;
        if is_punct(toks, k, '>') && !(k > 0 && is_punct(toks, k - 1, '-')) {
            depth += 1;
        } else if is_punct(toks, k, '<') {
            depth -= 1;
            if depth == 0 {
                open = Some(k);
                break;
            }
        }
    }
    let open = open.ok_or(())?;
    // Turbofish `Path::<T>::item`: the item belongs to the path before `::<`.
    if open >= 1 && is_pathsep(toks, open - 1) {
        let mut segs: Vec<String> = Vec::new();
        let mut q = open;
        while q >= 2 && is_pathsep(toks, q - 1) {
            match ident_at(toks, q - 2) {
                Some((n, _)) => segs.push(n.to_string()),
                None => break,
            }
            q -= 2;
            if segs.len() > MAX_PATH_SEGS {
                return Err(());
            }
        }
        segs.reverse();
        return Ok(if segs.is_empty() { None } else { Some(segs) });
    }
    let mut j = open + 1;
    loop {
        match toks.get(j).map(|t| &t.tok) {
            Some(Tok::Punct('&' | '*' | '<')) | Some(Tok::Lifetime) => j += 1,
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
    Ok(if segs.is_empty() { None } else { Some(segs) })
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

fn stage2_file(
    toks: &[Token],
    br: &Brackets,
    facts: &FileFacts,
    res: &Resolver,
    local: &LocalTypes,
    sink: &mut Sink,
) {
    let in_macro_args = macro_arg_mask(toks, br);
    sink.macro_budget = toks.len().saturating_mul(4) + 1024;
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
                check_attribute(sink, toks, br, i);
                i += 1;
            }
            Tok::Ident { name, raw } => {
                let after_dot = i > 0
                    && is_punct(toks, i - 1, '.')
                    && !(i > 1 && is_punct(toks, i - 2, '.'));
                if after_dot {
                    if is_punct(toks, i + 1, '(')
                        && FS_LIST_METHODS.contains(&name.as_str())
                        && !receiver_is_local_inherent(toks, br, res, local, i - 1, name)
                    {
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
                let closes_angle = i >= 2
                    && is_pathsep(toks, i - 1)
                    && is_punct(toks, i - 2, '>')
                    && !(i >= 3 && (is_punct(toks, i - 3, '=') || is_punct(toks, i - 3, '-')));
                if closes_angle {
                    match qualified_self_path(toks, i - 2) {
                        Ok(Some(prefix)) => qualified_prefix = prefix,
                        Ok(None) => {}
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
                if !qualified_prefix.is_empty() {
                    let mut q = qualified_prefix;
                    q.extend(segs);
                    sink.path(res, &q, is_macro, t.line, t.col);
                }
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
pub fn scan_sources(files: &[(&str, &str)]) -> Result<ScanResult, String> {
    let mut lexed: Vec<(&str, Vec<Token>)> = Vec::with_capacity(files.len());
    for (name, content) in files {
        let toks = lex(content).map_err(|e| format!("{name}: {e}"))?;
        lexed.push((name, toks));
    }

    let facts: Vec<FileFacts> = lexed.iter().map(|(_, t)| stage1_file(t)).collect();
    let brackets: Vec<Brackets> = lexed.iter().map(|(_, t)| Brackets::build(t)).collect();
    let mut local = LocalTypes::default();
    for ((_, toks), br) in lexed.iter().zip(brackets.iter()) {
        collect_local_types(toks, br, &mut local);
    }
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

    for (((name, toks), f), br) in lexed.iter().zip(facts.iter()).zip(brackets.iter()) {
        let mut sink = Sink {
            file: name,
            result: &mut result,
            seen: HashSet::new(),
            cand_budget: CANDIDATE_BUDGET,
            macro_budget: 0,
        };
        stage2_file(toks, br, f, &resolver, &local, &mut sink);
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
