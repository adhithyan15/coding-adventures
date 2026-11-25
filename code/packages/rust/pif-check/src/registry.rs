//! The axiom registry (VER32 1.4 rules A2 and A3, 5.3 R-AX-1).
//!
//! # What it is for
//!
//! A claim file may declare **no axiom** except `Warrant.Assume.<id>` and registered `Src.*`
//! axioms, and each must match its registered statement *exactly*. "A name is trusted only if
//! its type was validated": a name alone proves nothing (`axiom propext : False`), so a
//! registry entry pins the *statement*.
//!
//! A registered statement is **relative to the environment prefix it was registered in**,
//! because the constants a statement mentions mean whatever the files before it made them
//! mean. So an entry is the quadruple the spec gives:
//!
//! ```text
//!   (name, universe parameters, binding-CTE digest of the type, digest of the environment prefix)
//! ```
//!
//! and it matches only when all four agree. The digest of the type is the SHA-256 of the
//! **binding CTE** of the axiom's declared type ([`crate::cte`]), so binder names do not
//! matter but every constant and every level does.
//!
//! # The file format
//!
//! One strict line per entry, in the format of [`crate::control`]:
//!
//! ```text
//!   pif-registry 1
//!   axiom <name> <params> <type-digest> <prefix-digest>
//! ```
//!
//! `<name>` is a name CTE (`<"Warrant","Assume","gates","nand">`: canonical and free of
//! spaces); `<params>` is `-` for none or a comma-separated list of name CTEs; the digests
//! are 64 lowercase hex digits, with or without `sha256:`. A name that is neither
//! `Warrant.Assume.<id>` nor `Src.<profile>.<name>` is refused at load time, as is an
//! exact duplicate line.
//!
//! The registry is *gate configuration*: registering a `Src.*` axiom needs a consistency
//! argument (A3), which is a human decision recorded outside this crate, and a registered
//! `Src.*` axiom is never bucket (a).

use crate::control::{digest_at, rows};
use crate::cte::{goal_digest, parse_name, parse_name_list};
use crate::digest::Digest;
use crate::error::CheckError;
use lean_kernel::{Expr, ExprStore, Name, NamePart};
use std::collections::HashMap;

/// One registered statement.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: Name,
    pub params: Vec<Name>,
    pub type_digest: Digest,
    pub prefix: Digest,
}

/// The registry.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Registry {
    by_name: HashMap<Name, Vec<Entry>>,
}

/// Is the name `Warrant.Assume.<id>` or `Src.<profile>.<name>`: the only shapes a registry
/// holds and the reader consults (R-AX-1). Returns which.
pub fn shape(name: &Name) -> Option<Shape> {
    let p = name.parts();
    let s = |i: usize, t: &str| matches!(p.get(i), Some(NamePart::Str(x)) if x == t);
    if p.len() > 2 && s(0, "Warrant") && s(1, "Assume") {
        Some(Shape::WarrantAssume)
    } else if p.len() > 2 && s(0, "Src") {
        Some(Shape::Src)
    } else {
        None
    }
}

/// The two registrable name shapes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    WarrantAssume,
    Src,
}

impl Registry {
    /// Parse a registry file.
    pub fn parse(text: &str) -> Result<Registry, CheckError> {
        let bad = |line, why| CheckError::BadControlFile {
            what: "registry",
            line,
            why,
        };
        let mut r = Registry::default();
        for (n, f) in rows("registry", text, "pif-registry")? {
            let ["axiom", name, params, ty, prefix] = f.as_slice() else {
                return Err(bad(
                    n,
                    "expected `axiom <name> <params> <type digest> <prefix digest>`",
                ));
            };
            let name = parse_name(name).map_err(|_| bad(n, "bad name"))?;
            if shape(&name).is_none() {
                return Err(bad(n, "not a Warrant.Assume.* or Src.* name"));
            }
            let params = if *params == "-" {
                vec![]
            } else {
                parse_name_list(params).map_err(|_| bad(n, "bad universe parameter list"))?
            };
            let entry = Entry {
                name: name.clone(),
                params,
                type_digest: digest_at("registry", n, ty)?,
                prefix: digest_at("registry", n, prefix)?,
            };
            let slot = r.by_name.entry(name).or_default();
            if slot.contains(&entry) {
                return Err(bad(n, "duplicate entry"));
            }
            slot.push(entry);
        }
        Ok(r)
    }

    /// Add an entry programmatically (tests, tools that write registries).
    pub fn add(&mut self, e: Entry) {
        self.by_name.entry(e.name.clone()).or_default().push(e);
    }

    /// The number of entries.
    pub fn len(&self) -> usize {
        self.by_name.values().map(Vec::len).sum()
    }

    /// Is the registry empty?
    pub fn is_empty(&self) -> bool {
        self.by_name.is_empty()
    }

    /// Does an axiom with this name, universe parameters and type digest, declared under the
    /// environment prefix `prefix`, match a registered statement exactly (A2)?
    pub fn matches(
        &self,
        name: &Name,
        params: &[Name],
        type_digest: &Digest,
        prefix: &Digest,
    ) -> bool {
        self.by_name.get(name).is_some_and(|v| {
            v.iter()
                .any(|e| e.params == params && e.type_digest == *type_digest && e.prefix == *prefix)
        })
    }

    /// [`Registry::matches`] for a type held in the kernel store. An encoding failure (a
    /// type over the cap, a free variable) is "no match".
    pub fn matches_term(
        &self,
        name: &Name,
        params: &[Name],
        ty: Expr,
        store: &ExprStore,
        prefix: &Digest,
    ) -> bool {
        goal_digest(store, ty).is_ok_and(|d| self.matches(name, params, &d, prefix))
    }
}
