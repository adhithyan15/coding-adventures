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
use crate::syntax::{angle_close, scan_header, FileCtx, Header};

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
        crate::steps::tick();
        // attributes
        while j < close && t[j].is_punct("#") && t.get(j + 1).is_some_and(|x| x.is_punct("[")) {
            crate::steps::tick();
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
                crate::steps::tick();
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
                crate::steps::tick();
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

/// A place where a structural scan gave up.  Every gap becomes a **VT03**
/// finding (fail closed): the check never skips an item it could not read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gap {
    /// Index into the file list.
    pub file: usize,
    /// 1-based line.
    pub line: u32,
    /// What could not be read.
    pub message: String,
}

/// `struct S(pub u8, u8);`: the fields of a tuple body, named by position.
fn parse_tuple_fields(f: &FileCtx, open: usize, close: usize) -> Vec<Field> {
    let t = &f.toks;
    let mut out = Vec::new();
    let mut j = open + 1;
    let mut index = 0usize;
    while j < close {
        crate::steps::tick();
        while j < close && t[j].is_punct("#") && t.get(j + 1).is_some_and(|x| x.is_punct("[")) {
            crate::steps::tick();
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
            crate::steps::tick();
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

/// Collect every `struct`, `enum`, `union` and `type` of every file, and the
/// [`Gap`]s where a header could not be read to its end.
///
/// A definition's header is read in order: the name, then a generic list
/// (skipped with [`angle_close`], so `struct B<const N: usize>` does not end at
/// the `const`), then the body or `;` (found with [`scan_header`]).  If any
/// step gives up, a gap is recorded and the identifiers read so far are kept.
pub fn collect_types(files: &[FileCtx]) -> (Vec<TypeDef>, Vec<Gap>) {
    let mut out = Vec::new();
    let mut gaps = Vec::new();
    for (fi, f) in files.iter().enumerate() {
        let t = &f.toks;
        for i in 0..t.len() {
            crate::steps::tick();
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
            let mut gap = |line: u32, what: &str| {
                gaps.push(Gap {
                    file: fi,
                    line,
                    message: format!(
                        "{what} of `{keyword} {}` could not be read (fail closed)",
                        name_tok.text
                    ),
                });
            };
            let mut k = i + 2;
            let mut readable = true;
            if t.get(k).is_some_and(|x| x.is_punct("<")) {
                match angle_close(t, &f.pair, k) {
                    Some(c) => k = c + 1,
                    None => {
                        gap(t[k].line, "the generic list");
                        readable = false;
                    }
                }
            }
            let mut fields = Vec::new();
            let mut end = (i + 1).min(t.len() - 1);
            if readable {
                if keyword != "enum"
                    && keyword != "type"
                    && t.get(k).is_some_and(|x| x.is_punct("("))
                {
                    let close = f.pair[k];
                    fields = parse_tuple_fields(f, k, close);
                    end = match scan_header(t, &f.pair, close + 1) {
                        Header::End(e) => e,
                        Header::GaveUp(g) => {
                            gap(t[k].line, "the where clause");
                            g.saturating_sub(1).max(close)
                        }
                    };
                } else {
                    match scan_header(t, &f.pair, k) {
                        Header::End(b) if t[b].is_punct("{") => {
                            if keyword == "struct" || keyword == "union" {
                                fields = parse_fields(f, b, f.pair[b]);
                            }
                            end = f.pair[b];
                        }
                        Header::End(b) => end = b,
                        Header::GaveUp(g) => {
                            gap(t[i].line, "the header");
                            end = g.saturating_sub(1).max(i + 1);
                        }
                    }
                }
            }
            let last = end.min(t.len() - 1);
            let idents: Vec<String> = if last >= i + 2 {
                t[i + 2..=last]
                    .iter()
                    .filter(|x| x.kind == Kind::Ident)
                    .take(MAX_IDENTS_PER_ITEM)
                    .map(|x| x.text.clone())
                    .collect()
            } else {
                Vec::new()
            };
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
    (out, gaps)
}
