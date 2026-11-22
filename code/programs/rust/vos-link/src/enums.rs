//! # CK10: the Rust enums and the Warrant variants say the same thing (VOS-039b-2a)
//!
//! A contract crate states each command or answer type TWICE: as a Rust `enum` that the
//! implementations compile against, and as a Warrant `variant` that the claims reason about.
//!
//! ```text
//!   src/lib.rs               pub enum Pick { Idle, Run(u8) }
//!   warrant/contract.part    variant Pick = Idle | Run(t: U8)
//! ```
//!
//! If the two drift apart (a constructor renamed on one side, a payload widened, the variants
//! reordered) every claim is still *proved*, about a type nobody ships.  Before this check the
//! only guard was the generated evidence harness failing to compile, which says "something
//! differs" long after the contract was edited.
//!
//! ## Generate and compare, never lint
//!
//! The check follows the one principle of `check-doc`, `check-component` and
//! `check-contract`: **generate the canonical artifact from each side and compare the bytes.**
//! One line per enum, rendered by one function from each side:
//!
//! ```text
//!   variant Pick = Idle | Run(U8)       <- the Rust enum, read by vos-shape-check's token scan
//!   variant Pick = Idle | Run(U8)       <- the Warrant variant, from the warrant-grammar AST
//! ```
//!
//! The type map is the whole of the correspondence:
//!
//! ```text
//!   (a named-field variant renders `Put(slot: U8, on: Bool)`, with its field names)
//!   Rust                         renders as          Warrant                    renders as
//!   u8                           U8                  U8                         U8
//!   bool                         Bool                Bool                       Bool
//!   [u8; 4]                      Bytes<4>            Bytes<4>                   Bytes<4>
//!   Name (a struct/enum here)    Name                Name                       Name
//!   anything else                ?<as written>       List<..>, Option<..>, ...  as written
//! ```
//!
//! A leading `?` is not a legal Warrant character, so a Rust type the map does not know can
//! never equal a Warrant type: the safe direction, and exactly the types SH19 rejects.  A
//! tuple variant and a named-field variant both render their TYPES only, because Warrant
//! names a constructor's fields freely (`Run(t: U8)` renders `Run(U8)`) and the generated
//! harness marshals by position, so a tuple variant's field names are not compared.  A
//! NAMED-field Rust variant (`Put { slot: u8 }`) is different: its field names are part of
//! the Rust type, so it renders `Put(slot: U8)` and the Warrant constructor of that name
//! must spell the same field names (the same function renders the Warrant side, told by the
//! Rust side which constructors are named).
//!
//! ## Direction and residue
//!
//! The check runs **Rust to Warrant**: every Rust `enum` of the crate must have a Warrant
//! `variant` of its name that renders to the same bytes.  A Warrant variant with no Rust
//! enum is a model-only type and is allowed.  A rename on either side leaves a Rust enum
//! without a twin, so it is caught.  The published artifact is the list of Rust lines, in
//! source order, written into `warrant/lemmas.table` where CK09 byte-pins them.
//!
//! Not done, and said so in VOS04 5.6: the `enum ... ctors=` lines of the evidence bindings
//! against the contract (the generated harness compiles against the contract's own enums).
//! The Rust side is lexed as edition 2021, which is what every contract crate declares.

use std::collections::{BTreeMap, BTreeSet};

use coding_adventures_warrant_grammar::parser::{Declaration, TypeExpr, Width};
use vos_shape_check::items::{collect_types, TypeDef};
use vos_shape_check::lexer::{lex_documents, Edition};
use vos_shape_check::syntax::FileCtx;
use vos_shape_check::SourceFile;

use crate::contract_check::with_file;
use crate::error::Finding;

/// `s` with control characters and quotes escaped, for a message (names come from source
/// text, so nothing in them may break the one-line shape of a finding).
fn show(s: &str) -> String {
    let q = format!("{s:?}");
    q[1..q.len() - 1].to_string()
}

/// One Rust enum of the crate, as the check sees it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RustEnum {
    /// Crate-relative path of the file it is in.
    pub file: String,
    /// Line of its name.
    pub line: u32,
    /// Its name.
    pub name: String,
    /// `variant <Name> = <C1> | <C2>(<T>, ..)`.
    pub line_text: String,
    /// The constructors written with braces: they render with their field names.
    pub named: BTreeSet<String>,
}

/// `Name = A | B(U8, Bool)` from constructor texts.
fn join_variant(name: &str, ctors: &[String]) -> String {
    format!("variant {name} = {}", ctors.join(" | "))
}

/// A constructor: its name, and its payload in parentheses when it has any.  `fields` are
/// `(field name, type)`; the names are printed only when `named` is set.
fn ctor(name: &str, named: bool, fields: &[(String, String)]) -> String {
    if fields.is_empty() {
        return name.to_string();
    }
    let parts: Vec<String> = fields
        .iter()
        .map(|(n, t)| {
            if named {
                format!("{n}: {t}")
            } else {
                t.clone()
            }
        })
        .collect();
    format!("{name}({})", parts.join(", "))
}

/// The Rust field type as the Warrant spelling, or `?<as written>` when the map has none.
fn rust_type(ty: &str, local: &BTreeSet<&str>) -> String {
    match ty {
        "u8" => return "U8".to_string(),
        "bool" => return "Bool".to_string(),
        _ => {}
    }
    if let Some(len) = ty
        .strip_prefix("[ u8 ; ")
        .and_then(|r| r.strip_suffix(" ]"))
    {
        if !len.is_empty() && len.bytes().all(|b| b.is_ascii_digit()) {
            return format!("Bytes<{len}>");
        }
    }
    if local.contains(ty) {
        return ty.to_string();
    }
    format!("?{ty}")
}

/// The Rust enums of `sources`, in source order (files by path, then position), each rendered.
///
/// `Err` holds the problems that made the reading incomplete: a file that does not lex, or an
/// item the token scan could not read.  They are findings (fail closed), never skipped.
pub fn rust_enums(sources: &[SourceFile]) -> Result<Vec<RustEnum>, Vec<String>> {
    let mut problems: Vec<String> = Vec::new();
    let texts: Vec<&str> = sources.iter().map(|s| s.text.as_str()).collect();
    let mut files: Vec<FileCtx> = Vec::new();
    // `lex_documents` builds ONE grammar lexer for all the files.
    for (s, lexed) in sources.iter().zip(lex_documents(&texts, Edition::E2021)) {
        match lexed.and_then(|toks| FileCtx::from_tokens(&s.path, toks)) {
            Ok(f) => files.push(f),
            Err(e) => problems.push(format!(
                "{}:{}: cannot lex: {} (fail closed)",
                show(&s.path),
                e.line,
                show(&e.message)
            )),
        }
    }
    let (types, gaps) = collect_types(&files);
    for g in &gaps {
        problems.push(format!(
            "{}:{}: {}",
            show(&files[g.file].path),
            g.line,
            show(&g.message)
        ));
    }
    if !problems.is_empty() {
        return Err(problems);
    }
    let local: BTreeSet<&str> = types
        .iter()
        .filter(|t| t.keyword == "struct" || t.keyword == "enum")
        .map(|t| t.name.as_str())
        .collect();
    Ok(types
        .iter()
        .filter(|t| t.keyword == "enum")
        .map(|t| render_rust(t, &files[t.file].path, &local))
        .collect())
}

fn render_rust(t: &TypeDef, path: &str, local: &BTreeSet<&str>) -> RustEnum {
    let ctors: Vec<String> = t
        .variants
        .iter()
        .map(|v| {
            let fields: Vec<(String, String)> = v
                .fields
                .iter()
                .map(|f| (f.name.clone(), rust_type(&f.ty, local)))
                .collect();
            ctor(&v.name, v.named, &fields)
        })
        .collect();
    RustEnum {
        file: path.to_string(),
        line: t.line,
        name: t.name.clone(),
        line_text: join_variant(&t.name, &ctors),
        named: t
            .variants
            .iter()
            .filter(|v| v.named)
            .map(|v| v.name.clone())
            .collect(),
    }
}

/// The Warrant spelling of a type expression.
fn warrant_type(t: &TypeExpr<'_>) -> String {
    match t {
        TypeExpr::Bytes { width: None, .. } => "Bytes".to_string(),
        TypeExpr::Bytes {
            width: Some(Width::Literal(n)),
            ..
        } => format!("Bytes<{n}>"),
        TypeExpr::Bytes {
            width: Some(Width::Named(n)),
            ..
        } => format!("Bytes<{n}>"),
        TypeExpr::List { inner, .. } => format!("List<{}>", warrant_type(inner)),
        TypeExpr::Option { inner, .. } => format!("Option<{}>", warrant_type(inner)),
        TypeExpr::Result {
            ok_type, err_name, ..
        } => format!("Result<{}, {err_name}>", warrant_type(ok_type)),
        TypeExpr::Named { name, .. } => (*name).to_string(),
        TypeExpr::Tuple { elems, .. } => {
            let parts: Vec<String> = elems.iter().map(warrant_type).collect();
            format!("({})", parts.join(", "))
        }
    }
}

/// The Warrant `variant` declarations of the document `src`, by name, each rendered.
/// `named(enum, ctor)` says whether the Rust side wrote that constructor with braces, in which
/// case its field names are printed too.
///
/// `Err` is the reason the document did not parse.
pub fn warrant_variants(
    src: &str,
    named: &dyn Fn(&str, &str) -> bool,
) -> Result<BTreeMap<String, String>, String> {
    with_file(src, |file| {
        let mut out = BTreeMap::new();
        for d in &file.declarations {
            if let Declaration::Variant(v) = d {
                let ctors: Vec<String> = v
                    .ctors
                    .iter()
                    .map(|c| {
                        let fields: Vec<(String, String)> = c
                            .fields
                            .iter()
                            .map(|f| (f.name.to_string(), warrant_type(&f.ty)))
                            .collect();
                        ctor(c.name, named(v.name, c.name), &fields)
                    })
                    .collect();
                out.insert(v.name.to_string(), join_variant(v.name, &ctors));
            }
        }
        out
    })
}

/// Compare the two sides: the CK10 findings, and the lines of the table (the Rust lines, in
/// source order; only meaningful when there is no finding).
pub fn correspond(
    rust: &[RustEnum],
    warrant: &BTreeMap<String, String>,
) -> (Vec<Finding>, Vec<String>) {
    let mut findings = Vec::new();
    let mut lines = Vec::new();
    for e in rust {
        let at = format!("{}:{}", show(&e.file), e.line);
        match warrant.get(&e.name) {
            None => findings.push(Finding::new(
                "CK10",
                format!(
                    "{at}: enum `{}` has no Warrant `variant {}` in warrant/contract.part (a rename on either side lands here); the Rust side renders `{}`",
                    show(&e.name),
                    show(&e.name),
                    show(&e.line_text)
                ),
            )),
            Some(w) if *w != e.line_text => findings.push(Finding::new(
                "CK10",
                format!(
                    "{at}: enum `{}` and its Warrant variant differ; Rust renders `{}`, warrant/contract.part renders `{}`",
                    show(&e.name),
                    show(&e.line_text),
                    show(w)
                ),
            )),
            Some(_) => lines.push(e.line_text.clone()),
        }
    }
    (findings, lines)
}
