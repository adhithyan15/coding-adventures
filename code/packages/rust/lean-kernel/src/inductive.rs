//! Inductive types: checking a declaration and *deriving* its recursor.
//!
//! # What the user supplies, and what the kernel makes
//!
//! The user hands over an [`InductiveDecl`]: universe parameters, the number of
//! parameters, and one inductive type with its constructors, each just a name and a type
//! (Lean's `InductiveVal` / `ConstructorVal` shape). The kernel checks the block and then
//! adds three kinds of declaration, atomically:
//!
//! ```text
//!   inductive   T.{ls} : (params) -> (indices) -> Sort l
//!   constructor T.c.{ls} : (params) -> (fields) -> T params indices
//!   recursor    T.rec.{u, ls} : (params) -> (motive) -> (minors) -> (indices) -> (major) -> motive indices major
//! ```
//!
//! The recursor is never supplied by the user: it is a *derived* rule, and a forged one
//! would be an axiom with computation rules, so `add_declaration` refuses `Recursor`,
//! `Constructor` and `Inductive` declarations outright.
//!
//! ```text
//!   Nat.rec.{u} : (motive : Nat -> Sort u)
//!              -> motive Nat.zero
//!              -> ((n : Nat) -> motive n -> motive (Nat.succ n))      -- minor premises
//!              -> (t : Nat) -> motive t
//!   Eq.rec.{u_1, u} : {a : Sort u} -> {x : a} -> (motive : (y : a) -> Eq x y -> Sort u_1)
//!              -> motive x (Eq.refl x) -> {y : a} -> (t : Eq x y) -> motive y t
//! ```
//!
//! # The checks, each with what goes wrong without it
//!
//! ```text
//!   (names)   T, every constructor and T.rec are new and pairwise distinct
//!   (type)    ⊢ T's type : Sort _,  as a syntactic Pi telescope  (params) (indices) -> Sort l
//!   (params)  each constructor starts with exactly the type's parameter binders (def-eq)
//!   (ctor)    ⊢ c's type : Sort _,  and after the params and fields it returns
//!             T.{ls} params indices                (ls = the declared universe params)
//!   (univ)    every field's sort  <=  l      unless T is in Prop
//!   (pos)     T occurs in a field only as the conclusion (after Pi's) of the field's type
//! ```
//!
//! * *Names.* A duplicate would let a later declaration change what an earlier theorem
//!   means. `T.rec` in particular must be free, or an old `T.rec` would silently stay.
//! * *Type well-formed, and not mentioning itself.* If `T`'s own type could mention `T`,
//!   then `T : T -> Type` is accepted by the circular reading "assume T exists". Lean
//!   checks the type before `T` is in the environment; we check the same thing by refusing
//!   any occurrence of `T`'s name in it.
//! * *Parameters shared.* The recursor treats parameters as fixed; a constructor that
//!   changes them (`mk : (Foo Nat) -> Foo Bool`) would make `rec`'s minor premises
//!   ill-typed or silently drop information.
//! * *Return type.* The conclusion must be `T` applied to *exactly the parameter
//!   variables* and indices; the universe arguments must be exactly `T`'s own parameters.
//!   `T.{u+1}` inside `T.{u}`'s constructor, or `T Nat` where a parameter was expected,
//!   would make a constructor build a *different* type than the recursor eliminates.
//! * *Universe constraint.* A field of sort `Sort k` with `k > l` lets `T : Type` store
//!   a `Type`, the classic route to Girard's paradox (`inductive Big : Type | mk : Type ->
//!   Big` is a retract of `Type`). Not enforced when `T` is in `Prop` (a proposition may
//!   mention anything; elimination is restricted instead, see below). Like Lean, if `T`'s
//!   sort is `Sort u` and `u` *could* be zero, the check is the strict one.
//! * *Strict positivity.* If `T` could occur to the left of an arrow
//!   (`mk : (Bad -> False) -> Bad`) then `Bad.rec` would give a function `Bad -> False`
//!   from `mk`, and applying it to `mk (that function)` proves `False`. Only the
//!   *conclusion* of a field's type may be `T params indices`; indices of that occurrence
//!   may not mention `T`. Types are put in whnf before the test, so a definition that
//!   unfolds to a negative occurrence is caught.
//!
//! Mutual blocks and **nested** occurrences (`T` inside the arguments of another
//! inductive type, e.g. `List T`) are `Unsupported` here (slice 3b): they need the
//! auxiliary-type translation, which is not written yet. A nested occurrence is told
//! apart from a plain non-positive one by the head of the offending type being a declared
//! inductive; anything else is simply not strictly positive.
//!
//! # The recursor
//!
//! Let `T` have parameters `As`, indices `is`, constructors `c_j` with fields `bs_j`.
//! A field `b : (xs) -> T As js` is *recursive*. The motive is
//! `motive : (is) -> (t : T As is) -> Sort u`. The minor premise for `c_j` is
//!
//! ```text
//!   minor_j : (bs_j) -> (ihs : for each recursive field b, (xs) -> motive js (b xs))
//!                    -> motive (indices of c_j's result) (c_j As bs_j)
//! ```
//!
//! and the computation rule stored for `c_j` is
//!
//! ```text
//!   T.rec As motive minors is (c_j As bs)  ~>  minor_j bs (fun xs => T.rec As motive minors js (b xs))
//! ```
//!
//! kept as the closed term `fun As motive minors bs => minor_j bs (...)` so `whnf` can
//! instantiate it (see `whnf.rs`, rule iota).
//!
//! ## Large elimination (may the motive be `Sort u`, or only `Prop`?)
//!
//! Proof irrelevance says two proofs of one proposition are equal. If a proof could be
//! *eliminated into data*, that would equate data: from `Or.inl h : Or p p` and
//! `Or.inr h`, `Or.rec` into `Bool` would give `true ≡ false`. So, as in Lean:
//!
//! ```text
//!   T is never in Prop (its sort is never 0 for any universe assignment)   -> Sort u
//!   T may be in Prop, with
//!       0 constructors                                                     -> Sort u  (False)
//!       1 constructor, every field is a Prop or occurs directly among the
//!                      indices of the result                               -> Sort u  (Eq, And, True)
//!       otherwise                                                          -> Prop only (Or, Exists)
//! ```
//!
//! "Prop only" recursors have no extra universe parameter (`Or.rec` has none, so `Or.rec.{1}`
//! is an arity error). The fresh motive-universe parameter is called `u`, or `u_1`, ... if
//! `u` is already taken.
//!
//! ## K-like
//!
//! `T` is a *K-target* when it is a `Prop` (sort exactly `0`), has one constructor and
//! that constructor has no fields (`Eq`, `True`). The recursor is marked `k`, and `whnf`
//! may then rewrite a non-constructor major premise (see `whnf.rs`).
//!
//! # Deliberately not done (slice 3b), each failing closed
//!
//! * mutual inductive types, nested inductive types: `Unsupported`;
//! * structure detection: structure projections and structure eta, so `Proj` stays
//!   `Unsupported` and there is no structure-eta at recursor majors;
//! * unit-like types (every inhabitant of a one-constructor, no-field type is equal):
//!   not implemented, so such terms are merely not *known* equal (rejects more);
//! * the `Nat` literal <-> `Nat.zero`/`Nat.succ` conversion: literals are `Unsupported`.
//!
//! # Atomicity
//!
//! The type, constructors and recursor are inserted into the environment *provisionally*
//! (so that later checks, which look constants up, see them), and every name inserted is
//! removed again if any check fails. After success the environment holds all three
//! kinds; after failure it is exactly as before (apart from interned expressions).
//!
//! # Resource limits
//!
//! At most [`MAX_CONSTRUCTORS`] constructors. Everything else (fields, telescope depth,
//! sizes) is covered by the checker's term-depth, step and node budgets, because the
//! recursor type is a Pi telescope whose depth grows with the number of binders: an
//! oversized declaration ends in `ResourceLimit`, never in a stack overflow.

use crate::checker::{KResult, TypeChecker};
use crate::env::{
    ConstructorInfo, DeclKind, Declaration, Environment, InductiveInfo, RecRule, RecursorInfo,
};
use crate::error::KernelError;
use crate::expr::{Binder, BinderInfo, Expr, ExprKind, ExprStore, FVarId};
use crate::level::{Level, MAX_LEVEL_DEPTH};
use crate::name::Name;
use std::collections::HashSet;

/// The most constructors one inductive type may have.
pub const MAX_CONSTRUCTORS: usize = 1024;

/// A constructor as supplied by the user: its name and its (closed) type.
#[derive(Clone, Debug)]
pub struct ConstructorDecl {
    /// The constructor's global name.
    pub name: Name,
    /// `(params) -> (fields) -> T params indices`.
    pub ty: Expr,
}

/// One inductive type of a block.
#[derive(Clone, Debug)]
pub struct InductiveTypeDecl {
    /// The type's global name.
    pub name: Name,
    /// `(params) -> (indices) -> Sort l`.
    pub ty: Expr,
    /// The constructors, in order.
    pub ctors: Vec<ConstructorDecl>,
}

/// An inductive declaration block. Exactly one type is supported in this slice.
#[derive(Clone, Debug)]
pub struct InductiveDecl {
    /// Universe parameters shared by the type, constructors and recursor.
    pub level_params: Vec<Name>,
    /// How many leading binders of the type (and of every constructor) are parameters.
    pub num_params: usize,
    /// The types of the block (mutual blocks have more than one: `Unsupported`).
    pub types: Vec<InductiveTypeDecl>,
}

/// The checked result, ready to insert.
struct Built {
    ctors: Vec<Declaration>,
    rec: Declaration,
}

/// A constructor field, opened as a free variable.
struct Field {
    id: FVarId,
    x: Expr,
    ty: Expr,
    lvl: Level,
}

/// A checked constructor, opened.
struct CtorShape {
    name: Name,
    fields: Vec<Field>,
    ret_indices: Vec<Expr>,
}

/// The inductive type, opened as a telescope of free variables.
struct Shape<'a> {
    name: &'a Name,
    ls: Vec<Level>,
    ind: Expr,
    nparams: usize,
    param_ids: Vec<FVarId>,
    params: Vec<Expr>,
    param_tys: Vec<Expr>,
    index_ids: Vec<FVarId>,
    indices: Vec<Expr>,
    level: Level,
}

/// Does the constant `name` occur anywhere in `e`? Explicit stack and a visited set, so a
/// hostile deep term or an exponentially shared DAG costs time linear in its distinct
/// nodes and no native stack.
/// Also returns how many distinct nodes were visited, so that checker callers can charge
/// the work to the step budget.
fn occurs_const(st: &ExprStore, e: Expr, name: &Name) -> (bool, usize) {
    let mut stack = vec![e];
    let mut seen = HashSet::new();
    while let Some(x) = stack.pop() {
        if !seen.insert(x) {
            continue;
        }
        match st.kind(x) {
            ExprKind::Const(n, _) if n == name => return (true, seen.len()),
            ExprKind::App(a, b) => stack.extend([*a, *b]),
            ExprKind::Lam(_, t, b) | ExprKind::Pi(_, t, b) => stack.extend([*t, *b]),
            ExprKind::Let(_, t, v, b) => stack.extend([*t, *v, *b]),
            ExprKind::Proj(_, _, y) => stack.push(*y),
            _ => {}
        }
    }
    (false, seen.len())
}

impl Environment {
    /// Check an inductive declaration and, only if every check passes, add the type, its
    /// constructors and the derived recursor. On any error the environment is unchanged.
    pub fn add_inductive(&mut self, decl: InductiveDecl) -> Result<(), KernelError> {
        let mut inserted = Vec::new();
        let r = self.add_inductive_inner(&decl, &mut inserted);
        if r.is_err() {
            for n in &inserted {
                self.decls.remove(n);
            }
        }
        r
    }

    fn add_inductive_inner(
        &mut self,
        d: &InductiveDecl,
        inserted: &mut Vec<Name>,
    ) -> Result<(), KernelError> {
        let t = match d.types.as_slice() {
            [] => return Err(KernelError::InvalidInductive("empty inductive block")),
            [t] => t,
            _ => {
                return Err(KernelError::Unsupported(
                    "mutual inductive types arrive in slice 3b",
                ))
            }
        };
        if t.ctors.len() > MAX_CONSTRUCTORS {
            return Err(KernelError::ResourceLimit("too many constructors"));
        }
        let mut seen_lp = HashSet::new();
        for p in &d.level_params {
            if !seen_lp.insert(p) {
                return Err(KernelError::DuplicateLevelParam(p.clone()));
            }
        }
        // Handles first (a foreign one would panic when inspected), then closedness.
        let all = std::iter::once(t.ty).chain(t.ctors.iter().map(|c| c.ty));
        for e in all.clone() {
            if !self.store.owns(e) {
                return Err(KernelError::ForeignExpr);
            }
        }
        for e in all {
            if self.store.has_loose_bvars(e) {
                return Err(KernelError::LooseBVar(self.store.loose_bvar_range(e) - 1));
            }
            if self.store.has_fvar(e) {
                return Err(KernelError::FreeVarInDeclaration);
            }
        }
        // (names)
        if t.name.is_anonymous() {
            return Err(KernelError::InvalidInductive("anonymous inductive name"));
        }
        if t.ctors.iter().any(|c| c.name.is_anonymous()) {
            return Err(KernelError::InvalidInductive("anonymous constructor name"));
        }
        let rec_name = Name::str(&t.name, "rec");
        let mut fresh: HashSet<&Name> = HashSet::new();
        for n in std::iter::once(&t.name)
            .chain(t.ctors.iter().map(|c| &c.name))
            .chain(std::iter::once(&rec_name))
        {
            if self.decls.contains_key(n) || !fresh.insert(n) {
                return Err(KernelError::AlreadyDeclared(n.clone()));
            }
        }
        // (type): T's type may not mention T (see the module documentation).
        if occurs_const(&self.store, t.ty, &t.name).0 {
            return Err(KernelError::InvalidInductive(
                "the type of an inductive type may not mention itself",
            ));
        }
        let mut pis = 0usize;
        let mut cur = t.ty;
        while let ExprKind::Pi(_, _, body) = self.store.kind(cur) {
            pis += 1;
            cur = *body;
        }
        // The recursor type is a Pi telescope with one binder per parameter, the motive,
        // each minor premise, each index and the major premise, and `infer_pi` gives it a
        // universe level that nests one `imax` per binder. Levels deeper than
        // MAX_LEVEL_DEPTH are refused by the checker, so a recursor with more binders than
        // that can never be typed: refuse it now, before any derivation work, instead of
        // doing quadratic `bind` work only to fail later. (Necessary, not sufficient: the
        // real bound is a little lower, and the checker still has the last word.)
        let rec_binders = pis.saturating_add(t.ctors.len()).saturating_add(2);
        if rec_binders > MAX_LEVEL_DEPTH as usize {
            return Err(KernelError::ResourceLimit(
                "recursor would have too many binders for the universe level depth limit",
            ));
        }
        if d.num_params > pis {
            return Err(KernelError::InvalidInductive(
                "the type has fewer binders than parameters",
            ));
        }
        // Provisional insertion of the type, so constructor types may mention it.
        let info = InductiveInfo {
            num_params: d.num_params,
            num_indices: pis - d.num_params,
            ctors: t.ctors.iter().map(|c| c.name.clone()).collect(),
            rec: rec_name.clone(),
        };
        self.decls.insert(
            t.name.clone(),
            Declaration {
                name: t.name.clone(),
                level_params: d.level_params.clone(),
                ty: t.ty,
                kind: DeclKind::Inductive(info),
            },
        );
        inserted.push(t.name.clone());
        // Phase A: every check, and the derivation of constructors and recursor.
        let built = {
            let mut tc = self.type_checker(&d.level_params);
            tc.check_inductive(d, t, &rec_name)?
        };
        let (rec_lps, rec_ty) = (built.rec.level_params.clone(), built.rec.ty);
        let rhss: Vec<Expr> = match &built.rec.kind {
            DeclKind::Recursor(r) => r.rules.iter().map(|x| x.rhs).collect(),
            _ => return Err(KernelError::Internal("recursor expected")),
        };
        for decl in built.ctors.into_iter().chain([built.rec]) {
            inserted.push(decl.name.clone());
            self.decls.insert(decl.name.clone(), decl);
        }
        // Phase B: defence in depth. The derived recursor and its rules are ordinary
        // terms, so the ordinary checker must accept them. A bug in the generator above
        // then fails closed here instead of installing an ill-typed rule.
        let mut tc = self.type_checker(&rec_lps);
        let s = tc.infer(rec_ty)?;
        tc.ensure_sort(s)?;
        for rhs in rhss {
            tc.infer(rhs)?;
        }
        Ok(())
    }
}

impl TypeChecker<'_> {
    fn check_inductive(
        &mut self,
        d: &InductiveDecl,
        t: &InductiveTypeDecl,
        rec_name: &Name,
    ) -> KResult<Built> {
        let ls: Vec<Level> = d.level_params.iter().map(Level::param).collect();
        let sort_of = self.infer(t.ty)?;
        self.ensure_sort(sort_of)?;
        // Open the type's Pi telescope, syntactically (no whnf: a type that is only
        // *reducible* to a telescope is rejected, fail closed).
        let (mut ids, mut xs, mut tys) = (vec![], vec![], vec![]);
        let mut cur = t.ty;
        while let ExprKind::Pi(m, dom, body) = self.store.kind(cur).clone() {
            let dom = self.store.instantiate_rev(dom, &xs);
            let dom = self.bounded(dom)?;
            let (id, x) = self.new_local(&m.0.name, dom, None)?;
            ids.push(id);
            xs.push(x);
            tys.push(dom);
            cur = body;
        }
        let result = self.store.instantiate_rev(cur, &xs);
        let result = self.bounded(result)?;
        let ExprKind::Sort(level) = self.store.kind(result).clone() else {
            return Err(KernelError::InvalidInductive(
                "the type of an inductive type must end in a sort",
            ));
        };
        let np = d.num_params;
        let ind = self.store.constant(&t.name, &ls);
        let sh = Shape {
            name: &t.name,
            ls,
            ind,
            nparams: np,
            param_ids: ids[..np].to_vec(),
            params: xs[..np].to_vec(),
            param_tys: tys[..np].to_vec(),
            index_ids: ids[np..].to_vec(),
            indices: xs[np..].to_vec(),
            level,
        };
        let mut ctors = Vec::with_capacity(t.ctors.len());
        for c in &t.ctors {
            ctors.push(self.check_ctor(&sh, c)?);
        }
        self.derive(d, &sh, &ctors, t, rec_name)
    }

    /// Is `t` exactly `T.{ls} params indices`, with indices not mentioning `T`? Returns
    /// the indices.
    fn valid_ind_app(&mut self, sh: &Shape, t: Expr) -> KResult<Option<Vec<Expr>>> {
        let head = self.store.get_app_fn(t);
        let ExprKind::Const(n, ls) = self.store.kind(head) else {
            return Ok(None);
        };
        if n != sh.name || ls.len() != sh.ls.len() || !ls.iter().zip(&sh.ls).all(|(a, b)| a == b) {
            return Ok(None);
        }
        let args = self.store.get_app_args(t);
        if args.len() != sh.nparams + sh.indices.len() || args[..sh.nparams] != sh.params[..] {
            return Ok(None);
        }
        let idx = args[sh.nparams..].to_vec();
        for &e in &idx {
            if self.occurs(e, sh.name)? {
                return Ok(None);
            }
        }
        Ok(Some(idx))
    }

    /// `occurs_const`, with the nodes visited charged to the step budget.
    fn occurs(&mut self, e: Expr, name: &Name) -> KResult<bool> {
        let (found, visited) = occurs_const(self.store, e, name);
        for _ in 0..visited {
            self.tick()?;
        }
        Ok(found)
    }

    /// Rules (params), (ctor), (univ) and (pos) for one constructor.
    fn check_ctor(&mut self, sh: &Shape, c: &ConstructorDecl) -> KResult<CtorShape> {
        let s = self.infer(c.ty)?;
        self.ensure_sort(s)?;
        let (mut vars, mut fields) = (vec![], vec![]);
        let mut cur = c.ty;
        while let ExprKind::Pi(m, dom, body) = self.store.kind(cur).clone() {
            let dom = self.store.instantiate_rev(dom, &vars);
            let dom = self.bounded(dom)?;
            if vars.len() < sh.nparams {
                // (params): same binder types as the inductive's, up to def-eq. The
                // constructor is then opened with the type's own parameter variables.
                if !self.is_def_eq(dom, sh.param_tys[vars.len()])? {
                    return Err(KernelError::InvalidInductive(
                        "constructor parameters do not match the inductive type's",
                    ));
                }
                vars.push(sh.params[vars.len()]);
            } else {
                let s = self.infer(dom)?;
                let lvl = self.ensure_sort(s)?;
                // (univ): field sort <= type sort, unless the type is a Prop.
                if !(sh.level.is_geq(&lvl) || sh.level.is_zero()) {
                    return Err(KernelError::UniverseTooLarge(c.name.clone()));
                }
                self.check_positivity(sh, &c.name, dom)?;
                let (id, x) = self.new_local(&m.0.name, dom, None)?;
                fields.push(Field {
                    id,
                    x,
                    ty: dom,
                    lvl,
                });
                vars.push(x);
            }
            cur = body;
        }
        if vars.len() < sh.nparams {
            return Err(KernelError::InvalidInductive(
                "constructor has fewer binders than parameters",
            ));
        }
        let ret = self.store.instantiate_rev(cur, &vars);
        let ret = self.bounded(ret)?;
        let Some(ret_indices) = self.valid_ind_app(sh, ret)? else {
            return Err(KernelError::InvalidInductive(
                "constructor must return the inductive type applied to its parameters and indices",
            ));
        };
        Ok(CtorShape {
            name: c.name.clone(),
            fields,
            ret_indices,
        })
    }

    /// Rule (pos), as a loop. `ty` is a field's type.
    ///
    /// ```text
    ///   whnf ty has no occurrence of T                    ok (non-recursive field)
    ///   whnf ty = (x : A) -> B   and T not in A           continue with B[x := fresh]
    ///   whnf ty = T params indices                         ok (recursive field)
    ///   anything else (T in A, or T inside other args)     rejected
    /// ```
    fn check_positivity(&mut self, sh: &Shape, ctor: &Name, ty: Expr) -> KResult<()> {
        let mut t = ty;
        loop {
            self.tick()?;
            t = self.whnf(t)?;
            if !self.occurs(t, sh.name)? {
                return Ok(());
            }
            if let ExprKind::Pi(m, dom, body) = self.store.kind(t).clone() {
                if self.occurs(dom, sh.name)? {
                    return Err(KernelError::NonPositiveOccurrence(ctor.clone()));
                }
                let (_, x) = self.new_local(&m.0.name, dom, None)?;
                let next = self.store.instantiate1(body, x);
                t = self.bounded(next)?;
                continue;
            }
            if self.valid_ind_app(sh, t)?.is_some() {
                return Ok(());
            }
            // `T` is inside the arguments of something else. If that something is a
            // declared inductive type this is a *nested* inductive (`List T`), which
            // needs the slice-3b translation; otherwise it is simply not positive.
            let head = self.store.get_app_fn(t);
            if let ExprKind::Const(n, _) = self.store.kind(head) {
                if n != sh.name
                    && matches!(
                        self.decls.get(n).map(|x| &x.kind),
                        Some(DeclKind::Inductive(_))
                    )
                {
                    return Err(KernelError::Unsupported(
                        "nested inductive types arrive in slice 3b",
                    ));
                }
            }
            return Err(KernelError::NonPositiveOccurrence(ctor.clone()));
        }
    }

    /// If a field's type is `(xs) -> T params js`, return the `xs` (ids and variables)
    /// and `js`: the field is recursive.
    #[allow(clippy::type_complexity)]
    fn rec_arg(
        &mut self,
        sh: &Shape,
        fty: Expr,
    ) -> KResult<Option<(Vec<FVarId>, Vec<Expr>, Vec<Expr>)>> {
        let (mut ids, mut xs) = (vec![], vec![]);
        let mut t = fty;
        loop {
            t = self.whnf(t)?;
            let ExprKind::Pi(m, dom, body) = self.store.kind(t).clone() else {
                break;
            };
            let (id, x) = self.new_local(&m.0.name, dom, None)?;
            ids.push(id);
            xs.push(x);
            let next = self.store.instantiate1(body, x);
            t = self.bounded(next)?;
        }
        Ok(self.valid_ind_app(sh, t)?.map(|js| (ids, xs, js)))
    }

    /// Wrap `body` in Pi (or lambda) binders for the free variables `ids`, taking each
    /// binder's name and type from the local context.
    fn bind(&mut self, is_pi: bool, ids: &[FVarId], body: Expr) -> KResult<Expr> {
        let mut r = self.store.abstract_fvars(body, ids);
        for i in (0..ids.len()).rev() {
            self.tick()?;
            let d = self
                .lctx
                .get(ids[i])
                .cloned()
                .ok_or(KernelError::UnknownFVar)?;
            let ty = self.store.abstract_fvars(d.ty, &ids[..i]);
            let b = Binder {
                name: d.name,
                info: BinderInfo::Default,
            };
            r = if is_pi {
                self.store.pi(b, ty, r)
            } else {
                self.store.lam(b, ty, r)
            };
        }
        self.bounded(r)
    }

    /// A new local variable whose type is first checked against the depth limit.
    fn local(&mut self, name: &str, ty: Expr) -> KResult<(FVarId, Expr)> {
        let ty = self.bounded(ty)?;
        self.new_local(&Name::from_dotted(name), ty, None)
    }

    /// The large-elimination decision, the K flag and the recursor with its rules.
    fn derive(
        &mut self,
        d: &InductiveDecl,
        sh: &Shape,
        ctors: &[CtorShape],
        t: &InductiveTypeDecl,
        rec_name: &Name,
    ) -> KResult<Built> {
        // May the motive go into every `Sort u`? See "Large elimination" above. Fields
        // are compared with `==` on handles: "occurs among the indices" means *is* one
        // of them (Lean's rule), not merely mentioned inside one.
        let only_prop = !sh.level.is_never_zero()
            && match ctors {
                [] => false,
                [c] => c
                    .fields
                    .iter()
                    .any(|f| !f.lvl.is_zero() && !c.ret_indices.contains(&f.x)),
                _ => true,
            };
        let k = sh.level.is_zero() && matches!(ctors, [c] if c.fields.is_empty());
        let (elim, rec_lps) = if only_prop {
            (Level::zero(), d.level_params.clone())
        } else {
            let mut u = Name::from_dotted("u");
            let mut i = 1;
            while d.level_params.contains(&u) {
                u = Name::from_dotted(&format!("u_{i}"));
                i += 1;
            }
            let mut lps = vec![u.clone()];
            lps.extend(d.level_params.iter().cloned());
            (Level::param(&u), lps)
        };
        let rec_ls: Vec<Level> = rec_lps.iter().map(Level::param).collect();
        let rec_c = self.store.constant(rec_name, &rec_ls);
        // The major premise `t : T params indices` and the motive.
        let major_ty = self
            .store
            .apps(sh.ind, &[sh.params.clone(), sh.indices.clone()].concat());
        let (major_id, major) = self.local("t", major_ty)?;
        let sort = self.store.sort(&elim);
        let motive_ids: Vec<FVarId> = sh.index_ids.iter().copied().chain([major_id]).collect();
        let motive_ty = self.bind(true, &motive_ids, sort)?;
        let (motive_id, motive) = self.local("motive", motive_ty)?;
        let motive_app = |tc: &mut Self, idx: &[Expr], m: Expr| {
            let a = tc.store.apps(motive, idx);
            tc.store.app(a, m)
        };
        // Minor premises. Per constructor remember what the rule needs.
        struct Minor {
            id: FVarId,
            x: Expr,
            ihs: Vec<(Vec<FVarId>, Vec<Expr>, Expr)>,
        }
        let mut minors: Vec<Minor> = Vec::with_capacity(ctors.len());
        for c in ctors {
            self.tick()?;
            let (mut ih_ids, mut ihs) = (vec![], vec![]);
            for f in &c.fields {
                if let Some((xids, xs, js)) = self.rec_arg(sh, f.ty)? {
                    let bx = self.store.apps(f.x, &xs);
                    let body = motive_app(self, &js, bx);
                    let ih_ty = self.bind(true, &xids, body)?;
                    let (ih_id, _) = self.local("ih", ih_ty)?;
                    ih_ids.push(ih_id);
                    ihs.push((xids, js, bx));
                }
            }
            let cl = self.store.constant(&c.name, &sh.ls);
            let fx: Vec<Expr> = c.fields.iter().map(|f| f.x).collect();
            let capp = self.store.apps(cl, &[sh.params.clone(), fx].concat());
            let body = motive_app(self, &c.ret_indices, capp);
            let binders: Vec<FVarId> = c
                .fields
                .iter()
                .map(|f| f.id)
                .chain(ih_ids.iter().copied())
                .collect();
            let minor_ty = self.bind(true, &binders, body)?;
            let (id, x) = self.local("minor", minor_ty)?;
            minors.push(Minor { id, x, ihs });
        }
        // The recursor type.
        let mut all_ids: Vec<FVarId> = sh.param_ids.clone();
        all_ids.push(motive_id);
        all_ids.extend(minors.iter().map(|m| m.id));
        all_ids.extend(sh.index_ids.iter().copied());
        all_ids.push(major_id);
        let concl = motive_app(self, &sh.indices, major);
        let rec_ty = self.bind(true, &all_ids, concl)?;
        // The computation rules.
        let mut fixed: Vec<Expr> = sh.params.clone();
        fixed.push(motive);
        fixed.extend(minors.iter().map(|m| m.x));
        let mut fixed_ids: Vec<FVarId> = sh.param_ids.clone();
        fixed_ids.push(motive_id);
        fixed_ids.extend(minors.iter().map(|m| m.id));
        let mut rules = Vec::with_capacity(ctors.len());
        let mut ctor_decls = Vec::with_capacity(ctors.len());
        for (j, (c, mn)) in ctors.iter().zip(&minors).enumerate() {
            self.tick()?;
            let mut args: Vec<Expr> = c.fields.iter().map(|f| f.x).collect();
            for (xids, js, bx) in &mn.ihs {
                let mut rargs = fixed.clone();
                rargs.extend(js.iter().copied());
                rargs.push(*bx);
                let call = self.store.apps(rec_c, &rargs);
                args.push(self.bind(false, xids, call)?);
            }
            let body = self.store.apps(mn.x, &args);
            let mut lam_ids = fixed_ids.clone();
            lam_ids.extend(c.fields.iter().map(|f| f.id));
            let rhs = self.bind(false, &lam_ids, body)?;
            rules.push(RecRule {
                ctor: c.name.clone(),
                num_fields: c.fields.len(),
                rhs,
            });
            ctor_decls.push(Declaration {
                name: c.name.clone(),
                level_params: d.level_params.clone(),
                ty: t.ctors[j].ty,
                kind: DeclKind::Constructor(ConstructorInfo {
                    induct: t.name.clone(),
                    index: j,
                    num_params: sh.nparams,
                    num_fields: c.fields.len(),
                }),
            });
        }
        let rec = Declaration {
            name: rec_name.clone(),
            level_params: rec_lps,
            ty: rec_ty,
            kind: DeclKind::Recursor(RecursorInfo {
                induct: t.name.clone(),
                num_params: sh.nparams,
                num_indices: sh.indices.len(),
                num_minors: ctors.len(),
                k,
                large_elim: !only_prop,
                rules,
            }),
        };
        Ok(Built {
            ctors: ctor_decls,
            rec,
        })
    }
}
