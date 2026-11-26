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
//!   iota    T.rec ... (c As bs) ...   ~>  the rule the kernel derived for c   (see `reduce_rec`)
//!   K       T.rec ... h ...           ~>  T.rec ... (c As) ...   (K-target T, type of h ≡ type of c As)
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
//! # Not implemented yet (slice 3b+), and what goes wrong without it
//!
//! * **Projections** of constructors (`Proj` is `Unsupported` in `infer`, and stuck here):
//!   structure fields never compute. Incomplete, not unsound.
//! * **Structure eta at a recursor's major premise** (`p ~> (p.1, p.2)`): such majors are
//!   merely stuck, so some valid terms are rejected.
//! * **Quotients** and **literal arithmetic** (`Nat.add 2 3 ~> 5` by GMP): without them
//!   the same computations are done by unary unfolding, which is slow and hits the
//!   resource limits. `Nat` literals are `Unsupported` (see `infer.rs`), so there is no
//!   literal-to-`Nat.succ` conversion at recursor majors either.
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
use crate::env::{DeclKind, Declaration, RecursorInfo};
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
                        let stuck = if new_head == head {
                            e
                        } else {
                            self.store.apps(new_head, &args)
                        };
                        // ... unless it is a recursor that can fire (rule iota).
                        match self.reduce_rec(stuck)? {
                            Some(r) => {
                                e = self.bounded(r)?;
                                continue;
                            }
                            None => return Ok(stuck),
                        }
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

    /// Rule iota (with K-like reduction): `e` is `T.rec` applied to its arguments.
    ///
    /// ```text
    ///   T.rec.{ls} As motive minors is (c As bs) extra...
    ///       ~>  rhs_c[ls] As motive minors bs extra...
    /// ```
    ///
    /// where `rhs_c` is the computation rule the kernel derived for constructor `c`
    /// (`inductive.rs`). The major premise is first put in whnf (it may itself need
    /// delta, iota or beta to reveal a constructor).
    ///
    /// Every guard answers `None` ("stuck", never "wrong"):
    ///
    /// * wrong number of universe levels, or too few arguments to reach the major
    ///   premise: without these checks `args[major_idx]` is out of range, or a partially
    ///   applied recursor is reduced as if it were saturated;
    /// * the major's head is not a constructor with a rule: nothing to fire;
    /// * the constructor application has *exactly* `params + fields` arguments. Lean
    ///   takes the last `fields` arguments of whatever it gets; with fewer arguments
    ///   that would feed parameters in as fields (only reachable from ill-typed terms,
    ///   which `whnf` is nevertheless asked about), so we insist on the exact count.
    ///
    /// ## K-like reduction
    ///
    /// For a K-target (`Eq`: a `Prop` with one constructor and no fields) the major
    /// premise `h : Eq a b` need not be a constructor. If `Eq a b` is definitionally
    /// `Eq a a`, we replace `h` by `Eq.refl a`, and iota fires:
    ///
    /// ```text
    ///   h : I params is        I params is ≡ type of (c params)
    ///   ─────────────────────────────────────────────────────  (K)
    ///   T.rec ... h  ~>  T.rec ... (c params)
    /// ```
    ///
    /// Sound because any two proofs of a `Prop` are equal (proof irrelevance), so `h` and
    /// `refl` are definitionally equal once their types agree; the *type* test is what
    /// makes the replacement well-typed. We follow Lean exactly: the major's type is put
    /// in whnf, its head must be the inductive type itself, the constructor is applied to
    /// the type's first `num_params` arguments, and the constructor application's
    /// inferred type must be `≡` the major's type (this unifies the indices). Without the
    /// type test, `Eq.rec` on `h : Eq a b` would reduce for *unequal* `a`, `b`, giving
    /// `a ≡ b` for any two terms. Without the K flag, K would apply to inductives with
    /// fields (where `c params` is not even a well-typed proof) or to data in `Type`,
    /// where proof irrelevance does not hold. Both are covered by negative tests.
    ///
    /// Structure eta at the major premise (`p ~> (p.1, p.2)`) is not implemented
    /// (slice 3b): such majors are merely stuck.
    fn reduce_rec(&mut self, e: Expr) -> KResult<Option<Expr>> {
        let head = self.store.get_app_fn(e);
        let ExprKind::Const(n, ls) = self.store.kind(head).clone() else {
            return Ok(None);
        };
        let decls = self.decls;
        let Some(Declaration {
            level_params,
            kind: DeclKind::Recursor(info),
            ..
        }) = decls.get(&n)
        else {
            return Ok(None);
        };
        let args = self.store.get_app_args(e);
        let fixed = info.num_params + 1 + info.num_minors;
        let major_idx = fixed + info.num_indices;
        if ls.len() != level_params.len() || args.len() <= major_idx {
            return Ok(None);
        }
        let mut major = args[major_idx];
        if info.k {
            major = self.k_major(info, major)?;
        }
        let major = self.whnf(major)?;
        let mhead = self.store.get_app_fn(major);
        let ExprKind::Const(cn, _) = self.store.kind(mhead).clone() else {
            return Ok(None);
        };
        let Some(rule) = info.rules.iter().find(|r| r.ctor == cn) else {
            return Ok(None);
        };
        let Some(DeclKind::Constructor(ci)) = decls.get(&cn).map(|d| &d.kind) else {
            return Ok(None);
        };
        let margs = self.store.get_app_args(major);
        if margs.len() != ci.num_params + rule.num_fields {
            return Ok(None);
        }
        let rhs = self
            .store
            .instantiate_level_params(rule.rhs, level_params, &ls)
            .map_err(|_| KernelError::Internal("level arity"))?;
        let rhs = self.store.apps(rhs, &args[..fixed]);
        let rhs = self.store.apps(rhs, &margs[ci.num_params..]);
        let rhs = self.store.apps(rhs, &args[major_idx + 1..]);
        Ok(Some(self.bounded(rhs)?))
    }

    /// The K step: if the type of `major` is `I params is` (with `I` the recursor's
    /// inductive) and `I params is` is `≡` the type of `c params`, return `c params`;
    /// otherwise return `major` unchanged. See `reduce_rec`.
    fn k_major(&mut self, info: &RecursorInfo, major: Expr) -> KResult<Expr> {
        let major_ty = self.infer(major)?;
        let ty = self.whnf(major_ty)?;
        let head = self.store.get_app_fn(ty);
        let ExprKind::Const(n, ls) = self.store.kind(head).clone() else {
            return Ok(major);
        };
        let targs = self.store.get_app_args(ty);
        let [rule] = info.rules.as_slice() else {
            return Ok(major);
        };
        if n != info.induct || targs.len() < info.num_params {
            return Ok(major);
        }
        let c = self.store.constant(&rule.ctor, &ls);
        let new = self.store.apps(c, &targs[..info.num_params]);
        let new = self.bounded(new)?;
        let new_ty = self.infer(new)?;
        Ok(if self.is_def_eq(new_ty, ty)? {
            new
        } else {
            major
        })
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
