//! Weak-head normal form: reduce a term until its outermost shape is stuck.
//!
//! "Weak-head" means: reduce only at the head of the term (the function position of an
//! application spine), and do not go inside binders or arguments. `whnf (f a b)` is
//! enough to learn whether the result is a `Pi`, a `Sort`, a stuck constant, and so on,
//! which is all the checker ever asks.
//!
//! # The rules implemented in this slice
//!
//! ```text
//!   beta    (fun x => b) a            ~>  b[x := a]
//!   zeta    let x : T := v; b         ~>  b[x := v]
//!   zeta    x    (x : T := v in Γ)    ~>  v          (a let-bound local variable)
//!   delta   c.{ls}   (definition)     ~>  value[ps := ls]
//! ```
//!
//! Delta unfolds **definitions only**. Axioms have no value. Theorems and opaque
//! definitions have one, but it is deliberately hidden: unfolding a theorem would make
//! the *content* of a proof matter to typing, so a proof change could change whether a
//! later declaration type-checks. Unfolding an opaque constant would break the promise
//! its author made. If we wrongly unfolded them, a declaration could be accepted that
//! relies on a proof's or an opaque body's internals; the test suite checks that
//! `whnf` leaves them alone.
//!
//! # Not implemented yet (slice 3), and what goes wrong without it
//!
//! * **iota** (recursor on a constructor) and **projections** of constructors: without
//!   them `Nat.rec` and structure fields never compute, so any statement whose truth
//!   needs computation is rejected. Incomplete, not unsound.
//! * **Quotients** and **literal arithmetic** (`Nat.add 2 3 ~> 5` by GMP): without them
//!   the same computations are done by unary unfolding, which is slow and hits the
//!   resource limits. Also, `Nat` literals do not yet unify with `Nat.succ` forms.
//! * **Eta for structures** and **unit-likes**: see `defeq.rs`.
//!
//! All of these only make the checker *reject more*, never accept more.
//!
//! # Termination
//!
//! Well-typed terms normalise, but the checker reduces terms it has not yet shown to be
//! well-typed, and e.g. `(fun x => x x) (fun x => x x)` reduces to itself forever. Every
//! iteration is charged to the step budget (`TypeChecker::tick`), and every term built is
//! checked against the depth limit, so such inputs end in `ResourceLimit`.

use crate::checker::{KResult, TypeChecker};
use crate::env::DeclKind;
use crate::error::KernelError;
use crate::expr::{Expr, ExprKind};

impl TypeChecker<'_> {
    /// Reduce `e` to weak-head normal form using beta, zeta and delta (definitions only).
    pub fn whnf(&mut self, e: Expr) -> KResult<Expr> {
        if let Some(&r) = self.whnf_cache.get(&e) {
            return Ok(r);
        }
        let r = self.guarded(&[e], |tc| {
            let mut cur = e;
            loop {
                tc.tick()?;
                cur = tc.whnf_core(cur)?;
                match tc.unfold_definition(cur)? {
                    Some(next) => cur = next,
                    None => return Ok(cur),
                }
            }
        })?;
        self.whnf_cache.insert(e, r);
        self.whnf_cache.insert(r, r);
        Ok(r)
    }

    /// Everything except delta: beta, zeta (let and let-bound variables). The result's
    /// head is a lambda-free, let-free, non-reducible head, possibly a constant that
    /// delta could still unfold.
    pub fn whnf_core(&mut self, e: Expr) -> KResult<Expr> {
        self.guarded(&[e], |tc| tc.whnf_core_loop(e))
    }

    fn whnf_core_loop(&mut self, mut e: Expr) -> KResult<Expr> {
        loop {
            self.tick()?;
            match self.store.kind(e).clone() {
                ExprKind::FVar(id) => match self.lctx.get(id).and_then(|d| d.value) {
                    Some(v) => e = v,
                    None => return Ok(e),
                },
                ExprKind::Let(_, _, v, b) => {
                    let next = self.store.instantiate1(b, v);
                    e = self.bounded(next)?;
                }
                ExprKind::App(..) => {
                    let head = self.store.get_app_fn(e);
                    let new_head = self.whnf_core(head)?;
                    let args = self.store.get_app_args(e);
                    if !matches!(self.store.kind(new_head), ExprKind::Lam(..)) {
                        // Stuck (or merely changed): rebuild only if the head moved.
                        return Ok(if new_head == head {
                            e
                        } else {
                            self.store.apps(new_head, &args)
                        });
                    }
                    // Beta with as many arguments as there are leading lambdas.
                    let mut body = new_head;
                    let mut used = 0;
                    while used < args.len() {
                        match self.store.kind(body) {
                            ExprKind::Lam(_, _, b) => {
                                body = *b;
                                used += 1;
                            }
                            _ => break,
                        }
                    }
                    let reduced = self.store.instantiate_rev(body, &args[..used]);
                    let reduced = self.bounded(reduced)?;
                    let rest = self.store.apps(reduced, &args[used..]);
                    e = self.bounded(rest)?;
                }
                _ => return Ok(e),
            }
        }
    }

    /// One delta step: if the head of `e` is a *definition*, replace it by its value
    /// (levels instantiated). Theorems, opaques, axioms and unknown constants are left.
    fn unfold_definition(&mut self, e: Expr) -> KResult<Option<Expr>> {
        let head = self.store.get_app_fn(e);
        let ExprKind::Const(n, ls) = self.store.kind(head).clone() else {
            return Ok(None);
        };
        let decls = self.decls;
        let Some(d) = decls.get(&n) else {
            return Ok(None);
        };
        let DeclKind::Definition { value, .. } = &d.kind else {
            return Ok(None);
        };
        if ls.len() != d.level_params.len() {
            return Err(KernelError::LevelArityMismatch {
                name: n,
                expected: d.level_params.len(),
                found: ls.len(),
            });
        }
        let v = self
            .store
            .instantiate_level_params(*value, &d.level_params, &ls)
            .map_err(|_| KernelError::Internal("level arity"))?;
        let args = self.store.get_app_args(e);
        let out = self.store.apps(v, &args);
        Ok(Some(self.bounded(out)?))
    }
}
