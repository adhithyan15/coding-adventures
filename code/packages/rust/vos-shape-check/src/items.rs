//! # Type definitions and functions, found lexically
//!
//! Several rules need to know what the crate *defines*:
//!
//! * SH3 asks whether a `const`'s type is a struct that hides a `Cell`;
//! * SH14 asks whether a component's `struct` has a `pub` field;
//! * SH17 compares the ghost `Image` against the production `State`;
//! * SH5 builds a call graph over `fn` items.
//!
//! A `struct` is found as the identifier `struct` followed by a name; its
//! fields are the comma-separated `name: type` pairs in the braces.  A comma
//! inside `HashMap<A, B>` must not split a field, so commas are counted only at
//! angle-bracket depth 0 (the `>` of `->` is a separate merged token and does
//! not count).

use crate::lexer::{Kind, Token};
use crate::syntax::{angle_close, FileCtx};

/// A field of a named-field struct.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Type as space-joined token text (for equality, not display).
    pub ty: String,
    /// Written as a bare `pub` (not `pub(crate)`).
    pub public: bool,
    /// Line of the field name.
    pub line: u32,
}

/// A `struct`, `enum`, `union` or `type` item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeDef {
    /// `struct`, `enum`, `union` or `type`.
    pub keyword: &'static str,
    /// Its name.
    pub name: String,
    /// Index into the file list.
    pub file: usize,
    /// Line of the name.
    pub line: u32,
    /// Defined inside the ghost module.
    pub in_ghost: bool,
    /// Named fields (empty for enums, tuple structs, aliases).
    pub fields: Vec<Field>,
    /// Every identifier written in the item (for the interior-mutability
    /// closure of SH3).
    pub idents: Vec<String>,
}

fn join(toks: &[Token]) -> String {
    let mut s = String::new();
    for t in toks {
        if !s.is_empty() {
            s.push(' ');
        }
        s.push_str(&t.text);
    }
    s
}

/// Parse `{ name: ty, ... }` between `open` and `close`.
fn parse_fields(f: &FileCtx, open: usize, close: usize) -> Vec<Field> {
    let t = &f.toks;
    let mut out = Vec::new();
    let mut j = open + 1;
    while j < close {
        // attributes
        while j < close && t[j].is_punct("#") && t.get(j + 1).is_some_and(|x| x.is_punct("[")) {
            j = f.pair[j + 1] + 1;
        }
        if j >= close {
            break;
        }
        let mut public = false;
        if t[j].is_ident("pub") {
            if t.get(j + 1).is_some_and(|x| x.is_punct("(")) {
                j = f.pair[j + 1] + 1;
            } else {
                public = true;
                j += 1;
            }
        }
        if j + 1 < close && t[j].kind == Kind::Ident && t[j + 1].is_punct(":") {
            let name = t[j].text.clone();
            let line = t[j].line;
            let ty_start = j + 2;
            let mut k = ty_start;
            let mut depth = 0usize;
            while k < close {
                let x = &t[k];
                if x.kind == Kind::Punct {
                    match x.text.as_str() {
                        "(" | "[" | "{" => {
                            k = f.pair[k];
                        }
                        "<" => depth += 1,
                        ">" => depth = depth.saturating_sub(1),
                        "," if depth == 0 => break,
                        _ => {}
                    }
                }
                k += 1;
            }
            out.push(Field {
                name,
                ty: join(&t[ty_start..k.min(close)]),
                public,
                line,
            });
            j = k + 1;
        } else {
            // Not a named field (a tuple-like or malformed list): skip to the
            // next top-level comma so one oddity does not hide the rest.
            while j < close && !t[j].is_punct(",") {
                if matches!(t[j].text.as_str(), "(" | "[" | "{") && t[j].kind == Kind::Punct {
                    j = f.pair[j];
                }
                j += 1;
            }
            j += 1;
        }
    }
    out
}

/// Most identifiers stored per item (bounds memory on adversarial input).
const MAX_IDENTS_PER_ITEM: usize = 1024;
/// Most tokens one item's extent may cover.
const MAX_ITEM_TOKENS: usize = 4096;

/// Keywords that begin another item: the extent of a type definition ends
/// before one, so `type a type a type a ...` costs two tokens per item and
/// not the rest of the file (a quadratic scan found by the security review).
fn begins_item(t: &Token) -> bool {
    t.kind == Kind::Ident
        && matches!(
            t.text.as_str(),
            "struct"
                | "enum"
                | "union"
                | "type"
                | "fn"
                | "impl"
                | "trait"
                | "mod"
                | "use"
                | "static"
                | "const"
        )
}

/// The last token of the type definition at `start`: the first `;` or `,` at
/// depth 0, or the end of the first `{ .. }` group (and a trailing `;`), but
/// never past the next item keyword or [`MAX_ITEM_TOKENS`] tokens.
fn type_extent(f: &FileCtx, start: usize) -> usize {
    let t = &f.toks;
    let limit = (start + MAX_ITEM_TOKENS).min(t.len());
    let mut k = start + 1;
    while k < limit {
        if begins_item(&t[k]) {
            return k - 1;
        }
        if t[k].kind == Kind::Punct {
            match t[k].text.as_str() {
                "(" | "[" => {
                    k = f.pair[k] + 1;
                    continue;
                }
                "{" => {
                    let e = f.pair[k];
                    return if t.get(e + 1).is_some_and(|x| x.is_punct(";")) {
                        e + 1
                    } else {
                        e
                    };
                }
                ";" | "," => return k,
                ")" | "]" | "}" => return k - 1,
                _ => {}
            }
        }
        k += 1;
    }
    limit.saturating_sub(1)
}

/// `struct S(pub u8, u8);`: the fields of a tuple body, named by position.
fn parse_tuple_fields(f: &FileCtx, open: usize, close: usize) -> Vec<Field> {
    let t = &f.toks;
    let mut out = Vec::new();
    let mut j = open + 1;
    let mut index = 0usize;
    while j < close {
        while j < close && t[j].is_punct("#") && t.get(j + 1).is_some_and(|x| x.is_punct("[")) {
            j = f.pair[j + 1] + 1;
        }
        if j >= close {
            break;
        }
        let line = t[j].line;
        let mut public = false;
        if t[j].is_ident("pub") {
            if t.get(j + 1).is_some_and(|x| x.is_punct("(")) {
                j = f.pair[j + 1] + 1;
            } else {
                public = true;
                j += 1;
            }
        }
        let ty_start = j;
        let mut depth = 0usize;
        while j < close {
            if t[j].kind == Kind::Punct {
                match t[j].text.as_str() {
                    "(" | "[" | "{" => j = f.pair[j],
                    "<" => depth += 1,
                    ">" => depth = depth.saturating_sub(1),
                    "," if depth == 0 => break,
                    _ => {}
                }
            }
            j += 1;
        }
        out.push(Field {
            name: index.to_string(),
            ty: join(&t[ty_start..j.min(close)]),
            public,
            line,
        });
        index += 1;
        j += 1;
    }
    out
}

/// Collect every `struct`, `enum`, `union` and `type` of every file.
pub fn collect_types(files: &[FileCtx]) -> Vec<TypeDef> {
    let mut out = Vec::new();
    for (fi, f) in files.iter().enumerate() {
        let t = &f.toks;
        for i in 0..t.len() {
            if f.in_attr[i] || t[i].kind != Kind::Ident {
                continue;
            }
            let keyword = match t[i].text.as_str() {
                "struct" => "struct",
                "enum" => "enum",
                "union" => "union",
                "type" => "type",
                _ => continue,
            };
            if i > 0 && t[i - 1].is_punct(".") {
                continue;
            }
            let Some(name_tok) = t.get(i + 1).filter(|x| x.kind == Kind::Ident) else {
                continue;
            };
            let end = type_extent(f, i);
            let idents: Vec<String> = if end >= i + 2 {
                t[i + 2..=end]
                    .iter()
                    .filter(|x| x.kind == Kind::Ident)
                    .take(MAX_IDENTS_PER_ITEM)
                    .map(|x| x.text.clone())
                    .collect()
            } else {
                Vec::new()
            };
            let mut fields = Vec::new();
            if keyword == "struct" || keyword == "union" {
                let mut k = i + 2;
                if t.get(k).is_some_and(|x| x.is_punct("<")) {
                    if let Some(c) = angle_close(t, &f.pair, k) {
                        k = c + 1;
                    }
                }
                while k <= end && !t[k].is_punct("{") && !t[k].is_punct(";") && !t[k].is_punct("(")
                {
                    k += 1;
                }
                if k <= end && t[k].is_punct("{") {
                    fields = parse_fields(f, k, f.pair[k]);
                } else if k <= end && t[k].is_punct("(") {
                    fields = parse_tuple_fields(f, k, f.pair[k]);
                }
            }
            out.push(TypeDef {
                keyword,
                name: name_tok.text.clone(),
                file: fi,
                line: name_tok.line,
                in_ghost: f.in_ghost(i),
                fields,
                idents,
            });
        }
    }
    out
}
