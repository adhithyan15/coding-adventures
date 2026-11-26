//! Type inference and checking: the typing rules of Lean 4's kernel, one per kind of
//! expression.
//!
//! Notation: `Γ ⊢ e : T` says "in local context Γ, `e` has type `T`". `T ≡ U` is
//! definitional equality ([`TypeChecker::is_def_eq`]), `whnf` is weak-head normal form.
//! There is deliberately no "infer only" fast path in this slice: `infer` *always*
//! checks every sub-term, so a term that `infer` returns a type for is fully
//! well-typed. (Lean's kernel has an infer-only mode that skips argument checks for
//! terms already known to be well-typed; that is an optimisation whose bugs would be
//! soundness bugs, so it waits until there is an oracle to test it against.)
//!
//! # The rules
//!
//! ```text
//!   Sort    ─────────────────────────      u declared
//!           Γ ⊢ Sort u : Sort (succ u)
//!
//!   Const   c.{ps} : T declared, |ls| = |ps|, ls declared
//!           ─────────────────────────────
//!           Γ ⊢ c.{ls} : T[ps := ls]
//!
//!   FVar    (x : T) ∈ Γ
//!           ─────────────
//!           Γ ⊢ x : T
//!
//!   App     Γ ⊢ f : F    whnf F = (x : A) → B    Γ ⊢ a : A'    A' ≡ A
//!           ───────────────────────────────────────────────────────────
//!           Γ ⊢ f a : B[x := a]
//!
//!   Lam     Γ ⊢ A : Sort u    Γ, x:A ⊢ b : B
//!           ──────────────────────────────────
//!           Γ ⊢ (fun x : A => b) : (x : A) → B
//!
//!   Pi      Γ ⊢ A : Sort u    Γ, x:A ⊢ B : Sort v
//!           ───────────────────────────────────────
//!           Γ ⊢ ((x : A) → B) : Sort (imax u v)
//!
//!   Let     Γ ⊢ T : Sort u    Γ ⊢ v : T'    T' ≡ T    Γ, x:T:=v ⊢ b : B
//!           ─────────────────────────────────────────────────────────────
//!           Γ ⊢ (let x : T := v; b) : B[x := v]
//!
//!   Lit     (not yet: rejected as Unsupported, see below)
//! ```
//!
//! What goes wrong if a premise is dropped: without `Sort` in Lam/Pi the binder type
//! could be any term (`fun (x : 5) => x`), so "types" are not types; without `T' ≡ T`
//! in App or Let, `f (a : Nat)` is accepted where `f` wants a `Prop`, and any proposition
//! has a proof; without the `imax` in Pi, impredicativity of `Prop` is lost or
//! over-granted, and Girard's paradox follows; without the arity check on constants, a
//! universe-polymorphic constant is instantiated with missing levels and ends up at a
//! universe lower than it should be.
//!
//! # Design choices not fixed by the spec
//!
//! * **`let` in an inferred type.** Like Lean 4's kernel, the type of `let x := v; b` is
//!   built with a `let` binder when `x` occurs in the body's type (and the binder is
//!   simply dropped otherwise), rather than substituting `v` eagerly. The two are
//!   definitionally equal (zeta); keeping the `let` keeps types small.
//! * **Literals** are `Unsupported`, whatever the environment holds. Typing `5` as the
//!   constant `Nat` is only sound if `Nat` *is* the inductive type `Nat`, and a name
//!   check cannot show that: an environment may say `def Nat : Prop := False`, after
//!   which `theorem bad : False := 5` would be accepted. VER11a §3.3 requires the
//!   primitive definitions (`Nat`, `String`) to be validated first; inductive types now
//!   exist (slice 3a), but the validation itself is a later slice, so literals stay
//!   `Unsupported`.
//! * **Proj** is `Unsupported` until structures (structure detection, projections and
//!   structure eta) are implemented in slice 3b.

use crate::checker::{KResult, TypeChecker};
use crate::error::KernelError;
use crate::expr::{Binder, Expr, ExprKind, FVarId};
use crate::level::Level;
use crate::name::Name;

impl TypeChecker<'_> {
    /// Infer (and thereby check) the type of `e`.
    pub fn infer(&mut self, e: Expr) -> KResult<Expr> {
        if let Some(&t) = self.infer_cache.get(&e) {
            return Ok(t);
        }
        let t = self.guarded(&[e], |tc| tc.infer_core(e))?;
        let t = self.bounded(t)?;
        self.infer_cache.insert(e, t);
        Ok(t)
    }

    /// Check that `e` has type `ty` (up to definitional equality).
    pub fn check(&mut self, e: Expr, ty: Expr) -> KResult<()> {
        let found = self.infer(e)?;
        if self.is_def_eq(found, ty)? {
            Ok(())
        } else {
            Err(KernelError::TypeMismatch {
                expected: ty,
                found,
            })
        }
    }

    /// `ty` must be (reduce to) a `Sort`; return its level. `ty` is typically the
    /// inferred type of a type: `ensure_sort(infer(A))` is "A is a type".
    pub fn ensure_sort(&mut self, ty: Expr) -> KResult<Level> {
        self.own(ty)?;
        if let ExprKind::Sort(l) = self.store.kind(ty) {
            return Ok(l.clone());
        }
        let w = self.whnf(ty)?;
        match self.store.kind(w) {
            ExprKind::Sort(l) => Ok(l.clone()),
            _ => Err(KernelError::NotASort(ty)),
        }
    }

    /// `ty` must be (reduce to) a `Pi`; return the `Pi` expression.
    pub fn ensure_pi(&mut self, ty: Expr) -> KResult<Expr> {
        self.own(ty)?;
        if matches!(self.store.kind(ty), ExprKind::Pi(..)) {
            return Ok(ty);
        }
        let w = self.whnf(ty)?;
        match self.store.kind(w) {
            ExprKind::Pi(..) => Ok(w),
            _ => Err(KernelError::NotAPi(ty)),
        }
    }

    fn infer_core(&mut self, e: Expr) -> KResult<Expr> {
        match self.store.kind(e).clone() {
            // A loose bound variable means the term is not closed: nothing to say.
            ExprKind::BVar(i) => Err(KernelError::LooseBVar(i)),
            ExprKind::FVar(id) => match self.lctx.get(id) {
                Some(d) => Ok(d.ty),
                None => Err(KernelError::UnknownFVar),
            },
            ExprKind::Sort(l) => {
                self.check_level(&l)?;
                Ok(self.store.sort(&Level::succ(&l)))
            }
            ExprKind::Const(n, ls) => self.infer_const(&n, &ls),
            ExprKind::App(..) => self.infer_app(e),
            ExprKind::Lam(..) => self.infer_lam(e),
            ExprKind::Pi(..) => self.infer_pi(e),
            ExprKind::Let(..) => self.infer_let(e),
            ExprKind::Lit(_) => Err(KernelError::Unsupported(
                "literals need primitive-definition validation (VER11a §3.3), slice 3b+",
            )),
            ExprKind::Proj(..) => Err(KernelError::Unsupported(
                "projections need structures (slice 3b)",
            )),
        }
    }

    fn infer_const(&mut self, n: &Name, ls: &[Level]) -> KResult<Expr> {
        let decls = self.decls;
        let Some(d) = decls.get(n) else {
            return Err(KernelError::UnknownConstant(n.clone()));
        };
        if ls.len() != d.level_params.len() {
            return Err(KernelError::LevelArityMismatch {
                name: n.clone(),
                expected: d.level_params.len(),
                found: ls.len(),
            });
        }
        for l in ls {
            self.check_level(l)?;
        }
        self.store
            .instantiate_level_params(d.ty, &d.level_params, ls)
            .map_err(|_| KernelError::Internal("level arity"))
    }

    /// Rule App, over a whole spine `f a1 ... an` at once so that a long spine costs a
    /// loop, not recursion depth.
    fn infer_app(&mut self, e: Expr) -> KResult<Expr> {
        let f = self.store.get_app_fn(e);
        let args = self.store.get_app_args(e);
        let mut fty = self.infer(f)?;
        for a in args {
            let pi = self.ensure_pi(fty)?;
            let ExprKind::Pi(_, dom, body) = self.store.kind(pi).clone() else {
                return Err(KernelError::NotAPi(pi));
            };
            let aty = self.infer(a)?;
            if !self.is_def_eq(aty, dom)? {
                return Err(KernelError::TypeMismatch {
                    expected: dom,
                    found: aty,
                });
            }
            let next = self.store.instantiate1(body, a);
            fty = self.bounded(next)?;
        }
        Ok(fty)
    }

    /// Open a telescope of consecutive binders of one kind (`want_pi` selects Pi or
    /// Lam), checking each binder type is a type. Returns the binder data, the opened
    /// binder types, the fvars and the opened body, and the level of each binder type.
    #[allow(clippy::type_complexity)]
    fn open_telescope(
        &mut self,
        e: Expr,
        want_pi: bool,
    ) -> KResult<(
        Vec<Binder>,
        Vec<Expr>,
        Vec<FVarId>,
        Vec<Expr>,
        Expr,
        Vec<Level>,
    )> {
        let (mut binders, mut tys, mut ids, mut xs, mut levels) =
            (vec![], vec![], vec![], vec![], vec![]);
        let mut cur = e;
        loop {
            let (m, t, b) = match self.store.kind(cur).clone() {
                ExprKind::Lam(m, t, b) if !want_pi => (m, t, b),
                ExprKind::Pi(m, t, b) if want_pi => (m, t, b),
                _ => break,
            };
            let t = self.store.instantiate_rev(t, &xs);
            let t = self.bounded(t)?;
            let sort_of_t = self.infer(t)?;
            levels.push(self.ensure_sort(sort_of_t)?);
            let (id, x) = self.new_local(&m.0.name, t, None)?;
            binders.push(m.0);
            tys.push(t);
            ids.push(id);
            xs.push(x);
            cur = b;
        }
        let body = self.store.instantiate_rev(cur, &xs);
        let body = self.bounded(body)?;
        Ok((binders, tys, ids, xs, body, levels))
    }

    /// Rule Lam. The body is inferred under fresh fvars, then the fvars are abstracted
    /// back and the binders wrapped round the result as Pis.
    fn infer_lam(&mut self, e: Expr) -> KResult<Expr> {
        let (binders, tys, ids, _xs, body, _levels) = self.open_telescope(e, false)?;
        let bty = self.infer(body)?;
        let mut r = self.store.abstract_fvars(bty, &ids);
        for i in (0..ids.len()).rev() {
            let ty_i = self.store.abstract_fvars(tys[i], &ids[..i]);
            r = self.store.pi(binders[i].clone(), ty_i, r);
        }
        self.bounded(r)
    }

    /// Rule Pi: the result lives in `Sort (imax u1 (imax u2 (... v)))`.
    fn infer_pi(&mut self, e: Expr) -> KResult<Expr> {
        let (_b, _t, _ids, _xs, body, levels) = self.open_telescope(e, true)?;
        let body_sort = self.infer(body)?;
        let mut lvl = self.ensure_sort(body_sort)?;
        for u in levels.iter().rev() {
            lvl = Level::imax(u, &lvl);
        }
        Ok(self.store.sort(&lvl))
    }

    /// Rule Let, over a chain of nested lets.
    fn infer_let(&mut self, e: Expr) -> KResult<Expr> {
        let (mut binders, mut tys, mut vals, mut ids, mut xs) =
            (vec![], vec![], vec![], vec![], vec![]);
        let mut cur = e;
        while let ExprKind::Let(m, t, v, b) = self.store.kind(cur).clone() {
            let t = self.store.instantiate_rev(t, &xs);
            let t = self.bounded(t)?;
            let v = self.store.instantiate_rev(v, &xs);
            let v = self.bounded(v)?;
            let sort_of_t = self.infer(t)?;
            self.ensure_sort(sort_of_t)?;
            let vty = self.infer(v)?;
            if !self.is_def_eq(vty, t)? {
                return Err(KernelError::TypeMismatch {
                    expected: t,
                    found: vty,
                });
            }
            let (id, x) = self.new_local(&m.0.name, t, Some(v))?;
            binders.push(m.0);
            tys.push(t);
            vals.push(v);
            ids.push(id);
            xs.push(x);
            cur = b;
        }
        let body = self.store.instantiate_rev(cur, &xs);
        let body = self.bounded(body)?;
        let mut r = self.infer(body)?;
        // Wrap from the inside out. The binder types and values still mention the outer
        // let-variables as fvars; they are abstracted when their own turn comes.
        for i in (0..ids.len()).rev() {
            let abs = self.store.abstract_fvars(r, &ids[i..=i]);
            r = if self.store.has_loose_bvar(abs, 0) {
                self.store.let_e(binders[i].clone(), tys[i], vals[i], abs)
            } else {
                self.store.lower_loose_bvars(abs, 1, 1)
            };
        }
        self.bounded(r)
    }
}
