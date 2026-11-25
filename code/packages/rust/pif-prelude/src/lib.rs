//! The PIF prelude generator (VER32 backlog item PIF-004b; spec sections 6.4 and 9.2).
//!
//! # What the prelude is
//!
//! A gate claim needs a small, reviewed vocabulary: the Booleans, the naturals, equality,
//! the logical connectives. The **PIF prelude** is that vocabulary as one PIF file, the first
//! file of every environment chain (VER32 3.7). It has two halves:
//!
//! 1. **The inductive types, under Lean's own names with Lean's own declarations.** `Bool`,
//!    `Nat`, `Int`, `Eq`, `And`, `Or`, `False`, `True`, `Exists`, `Option`, `List` and `Prod`
//!    have the universe parameters (and their names), parameter counts, indices, constructor
//!    names and constructor types that Lean 4's `Init` gives them, up to binder names, which
//!    are spelled in the ASCII class (`alpha` where Lean writes the Greek letter, R-NAM-3).
//!    `oracle/expected.sexp`, produced by running Lean once, pins this, and the tests compare
//!    every type, constructor and derived recursor with it.
//! 2. **Everything else lives in `Pif.*`**: `Pif.Not`, `Pif.not`, `Pif.and`, `Pif.or`,
//!    `Pif.xor`, `Pif.Nat.add` and `Pif.Result`. Their values are written
//!    directly with recursors (`Bool.rec`, `Nat.rec`), with no literals.
//!
//! # Why the native prelude is not `Init`
//!
//! `Init`'s function values are elaborator output: `Bool.and` goes through `match_1` and
//! `casesOn`, `Nat.add` through `brecOn` and `PProd`, `Prod.fst` is a projection (slice 3b of
//! the kernel). So **only the inductive types keep Lean's names**; the functions are our own
//! and are named `Pif.*`, so a later import of `Init` can never clash with them, and G-M1
//! proofs are regenerated or bridged, never silently shared (VER32 6.4).
//!
//! # Deliberate deviations from `Init` (each is listed in the README)
//!
//! * `And` and `Prod` are structures in Lean. Here they are plain one-constructor inductives
//!   with the same constructor type, because the kernel has no projections yet (slice 3b).
//!   Their types, constructors and recursors are identical.
//! * `Not` is a definition in Lean (`fun a => a -> False`). It is `Pif.Not` here, with the
//!   same type and value.
//! * `Iff` and `Nonempty` are not here: VER32 does not list them in the prelude (they are
//!   pinned separately for the `prim` roles of 3.5, a later slice). The oracle already records
//!   them.
//! * `Pif.Result` is here because VER32 6.4 and the PIF-004 row list it, although the chain
//!   paragraph of 9.2 says the ADL prelude holds `Result`; it is a generic result type. The
//!   gate-specific `GateError` is not here: it belongs in the gate model file (VER33 ADL-002).
//!   See README.
//!
//! # Trust
//!
//! This crate is an untrusted producer: the file it emits is read by the trusted reader,
//! which re-derives every recursor and type-checks every definition. The prelude's digest is
//! allow-listed by the gate and signed under `pif.prelude.faithful` (VER32 9.2).

use pif_writer::{BinderInfo, Expr, Hint, IndType, Level, Name, Writer};

use BinderInfo::{Default as D, Implicit as I};

/// A writer plus the little vocabulary of helpers the declarations below share.
struct G {
    w: Writer,
}

impl G {
    fn n(&mut self, s: &str) -> Name {
        self.w.name(s).expect("prelude names are valid ASCII")
    }
    fn c(&mut self, s: &str, ls: &[Level]) -> Expr {
        let n = self.n(s);
        self.w.constant(n, ls)
    }
    fn v(&mut self, i: u32) -> Expr {
        self.w.var(i)
    }
    fn pi(&mut self, info: BinderInfo, b: &str, ty: Expr, body: Expr) -> Expr {
        let n = self.n(b);
        self.w.pi(info, n, ty, body)
    }
    fn lam(&mut self, b: &str, ty: Expr, body: Expr) -> Expr {
        let n = self.n(b);
        self.w.lam(D, n, ty, body)
    }
    /// `a -> b` (a non-dependent Pi; the binder name is irrelevant).
    fn arrow(&mut self, a: Expr, b: Expr) -> Expr {
        self.pi(D, "a", a, b)
    }
    fn ap(&mut self, f: Expr, args: &[Expr]) -> Expr {
        self.w.app_many(f, args)
    }
    fn param(&mut self, s: &str) -> (Name, Level) {
        let n = self.n(s);
        (n, self.w.level_param(n))
    }
    /// One non-mutual inductive type; constructors are `(dotted name, type)`.
    fn inductive(&mut self, ps: &[Name], np: u32, name: &str, ty: Expr, ctors: &[(&str, Expr)]) {
        let name = self.n(name);
        let ctors = ctors.iter().map(|(s, t)| (self.n(s), *t)).collect();
        let t = IndType { name, ty, ctors };
        self.w.inductive(ps, np, &[t]).expect("well-formed block");
    }
    fn def(&mut self, name: &str, ty: Expr, val: Expr, height: u32) {
        let n = self.n(name);
        let h = Hint::Regular(height);
        self.w.definition(n, &[], ty, val, h).expect("definition");
    }
}

/// The universe `Type u` is `Sort (u + 1)`.
fn type_of(g: &mut G, u: Level) -> Expr {
    let s = g.w.level_succ(u);
    g.w.sort(s)
}

/// The whole prelude as canonical PIF text: a pure function with no inputs, so the committed
/// copy can be diffed against it.
pub fn generate() -> String {
    let w = Writer::new("native", "pif-prelude 0.1").expect("profile");
    let mut g = G {
        w: w.ascii_names_only(),
    };
    let (prop, type0) = (g.w.sort(Level::ZERO), type_of(&mut g, Level::ZERO));
    bool_nat_int(&mut g, type0);
    logic(&mut g, prop);
    containers(&mut g);
    natives(&mut g, prop);
    g.w.to_text()
}

/// `Bool`, `Nat` and `Int`: the data types of the gate model (all in `Type`).
fn bool_nat_int(g: &mut G, ty: Expr) {
    let b = g.c("Bool", &[]);
    g.inductive(&[], 0, "Bool", ty, &[("Bool.false", b), ("Bool.true", b)]);
    let n = g.c("Nat", &[]);
    let succ = g.arrow(n, n);
    g.inductive(&[], 0, "Nat", ty, &[("Nat.zero", n), ("Nat.succ", succ)]);
    let i = g.c("Int", &[]);
    let to_int = g.arrow(n, i);
    g.inductive(
        &[],
        0,
        "Int",
        ty,
        &[("Int.ofNat", to_int), ("Int.negSucc", to_int)],
    );
}

/// `Eq`, `And`, `Or`, `False`, `True` and `Exists`: the propositional vocabulary.
fn logic(g: &mut G, prop: Expr) {
    // Eq.{u_1} : {alpha : Sort u_1} -> alpha -> alpha -> Prop, one constructor, one index.
    let (u1n, u1) = g.param("u_1");
    let sort = g.w.sort(u1);
    let (v0, v1) = (g.v(0), g.v(1));
    let t = g.pi(D, "_", v1, prop);
    let t = g.pi(D, "a", v0, t);
    let eq_ty = g.pi(I, "alpha", sort, t);
    let eq = g.c("Eq", &[u1]);
    let body = g.ap(eq, &[v1, v0, v0]);
    let t = g.pi(D, "a", v0, body);
    let refl = g.pi(I, "alpha", sort, t);
    g.inductive(&[u1n], 2, "Eq", eq_ty, &[("Eq.refl", refl)]);

    // And and Or : Prop -> Prop -> Prop. And is a plain one-constructor inductive (no projections).
    let t = g.arrow(prop, prop);
    let conn = g.arrow(prop, t);
    let (and, or) = (g.c("And", &[]), g.c("Or", &[]));
    let (v2, v3) = (g.v(2), g.v(3));
    let body = g.ap(and, &[v3, v2]);
    let t = g.pi(D, "right", v1, body);
    let t = g.pi(D, "left", v1, t);
    let t = g.pi(I, "b", prop, t);
    let intro = g.pi(I, "a", prop, t);
    g.inductive(&[], 2, "And", conn, &[("And.intro", intro)]);
    let body = g.ap(or, &[v2, v1]);
    let t = g.pi(D, "h", v1, body);
    let t = g.pi(I, "b", prop, t);
    let inl = g.pi(I, "a", prop, t);
    let t = g.pi(D, "h", v0, body);
    let t = g.pi(I, "b", prop, t);
    let inr = g.pi(I, "a", prop, t);
    g.inductive(&[], 2, "Or", conn, &[("Or.inl", inl), ("Or.inr", inr)]);

    g.inductive(&[], 0, "False", prop, &[]);
    let tr = g.c("True", &[]);
    g.inductive(&[], 0, "True", prop, &[("True.intro", tr)]);

    // Exists.{u} : {alpha : Sort u} -> (alpha -> Prop) -> Prop
    let (un, u) = g.param("u");
    let sort = g.w.sort(u);
    let pred = g.arrow(v0, prop);
    let t = g.pi(D, "p", pred, prop);
    let ex_ty = g.pi(I, "alpha", sort, t);
    let ex = g.c("Exists", &[u]);
    let body = g.ap(ex, &[v3, v2]);
    let hyp = g.ap(v1, &[v0]);
    let t = g.pi(D, "h", hyp, body);
    let t = g.pi(D, "w", v1, t);
    let t = g.pi(I, "p", pred, t);
    let intro = g.pi(I, "alpha", sort, t);
    g.inductive(&[un], 2, "Exists", ex_ty, &[("Exists.intro", intro)]);
}

/// `Option`, `List` and `Prod`: the universe-polymorphic containers.
fn containers(g: &mut G) {
    let (un, u) = g.param("u");
    let ty_u = type_of(g, u);
    let (v0, v1, v2, v3) = (g.v(0), g.v(1), g.v(2), g.v(3));
    // Option.{u} (alpha : Type u) : Type u
    let opt_ty = g.pi(D, "alpha", ty_u, ty_u);
    let opt = g.c("Option", &[u]);
    let none = {
        let body = g.ap(opt, &[v0]);
        g.pi(I, "alpha", ty_u, body)
    };
    let body = g.ap(opt, &[v1]);
    let t = g.pi(D, "val", v0, body);
    let some = g.pi(I, "alpha", ty_u, t);
    g.inductive(
        &[un],
        1,
        "Option",
        opt_ty,
        &[("Option.none", none), ("Option.some", some)],
    );
    // List.{u} (alpha : Type u) : Type u, recursive in its tail
    let list = g.c("List", &[u]);
    let nil = {
        let body = g.ap(list, &[v0]);
        g.pi(I, "alpha", ty_u, body)
    };
    let tail = g.ap(list, &[v1]);
    let body = g.ap(list, &[v2]);
    let t = g.pi(D, "tail", tail, body);
    let t = g.pi(D, "head", v0, t);
    let cons = g.pi(I, "alpha", ty_u, t);
    g.inductive(
        &[un],
        1,
        "List",
        opt_ty,
        &[("List.nil", nil), ("List.cons", cons)],
    );
    // Prod.{u,v} (alpha : Type u) (beta : Type v) : Type (max u v)
    let (vn, v) = g.param("v");
    let ty_v = type_of(g, v);
    let (su, sv) = (g.w.level_succ(u), g.w.level_succ(v));
    let m = g.w.level_max(su, sv);
    let out = g.w.sort(m);
    let t = g.pi(D, "beta", ty_v, out);
    let prod_ty = g.pi(D, "alpha", ty_u, t);
    let prod = g.c("Prod", &[u, v]);
    let body = g.ap(prod, &[v3, v2]);
    let t = g.pi(D, "snd", v1, body);
    let t = g.pi(D, "fst", v1, t);
    let t = g.pi(I, "beta", ty_v, t);
    let mk = g.pi(I, "alpha", ty_u, t);
    g.inductive(&[un, vn], 2, "Prod", prod_ty, &[("Prod.mk", mk)]);
}

/// `Pif.*`: negation, the Boolean functions, `Nat` addition and the gate result type.
fn natives(g: &mut G, prop: Expr) {
    // Pif.Not (a : Prop) : Prop := a -> False
    let (fls, v0) = (g.c("False", &[]), g.v(0));
    let body = g.arrow(v0, fls);
    let val = g.lam("a", prop, body);
    let ty = g.arrow(prop, prop);
    g.def("Pif.Not", ty, val, 1);

    // Boolean functions by Bool.rec.{1} with the constant motive `fun _ => Bool`:
    //   Bool.rec motive (value at false) (value at true) (the Bool being split)
    let one = g.w.level_succ(Level::ZERO);
    let bool_t = g.c("Bool", &[]);
    let (ff, tt) = (g.c("Bool.false", &[]), g.c("Bool.true", &[]));
    let rec = g.c("Bool.rec", &[one]);
    let motive = g.lam("_", bool_t, bool_t);
    let v1 = g.v(1);
    let un = g.arrow(bool_t, bool_t);
    let bin = g.arrow(bool_t, un);
    let not_c = g.c("Pif.not", &[]);
    let r = g.ap(rec, &[motive, tt, ff, v0]);
    let val = g.lam("b", bool_t, r);
    g.def("Pif.not", un, val, 1);
    let not_b = g.ap(not_c, &[v0]);
    // a, b are #1, #0 inside `fun a b`; the cases are "a = false" then "a = true"
    for (name, at_false, at_true, height) in [
        ("Pif.and", ff, v0, 1),
        ("Pif.or", v0, tt, 1),
        ("Pif.xor", v0, not_b, 2),
    ] {
        let r = g.ap(rec, &[motive, at_false, at_true, v1]);
        let inner = g.lam("b", bool_t, r);
        let val = g.lam("a", bool_t, inner);
        g.def(name, bin, val, height);
    }

    // Pif.Nat.add n m := Nat.rec (fun _ => Nat) n (fun k ih => succ ih) m, recursion on m
    let (nat, zero_rec) = (g.c("Nat", &[]), g.c("Nat.rec", &[one]));
    let succ = g.c("Nat.succ", &[]);
    let nat_motive = g.lam("_", nat, nat);
    let step_body = g.ap(succ, &[v0]);
    let step = g.lam("ih", nat, step_body);
    let step = g.lam("k", nat, step);
    let r = g.ap(zero_rec, &[nat_motive, v1, step, v0]);
    let inner = g.lam("m", nat, r);
    let val = g.lam("n", nat, inner);
    let un = g.arrow(nat, nat);
    let ty = g.arrow(nat, un);
    g.def("Pif.Nat.add", ty, val, 1);

    // Pif.Result.{u,v} (alpha : Type u) (eps : Type v) : Type (max u v), a generic result
    // type. The gate-specific error type is NOT here: it belongs in the gate model file.
    let (un, u) = g.param("u");
    let (vn, v) = g.param("v");
    let (ty_u, ty_v) = (type_of(g, u), type_of(g, v));
    let (su, sv) = (g.w.level_succ(u), g.w.level_succ(v));
    let m = g.w.level_max(su, sv);
    let out = g.w.sort(m);
    let t = g.pi(D, "eps", ty_v, out);
    let res_ty = g.pi(D, "alpha", ty_u, t);
    let res = g.c("Pif.Result", &[u, v]);
    let (v2, v1) = (g.v(2), g.v(1));
    let body = g.ap(res, &[v2, v1]);
    let t = g.pi(D, "val", v1, body);
    let t = g.pi(I, "eps", ty_v, t);
    let ok = g.pi(I, "alpha", ty_u, t);
    let v0 = g.v(0);
    let t = g.pi(D, "err", v0, body);
    let t = g.pi(I, "eps", ty_v, t);
    let err = g.pi(I, "alpha", ty_u, t);
    let ctors = [("Pif.Result.ok", ok), ("Pif.Result.err", err)];
    g.inductive(&[un, vn], 2, "Pif.Result", res_ty, &ctors);
}
