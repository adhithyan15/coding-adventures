//! Shared helpers for the inductive-type tests: a tiny term-building DSL over the
//! kernel's `ExprStore`, and the standard library of inductive types the tests build by
//! hand (`Bool`, `Nat`, `Eq`, `And`, `Or`, `False`, `True`, `Option`, `Exists`).
//!
//! Binders are written with closures: `pi(st, "x", ty, |st, x| body)` opens the binder
//! with a fresh free variable, builds the body, and abstracts the variable back, so test
//! terms never mention de Bruijn indices.
#![allow(dead_code)]

use lean_kernel::{
    Binder, BinderInfo, ConstructorDecl, Declaration, Environment, Expr, ExprStore, InductiveDecl,
    InductiveTypeDecl, KernelError, Level, Name, ReducibilityHints,
};

pub fn n(s: &str) -> Name {
    Name::from_dotted(s)
}
pub fn lvl(k: u32) -> Level {
    Level::of_nat(k)
}
pub fn lp(s: &str) -> Level {
    Level::param(&n(s))
}
pub fn binder(s: &str) -> Binder {
    Binder {
        name: n(s),
        info: BinderInfo::Default,
    }
}

/// A constant with no universe arguments.
pub fn c(st: &mut ExprStore, s: &str) -> Expr {
    st.constant(&n(s), &[])
}
/// A constant with universe arguments.
pub fn cl(st: &mut ExprStore, s: &str, ls: &[Level]) -> Expr {
    st.constant(&n(s), ls)
}
pub fn sort(st: &mut ExprStore, k: u32) -> Expr {
    st.sort(&lvl(k))
}
pub fn sort_l(st: &mut ExprStore, l: &Level) -> Expr {
    st.sort(l)
}
pub fn app(st: &mut ExprStore, f: Expr, args: &[Expr]) -> Expr {
    st.apps(f, args)
}

fn bind(
    st: &mut ExprStore,
    is_pi: bool,
    name: &str,
    ty: Expr,
    f: impl FnOnce(&mut ExprStore, Expr) -> Expr,
) -> Expr {
    let id = st.fresh_fvar_id();
    let x = st.fvar(id);
    let body = f(st, x);
    let body = st.abstract_fvars(body, &[id]);
    if is_pi {
        st.pi(binder(name), ty, body)
    } else {
        st.lam(binder(name), ty, body)
    }
}
pub fn pi(
    st: &mut ExprStore,
    name: &str,
    ty: Expr,
    f: impl FnOnce(&mut ExprStore, Expr) -> Expr,
) -> Expr {
    bind(st, true, name, ty, f)
}
pub fn lam(
    st: &mut ExprStore,
    name: &str,
    ty: Expr,
    f: impl FnOnce(&mut ExprStore, Expr) -> Expr,
) -> Expr {
    bind(st, false, name, ty, f)
}
/// `a -> b`, where `b` does not depend on the argument.
pub fn arrow(st: &mut ExprStore, a: Expr, b: Expr) -> Expr {
    pi(st, "_", a, |_, _| b)
}

pub fn ctor(name: &str, ty: Expr) -> ConstructorDecl {
    ConstructorDecl { name: n(name), ty }
}
pub fn single(
    level_params: &[&str],
    num_params: usize,
    name: &str,
    ty: Expr,
    ctors: Vec<ConstructorDecl>,
) -> InductiveDecl {
    InductiveDecl {
        level_params: level_params.iter().map(|s| n(s)).collect(),
        num_params,
        types: vec![InductiveTypeDecl {
            name: n(name),
            ty,
            ctors,
        }],
    }
}

/// `Nat` numeral as a `Nat.succ` chain.
pub fn num(st: &mut ExprStore, k: u32) -> Expr {
    let mut e = c(st, "Nat.zero");
    let s = c(st, "Nat.succ");
    for _ in 0..k {
        e = st.app(s, e);
    }
    e
}

/// `Eq.{u} ty a b`.
pub fn eq(st: &mut ExprStore, u: &Level, ty: Expr, a: Expr, b: Expr) -> Expr {
    let e = cl(st, "Eq", std::slice::from_ref(u));
    app(st, e, &[ty, a, b])
}
/// `Eq.refl.{u} ty a`.
pub fn refl(st: &mut ExprStore, u: &Level, ty: Expr, a: Expr) -> Expr {
    let e = cl(st, "Eq.refl", std::slice::from_ref(u));
    app(st, e, &[ty, a])
}

pub fn add_def(
    env: &mut Environment,
    name: &str,
    lps: &[&str],
    ty: Expr,
    val: Expr,
) -> Result<(), KernelError> {
    env.add_declaration(Declaration::definition(
        n(name),
        lps.iter().map(|s| n(s)).collect(),
        ty,
        val,
        ReducibilityHints::Regular(1),
    ))
}
pub fn add_thm(
    env: &mut Environment,
    name: &str,
    lps: &[&str],
    ty: Expr,
    val: Expr,
) -> Result<(), KernelError> {
    env.add_declaration(Declaration::theorem(
        n(name),
        lps.iter().map(|s| n(s)).collect(),
        ty,
        val,
    ))
}

// -------------------------------------------------------------------------------------
// The standard library of hand-built inductive types.
// -------------------------------------------------------------------------------------

pub fn add_bool(env: &mut Environment) {
    let st = &mut env.store;
    let ty = sort(st, 1);
    let b = c(st, "Bool");
    env.add_inductive(single(
        &[],
        0,
        "Bool",
        ty,
        vec![ctor("Bool.false", b), ctor("Bool.true", b)],
    ))
    .unwrap();
}

pub fn add_nat(env: &mut Environment) {
    let st = &mut env.store;
    let ty = sort(st, 1);
    let nat = c(st, "Nat");
    let succ_ty = arrow(st, nat, nat);
    env.add_inductive(single(
        &[],
        0,
        "Nat",
        ty,
        vec![ctor("Nat.zero", nat), ctor("Nat.succ", succ_ty)],
    ))
    .unwrap();
}

pub fn add_false(env: &mut Environment) {
    let st = &mut env.store;
    let ty = sort(st, 0);
    env.add_inductive(single(&[], 0, "False", ty, vec![]))
        .unwrap();
}

pub fn add_true(env: &mut Environment) {
    let st = &mut env.store;
    let ty = sort(st, 0);
    let t = c(st, "True");
    env.add_inductive(single(&[], 0, "True", ty, vec![ctor("True.intro", t)]))
        .unwrap();
}

/// `inductive Eq.{u} {α : Sort u} (a : α) : α → Prop | refl : Eq a a`
pub fn add_eq(env: &mut Environment) {
    let st = &mut env.store;
    let u = lp("u");
    let su = sort_l(st, &u);
    let prop = sort(st, 0);
    let ty = pi(st, "α", su, |st, a| {
        pi(st, "a", a, |st, _| arrow(st, a, prop))
    });
    let refl_ty = pi(st, "α", su, |st, a| {
        pi(st, "a", a, |st, x| eq(st, &u, a, x, x))
    });
    env.add_inductive(single(&["u"], 2, "Eq", ty, vec![ctor("Eq.refl", refl_ty)]))
        .unwrap();
}

/// `And (a b : Prop) : Prop | intro : a → b → And a b`
pub fn add_and(env: &mut Environment) {
    let st = &mut env.store;
    let prop = sort(st, 0);
    let ty = pi(st, "a", prop, |st, _| arrow(st, prop, prop));
    let and = c(st, "And");
    let intro = pi(st, "a", prop, |st, a| {
        pi(st, "b", prop, |st, b| {
            let ab = app(st, and, &[a, b]);
            let inner = arrow(st, b, ab);
            arrow(st, a, inner)
        })
    });
    env.add_inductive(single(&[], 2, "And", ty, vec![ctor("And.intro", intro)]))
        .unwrap();
}

/// `Or (a b : Prop) : Prop | inl : a → Or a b | inr : b → Or a b`
pub fn add_or(env: &mut Environment) {
    let st = &mut env.store;
    let prop = sort(st, 0);
    let ty = pi(st, "a", prop, |st, _| arrow(st, prop, prop));
    let or = c(st, "Or");
    let inl = pi(st, "a", prop, |st, a| {
        pi(st, "b", prop, |st, b| {
            let r = app(st, or, &[a, b]);
            arrow(st, a, r)
        })
    });
    let inr = pi(st, "a", prop, |st, a| {
        pi(st, "b", prop, |st, b| {
            let r = app(st, or, &[a, b]);
            arrow(st, b, r)
        })
    });
    env.add_inductive(single(
        &[],
        2,
        "Or",
        ty,
        vec![ctor("Or.inl", inl), ctor("Or.inr", inr)],
    ))
    .unwrap();
}

/// `Option.{u} (α : Type u) : Type u | none | some : α → Option α`
pub fn add_option(env: &mut Environment) {
    let st = &mut env.store;
    let u = lp("u");
    let tu = sort_l(st, &Level::succ(&u));
    let ty = pi(st, "α", tu, |_, _| tu);
    let opt = cl(st, "Option", std::slice::from_ref(&u));
    let none = pi(st, "α", tu, |st, a| app(st, opt, &[a]));
    let some = pi(st, "α", tu, |st, a| {
        let r = app(st, opt, &[a]);
        arrow(st, a, r)
    });
    env.add_inductive(single(
        &["u"],
        1,
        "Option",
        ty,
        vec![ctor("Option.none", none), ctor("Option.some", some)],
    ))
    .unwrap();
}

/// `Exists.{u} {α : Sort u} (p : α → Prop) : Prop | intro : (w : α) → p w → Exists p`
pub fn add_exists(env: &mut Environment) {
    let st = &mut env.store;
    let u = lp("u");
    let su = sort_l(st, &u);
    let prop = sort(st, 0);
    let ty = pi(st, "α", su, |st, a| {
        let pt = arrow(st, a, prop);
        arrow(st, pt, prop)
    });
    let ex = cl(st, "Exists", std::slice::from_ref(&u));
    let intro = pi(st, "α", su, |st, a| {
        let pt = arrow(st, a, prop);
        pi(st, "p", pt, |st, p| {
            pi(st, "w", a, |st, w| {
                let pw = app(st, p, &[w]);
                let r = app(st, ex, &[a, p]);
                arrow(st, pw, r)
            })
        })
    });
    env.add_inductive(single(
        &["u"],
        2,
        "Exists",
        ty,
        vec![ctor("Exists.intro", intro)],
    ))
    .unwrap();
}

/// `List.{u} (α : Type u) : Type u | nil | cons : α → List α → List α`
pub fn add_list(env: &mut Environment) {
    let st = &mut env.store;
    let u = lp("u");
    let tu = sort_l(st, &Level::succ(&u));
    let ty = pi(st, "α", tu, |_, _| tu);
    let list = cl(st, "List", std::slice::from_ref(&u));
    let nil = pi(st, "α", tu, |st, a| app(st, list, &[a]));
    let cons = pi(st, "α", tu, |st, a| {
        let la = app(st, list, &[a]);
        let tail = arrow(st, la, la);
        arrow(st, a, tail)
    });
    env.add_inductive(single(
        &["u"],
        1,
        "List",
        ty,
        vec![ctor("List.nil", nil), ctor("List.cons", cons)],
    ))
    .unwrap();
}

/// An environment with every standard type.
pub fn std_env() -> Environment {
    let mut env = Environment::new();
    add_bool(&mut env);
    add_nat(&mut env);
    add_false(&mut env);
    add_true(&mut env);
    add_eq(&mut env);
    add_and(&mut env);
    add_or(&mut env);
    add_option(&mut env);
    add_exists(&mut env);
    env
}
