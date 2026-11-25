//! Definitional equality: when are two terms "the same" for the type checker?
//!
//! `a ≡ b` is the smallest relation that is reflexive, symmetric, transitive, a
//! congruence, and closed under beta, zeta and delta (this slice). The checker needs it
//! for every application (`argument type ≡ domain`) and every declaration (`inferred type
//! ≡ declared type`), so it is where "accept" is decided. A false positive here is a
//! soundness bug; a false negative only rejects a valid term (and is what the missing
//! rules below cause).
//!
//! # The algorithm, in order
//!
//! 1. **Identical handles.** Terms are hash-consed, so equal handles are equal terms.
//! 2. **Cache.** A previous verdict for the pair (either order) is reused.
//! 3. **Structural cases**:
//!    * `Sort u` vs `Sort v`: equal iff the levels are equivalent (`levels_equiv`).
//!    * `c.{ls}` vs `c.{ms}`: same name and pairwise-equivalent levels is a yes; otherwise
//!      fall through, because `c` may unfold to something that ignores the levels.
//!    * `fun`/`fun` and `Pi`/`Pi`: binder types must be `≡`, then the bodies are compared
//!      with *one shared fresh variable* in place of the bound one. Sharing is essential:
//!      comparing `fun x => x` with `fun y => c` must not equate `x` with `c`.
//!    * `f a1..an` vs `g b1..bn`: if `f ≡ g` and each `ai ≡ bi`, equal (congruence).
//!      If this fails we do not answer "no": the two sides may still agree after
//!      unfolding (`f 1` and `g 2` may both unfold to `7`).
//! 4. **Proof irrelevance** (see below): two proofs of the same proposition are equal.
//!    This comes *before* reduction on purpose: there is no need to normalise a proof
//!    to compare it, and proofs are the terms most likely to be huge.
//! 5. **Otherwise reduce.** Put both sides in whnf; if either changed, compare the
//!    results (a second round starts from whnf terms, so reduction is not retried
//!    again). If neither changed, try **eta** (below); failing that the terms are
//!    different: answer no.
//!
//! # Proof irrelevance
//!
//! ```text
//!   Γ ⊢ a : P    Γ ⊢ P : Prop    Γ ⊢ b : Q    P ≡ Q
//!   ──────────────────────────────────────────────────
//!                    a ≡ b
//! ```
//!
//! Sound because `Prop` is proof irrelevant in Lean's theory: nothing can observe *which*
//! proof of a proposition it has, *provided* proofs cannot be eliminated into data. That
//! proviso is exactly the large-elimination restriction enforced when recursors are
//! generated (`inductive.rs`): `Or.rec` cannot target `Type`, or `Or.inl h ≡ Or.inr h`
//! would give `true ≡ false`. We are as strict as Lean: the *type's* type must be `Prop`
//! (checked with whnf), and the two types must be `≡` (a mismatch is not an equality
//! verdict, just "no conclusion", and the other rules continue). Without the `Prop`
//! test, any two terms of one type would be equal and every theorem about data
//! (`true ≠ false`) would be "provable" in reverse; without the type test, `a : P` and
//! `b : Q` would be equal whenever both are proofs. `Sort` and `Pi` terms are skipped: a
//! type is never a proof (its type is a `Sort`, which is not a proposition), so this is
//! only a shortcut.
//!
//! # Eta for functions
//!
//! ```text
//!   Γ ⊢ f : (x : A) -> B           f is not a lambda
//!   ─────────────────────────────────────────────────
//!        (fun x : A => f x) ≡ f
//! ```
//!
//! When one side is a lambda and the other is not (and cannot be reduced any further),
//! the other side's type is put in whnf; if it is a Pi `(x : A) -> B`, the other side is
//! replaced by `fun x : A => other x` and the two lambdas are compared. Sound because
//! eta is a theorem of the theory for anything of function type, and the expansion is
//! built from the type the kernel itself inferred. Without it `fun x => f x` and `f` are
//! different, so e.g. a rewrite by `fun x => f x = f` fails (incomplete, not unsound).
//!
//! # Not implemented yet (slice 3b+), and what goes wrong without it
//!
//! * **Lazy delta with hints.** We unfold *both* sides completely to whnf rather than
//!   unfolding the "taller" side first. Correct, but it can do exponentially more work
//!   than Lean's kernel (`f x ≡ f y` unfolds `f` twice instead of comparing `x ≡ y`);
//!   the congruence shortcut in step 3 and the cache soften this, and the work limits
//!   keep it safe.
//! * **Unit-like types** (every inhabitant of a one-constructor, no-field type is equal):
//!   two such values are only `≡` if they reduce to the same thing.
//! * **Structure eta, projections, literals** (`5 ≡ Nat.succ 4`).
//!
//! Every omission makes the checker answer *no* more often, never *yes*.
//!
//! # Caching
//!
//! Verdicts (both yes and no) are cached by handle pair; see the `checker` module for why
//! that is sound. A `ResourceLimit` error is never cached.

use crate::checker::{KResult, TypeChecker};
use crate::error::KernelError;
use crate::expr::{Binder, Expr, ExprKind};
use crate::level::{levels_equiv, Level};

impl TypeChecker<'_> {
    /// Are `a` and `b` definitionally equal?
    pub fn is_def_eq(&mut self, a: Expr, b: Expr) -> KResult<bool> {
        self.own(a)?;
        self.own(b)?;
        if a == b {
            return Ok(true);
        }
        if let Some(&v) = self.eq_cache.get(&(a, b)).or(self.eq_cache.get(&(b, a))) {
            return Ok(v);
        }
        let v = self.guarded(&[a, b], |tc| tc.def_eq_core(a, b))?;
        self.eq_cache.insert((a, b), v);
        Ok(v)
    }

    fn def_eq_core(&mut self, a: Expr, b: Expr) -> KResult<bool> {
        match (self.store.kind(a).clone(), self.store.kind(b).clone()) {
            (ExprKind::Sort(l1), ExprKind::Sort(l2)) => return Ok(l1.is_equiv(&l2)),
            (ExprKind::Const(n1, ls1), ExprKind::Const(n2, ls2))
                if n1 == n2 && levels_equiv(&ls1, &ls2) =>
            {
                return Ok(true)
            }
            (ExprKind::Lam(..), ExprKind::Lam(..)) | (ExprKind::Pi(..), ExprKind::Pi(..)) => {
                return self.def_eq_binders(a, b);
            }
            (ExprKind::App(..), ExprKind::App(..)) if self.def_eq_apps(a, b)? => {
                return Ok(true);
            }
            _ => {}
        }
        if self.proof_irrel(a, b)? {
            return Ok(true);
        }
        let (wa, wb) = (self.whnf(a)?, self.whnf(b)?);
        if wa == a && wb == b {
            return self.eta(a, b);
        }
        self.is_def_eq(wa, wb)
    }

    /// `infer` for the *optional* rules (proof irrelevance, eta), which only ever add a
    /// "yes" verdict. If the term cannot be typed (an undeclared constant, a literal or
    /// projection the kernel does not support yet, ...) the rule simply does not apply:
    /// the comparison goes on exactly as it did before the rule existed, which is the
    /// conservative side. Limit and internal errors still propagate (an exhausted budget
    /// must stay an error, never turn into an answer).
    fn try_infer(&mut self, e: Expr) -> KResult<Option<Expr>> {
        match self.infer(e) {
            Ok(t) => Ok(Some(t)),
            Err(
                err @ (KernelError::ResourceLimit(_)
                | KernelError::ForeignExpr
                | KernelError::Internal(_)),
            ) => Err(err),
            Err(_) => Ok(None),
        }
    }

    /// Is `ty` a proposition, i.e. is its type `Sort 0` (up to whnf and level equivalence)?
    fn is_prop(&mut self, ty: Expr) -> KResult<bool> {
        let Some(s) = self.try_infer(ty)? else {
            return Ok(false);
        };
        let w = self.whnf(s)?;
        Ok(matches!(self.store.kind(w), ExprKind::Sort(l) if l.is_equiv(&Level::zero())))
    }

    /// Proof irrelevance: `true` only when `a` and `b` are proofs of `≡` propositions.
    /// "No conclusion" is `false`, never an inequality verdict.
    fn proof_irrel(&mut self, a: Expr, b: Expr) -> KResult<bool> {
        let skip = |tc: &Self, e| {
            matches!(
                tc.store.kind(e),
                ExprKind::Sort(_) | ExprKind::Pi(..) | ExprKind::Lit(_)
            )
        };
        if skip(self, a) || skip(self, b) {
            return Ok(false);
        }
        let Some(ta) = self.try_infer(a)? else {
            return Ok(false);
        };
        if !self.is_prop(ta)? {
            return Ok(false);
        }
        match self.try_infer(b)? {
            Some(tb) => self.is_def_eq(ta, tb),
            None => Ok(false),
        }
    }

    /// Eta for functions, both directions (see the module documentation).
    fn eta(&mut self, a: Expr, b: Expr) -> KResult<bool> {
        for (f, g) in [(a, b), (b, a)] {
            if !matches!(self.store.kind(f), ExprKind::Lam(..))
                || matches!(self.store.kind(g), ExprKind::Lam(..))
            {
                continue;
            }
            let Some(gt) = self.try_infer(g)? else {
                continue;
            };
            let pi = self.whnf(gt)?;
            let ExprKind::Pi(m, dom, _) = self.store.kind(pi).clone() else {
                continue;
            };
            // `g` is closed (no loose bound variables), so `g #0` needs no lifting.
            let x = self.store.bvar(0);
            let body = self.store.app(g, x);
            let expanded = self.store.lam(m.0, dom, body);
            let expanded = self.bounded(expanded)?;
            return self.is_def_eq(f, expanded);
        }
        Ok(false)
    }

    /// Congruence: same number of arguments, `≡` heads, pairwise `≡` arguments.
    fn def_eq_apps(&mut self, a: Expr, b: Expr) -> KResult<bool> {
        let (ha, hb) = (self.store.get_app_fn(a), self.store.get_app_fn(b));
        let (xs, ys) = (self.store.get_app_args(a), self.store.get_app_args(b));
        if xs.len() != ys.len() || !self.is_def_eq(ha, hb)? {
            return Ok(false);
        }
        for (x, y) in xs.into_iter().zip(ys) {
            if !self.is_def_eq(x, y)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// `(kind is Pi, binder, type, body)` of a lambda or Pi.
    fn binder_parts(&self, e: Expr) -> Option<(bool, Binder, Expr, Expr)> {
        match self.store.kind(e) {
            ExprKind::Lam(m, t, b) => Some((false, m.0.clone(), *t, *b)),
            ExprKind::Pi(m, t, b) => Some((true, m.0.clone(), *t, *b)),
            _ => None,
        }
    }

    /// Compare two telescopes of the same binder kind, one shared fresh variable per
    /// binder, in a loop (so a long telescope costs no recursion depth).
    fn def_eq_binders(&mut self, a: Expr, b: Expr) -> KResult<bool> {
        let (mut x, mut y) = (a, b);
        let mut vars: Vec<Expr> = Vec::new();
        while let (Some((p1, m, t1, b1)), Some((p2, _, t2, b2))) =
            (self.binder_parts(x), self.binder_parts(y))
        {
            if p1 != p2 {
                break;
            }
            let t1 = self.store.instantiate_rev(t1, &vars);
            let t1 = self.bounded(t1)?;
            let t2 = self.store.instantiate_rev(t2, &vars);
            let t2 = self.bounded(t2)?;
            if !self.is_def_eq(t1, t2)? {
                return Ok(false);
            }
            let (_, v) = self.new_local(&m.name, t1, None)?;
            vars.push(v);
            (x, y) = (b1, b2);
        }
        let xo = self.store.instantiate_rev(x, &vars);
        let xo = self.bounded(xo)?;
        let yo = self.store.instantiate_rev(y, &vars);
        let yo = self.bounded(yo)?;
        self.is_def_eq(xo, yo)
    }
}
