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
    /// True for tokens inside an inline `mod ghost { .. }` body.
    ghost_mask: Vec<bool>,
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

/// Find every attribute in `toks`, outermost only: a `#[` that sits inside
/// another attribute's brackets is part of that attribute's body (so 40,000
/// nested `#[` cost one attribute, not a quadratic number of ranges).
pub fn find_attrs(toks: &[Token], pair: &[usize]) -> Vec<Attr> {
    let mut out: Vec<Attr> = Vec::new();
    let mut next_free = 0usize;
    for i in 0..toks.len() {
        if i < next_free || !toks[i].is_punct("#") {
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
            next_free = pair[open] + 1;
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
    // Attributes that start inside an already-masked region are skipped, so
    // the whole pass is linear however many `#[cfg(test)]` there are.
    let mut masked_to = 0usize;
    for a in &attrs {
        if a.start < masked_to || a.inner || !is_cfg_test(&toks, a) {
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
        let stop = end.max(a.close);
        for d in &mut drop[a.start..=stop] {
            *d = true;
        }
        masked_to = stop + 1;
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
        let mut ghost_mask = vec![false; toks.len()];
        // Mods are in token order and nest or are disjoint, so marking from the
        // highest point reached keeps this linear however deep they nest.
        let mut hi = 0usize;
        for g in &ghost_mods {
            if let Some((a, b)) = g.body {
                let from = a.max(hi);
                if b > from {
                    for m in &mut ghost_mask[from..b] {
                        *m = true;
                    }
                    hi = b;
                }
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
            ghost_mask,
        })
    }

    /// True if token `i` is inside the ghost module.
    pub fn in_ghost(&self, i: usize) -> bool {
        self.ghost_file || self.ghost_mask.get(i).copied().unwrap_or(false)
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
                if let Ok(n) = self.attrs.binary_search_by_key(&open, |a| a.open) {
                    out.push(self.attrs[n]);
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
/// than 256 tokens).
pub fn angle_close(toks: &[Token], pair: &[usize], open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut k = open;
    let limit = (open + 256).min(toks.len());
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

/// The index of the `<` matching the `>` at `gt`, scanning backwards (at most
/// 256 tokens).  Used for `<A as B>::f` and `Foo::<u8>::f` qualifiers.
pub fn matching_lt(t: &[Token], gt: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut k = gt;
    let floor = gt.saturating_sub(256);
    loop {
        if t[k].is_punct(">") {
            depth += 1;
        } else if t[k].is_punct("<") {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(k);
            }
        }
        if k == floor {
            return None;
        }
        k -= 1;
    }
}

/// The farthest a header scan (from `fn`, `impl`, `trait`, `struct` to its
/// body) may travel, in tokens.  Real headers are far shorter; the bound keeps
/// a file of 100,000 unterminated `fn f` from costing a scan to the end of the
/// file each (a quadratic found by the security review).
pub const MAX_HEADER_TOKENS: usize = 1024;

/// Scan forward from `from` to the token that ends an item header: the first
/// `{` or `;` at bracket depth 0 (groups `( )` and `[ ]` are skipped whole),
/// stopping early at a keyword that always starts a NEW item (`struct`, `enum`,
/// `union`, `trait`, `mod`, `use`) or after [`MAX_HEADER_TOKENS`].  The result
/// is the index of that token (callers test `is_punct("{")`), or the index
/// where the scan gave up.
pub fn scan_to_body(t: &[Token], pair: &[usize], from: usize) -> usize {
    let limit = (from + MAX_HEADER_TOKENS).min(t.len());
    let mut k = from;
    while k < limit {
        let x = &t[k];
        match x.kind {
            Kind::Punct => match x.text.as_str() {
                "{" | ";" => return k,
                "(" | "[" => k = pair[k],
                _ => {}
            },
            Kind::Ident
                if matches!(
                    x.text.as_str(),
                    "struct" | "enum" | "union" | "trait" | "mod" | "use"
                ) =>
            {
                return k;
            }
            _ => {}
        }
        k += 1;
    }
    limit
}
