//! Hash-consed expressions and the de Bruijn operations on them.
//!
//! # The terms of Lean's type theory
//!
//! ```text
//!   Expr ::= #i                      bound variable, de Bruijn index i
//!          | fvar id                 free variable (a local hypothesis), unique id
//!          | Sort l                  a universe
//!          | c.{l1,...,ln}           constant applied to universe levels
//!          | f a                     application
//!          | fun (x : A) => b        lambda
//!          | (x : A) -> b            dependent function type (Pi, written forallE in Lean)
//!          | let x : A := v; b       let
//!          | lit                     a Nat or String literal
//!          | proj S i e              the i-th field of a structure value e of type S
//! ```
//!
//! # De Bruijn indices (and what goes wrong with them)
//!
//! A bound variable is not named but *counted*: `#0` is the nearest enclosing binder,
//! `#1` the next one out. `fun x => fun y => x` is `lam. lam. #1`. This makes alpha
//! equivalence (renaming) a non-issue: `fun x => x` and `fun y => y` are the same term.
//! The price is bookkeeping: whenever a term moves under or out of a binder, the
//! indices that point *outside* it must be shifted, or they silently start pointing at
//! the wrong binder (variable capture). A *loose* bound variable is one whose index
//! reaches past every binder inside the term, i.e. it refers to a binder outside. Every
//! node caches its *loose bvar range*: one more than the largest loose index, or `0` for
//! a closed term. A subterm whose range is within the current binder depth contains
//! nothing that an operation could change, so operations return it untouched without
//! even looking inside; this is what keeps them fast on shared DAGs.
//!
//! # Hash-consing
//!
//! An [`ExprStore`] interns every node: building the same structure twice returns the
//! same [`Expr`] handle. Equality of terms is then equality of a `u32`, and a term's hash
//! is cached in its node. The type checker leans on this: it can cache results keyed by
//! handle, and "have we already seen this term?" is a table lookup.
//!
//! **Binder names and binder info are metadata, never identity.** `fun x => x` and
//! `fun y => y` (or one with an implicit binder) must be the same node, because Lean's
//! kernel never looks at them and a checker that did could reject or, worse, confuse
//! terms differently from the reference. They are carried inside [`Meta`], whose equality
//! is always `true` and whose hash contributes nothing; the node keeps the metadata of
//! whichever spelling was interned first, which is only used for printing.
//!
//! A handle belongs to the store that made it; using it with another store panics (a
//! silent mix-up would alias unrelated terms, which in a proof checker is a soundness
//! hole, not a crash worth avoiding).
//!
//! Traversals are recursive; extremely deep terms (hundreds of thousands of nested
//! nodes) can exhaust the native stack.

use crate::level::{ArityMismatch, Level};
use crate::name::Name;
use bignum_core::BigInteger;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// A handle to an interned expression. Equal handles are equal terms and vice versa
/// (within one store).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct Expr {
    store: u64,
    idx: u32,
}

/// The unique id of a free variable. The fields are private: an id can only come from
/// [`ExprStore::fresh_fvar_id`] or [`ExprStore::issue_fvar_id_at`], so two different local
/// hypotheses can never be conflated by a hand-built id, and an id carries the store that
/// issued it.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct FVarId {
    store: u64,
    n: u64,
}

impl FVarId {
    /// The number of this id within its store (for printing and for importers).
    pub fn index(&self) -> u64 {
        self.n
    }
}

/// Error from [`ExprStore::issue_fvar_id_at`]: the requested number was already issued.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct FVarIdTaken(pub u64);

/// Convert a count or position to `u32`, panicking (never truncating) when it does not
/// fit. A silently wrapped node index would make a new handle alias an old node.
pub fn checked_u32(n: usize, what: &str) -> u32 {
    u32::try_from(n).unwrap_or_else(|_| panic!("{what} does not fit in u32"))
}

/// How a binder was written. Printing and elaboration only; the kernel ignores it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BinderInfo {
    /// `(x : A)`
    Default,
    /// `{x : A}`
    Implicit,
    /// `⦃x : A⦄`
    StrictImplicit,
    /// `[x : A]`
    InstImplicit,
}

/// A binder's name and style: the printing-only part of `fun`, `Pi` and `let`.
#[derive(Clone, Debug)]
pub struct Binder {
    /// The name shown when printing.
    pub name: Name,
    /// How the binder was written.
    pub info: BinderInfo,
}

/// A wrapper that is invisible to equality and hashing; see the module documentation.
#[derive(Clone, Debug)]
pub struct Meta<T>(pub T);
impl<T> PartialEq for Meta<T> {
    fn eq(&self, _: &Meta<T>) -> bool {
        true
    }
}
impl<T> Eq for Meta<T> {}
impl<T> Hash for Meta<T> {
    fn hash<H: Hasher>(&self, _: &mut H) {}
}

/// A literal. `Nat` literals hold an arbitrary-precision natural number.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum Literal {
    /// A natural-number literal.
    Nat(BigInteger),
    /// A string literal.
    Str(String),
}

/// The shape of an expression. Children are handles into the same store.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum ExprKind {
    /// Bound variable (de Bruijn index).
    BVar(u32),
    /// Free variable.
    FVar(FVarId),
    /// `Sort l`.
    Sort(Level),
    /// Constant with universe arguments.
    Const(Name, Arc<[Level]>),
    /// Application `f a`.
    App(Expr, Expr),
    /// `fun (x : ty) => body`; `body` has the new binder as `#0`.
    Lam(Meta<Binder>, Expr, Expr),
    /// `(x : ty) -> body`.
    Pi(Meta<Binder>, Expr, Expr),
    /// `let x : ty := val; body`; `body` has the new binder as `#0`.
    Let(Meta<Binder>, Expr, Expr, Expr),
    /// A literal.
    Lit(Literal),
    /// Structure projection: structure name, field index, the structure value.
    Proj(Name, u32, Expr),
}

/// An interned node together with the facts cached about it.
struct Node {
    kind: ExprKind,
    hash: u64,
    loose_range: u32,
    has_fvar: bool,
    has_param: bool,
}

static NEXT_STORE: AtomicU64 = AtomicU64::new(0);

/// The interning table. All construction and all operations go through it.
pub struct ExprStore {
    id: u64,
    nodes: Vec<Node>,
    index: HashMap<ExprKind, Expr>,
    next_fvar: u64,
    node_cap: Option<usize>,
    over_budget: bool,
}

impl Default for ExprStore {
    fn default() -> Self {
        ExprStore::new()
    }
}

/// Bound-variable indices must stay below this ceiling (`u32::MAX / 2`), so adding two
/// valid quantities (an index and a shift, a cut-off and a depth) cannot overflow.
pub const MAX_BVAR_INDEX: u32 = u32::MAX / 2;

/// The error from [`ExprStore::try_bvar`]: the index was at or above [`MAX_BVAR_INDEX`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BVarIndexTooLarge(pub u32);

/// Checked index arithmetic: overflow is a loud panic, never a wrapped (wrong) index.
fn shifted(a: u32, b: u32) -> u32 {
    a.checked_add(b)
        .expect("de Bruijn index arithmetic overflowed u32")
}

/// Memo table for one traversal: (subterm, binder depth) to result.
type Cache = HashMap<(Expr, u32), Expr>;

impl ExprStore {
    /// An empty store.
    pub fn new() -> ExprStore {
        ExprStore {
            id: NEXT_STORE.fetch_add(1, Ordering::Relaxed),
            nodes: Vec::new(),
            index: HashMap::new(),
            next_fvar: 0,
            node_cap: None,
            over_budget: false,
        }
    }

    /// Limit the store to `cap` nodes (`None` removes the limit). A traversal
    /// (`instantiate`, `abstract_fvars`, lifting, level instantiation) that finds the store over
    /// the cap stops descending and sets [`ExprStore::budget_exceeded`]; its result is then
    /// meaningless and the caller must discard it. This bounds the nodes one call can
    /// intern: a DAG of `n` shared nodes under `d` binder depths can otherwise create
    /// `n * d` distinct nodes inside a single call, before any caller-side check runs.
    /// Setting or clearing the cap also clears the flag.
    pub fn set_node_cap(&mut self, cap: Option<usize>) {
        self.node_cap = cap;
        self.over_budget = false;
    }
    /// Did a traversal stop because of the node cap? (Sticky until the cap is reset.)
    pub fn budget_exceeded(&self) -> bool {
        self.over_budget
    }
    /// Was `e` made by this store? A handle from another store must be refused by
    /// callers that take handles from the outside: using it would panic.
    pub fn owns(&self, e: Expr) -> bool {
        e.store == self.id && (e.idx as usize) < self.nodes.len()
    }

    /// Number of distinct nodes interned so far.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
    /// Is the store empty?
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    fn node(&self, e: Expr) -> &Node {
        assert_eq!(
            e.store, self.id,
            "Expr used with a store that did not create it"
        );
        &self.nodes[e.idx as usize]
    }

    /// The shape of `e`.
    pub fn kind(&self, e: Expr) -> &ExprKind {
        &self.node(e).kind
    }
    /// The cached structural hash of `e` (independent of insertion order).
    pub fn hash_value(&self, e: Expr) -> u64 {
        self.node(e).hash
    }
    /// One more than the largest loose bound-variable index in `e`; `0` when closed.
    pub fn loose_bvar_range(&self, e: Expr) -> u32 {
        self.node(e).loose_range
    }
    /// Does `e` contain a loose bound variable?
    pub fn has_loose_bvars(&self, e: Expr) -> bool {
        self.loose_bvar_range(e) > 0
    }
    /// Does `e` mention a free variable? (Cached.)
    pub fn has_fvar(&self, e: Expr) -> bool {
        self.node(e).has_fvar
    }
    /// Does `e` mention a universe parameter? (Cached.)
    pub fn has_level_param(&self, e: Expr) -> bool {
        self.node(e).has_param
    }

    /// Intern `kind`, computing and caching its facts the first time it is seen.
    fn mk(&mut self, kind: ExprKind) -> Expr {
        if let Some(&e) = self.index.get(&kind) {
            return e;
        }
        let r = |s: &ExprStore, e: Expr| s.node(e).loose_range;
        let f = |s: &ExprStore, e: Expr| s.node(e).has_fvar;
        let p = |s: &ExprStore, e: Expr| s.node(e).has_param;
        let mut h = DefaultHasher::new();
        std::mem::discriminant(&kind).hash(&mut h);
        let (loose_range, has_fvar, has_param) = match &kind {
            ExprKind::BVar(i) => {
                i.hash(&mut h);
                (i + 1, false, false) // i < MAX_BVAR_INDEX (see try_bvar)
            }
            ExprKind::FVar(id) => {
                id.hash(&mut h);
                (0, true, false)
            }
            ExprKind::Sort(l) => {
                l.hash(&mut h);
                (0, false, l.has_param())
            }
            ExprKind::Const(n, ls) => {
                n.hash(&mut h);
                ls.hash(&mut h);
                (0, false, ls.iter().any(Level::has_param))
            }
            ExprKind::App(a, b) => {
                self.node(*a).hash.hash(&mut h);
                self.node(*b).hash.hash(&mut h);
                (
                    r(self, *a).max(r(self, *b)),
                    f(self, *a) || f(self, *b),
                    p(self, *a) || p(self, *b),
                )
            }
            ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => {
                self.node(*t).hash.hash(&mut h);
                self.node(*b).hash.hash(&mut h);
                // The body's #0 is bound by this binder, so it is not loose out here.
                (
                    r(self, *t).max(r(self, *b).saturating_sub(1)),
                    f(self, *t) || f(self, *b),
                    p(self, *t) || p(self, *b),
                )
            }
            ExprKind::Let(_, t, v, b) => {
                for c in [t, v, b] {
                    self.node(*c).hash.hash(&mut h);
                }
                (
                    r(self, *t)
                        .max(r(self, *v))
                        .max(r(self, *b).saturating_sub(1)),
                    f(self, *t) || f(self, *v) || f(self, *b),
                    p(self, *t) || p(self, *v) || p(self, *b),
                )
            }
            ExprKind::Lit(l) => {
                l.hash(&mut h);
                (0, false, false)
            }
            ExprKind::Proj(n, i, x) => {
                n.hash(&mut h);
                i.hash(&mut h);
                self.node(*x).hash.hash(&mut h);
                (r(self, *x), f(self, *x), p(self, *x))
            }
        };
        let e = Expr {
            store: self.id,
            idx: checked_u32(self.nodes.len(), "ExprStore node count (node limit)"),
        };
        self.nodes.push(Node {
            kind: kind.clone(),
            hash: h.finish(),
            loose_range,
            has_fvar,
            has_param,
        });
        self.index.insert(kind, e);
        e
    }

    // ---- constructors -----------------------------------------------------------

    /// Bound variable `#i`. Panics if `i >= MAX_BVAR_INDEX`; use
    /// [`ExprStore::try_bvar`] for untrusted indices (e.g. from an export file).
    pub fn bvar(&mut self, i: u32) -> Expr {
        self.try_bvar(i)
            .expect("bound-variable index at or above MAX_BVAR_INDEX")
    }
    /// Bound variable `#i`, or an error if `i >= MAX_BVAR_INDEX`. The ceiling keeps every
    /// later shift (`i + d`, `range = i + 1`) far from `u32` overflow, which in a release
    /// build would silently wrap into a *different*, wrong term.
    pub fn try_bvar(&mut self, i: u32) -> Result<Expr, BVarIndexTooLarge> {
        if i >= MAX_BVAR_INDEX {
            return Err(BVarIndexTooLarge(i));
        }
        Ok(self.mk(ExprKind::BVar(i)))
    }
    /// A fresh free-variable id, never returned before by this store.
    pub fn fresh_fvar_id(&mut self) -> FVarId {
        self.next_fvar += 1;
        FVarId {
            store: self.id,
            n: self.next_fvar - 1,
        }
    }
    /// For importers that carry their own numbering: issue the id numbered `n`, which must
    /// not have been issued yet (it advances the same counter that `fresh_fvar_id` uses,
    /// so a later fresh id can never collide with it).
    pub fn issue_fvar_id_at(&mut self, n: u64) -> Result<FVarId, FVarIdTaken> {
        if n < self.next_fvar {
            return Err(FVarIdTaken(n));
        }
        self.next_fvar = n.checked_add(1).expect("fvar number overflow");
        Ok(FVarId { store: self.id, n })
    }
    /// The free variable with the given id. Panics if this store never issued the id
    /// (made by another store, or not issued at all).
    pub fn fvar(&mut self, id: FVarId) -> Expr {
        assert!(
            id.store == self.id && id.n < self.next_fvar,
            "FVarId was not issued by this store"
        );
        self.mk(ExprKind::FVar(id))
    }
    /// `Sort l`.
    pub fn sort(&mut self, l: &Level) -> Expr {
        self.mk(ExprKind::Sort(l.clone()))
    }
    /// A constant with its universe arguments.
    pub fn constant(&mut self, n: &Name, levels: &[Level]) -> Expr {
        self.mk(ExprKind::Const(n.clone(), levels.into()))
    }
    /// `f a`.
    pub fn app(&mut self, f: Expr, a: Expr) -> Expr {
        self.mk(ExprKind::App(f, a))
    }
    /// `f a1 ... an`.
    pub fn apps(&mut self, f: Expr, args: &[Expr]) -> Expr {
        args.iter().fold(f, |acc, &a| self.app(acc, a))
    }
    /// `fun (name : ty) => body`.
    pub fn lam(&mut self, b: Binder, ty: Expr, body: Expr) -> Expr {
        self.mk(ExprKind::Lam(Meta(b), ty, body))
    }
    /// `(name : ty) -> body`.
    pub fn pi(&mut self, b: Binder, ty: Expr, body: Expr) -> Expr {
        self.mk(ExprKind::Pi(Meta(b), ty, body))
    }
    /// `let name : ty := val; body`.
    pub fn let_e(&mut self, b: Binder, ty: Expr, val: Expr, body: Expr) -> Expr {
        self.mk(ExprKind::Let(Meta(b), ty, val, body))
    }
    /// A natural-number literal.
    pub fn nat_lit(&mut self, n: BigInteger) -> Expr {
        self.mk(ExprKind::Lit(Literal::Nat(n)))
    }
    /// A string literal.
    pub fn str_lit(&mut self, s: &str) -> Expr {
        self.mk(ExprKind::Lit(Literal::Str(s.to_string())))
    }
    /// Projection of field `idx` out of `e`, a value of structure `strukt`.
    pub fn proj(&mut self, strukt: &Name, idx: u32, e: Expr) -> Expr {
        self.mk(ExprKind::Proj(strukt.clone(), idx, e))
    }

    /// The head of an application spine: `get_app_fn(f a b) = f`.
    pub fn get_app_fn(&self, mut e: Expr) -> Expr {
        while let ExprKind::App(f, _) = self.kind(e) {
            e = *f;
        }
        e
    }
    /// The arguments of an application spine, in order: `get_app_args(f a b) = [a, b]`.
    pub fn get_app_args(&self, mut e: Expr) -> Vec<Expr> {
        let mut args = Vec::new();
        while let ExprKind::App(f, a) = self.kind(e) {
            args.push(*a);
            e = *f;
        }
        args.reverse();
        args
    }

    // ---- the one traversal ------------------------------------------------------

    /// Rebuild `e` bottom-up, calling `f(store, subterm, depth)` on each subterm first;
    /// `Some(r)` replaces the subterm by `r` without descending, `None` descends.
    /// `depth` counts the binders crossed, which is how de Bruijn operations know which
    /// indices are "theirs". Results are memoised per (subterm, depth), so shared
    /// subterms are processed once. Unchanged nodes keep their handle (and so their
    /// binder names).
    fn replace(
        &mut self,
        e: Expr,
        depth: u32,
        cache: &mut Cache,
        f: &mut dyn FnMut(&mut ExprStore, Expr, u32) -> Option<Expr>,
    ) -> Expr {
        if let Some(&r) = cache.get(&(e, depth)) {
            return r;
        }
        if self.node_cap.is_some_and(|c| self.nodes.len() > c) {
            self.over_budget = true;
            return e;
        }
        let r = match f(self, e, depth) {
            Some(r) => r,
            None => match self.kind(e).clone() {
                ExprKind::App(a, b) => {
                    let (a2, b2) = (
                        self.replace(a, depth, cache, f),
                        self.replace(b, depth, cache, f),
                    );
                    if (a2, b2) == (a, b) {
                        e
                    } else {
                        self.app(a2, b2)
                    }
                }
                ExprKind::Lam(m, t, b) | ExprKind::Pi(m, t, b) => {
                    let (t2, b2) = (
                        self.replace(t, depth, cache, f),
                        self.replace(b, depth + 1, cache, f),
                    );
                    if (t2, b2) == (t, b) {
                        e
                    } else if matches!(self.kind(e), ExprKind::Lam(..)) {
                        self.mk(ExprKind::Lam(m, t2, b2))
                    } else {
                        self.mk(ExprKind::Pi(m, t2, b2))
                    }
                }
                ExprKind::Let(m, t, v, b) => {
                    let t2 = self.replace(t, depth, cache, f);
                    let v2 = self.replace(v, depth, cache, f);
                    let b2 = self.replace(b, depth + 1, cache, f);
                    if (t2, v2, b2) == (t, v, b) {
                        e
                    } else {
                        self.mk(ExprKind::Let(m, t2, v2, b2))
                    }
                }
                ExprKind::Proj(n, i, x) => {
                    let x2 = self.replace(x, depth, cache, f);
                    if x2 == x {
                        e
                    } else {
                        self.mk(ExprKind::Proj(n, i, x2))
                    }
                }
                ExprKind::BVar(_)
                | ExprKind::FVar(_)
                | ExprKind::Sort(_)
                | ExprKind::Const(..)
                | ExprKind::Lit(_) => e,
            },
        };
        cache.insert((e, depth), r);
        r
    }

    fn run(
        &mut self,
        e: Expr,
        f: &mut dyn FnMut(&mut ExprStore, Expr, u32) -> Option<Expr>,
    ) -> Expr {
        self.replace(e, 0, &mut Cache::new(), f)
    }

    // ---- de Bruijn operations ---------------------------------------------------

    /// Add `d` to every loose bound variable with index `>= s` (indices below `s` are
    /// left alone). Used when a term is moved under `d` more binders: forgetting it
    /// makes the term's free references point at the new binders instead (capture).
    pub fn lift_loose_bvars(&mut self, e: Expr, s: u32, d: u32) -> Expr {
        if d == 0 || self.loose_bvar_range(e) <= s {
            return e;
        }
        self.run(e, &mut |st, x, depth| {
            if st.loose_bvar_range(x) <= shifted(s, depth) {
                return Some(x);
            }
            match st.kind(x) {
                ExprKind::BVar(i) => {
                    let i = *i;
                    Some(st.bvar(shifted(i, d)))
                }
                _ => None,
            }
        })
    }

    /// Subtract `d` from every loose bound variable with index `>= s`. The caller
    /// guarantees none has index in `s - d .. s` (they would go negative); this is the
    /// inverse of [`ExprStore::lift_loose_bvars`] and is used after a binder is removed.
    pub fn lower_loose_bvars(&mut self, e: Expr, s: u32, d: u32) -> Expr {
        assert!(s >= d, "lower_loose_bvars: s must be at least d");
        if d == 0 || self.loose_bvar_range(e) <= s {
            return e;
        }
        self.run(e, &mut |st, x, depth| {
            if st.loose_bvar_range(x) <= shifted(s, depth) {
                return Some(x);
            }
            match st.kind(x) {
                ExprKind::BVar(i) => {
                    let i = *i;
                    Some(st.bvar(i - d))
                }
                _ => None,
            }
        })
    }

    fn instantiate_with(&mut self, e: Expr, subst: &[Expr], rev: bool) -> Expr {
        let n = checked_u32(subst.len(), "substitution length");
        if n == 0 || !self.has_loose_bvars(e) {
            return e;
        }
        self.run(e, &mut |st, x, depth| {
            if st.loose_bvar_range(x) <= depth {
                return Some(x); // nothing loose at this depth: not ours to touch
            }
            let i = match st.kind(x) {
                ExprKind::BVar(i) => *i,
                _ => return None,
            };
            let j = i - depth; // index relative to the binders being instantiated
            Some(if j < n {
                let v = if rev {
                    subst[(n - 1 - j) as usize]
                } else {
                    subst[j as usize]
                };
                // The substituted term sits `depth` binders deeper than where it came from.
                st.lift_loose_bvars(v, 0, depth)
            } else {
                st.bvar(i - n) // the removed binders no longer count
            })
        })
    }

    /// Replace loose `#0 .. #n-1` by `subst[0] .. subst[n-1]` and shift the remaining
    /// loose indices down by `n` (the `n` binders have been consumed). Substituted
    /// terms are lifted over the binders they land under, so they cannot be captured.
    pub fn instantiate(&mut self, e: Expr, subst: &[Expr]) -> Expr {
        self.instantiate_with(e, subst, false)
    }
    /// Like [`ExprStore::instantiate`] but `#0` becomes the *last* element: the order in
    /// which binders were opened. It is the exact inverse of [`ExprStore::abstract_fvars`].
    pub fn instantiate_rev(&mut self, e: Expr, subst: &[Expr]) -> Expr {
        self.instantiate_with(e, subst, true)
    }
    /// Instantiate `#0` with a single term (beta-reduction's substitution step).
    pub fn instantiate1(&mut self, e: Expr, v: Expr) -> Expr {
        self.instantiate_with(e, &[v], false)
    }

    /// Turn free variables into bound ones: `fvars[i]` becomes `#(n-1-i)` (so the last
    /// variable is the innermost, `#0`), offset by the binders crossed on the way down.
    /// Wrapping the result in `n` binders and instantiating with
    /// [`ExprStore::instantiate_rev`] gives back the original term.
    pub fn abstract_fvars(&mut self, e: Expr, fvars: &[FVarId]) -> Expr {
        let n = checked_u32(fvars.len(), "fvar list length");
        if n == 0 || !self.has_fvar(e) {
            return e;
        }
        self.run(e, &mut |st, x, depth| {
            if !st.has_fvar(x) {
                return Some(x);
            }
            match st.kind(x) {
                ExprKind::FVar(id) => {
                    let id = *id;
                    Some(match fvars.iter().position(|f| *f == id) {
                        Some(i) => st.bvar(shifted(depth, n - 1 - i as u32)),
                        None => x,
                    })
                }
                _ => None,
            }
        })
    }

    /// Replace universe parameters throughout `e`: `params[i]` becomes `levels[i]`. The
    /// two slices must have equal length, otherwise `ArityMismatch` (never a silent
    /// truncation, which would leave some parameter uninstantiated).
    pub fn instantiate_level_params(
        &mut self,
        e: Expr,
        params: &[Name],
        levels: &[Level],
    ) -> Result<Expr, ArityMismatch> {
        if params.len() != levels.len() {
            return Err(ArityMismatch {
                params: params.len(),
                levels: levels.len(),
            });
        }
        if params.is_empty() || !self.has_level_param(e) {
            return Ok(e);
        }
        Ok(self.run(e, &mut |st, x, _| {
            if !st.has_level_param(x) {
                return Some(x);
            }
            match st.kind(x).clone() {
                ExprKind::Sort(l) => Some(st.sort(&l.instantiate_unchecked(params, levels))),
                ExprKind::Const(n, ls) => {
                    let new: Vec<Level> = ls
                        .iter()
                        .map(|l| l.instantiate_unchecked(params, levels))
                        .collect();
                    Some(st.constant(&n, &new))
                }
                _ => None,
            }
        }))
    }

    /// Does `e` contain the loose bound variable `#i` (counted from outside `e`)?
    pub fn has_loose_bvar(&self, e: Expr, i: u32) -> bool {
        fn go(st: &ExprStore, e: Expr, i: u32, seen: &mut HashSet<(Expr, u32)>) -> bool {
            if st.loose_bvar_range(e) <= i || !seen.insert((e, i)) {
                return false; // no such index inside, or this exact query already ran
            }
            match st.kind(e) {
                ExprKind::BVar(j) => *j == i,
                ExprKind::App(a, b) => go(st, *a, i, seen) || go(st, *b, i, seen),
                ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => {
                    go(st, *t, i, seen) || go(st, *b, i + 1, seen)
                }
                ExprKind::Let(_, t, v, b) => {
                    go(st, *t, i, seen) || go(st, *v, i, seen) || go(st, *b, i + 1, seen)
                }
                ExprKind::Proj(_, _, x) => go(st, *x, i, seen),
                _ => false,
            }
        }
        go(self, e, i, &mut HashSet::new())
    }
}
