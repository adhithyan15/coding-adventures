//! The type checker's state, and the resource limits that make it safe on hostile input.
//!
//! # Why limits are part of soundness
//!
//! The kernel reads terms it did not write. Three things can go wrong without any
//! logical error:
//!
//! * **Deep recursion.** A term nested a hundred thousand levels deep, or an
//!   ill-typed term that keeps reducing into a deeper one, would overflow the native
//!   stack: a crash, which for a trusted checker is a denial of service at best.
//! * **Endless reduction.** `(fun x => x x) (fun x => x x)` is not well typed, but the
//!   checker reduces before it knows that, and would loop forever.
//! * **Exponential work without recursion.** Delta unfolding can double the size of a
//!   term per step (`f (n+1) := f n -> f n`), so even a loop that never recurses deeply
//!   can need `2^40` steps or gigabytes of memory.
//!
//! So every entry into `infer`, `whnf` and `is_def_eq` passes through [`TypeChecker::guarded`],
//! which enforces four explicit limits, all reported as
//! [`KernelError::ResourceLimit`] (rejection, never acceptance):
//!
//! | limit | default | bounds |
//! |-------|---------|--------|
//! | `max_call_depth` | 200 | nesting of `infer` / `whnf` / `is_def_eq` calls (the native stack) |
//! | `max_term_depth` | 1000 | structural depth of any term the checker looks at or builds, so the store's recursive traversals (`instantiate`, `abstract`) are bounded too |
//! | `max_steps` | 2,000,000 | total work: calls plus reduction steps, per checker |
//! | `max_new_nodes` | 4,000,000 | expressions interned since the checker was made (memory); also enforced *inside* each store traversal through `ExprStore::set_node_cap` |
//! | level depth / size | 128 / 512 | nesting and tree size of every universe level in a `Sort` or constant (`MAX_LEVEL_DEPTH`, `MAX_LEVEL_SIZE` in `level.rs`) |
//!
//! Binder telescopes and application spines are processed by loops, not recursion, so a
//! long `f a1 ... a900` costs depth 1, not 900 (but its *term depth* is still 900).
//! Real terms deeper than `max_term_depth` (e.g. very long list literals) are rejected
//! until a later slice moves the store's traversals to explicit work stacks.
//!
//! # Caches
//!
//! `infer`, `whnf` and `is_def_eq` results are cached by expression handle. This is sound
//! because (a) expressions are hash-consed, so a handle is a term, (b) free-variable ids
//! are never reused and their local-context entries never change, so a term mentioning
//! an fvar means the same thing for the checker's whole life, and (c) the environment is
//! append-only and borrowed immutably while a checker exists. The caches are also what
//! turns the exponential delta chain into linear work.

use crate::env::Declaration;
use crate::error::KernelError;
use crate::expr::{Expr, ExprKind, ExprStore, FVarId};
use crate::lctx::{LocalContext, LocalDecl};
use crate::level::Level;
use crate::name::Name;
use std::collections::HashMap;

/// The documented resource limits (see the module documentation).
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// Maximum nesting of checker calls.
    pub max_call_depth: u32,
    /// Maximum structural depth of a term.
    pub max_term_depth: u32,
    /// Maximum total steps per checker.
    pub max_steps: u64,
    /// Maximum expressions interned per checker.
    pub max_new_nodes: usize,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_call_depth: 200,
            max_term_depth: 1000,
            max_steps: 2_000_000,
            max_new_nodes: 4_000_000,
        }
    }
}

/// Result of every checker operation.
pub type KResult<T> = Result<T, KernelError>;

/// A type checker over a borrowed environment. Make one with
/// [`crate::Environment::type_checker`].
pub struct TypeChecker<'a> {
    pub(crate) store: &'a mut ExprStore,
    pub(crate) decls: &'a HashMap<Name, Declaration>,
    pub(crate) lctx: LocalContext,
    level_params: Vec<Name>,
    limits: Limits,
    depth: u32,
    steps: u64,
    start_nodes: usize,
    pub(crate) infer_cache: HashMap<Expr, Expr>,
    pub(crate) whnf_cache: HashMap<Expr, Expr>,
    pub(crate) eq_cache: HashMap<(Expr, Expr), bool>,
    /// Per term: (structural depth, every universe level inside is within limits).
    depth_cache: HashMap<Expr, (u32, bool)>,
}

/// The cap on interned nodes only lives while a checker does; afterwards the store is
/// unrestricted again.
impl Drop for TypeChecker<'_> {
    fn drop(&mut self) {
        self.store.set_node_cap(None);
    }
}

impl<'a> TypeChecker<'a> {
    pub(crate) fn new(
        store: &'a mut ExprStore,
        decls: &'a HashMap<Name, Declaration>,
        level_params: Vec<Name>,
    ) -> TypeChecker<'a> {
        let start_nodes = store.len();
        store.set_node_cap(Some(
            start_nodes.saturating_add(Limits::default().max_new_nodes),
        ));
        TypeChecker {
            store,
            decls,
            lctx: LocalContext::new(),
            level_params,
            limits: Limits::default(),
            depth: 0,
            steps: 0,
            start_nodes,
            infer_cache: HashMap::new(),
            whnf_cache: HashMap::new(),
            eq_cache: HashMap::new(),
            depth_cache: HashMap::new(),
        }
    }

    /// Replace the resource limits (tests use small ones to exercise each limit).
    pub fn with_limits(mut self, limits: Limits) -> TypeChecker<'a> {
        self.limits = limits;
        let cap = self.start_nodes.saturating_add(limits.max_new_nodes);
        self.store.set_node_cap(Some(cap));
        self
    }

    /// The expression store, for building terms to check.
    pub fn store(&mut self) -> &mut ExprStore {
        self.store
    }

    /// The local context (free variables the checker has introduced).
    pub fn lctx(&self) -> &LocalContext {
        &self.lctx
    }

    /// Count one unit of work; fail when a budget is exhausted.
    pub(crate) fn tick(&mut self) -> KResult<()> {
        self.steps += 1;
        if self.steps > self.limits.max_steps {
            return Err(KernelError::ResourceLimit("step budget exhausted"));
        }
        if self.store.budget_exceeded()
            || self.store.len().saturating_sub(self.start_nodes) > self.limits.max_new_nodes
        {
            return Err(KernelError::ResourceLimit("expression budget exhausted"));
        }
        Ok(())
    }

    /// Run `f` one call-level deeper, after checking the call-depth, work and term-depth
    /// limits for the operands `es`.
    pub(crate) fn guarded<T>(
        &mut self,
        es: &[Expr],
        f: impl FnOnce(&mut Self) -> KResult<T>,
    ) -> KResult<T> {
        if self.depth >= self.limits.max_call_depth {
            return Err(KernelError::ResourceLimit("recursion depth"));
        }
        self.tick()?;
        for &e in es {
            self.bounded(e)?;
        }
        self.depth += 1;
        let r = f(self);
        self.depth -= 1;
        r
    }

    /// Return `e` if its structural depth is within `max_term_depth`. Called on every
    /// operand and on every term the checker builds, before anything recurses into it.
    ///
    /// Three more things are checked here, because every term the checker touches passes
    /// through it: the handle belongs to this store (a foreign handle would panic), no
    /// store traversal has run past the node cap (its result would be meaningless), and
    /// every universe level inside a `Sort` or constant is within the level limits. The
    /// last one matters because levels nest too: a term-depth limit that ignores them lets
    /// `Sort (max (max (... v) v) v)`, a million levels deep, overflow the stack in
    /// level normalisation or comparison.
    pub(crate) fn bounded(&mut self, e: Expr) -> KResult<Expr> {
        if !self.store.owns(e) {
            return Err(KernelError::ForeignExpr);
        }
        if self.store.budget_exceeded() {
            return Err(KernelError::ResourceLimit("expression budget exhausted"));
        }
        let (depth, levels_ok) = self.term_depth(e);
        if !levels_ok {
            return Err(KernelError::ResourceLimit("universe level too large"));
        }
        if depth > self.limits.max_term_depth {
            return Err(KernelError::ResourceLimit("term nesting too deep"));
        }
        Ok(e)
    }

    /// Refuse a handle from another store.
    pub(crate) fn own(&self, e: Expr) -> KResult<()> {
        if self.store.owns(e) {
            Ok(())
        } else {
            Err(KernelError::ForeignExpr)
        }
    }

    /// Structural depth of `e` (a leaf is 1), computed with an explicit stack and
    /// memoised, so measuring a hostile 100,000-deep term cannot itself overflow.
    fn term_depth(&mut self, e: Expr) -> (u32, bool) {
        let mut stack = vec![e];
        while let Some(&x) = stack.last() {
            if self.depth_cache.contains_key(&x) {
                stack.pop();
                continue;
            }
            let (kids, own_levels_ok): (Vec<Expr>, bool) = match self.store.kind(x) {
                ExprKind::App(a, b) => (vec![*a, *b], true),
                ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => (vec![*t, *b], true),
                ExprKind::Let(_, t, v, b) => (vec![*t, *v, *b], true),
                ExprKind::Proj(_, _, y) => (vec![*y], true),
                ExprKind::Sort(l) => (vec![], l.is_within_limits()),
                ExprKind::Const(_, ls) => (vec![], ls.iter().all(Level::is_within_limits)),
                _ => (vec![], true),
            };
            let (mut deepest, mut ok) = (0u32, own_levels_ok);
            let mut pending = false;
            for k in kids {
                match self.depth_cache.get(&k) {
                    Some(&(d, o)) => {
                        deepest = deepest.max(d);
                        ok &= o;
                    }
                    None => {
                        stack.push(k);
                        pending = true;
                    }
                }
            }
            if !pending {
                self.depth_cache.insert(x, (deepest.saturating_add(1), ok));
                stack.pop();
            }
        }
        self.depth_cache
            .get(&e)
            .copied()
            .unwrap_or((u32::MAX, false))
    }

    /// Introduce a fresh local variable `name : ty` (optionally `:= value`), returning
    /// its id and the expression for it.
    pub(crate) fn new_local(
        &mut self,
        name: &Name,
        ty: Expr,
        value: Option<Expr>,
    ) -> KResult<(FVarId, Expr)> {
        let id = self.store.fresh_fvar_id();
        let x = self.store.fvar(id);
        self.lctx.insert(
            id,
            LocalDecl {
                name: name.clone(),
                ty,
                value,
            },
        )?;
        Ok((id, x))
    }

    /// Every universe parameter in `l` must be one the enclosing declaration declared.
    /// Without this, `Sort u` with an unbound `u` would be accepted and later
    /// instantiated to anything. Walks with an explicit stack (levels can be deep).
    pub(crate) fn check_level(&self, l: &Level) -> KResult<()> {
        if !l.is_within_limits() {
            return Err(KernelError::ResourceLimit("universe level too large"));
        }
        let mut stack = vec![l];
        while let Some(x) = stack.pop() {
            if !x.has_param() {
                continue;
            }
            match x.kind() {
                crate::level::LevelKind::Param(n) => {
                    if !self.level_params.contains(n) {
                        return Err(KernelError::UndeclaredLevelParam(n.clone()));
                    }
                }
                crate::level::LevelKind::Succ(a) => stack.push(a),
                crate::level::LevelKind::Max(a, b) | crate::level::LevelKind::IMax(a, b) => {
                    stack.push(a);
                    stack.push(b);
                }
                crate::level::LevelKind::Zero => {}
            }
        }
        Ok(())
    }
}
