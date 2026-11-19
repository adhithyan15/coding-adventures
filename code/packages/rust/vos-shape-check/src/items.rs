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
//!
//! ## Enum variants (VOS-039b-2a)
//!
//! SH19 (the command-type check) and CK10 (`vos-link check-contract`) need the
//! *variants* of an `enum`.  They are read by the same token-tree scan, with the
//! same bracket table, not by a second parser.  The body of an enum is a list of
//! variants separated by commas at depth 0; each variant is, in order:
//!
//! ```text
//!   #[attr]*   Name   ( T1, T2 )      a tuple variant   (fields named "0", "1", ..)
//!   #[attr]*   Name   { a: T1, b: T2 } a named-field variant
//!   #[attr]*   Name                    a unit variant
//!   #[attr]*   Name   = <expr>         a unit variant with a discriminant
//! ```
//!
//! A variant that does not fit one of these shapes is a [`Gap`] (reported as
//! VT03, fail closed): the scan never silently drops a variant it could not read.

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

/// A variant of an `enum` (VOS-039b-2a).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variant {
    /// The constructor name.
    pub name: String,
    /// Line of the name.
    pub line: u32,
    /// Written with braces (`V { a: u8 }`), as opposed to parentheses or nothing.
    pub named: bool,
    /// Its fields in order (a tuple variant's are named `0`, `1`, ..).
    pub fields: Vec<Field>,
    /// Carries `#[non_exhaustive]`.
    pub non_exhaustive: bool,
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
    /// Fields of a struct or union, named or (for a tuple struct) by position;
    /// empty for enums and aliases.
    pub fields: Vec<Field>,
    /// The variants of an `enum`, in source order (empty for every other item).
    pub variants: Vec<Variant>,
    /// Written with a generic list (`struct S<T>`), whether or not it could be read.
    pub generic: bool,
    /// Carries `#[non_exhaustive]`.
    pub non_exhaustive: bool,
    /// A field or variant list had an entry the scan could not read, so
    /// `fields` or `variants` is NOT the whole truth.  (For an enum this is also
    /// a [`Gap`]; a struct is only judged by SH19, which fails closed on it.)
    pub malformed: bool,
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

/// Parse `{ name: ty, ... }` between `open` and `close`, and say whether an
/// entry was skipped because it was not a `name: type` pair (the fields returned
/// are then incomplete).
fn parse_fields_checked(f: &FileCtx, open: usize, close: usize) -> (Vec<Field>, bool) {
    let t = &f.toks;
    let mut out = Vec::new();
    let mut skipped = false;
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
            skipped = true;
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
    (out, skipped)
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

/// Is the attribute group `#[ .. ]` whose `[` is at `open` a `non_exhaustive`?
/// Any identifier of that name inside it counts (`cfg_attr(.., non_exhaustive)`
/// included): the safe direction for SH19.
fn names_non_exhaustive(f: &FileCtx, open: usize) -> bool {
    f.toks[open + 1..f.pair[open]]
        .iter()
        .any(|x| x.kind == Kind::Ident && x.text == "non_exhaustive")
}

/// The index just past the next top-level comma at or after `j` (or `close`):
/// groups are skipped whole, so a comma inside `(..)` does not end the entry.
fn skip_entry(f: &FileCtx, mut j: usize, close: usize) -> usize {
    let t = &f.toks;
    while j < close {
        crate::steps::tick();
        if t[j].kind == Kind::Punct {
            match t[j].text.as_str() {
                "(" | "[" | "{" => j = f.pair[j],
                "," => return j + 1,
                _ => {}
            }
        }
        j += 1;
    }
    close
}

/// Read the variants of the enum body `{ .. }` from `open` to `close`.  Returns
/// the variants and the `(line, what)` of every entry that could not be read.
fn parse_variants(f: &FileCtx, open: usize, close: usize) -> (Vec<Variant>, Vec<(u32, String)>) {
    let t = &f.toks;
    let mut out = Vec::new();
    let mut bad: Vec<(u32, String)> = Vec::new();
    let mut j = open + 1;
    while j < close {
        crate::steps::tick();
        let mut non_exhaustive = false;
        while j < close && t[j].is_punct("#") && t.get(j + 1).is_some_and(|x| x.is_punct("[")) {
            crate::steps::tick();
            non_exhaustive |= names_non_exhaustive(f, j + 1);
            j = f.pair[j + 1] + 1;
        }
        if j >= close {
            break;
        }
        if t[j].kind != Kind::Ident {
            bad.push((
                t[j].line,
                "a variant that does not start with a name".into(),
            ));
            j = skip_entry(f, j, close);
            continue;
        }
        let name = t[j].text.clone();
        let line = t[j].line;
        j += 1;
        let mut named = false;
        let mut fields = Vec::new();
        let mut malformed = false;
        if j < close && t[j].is_punct("(") {
            fields = parse_tuple_fields(f, j, f.pair[j]);
            j = f.pair[j] + 1;
        } else if j < close && t[j].is_punct("{") {
            named = true;
            (fields, malformed) = parse_fields_checked(f, j, f.pair[j]);
            j = f.pair[j] + 1;
        }
        if malformed {
            bad.push((line, format!("the fields of variant `{name}`")));
        }
        // An optional discriminant `= expr`, then the comma or the end.
        if j < close && t[j].is_punct("=") {
            j = skip_entry(f, j, close);
        } else if j < close && t[j].is_punct(",") {
            j += 1;
        } else if j < close {
            bad.push((line, format!("the tokens after variant `{name}`")));
            j = skip_entry(f, j, close);
        }
        out.push(Variant {
            name,
            line,
            named,
            fields,
            non_exhaustive,
        });
    }
    (out, bad)
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
            let generic = t.get(k).is_some_and(|x| x.is_punct("<"));
            let non_exhaustive = f
                .attrs_before(i)
                .iter()
                .any(|a| names_non_exhaustive(f, a.open));
            if generic {
                match angle_close(t, &f.pair, k) {
                    Some(c) => k = c + 1,
                    None => {
                        gap(t[k].line, "the generic list");
                        readable = false;
                    }
                }
            }
            let mut fields = Vec::new();
            let mut variants = Vec::new();
            let mut malformed = !readable;
            let mut end = (i + 1).min(t.len() - 1);
            if readable {
                if keyword != "enum"
                    && keyword != "type"
                    && t.get(k).is_some_and(|x| x.is_punct("("))
                {
                    let close = f.pair[k];
                    fields = parse_tuple_fields(f, k, close);
                    malformed |= fields.iter().any(|x| x.ty.is_empty());
                    end = match scan_header(t, &f.pair, close + 1) {
                        Header::End(e) => e,
                        Header::GaveUp(g) => {
                            gap(t[k].line, "the where clause");
                            malformed = true;
                            g.saturating_sub(1).max(close)
                        }
                    };
                } else {
                    match scan_header(t, &f.pair, k) {
                        Header::End(b) if t[b].is_punct("{") => {
                            if keyword == "struct" || keyword == "union" {
                                let (fs, skipped) = parse_fields_checked(f, b, f.pair[b]);
                                fields = fs;
                                malformed |= skipped;
                            } else if keyword == "enum" {
                                let (vs, bad) = parse_variants(f, b, f.pair[b]);
                                variants = vs;
                                malformed |= !bad.is_empty();
                                for (line, what) in bad {
                                    gap(line, &format!("{what} (an enum body)"));
                                }
                            }
                            end = f.pair[b];
                        }
                        Header::End(b) => end = b,
                        Header::GaveUp(g) => {
                            gap(t[i].line, "the header");
                            malformed = true;
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
                variants,
                generic,
                non_exhaustive,
                malformed,
                idents,
            });
        }
    }
    (out, gaps)
}
