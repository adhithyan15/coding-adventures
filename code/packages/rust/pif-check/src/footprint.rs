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
use std::rc::Rc;

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

/// A node's dependencies and whether it is an axiom, once discovered.
type Found = Option<(Vec<Name>, bool)>;

/// A footprint calculator that **remembers** what it has computed.
///
/// Thousands of claims over one prelude would otherwise each re-walk the prelude. The
/// dependency graph is condensed so that an inductive type, its constructors and its recursor
/// form one node (they refer to each other); every other declaration is its own node. The
/// condensed graph is acyclic, so footprints are computed bottom-up once per node, with an
/// explicit stack, and shared (`Rc`) between every query that reaches the node.
pub struct Footprinter<'a> {
    l: &'a Loaded,
    memo: HashMap<Name, Rc<Vec<Name>>>,
}

impl<'a> Footprinter<'a> {
    /// A calculator over a loaded chain.
    pub fn new(l: &'a Loaded) -> Footprinter<'a> {
        Footprinter {
            l,
            memo: HashMap::new(),
        }
    }

    fn decl(&self, n: &Name) -> Result<&'a lean_kernel::Declaration, CheckError> {
        self.l
            .env
            .get(n)
            .ok_or_else(|| CheckError::UnknownConstant(crate::report::dotted(n)))
    }

    /// The node a name belongs to: the inductive type for a constructor or recursor.
    fn rep(&self, n: &Name) -> Result<Name, CheckError> {
        Ok(match &self.decl(n)?.kind {
            DeclKind::Constructor(i) => i.induct.clone(),
            DeclKind::Recursor(i) => i.induct.clone(),
            _ => n.clone(),
        })
    }

    /// The nodes one node refers to directly (outside itself), and whether it is an axiom.
    fn deps(&self, r: &Name) -> Result<(Vec<Name>, bool), CheckError> {
        let store = &self.l.env.store;
        let d = self.decl(r)?;
        let (mut raw, mut seen) = (Vec::new(), HashSet::new());
        let mut is_axiom = false;
        scan(store, d.ty, &mut seen, &mut raw);
        match &d.kind {
            DeclKind::Axiom => is_axiom = true,
            DeclKind::Definition { value, .. }
            | DeclKind::Theorem { value }
            | DeclKind::Opaque { value } => scan(store, *value, &mut seen, &mut raw),
            DeclKind::Inductive(info) => {
                for m in info.ctors.iter().chain(std::iter::once(&info.rec)) {
                    let md = self.decl(m)?;
                    scan(store, md.ty, &mut seen, &mut raw);
                    if let DeclKind::Recursor(ri) = &md.kind {
                        for rule in &ri.rules {
                            scan(store, rule.rhs, &mut seen, &mut raw);
                        }
                    }
                }
            }
            // A constructor or recursor is never a node of its own (see `rep`).
            DeclKind::Constructor(_) | DeclKind::Recursor(_) => {}
        }
        let (mut out, mut have) = (Vec::new(), HashSet::new());
        for c in raw {
            let cr = self.rep(&c)?;
            if cr != *r && have.insert(cr.clone()) {
                out.push(cr);
            }
        }
        Ok((out, is_axiom))
    }

    /// The footprint of `root`, sorted by canonical name.
    pub fn of(&mut self, root: &Name) -> Result<Vec<Name>, CheckError> {
        let cyclic = || CheckError::UnknownConstant("cyclic dependency".into());
        let start = self.rep(root)?;
        // Post-order over the condensed graph with an explicit stack: a node is pushed once to
        // discover its dependencies and once more to combine them.
        let mut stack: Vec<(Name, Found)> = vec![(start.clone(), None)];
        let mut open: HashSet<Name> = HashSet::new();
        while let Some((node, found)) = stack.pop() {
            if self.memo.contains_key(&node) {
                continue;
            }
            match found {
                None => {
                    if !open.insert(node.clone()) {
                        // Cannot happen in an append-only environment; refuse rather than guess.
                        return Err(cyclic());
                    }
                    let (deps, ax) = self.deps(&node)?;
                    let pending: Vec<Name> = deps
                        .iter()
                        .filter(|x| !self.memo.contains_key(*x))
                        .cloned()
                        .collect();
                    stack.push((node, Some((deps, ax))));
                    stack.extend(pending.into_iter().map(|x| (x, None)));
                }
                Some((deps, is_axiom)) => {
                    let mut all: Vec<Name> = Vec::new();
                    if is_axiom {
                        all.push(node.clone());
                    }
                    for x in &deps {
                        all.extend(self.memo.get(x).ok_or_else(cyclic)?.iter().cloned());
                    }
                    all.sort_by_cached_key(name_cte);
                    all.dedup();
                    open.remove(&node);
                    self.memo.insert(node, Rc::new(all));
                }
            }
        }
        Ok(self
            .memo
            .get(&start)
            .map(|v| v.to_vec())
            .unwrap_or_default())
    }
}

/// The axioms `root` depends on (its footprint, including `root` itself if it is one), in a
/// deterministic order (sorted by canonical name). An unknown constant is an error: the kernel
/// would have refused the reference, so this means the environment is not the one the term was
/// checked in. For many queries over one environment use a [`Footprinter`], which shares work.
pub fn footprint(l: &Loaded, root: &Name) -> Result<Vec<Name>, CheckError> {
    Footprinter::new(l).of(root)
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
