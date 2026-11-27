//! Rule G7: the names a signer reads (VER32 3.6 and 3.8).
//!
//! # The attack
//!
//! A person signs the *printed* goal. If a name in it can be spelled so that two different
//! names look alike, the signature binds the wrong term: a Cyrillic `а` (U+0430) next to a
//! Latin `a`, a zero-width character inside `Eq`, or a binder that is *called* `Eq` so that
//! `Eq a b` reads as the library relation while meaning the bound variable. The binding CTE
//! digest is exact, but the human check on top of it is only as good as the text.
//!
//! # The rule
//!
//! **Every name in a goal's display CTE is in the ASCII class** of 3.8: each component of a
//! constant name, a binder name and a universe parameter name matches
//! `[A-Za-z_][A-Za-z0-9_']*`. In addition, **binder names**:
//!
//! * are **single-component** (`a`, never `Bool.false`);
//! * are **pairwise distinct** over the whole goal (the literal reading of G7; the
//!   anonymous name `_` is exempt, because the arrow `A -> B` is a binder named `_` and a
//!   goal may contain many);
//! * are **not equal to any constant name in the environment** (a binder called `Eq` while an
//!   `Eq` exists).
//!
//! Numeric name components (`nnum`) are refused in a goal: the ASCII class has no spelling
//! for them (the reader gives them no ASCII standing either, R-NAM-3).

use crate::cte::{Dterm, Lv, Node};
use lean_kernel::{Environment, Name, NamePart};
use std::collections::HashSet;

/// Which part of G7 was broken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameRule {
    /// A component outside `[A-Za-z_][A-Za-z0-9_']*` (a homoglyph, a zero-width character,
    /// a dot or a space inside a component), or an empty component.
    NotAscii,
    /// A numeric component, or the anonymous name where a name is needed.
    NumericOrEmpty,
    /// A binder name with more than one component.
    BinderNotSingle,
    /// Two binders with the same name.
    DuplicateBinder,
    /// A binder named like a constant of the environment.
    BinderIsConstant,
}

impl NameRule {
    /// The word printed in reports.
    pub fn word(self) -> &'static str {
        match self {
            NameRule::NotAscii => "not-ascii",
            NameRule::NumericOrEmpty => "numeric-or-empty",
            NameRule::BinderNotSingle => "binder-not-single-component",
            NameRule::DuplicateBinder => "duplicate-binder",
            NameRule::BinderIsConstant => "binder-is-constant",
        }
    }
}

/// A violation, with the offending name spelled for a human (non-ASCII shown as `\u{h}`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NameViolation {
    pub rule: NameRule,
    pub name: String,
}

/// `[A-Za-z_][A-Za-z0-9_']*`.
pub fn ascii_class(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|x| x.is_ascii_alphabetic() || x == '_')
        && c.all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '\'')
}

/// A name spelled for a report: components joined by `.`, anything outside printable ASCII
/// written as `\u{h}` so the report itself is unambiguous.
fn spelled(n: &Name) -> String {
    let comp = |p: &NamePart| match p {
        NamePart::Num(k) => k.to_string(),
        NamePart::Str(s) => s
            .chars()
            .map(|c| {
                if ('!'..='~').contains(&c) {
                    c.to_string()
                } else {
                    format!("\\u{{{:x}}}", c as u32)
                }
            })
            .collect(),
    };
    n.parts().iter().map(comp).collect::<Vec<_>>().join(".")
}

/// The ASCII-class check for one name: at least one component, all string components in
/// the class.
fn check_ascii(n: &Name) -> Result<(), NameViolation> {
    let v = |rule| NameViolation {
        rule,
        name: spelled(n),
    };
    let parts = n.parts();
    if parts.is_empty() || parts.iter().any(|p| matches!(p, NamePart::Num(_))) {
        return Err(v(NameRule::NumericOrEmpty));
    }
    if parts
        .iter()
        .any(|p| !matches!(p, NamePart::Str(s) if ascii_class(s)))
    {
        return Err(v(NameRule::NotAscii));
    }
    Ok(())
}

/// Check G7 on a goal's display term. With `binders` false only constants and universe
/// parameters are checked (used when no display CTE was supplied and the term's binder
/// names are the kernel store's arbitrary choice).
pub fn check_goal_names(t: &Dterm, env: &Environment, binders: bool) -> Result<(), NameViolation> {
    for l in &t.levels {
        if let Lv::Param(n) = l {
            check_ascii(n)?;
        }
    }
    let mut seen: HashSet<Name> = HashSet::new();
    for node in &t.nodes {
        match node {
            Node::Const(n, _) | Node::Proj(n, ..) => check_ascii(n)?,
            Node::Lam(_, b, ..) | Node::Pi(_, b, ..) | Node::Let(b, ..) if binders => {
                check_ascii(b)?;
                let v = |rule| NameViolation {
                    rule,
                    name: spelled(b),
                };
                if b.parts().len() != 1 {
                    return Err(v(NameRule::BinderNotSingle));
                }
                let anonymous = *b == Name::from_dotted("_");
                if !anonymous && !seen.insert(b.clone()) {
                    return Err(v(NameRule::DuplicateBinder));
                }
                if env.contains(b) {
                    return Err(v(NameRule::BinderIsConstant));
                }
            }
            _ => {}
        }
    }
    Ok(())
}
