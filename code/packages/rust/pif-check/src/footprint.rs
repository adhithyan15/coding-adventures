//! Axiom footprints and their classification (VER32 1.4, G1, G2, 9.2).
//!
//! # The footprint
//!
//! A theorem is only as good as the assumptions under it. The **footprint** of a constant is
//! the set of axioms its type and its value depend on, transitively, over the whole
//! environment chain:
//!
//! ```text
//!   footprint(c) = { c }                       if c is an axiom   (G2: it includes itself)
//!                = union of footprint(d) for every constant d in c's type and value
//! ```
//!
//! The walk starts from the constant itself, **not only from its value** (G2): an axiom
//! `T : G` has no value, and a value-only walk would give it an empty footprint, so a claim
//! "proved" by `axiom T : G` would look assumption-free. (The reader's G1 already refuses a
//! goal on an axiom; this keeps the footprint honest on its own.)
//!
//! Inductive types contribute their constructors and recursor, constructors and recursors
//! contribute their type, and a recursor its computation rules, so a dependency hidden in
//! how a type reduces is not missed. The walk is conservative: it may include an axiom a
//! smaller analysis could prove irrelevant, never the reverse.
//!
//! **Depth is attacker-controlled**, so the walk uses explicit stacks and a visited set over
//! both names and (hash-consed) expressions; each node and each constant is visited once.
//!
//! # Classification
//!
//! Every axiom in a footprint lands in a bucket (VER31). What the gate can honestly say
//! today:
//!
//! * `Warrant.Assume.<id>` and `Src.<profile>.<name>` axioms that **exactly match a registry
//!   entry** (name, universe parameters, binding-CTE digest of the type, digest of the
//!   environment prefix the axiom was declared under) are *registered assumptions*, bucket
//!   (c). A registered `Src.*` axiom is never bucket (a), and the report says "relative to
//!   Lean + these axioms; consistency with the kernel logic not established" (A3).
//! * **Everything else is an unclassified (c) assumption.** That includes `propext`,
//!   `Classical.choice` and `Quot.sound`: A1 counts them as the kernel logic's own axioms
//!   (bucket (a)) only if a validated `axiom.*` `prim` role bound them, and roles are
//!   kernel slice 4, not implemented yet. Until then the gate fails closed: a name proves
//!   nothing, so it is classified as the worst case.

use crate::chain::Loaded;
use crate::cte::{goal_digest, name_cte};
use crate::error::CheckError;
use crate::registry::{shape, Registry, Shape};
use lean_kernel::{DeclKind, Expr, ExprKind, ExprStore, Name};
use std::collections::{HashMap, HashSet};

/// How the gate classified an axiom.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxClass {
    /// A registered `Warrant.Assume.*` statement.
    WarrantAssume,
    /// A registered `Src.*` statement (never bucket (a)).
    Src,
    /// Anything else: an unclassified (c) assumption.
    Unclassified,
}

impl AxClass {
    /// The word printed in the report.
    pub fn word(self) -> &'static str {
        match self {
            AxClass::WarrantAssume => "warrant-assume",
            AxClass::Src => "src",
            AxClass::Unclassified => "unclassified",
        }
    }
}

/// One axiom in a footprint, classified.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Classified {
    pub name: Name,
    /// The chain position of the file that declared it.
    pub file: usize,
    pub class: AxClass,
    /// The VER31 bucket: always `'c'` today (see the module documentation).
    pub bucket: char,
    pub note: &'static str,
}

/// Push every constant name occurring in `e` onto `names`, walking the DAG once.
fn scan(store: &ExprStore, e: Expr, seen: &mut HashSet<Expr>, names: &mut Vec<Name>) {
    let mut work = vec![e];
    while let Some(x) = work.pop() {
        if !seen.insert(x) {
            continue;
        }
        match store.kind(x) {
            ExprKind::Const(n, _) => names.push(n.clone()),
            ExprKind::App(f, a) => work.extend([*f, *a]),
            ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => work.extend([*t, *b]),
            ExprKind::Let(_, t, v, b) => work.extend([*t, *v, *b]),
            ExprKind::Proj(n, _, y) => {
                names.push(n.clone());
                work.push(*y);
            }
            _ => {}
        }
    }
}

/// The axioms `root` depends on (its footprint, including `root` itself if it is one),
/// in a deterministic order (sorted by canonical name). An unknown constant is an error:
/// the kernel would have refused the reference, so this means the environment is not the
/// one the term was checked in.
pub fn footprint(l: &Loaded, root: &Name) -> Result<Vec<Name>, CheckError> {
    let store = &l.env.store;
    let (mut seen_names, mut seen_exprs): (HashSet<Name>, HashSet<Expr>) =
        (HashSet::new(), HashSet::new());
    let mut names = vec![root.clone()];
    let mut axioms: Vec<Name> = Vec::new();
    while let Some(n) = names.pop() {
        if !seen_names.insert(n.clone()) {
            continue;
        }
        let d = l
            .env
            .get(&n)
            .ok_or_else(|| CheckError::UnknownConstant(crate::report::dotted(&n)))?;
        scan(store, d.ty, &mut seen_exprs, &mut names);
        match &d.kind {
            DeclKind::Axiom => axioms.push(n.clone()),
            DeclKind::Definition { value, .. }
            | DeclKind::Theorem { value }
            | DeclKind::Opaque { value } => {
                scan(store, *value, &mut seen_exprs, &mut names);
            }
            DeclKind::Inductive(info) => {
                names.extend(info.ctors.iter().cloned());
                names.push(info.rec.clone());
            }
            DeclKind::Constructor(info) => names.push(info.induct.clone()),
            DeclKind::Recursor(info) => {
                names.push(info.induct.clone());
                for rule in &info.rules {
                    names.push(rule.ctor.clone());
                    scan(store, rule.rhs, &mut seen_exprs, &mut names);
                }
            }
        }
    }
    axioms.sort_by_cached_key(name_cte);
    Ok(axioms)
}

/// Classify the axioms of a footprint. `declared_in` maps each declared name to the chain
/// position of its file (see [`declared_in`]); `registry` is the gate's axiom registry.
pub fn classify(
    l: &Loaded,
    axioms: &[Name],
    declared_in: &HashMap<Name, usize>,
    registry: Option<&Registry>,
) -> Vec<Classified> {
    axioms
        .iter()
        .map(|name| {
            let file = declared_in.get(name).copied().unwrap_or(usize::MAX);
            let (class, note) = classify_one(l, name, file, registry);
            Classified {
                name: name.clone(),
                file,
                class,
                bucket: 'c',
                note,
            }
        })
        .collect()
}

fn classify_one(
    l: &Loaded,
    name: &Name,
    file: usize,
    registry: Option<&Registry>,
) -> (AxClass, &'static str) {
    let registered = || {
        let (Some(reg), Some(decl), Some(f)) = (registry, l.env.get(name), l.files.get(file))
        else {
            return false;
        };
        // The statement is relative to the environment prefix the axiom was declared under.
        goal_digest(&l.env.store, decl.ty)
            .is_ok_and(|d| reg.matches(name, &decl.level_params, &d, &f.prefix))
    };
    match shape(name) {
        Some(Shape::WarrantAssume) if registered() => (AxClass::WarrantAssume, "registered Warrant assumption"),
        Some(Shape::Src) if registered() => (
            AxClass::Src,
            "registered Src axiom: relative to Lean + these axioms; consistency with the kernel logic not established",
        ),
        Some(_) => (AxClass::Unclassified, "named like a registrable axiom but not matching a registry entry exactly"),
        None if is_logic_axiom_name(name) => (
            AxClass::Unclassified,
            "logic axiom name: counts as kernel logic only if a validated axiom.* role bound it (A1), and roles are not implemented yet",
        ),
        None => (AxClass::Unclassified, "no classification rule applies"),
    }
}

/// `propext`, `Classical.choice`, `Quot.sound`: the names A1 talks about.
fn is_logic_axiom_name(n: &Name) -> bool {
    [
        Name::from_dotted("propext"),
        Name::from_dotted("Classical.choice"),
        Name::from_dotted("Quot.sound"),
    ]
    .contains(n)
}

/// Which chain position declared each name (every accepted declaration, including an
/// inductive's constructors and recursor).
pub fn declared_in(l: &Loaded) -> HashMap<Name, usize> {
    let mut m = HashMap::new();
    for (i, f) in l.files.iter().enumerate() {
        for d in &f.decls {
            m.insert(d.name.clone(), i);
        }
    }
    m
}
