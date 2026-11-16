//! # Just enough structure over the tokens
//!
//! We do not parse Rust.  A parser would be a second thing to trust.  What the
//! rules need is a handful of structural facts that a token stream plus
//! bracket matching can give reliably:
//!
//! * which token closes which `(`, `[`, `{` (`pair`);
//! * which tokens sit inside an attribute `#[...]` (those are examined by the
//!   attribute rules only, never as code);
//! * where an item starts and ends (`item_end`), used to cut out
//!   `#[cfg(test)]` items and to bound `struct` and `const` definitions;
//! * where the `ghost` module is.
//!
//! ## The `#[cfg(test)]` mask
//!
//! An item carrying exactly `#[cfg(test)]` is removed before any rule runs.
//! It is compiled only by `cargo test`, never into an image and never into
//! the evidence library (VOS03 section 1).  This is a trust decision, stated
//! in the spec, and it is the one place the check chooses not to look.

use crate::lexer::{lex, pair_brackets, Kind, LexError, Token};

/// An attribute `#[...]` or `#![...]`, as token indices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attr {
    /// Index of the `#`.
    pub start: usize,
    /// Index of the `[`.
    pub open: usize,
    /// Index of the `]`.
    pub close: usize,
    /// `#!` rather than `#`.
    pub inner: bool,
}

/// An inline or out-of-line `mod ghost`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GhostMod {
    /// Index of the `mod` keyword.
    pub mod_idx: usize,
    /// For `mod ghost { .. }`, the token range inside the braces.
    pub body: Option<(usize, usize)>,
}

/// One source file, lexed and indexed.
#[derive(Debug)]
pub struct FileCtx {
    /// Crate-relative path, `/`-separated.
    pub path: String,
    /// Tokens after the `cfg(test)` mask.
    pub toks: Vec<Token>,
    /// Bracket partners (see [`pair_brackets`]).
    pub pair: Vec<usize>,
    /// True for tokens inside an attribute.
    pub in_attr: Vec<bool>,
    /// Every attribute.
    pub attrs: Vec<Attr>,
    /// True if the whole file is the ghost module (`src/ghost.rs`,
    /// `src/ghost/**`).
    pub ghost_file: bool,
    /// Every `mod ghost` item.
    pub ghost_mods: Vec<GhostMod>,
}

/// Rust keywords (strict and reserved), used to tell `name[` from `in [`.
pub fn is_keyword(s: &str) -> bool {
    matches!(
        s,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
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
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
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
            | "try"
    )
}

/// Find every attribute in `toks`.
pub fn find_attrs(toks: &[Token], pair: &[usize]) -> Vec<Attr> {
    let mut out = Vec::new();
    for i in 0..toks.len() {
        if !toks[i].is_punct("#") {
            continue;
        }
        let inner = toks.get(i + 1).is_some_and(|t| t.is_punct("!"));
        let open = i + 1 + usize::from(inner);
        if toks.get(open).is_some_and(|t| t.is_punct("[")) {
            out.push(Attr {
                start: i,
                open,
                close: pair[open],
                inner,
            });
        }
    }
    out
}

/// Index of the last token of the item that starts at `start`.
///
/// An item ends at its first `;` or `,` at bracket depth 0, or after the
/// first `{ .. }` group (plus a trailing `;`), or just before the closing
/// bracket of the enclosing group.  Groups `( )` and `[ ]` are skipped whole,
/// so `[u8; 4]` does not end it.
pub fn item_end(toks: &[Token], pair: &[usize], start: usize) -> usize {
    let mut k = start;
    while k < toks.len() {
        let t = &toks[k];
        if t.kind == Kind::Punct {
            match t.text.as_str() {
                "(" | "[" => {
                    k = pair[k] + 1;
                    continue;
                }
                "{" => {
                    let e = pair[k];
                    return if toks.get(e + 1).is_some_and(|t| t.is_punct(";")) {
                        e + 1
                    } else {
                        e
                    };
                }
                ";" | "," => return k,
                ")" | "]" | "}" => return k.saturating_sub(1).max(start),
                _ => {}
            }
        }
        k += 1;
    }
    toks.len().saturating_sub(1)
}

/// True if the attribute is exactly `cfg(test)`.
fn is_cfg_test(toks: &[Token], a: &Attr) -> bool {
    let inner = &toks[a.open + 1..a.close];
    inner.len() == 4
        && inner[0].is_ident("cfg")
        && inner[1].is_punct("(")
        && inner[2].is_ident("test")
        && inner[3].is_punct(")")
}

/// Remove every item annotated `#[cfg(test)]` (the attribute included).
fn mask_cfg_test(toks: Vec<Token>, pair: &[usize]) -> Vec<Token> {
    let attrs = find_attrs(&toks, pair);
    let mut drop = vec![false; toks.len()];
    for a in &attrs {
        if a.inner || !is_cfg_test(&toks, a) {
            continue;
        }
        // Skip any further attributes between this one and the item.
        let mut j = a.close + 1;
        while toks.get(j).is_some_and(|t| t.is_punct("#"))
            && toks.get(j + 1).is_some_and(|t| t.is_punct("["))
        {
            j = pair[j + 1] + 1;
        }
        let end = if j < toks.len() {
            item_end(&toks, pair, j)
        } else {
            toks.len() - 1
        };
        for d in &mut drop[a.start..=end.max(a.close)] {
            *d = true;
        }
    }
    toks.into_iter()
        .zip(drop)
        .filter_map(|(t, d)| (!d).then_some(t))
        .collect()
}

impl FileCtx {
    /// Lex and index one file.
    pub fn load(path: &str, src: &str) -> Result<FileCtx, LexError> {
        let raw = lex(src)?;
        let pair0 = pair_brackets(&raw)?;
        let toks = mask_cfg_test(raw, &pair0);
        let pair = pair_brackets(&toks)?;
        let attrs = find_attrs(&toks, &pair);
        let mut in_attr = vec![false; toks.len()];
        for a in &attrs {
            for x in &mut in_attr[a.start..=a.close] {
                *x = true;
            }
        }
        let mut ghost_mods = Vec::new();
        for i in 0..toks.len() {
            if in_attr[i] || !toks[i].is_ident("mod") {
                continue;
            }
            if toks.get(i + 1).is_some_and(|t| t.is_ident("ghost")) {
                let body = match toks.get(i + 2) {
                    Some(t) if t.is_punct("{") => Some((i + 3, pair[i + 2])),
                    _ => None,
                };
                ghost_mods.push(GhostMod { mod_idx: i, body });
            }
        }
        let ghost_file = path == "src/ghost.rs" || path.starts_with("src/ghost/");
        Ok(FileCtx {
            path: path.to_string(),
            toks,
            pair,
            in_attr,
            attrs,
            ghost_file,
            ghost_mods,
        })
    }

    /// True if token `i` is inside the ghost module.
    pub fn in_ghost(&self, i: usize) -> bool {
        self.ghost_file
            || self
                .ghost_mods
                .iter()
                .any(|g| g.body.is_some_and(|(a, b)| i >= a && i < b))
    }

    /// The attributes attached to the item whose first keyword is at `idx`
    /// (walking backwards over `pub`, `pub(..)` and attribute groups).
    pub fn attrs_before(&self, idx: usize) -> Vec<Attr> {
        let mut out = Vec::new();
        let mut j = idx;
        // visibility
        if j >= 1 && self.toks[j - 1].is_punct(")") {
            let open = self.pair[j - 1];
            if open >= 1 && self.toks[open - 1].is_ident("pub") {
                j = open - 1;
            }
        } else if j >= 1 && self.toks[j - 1].is_ident("pub") {
            j -= 1;
        }
        while j >= 1 && self.toks[j - 1].is_punct("]") {
            let open = self.pair[j - 1];
            if open >= 1 && self.toks[open - 1].is_punct("#") {
                if let Some(a) = self.attrs.iter().find(|a| a.open == open) {
                    out.push(*a);
                }
                j = open - 1;
            } else {
                break;
            }
        }
        out.reverse();
        out
    }

    /// The tokens strictly inside an attribute's brackets.
    pub fn attr_body(&self, a: &Attr) -> &[Token] {
        &self.toks[a.open + 1..a.close]
    }
}

/// Given the index of a `<`, the index of its matching `>` (not counting the
/// `>` of `->` or `=>`, which the lexer merges), or `None` when the group
/// is not a plausible generic list (a `;`, `{` or `}` at depth 1, or more
/// than 4,096 tokens).
pub fn angle_close(toks: &[Token], pair: &[usize], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut k = open;
    let limit = (open + 4_096).min(toks.len());
    while k < limit {
        let t = &toks[k];
        if t.kind == Kind::Punct {
            match t.text.as_str() {
                "<" => depth += 1,
                ">" => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(k);
                    }
                }
                "(" | "[" => {
                    k = pair[k];
                }
                ";" | "{" | "}" => return None,
                _ => {}
            }
        }
        k += 1;
    }
    None
}

/// A `const`-style name: upper-case letters, digits and `_`, at least one
/// letter (`LEN`, `N`, `MAX_8`).
pub fn is_const_name(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_uppercase())
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}
