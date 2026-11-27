//! The printer's checks (VER32 3.6 N5, R3, 1.1 `tcb.pif.printer`).
//!
//! # What is checked, and why it replaces trusting the printer
//!
//! The text a person signs or reviews is the printer's output. VER32 allows the printer to be
//! *outside* the trusted base only if the gate checks that printing loses nothing:
//!
//! * **For every goal**: `parse(print(G)) = G` in the display CTE, using the independent
//!   parser ([`crate::ppparse`]) ([`check_goal`]);
//! * **For every file a person reviews** (the prelude, ADL layers, model, axioms file):
//!   `parse(print(F)) = F` over the whole file ([`check_reviewed_file`]).
//!
//! The second check has a twist that matters. A file model is decoded from the file *text* by
//! a second reader ([`crate::filemodel::decode`]) so that it keeps the file's own binder
//! names (the kernel's store keeps the first-interned ones). To make sure that second reader
//! has not decoded a *different* file from the one the trusted reader gave the kernel,
//! [`check_reviewed_file`] also compares them declaration by declaration: the decoded type
//! and value of every declaration, erased to the binding CTE, must equal the binding CTE of
//! what the kernel accepted, and the set of declared names must match. A disagreement is a
//! reader or decoder bug and is fatal.
//!
//! Everything fails closed: a term too large to unfold, a name with no printed spelling, a
//! shadowed binder, all refuse the file.

use crate::cte::{binding_cte, Dterm};
use crate::filemodel::{decode, DeclModel, FileModel, HintModel, ModelError};
use crate::filetext::{parse_file, print_file};
use crate::ppparse::{parse_term, PpError};
use crate::printer::{print_term, PrintError};
use lean_kernel::pif::{Accepted, Kind};
use lean_kernel::{DeclKind, Environment, Expr, Name, ReducibilityHints};
use std::collections::HashSet;

/// Why a printer check failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrintCheckError {
    /// The file text could not be decoded, printed or parsed.
    Model(ModelError),
    /// A goal could not be printed.
    Print(PrintError),
    /// A goal's printed text did not parse.
    Parse(PpError),
    /// `parse(print(x)) != x`: the printer lost information.
    RoundTrip,
    /// The decoded file and the kernel's environment disagree (which declaration).
    KernelDisagrees(String),
}

/// `parse(print(G)) = G` for one goal's display term; returns the printed text (the readable
/// form a signer sees).
pub fn check_goal(g: &Dterm) -> Result<String, PrintCheckError> {
    let text = print_term(g).map_err(PrintCheckError::Print)?;
    let back = parse_term(&text).map_err(PrintCheckError::Parse)?;
    // Equal trees have equal display encodings (the encoding is injective), which also
    // makes the comparison independent of how each side laid out its arena.
    match (g.display(), back.display()) {
        (Ok(a), Ok(b)) if a == b => Ok(text),
        _ => Err(PrintCheckError::RoundTrip),
    }
}

fn bind(d: &Dterm) -> Result<String, PrintCheckError> {
    d.binding()
        .map_err(|e| PrintCheckError::KernelDisagrees(format!("encoding failed: {e:?}")))
}

fn kern(env: &Environment, e: Expr) -> Result<String, PrintCheckError> {
    binding_cte(&env.store, e)
        .map_err(|e| PrintCheckError::KernelDisagrees(format!("encoding failed: {e:?}")))
}

/// The decoded term erases to the kernel's term.
fn same(env: &Environment, d: &Dterm, e: Expr, what: &str) -> Result<(), PrintCheckError> {
    if bind(d)? == kern(env, e)? {
        Ok(())
    } else {
        Err(PrintCheckError::KernelDisagrees(what.to_string()))
    }
}

fn hint_eq(h: &HintModel, k: &ReducibilityHints) -> bool {
    matches!(
        (h, k),
        (HintModel::Abbrev, ReducibilityHints::Abbrev)
            | (HintModel::Opaque, ReducibilityHints::Opaque)
    ) || matches!((h, k), (HintModel::Regular(a), ReducibilityHints::Regular(b)) if a == b)
}

/// The decoded file agrees with the environment the kernel built from it.
pub fn agrees_with_kernel(
    m: &FileModel,
    env: &Environment,
    accepted: &[Accepted],
) -> Result<(), PrintCheckError> {
    let dis = |what: String| PrintCheckError::KernelDisagrees(what);
    let mut declared: HashSet<Name> = HashSet::new();
    for d in &m.decls {
        match d {
            DeclModel::Axiom { name, params, ty }
            | DeclModel::Theorem {
                name, params, ty, ..
            }
            | DeclModel::Opaque {
                name, params, ty, ..
            }
            | DeclModel::Def {
                name, params, ty, ..
            } => {
                let k = env
                    .get(name)
                    .ok_or_else(|| dis(format!("{name:?} missing from the environment")))?;
                declared.insert(name.clone());
                if k.level_params != *params {
                    return Err(dis("universe parameters".into()));
                }
                same(env, ty, k.ty, "type")?;
                match (d, &k.kind) {
                    (DeclModel::Axiom { .. }, DeclKind::Axiom) => {}
                    (DeclModel::Theorem { value, .. }, DeclKind::Theorem { value: v })
                    | (DeclModel::Opaque { value, .. }, DeclKind::Opaque { value: v }) => {
                        same(env, value, *v, "value")?
                    }
                    (
                        DeclModel::Def { value, hint, .. },
                        DeclKind::Definition { value: v, hints },
                    ) => {
                        same(env, value, *v, "value")?;
                        if !hint_eq(hint, hints) {
                            return Err(dis("reducibility hint".into()));
                        }
                    }
                    _ => return Err(dis("declaration kind".into())),
                }
            }
            DeclModel::Inductive {
                params,
                num_params,
                types,
            } => {
                for t in types {
                    let k = env
                        .get(&t.name)
                        .ok_or_else(|| dis("inductive type missing".into()))?;
                    declared.insert(t.name.clone());
                    let DeclKind::Inductive(info) = &k.kind else {
                        return Err(dis("inductive kind".into()));
                    };
                    let names: Vec<&Name> = t.ctors.iter().map(|c| &c.0).collect();
                    if k.level_params != *params
                        || info.num_params != *num_params as usize
                        || info.ctors.iter().ne(names.iter().copied())
                    {
                        return Err(dis("inductive shape".into()));
                    }
                    same(env, &t.ty, k.ty, "inductive type")?;
                    for (cn, cty) in &t.ctors {
                        let c = env
                            .get(cn)
                            .ok_or_else(|| dis("constructor missing".into()))?;
                        declared.insert(cn.clone());
                        same(env, cty, c.ty, "constructor type")?;
                    }
                }
            }
            DeclModel::Goal { .. } => {}
        }
    }
    // Every accepted declaration (but the kernel-derived recursors) is in the model, and no more.
    let accepted: HashSet<Name> = accepted
        .iter()
        .filter(|a| a.kind != Kind::Recursor)
        .map(|a| a.name.clone())
        .collect();
    if accepted == declared {
        Ok(())
    } else {
        Err(dis("the set of declared names".into()))
    }
}

/// The checks for one file a person reviews: decode its text, `parse(print(F)) = F`, and
/// agreement with the kernel. Returns the printed text.
pub fn check_reviewed_file(
    bytes: &[u8],
    env: &Environment,
    accepted: &[Accepted],
) -> Result<String, PrintCheckError> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| PrintCheckError::Model(ModelError::Decode("not UTF-8", 1)))?;
    let model = decode(text).map_err(PrintCheckError::Model)?;
    let printed = print_file(&model).map_err(PrintCheckError::Model)?;
    let back = parse_file(&printed).map_err(PrintCheckError::Model)?;
    if back != model {
        return Err(PrintCheckError::RoundTrip);
    }
    agrees_with_kernel(&model, env, accepted)?;
    Ok(printed)
}
