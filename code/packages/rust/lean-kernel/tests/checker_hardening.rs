//! Resource limits and unsound-acceptance regressions for the type checker.
//!
//! The kernel reads untrusted terms: it must answer (usually "no") in bounded time and
//! stack for any input, and it must never accept a proof of `False`.

use lean_kernel::{
    Binder, BinderInfo, Declaration, Environment, Expr, ExprStore, KernelError, Level, Limits,
    Name, ReducibilityHints,
};
use std::time::{Duration, Instant};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}
fn b(s: &str) -> Binder {
    Binder {
        name: n(s),
        info: BinderInfo::Default,
    }
}
fn con(st: &mut ExprStore, s: &str) -> Expr {
    st.constant(&n(s), &[])
}
fn sort(st: &mut ExprStore, k: u32) -> Expr {
    st.sort(&Level::of_nat(k))
}

fn base() -> Environment {
    let mut env = Environment::new();
    let ty = sort(&mut env.store, 1);
    env.add_declaration(Declaration::axiom(n("Nat"), vec![], ty))
        .unwrap();
    let nat = con(&mut env.store, "Nat");
    env.add_declaration(Declaration::axiom(n("Nat.zero"), vec![], nat))
        .unwrap();
    env
}

fn is_limit<T: std::fmt::Debug>(r: Result<T, KernelError>) -> bool {
    matches!(r, Err(KernelError::ResourceLimit(_)))
}

// ------------------------------------------------------------ deep terms

/// Run `f` on a thread with a deliberately small stack: if the checker recursed over a
/// 100,000-deep term it would overflow and abort the whole test binary.
fn on_small_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}

const DEEP: usize = 100_000;

#[test]
fn deeply_nested_terms_are_a_resource_limit_not_a_crash() {
    on_small_stack(|| {
        let mut env = base();
        let st = &mut env.store;
        let (nat, zero) = (con(st, "Nat"), con(st, "Nat.zero"));
        // left spine: ((zero zero) zero) ...
        let mut spine = zero;
        // right-nested: zero (zero (zero ...))
        let mut right = zero;
        // binders: fun _ : Nat => fun _ : Nat => ... zero
        let mut lams = zero;
        let mut pis = nat;
        let mut lets = zero;
        for _ in 0..DEEP {
            spine = st.app(spine, zero);
            right = st.app(zero, right);
            lams = st.lam(b("x"), nat, lams);
            pis = st.pi(b("x"), nat, pis);
            lets = st.let_e(b("x"), nat, zero, lets);
        }
        {
            let mut tc = env.type_checker(&[]);
            for deep in [spine, right, lams, pis, lets] {
                assert!(is_limit(tc.infer(deep)), "infer");
                assert!(is_limit(tc.whnf(deep)), "whnf");
                assert!(is_limit(tc.is_def_eq(deep, nat)), "is_def_eq");
            }
        }
        // as a declaration, and the environment is unchanged
        let before = env.len();
        for (i, deep) in [spine, right, lams, pis, lets].into_iter().enumerate() {
            let d = Declaration::definition(
                n(&format!("deep{i}")),
                vec![],
                nat,
                deep,
                ReducibilityHints::Abbrev,
            );
            assert!(is_limit(env.add_declaration(d)));
        }
        assert_eq!(env.len(), before);
    });
}

#[test]
fn term_that_grows_by_reduction_is_bounded() {
    on_small_stack(|| {
        // (fun x => (x, x)) doubling is size-bounded by the step/node budgets; a
        // self-application loop never terminates, so it must hit the step budget.
        let mut env = base();
        let st = &mut env.store;
        let nat = con(st, "Nat");
        let b0 = st.bvar(0);
        let xx = st.app(b0, b0);
        let omega_f = st.lam(b("x"), nat, xx);
        let omega = st.app(omega_f, omega_f);
        let limits = Limits {
            max_steps: 10_000,
            ..Limits::default()
        };
        let mut tc = env.type_checker(&[]).with_limits(limits);
        assert!(is_limit(tc.whnf(omega)));
        // the default budget is finite too (2M steps; not run here, it takes seconds)
        assert_eq!(Limits::default().max_steps, 2_000_000);
    });
}

#[test]
fn each_limit_reports_resource_limit() {
    let mut env = base();
    let st = &mut env.store;
    let (nat, zero) = (con(st, "Nat"), con(st, "Nat.zero"));
    let id_ty = {
        let (b0, b1) = (st.bvar(0), st.bvar(1));
        let a = st.pi(b("a"), b0, b1);
        let t = sort(st, 1);
        st.pi(b("A"), t, a)
    };
    let id_val = {
        let (b0, t) = (st.bvar(0), sort(st, 1));
        let inner = st.lam(b("a"), b0, b0);
        st.lam(b("A"), t, inner)
    };
    env.add_declaration(Declaration::definition(
        n("idT"),
        vec![],
        id_ty,
        id_val,
        ReducibilityHints::Abbrev,
    ))
    .unwrap();
    let st = &mut env.store;
    let idt = con(st, "idT");
    let inner = st.apps(idt, &[nat, zero]);
    let nested = st.apps(idt, &[nat, inner]);
    // (first, while the checker still has to create the instantiated types itself:
    // the store is hash-consed, so a later run would find them already interned)
    // node budget: any new expression exceeds a budget of zero
    let mut tc = env.type_checker(&[]).with_limits(Limits {
        max_new_nodes: 0,
        ..Limits::default()
    });
    assert!(is_limit(tc.infer(nested)));
    // call depth
    drop(tc);
    let mut tc = env.type_checker(&[]).with_limits(Limits {
        max_call_depth: 2,
        ..Limits::default()
    });
    assert!(is_limit(tc.infer(nested)));
    // term depth
    drop(tc);
    let mut tc = env.type_checker(&[]).with_limits(Limits {
        max_term_depth: 3,
        ..Limits::default()
    });
    assert!(is_limit(tc.infer(nested)));
    // step budget
    drop(tc);
    let mut tc = env.type_checker(&[]).with_limits(Limits {
        max_steps: 3,
        ..Limits::default()
    });
    assert!(is_limit(tc.infer(nested)));
    // defaults accept it
    drop(tc);
    assert!(env.type_checker(&[]).infer(nested).is_ok());
}

#[test]
fn long_binder_telescope_and_spine_do_not_use_recursion_depth() {
    // 150 nested binders and a 150-argument spine are well inside the term-depth limit
    // and must work even with a call-depth limit far smaller than 150.
    let mut env = base();
    let st = &mut env.store;
    let (nat, zero) = (con(st, "Nat"), con(st, "Nat.zero"));
    let mut lam = zero;
    let mut ty = nat;
    for _ in 0..150 {
        lam = st.lam(b("x"), nat, lam);
        ty = st.pi(b("x"), nat, ty);
    }
    let mut tc = env.type_checker(&[]).with_limits(Limits {
        max_call_depth: 8,
        ..Limits::default()
    });
    assert_eq!(tc.infer(lam).unwrap(), ty);
    let args = vec![zero; 150];
    let st = tc.store();
    let app = st.apps(lam, &args);
    assert_eq!(tc.infer(app).unwrap(), nat);
}

// ------------------------------------------------------ exponential delta

/// `f0 := Nat -> Nat`, `f(k+1) := f k -> f k`, for each prefix in `names`.
fn add_chain(env: &mut Environment, prefix: &str, depth: usize, base_val: Expr) {
    let ty = sort(&mut env.store, 1);
    env.add_declaration(Declaration::definition(
        n(&format!("{prefix}0")),
        vec![],
        ty,
        base_val,
        ReducibilityHints::Regular(0),
    ))
    .unwrap();
    for k in 1..=depth {
        let prev = con(&mut env.store, &format!("{prefix}{}", k - 1));
        let val = env.store.pi(b("_"), prev, prev);
        env.add_declaration(Declaration::definition(
            n(&format!("{prefix}{k}")),
            vec![],
            ty,
            val,
            ReducibilityHints::Regular(k as u32),
        ))
        .unwrap();
    }
}

#[test]
fn exponential_delta_chain_is_fast_or_limited_never_a_hang() {
    let mut env = base();
    let nat = con(&mut env.store, "Nat");
    let nn = env.store.pi(b("_"), nat, nat);
    add_chain(&mut env, "f", 40, nn);
    add_chain(&mut env, "g", 40, nn);
    // a chain that differs at the bottom: h0 := Nat -> Nat -> Nat
    let inner = env.store.pi(b("_"), nat, nn);
    add_chain(&mut env, "h", 40, inner);
    let (f40, g40, h40) = (
        con(&mut env.store, "f40"),
        con(&mut env.store, "g40"),
        con(&mut env.store, "h40"),
    );
    let start = Instant::now();
    let mut tc = env.type_checker(&[]);
    // the same shape: the verdict cache makes this linear, not 2^40
    assert_eq!(tc.is_def_eq(f40, g40), Ok(true));
    // different shapes: answered no, quickly
    assert_eq!(tc.is_def_eq(f40, h40), Ok(false));
    assert!(start.elapsed() < Duration::from_secs(20));
    // with a small work budget the same question is a resource limit, not a hang
    drop(tc);
    let mut tc = env.type_checker(&[]).with_limits(Limits {
        max_steps: 50,
        ..Limits::default()
    });
    assert!(is_limit(tc.is_def_eq(f40, g40)));
}

// ----------------------------------------------- unsound-acceptance attempts

fn env_with_false() -> Environment {
    let mut env = base();
    let prop = sort(&mut env.store, 0);
    env.add_declaration(Declaration::axiom(n("False"), vec![], prop))
        .unwrap();
    env
}

#[test]
fn false_from_nothing_is_rejected() {
    let mut env = env_with_false();
    let st = &mut env.store;
    let (fls, zero, nat, b0) = (
        con(st, "False"),
        con(st, "Nat.zero"),
        con(st, "Nat"),
        st.bvar(0),
    );
    let sort0 = sort(st, 0);
    let id_false = st.lam(b("h"), fls, b0);
    let bad_proofs = [
        zero,                   // a Nat, not a proof of False
        nat,                    // a type
        sort0,                  // Prop itself
        id_false,               // False -> False
        con(st, "oops"),        // a constant that does not exist
        st.app(id_false, zero), // ill-typed application
    ];
    for (i, v) in bad_proofs.into_iter().enumerate() {
        let name = format!("bad{i}");
        let d = Declaration::theorem(n(&name), vec![], fls, v);
        assert!(env.add_declaration(d).is_err(), "bad{i} was accepted");
        assert!(!env.contains(&n(&name)));
    }
    // sanity: `False -> False` is provable, so the harness is not rejecting everything
    let st = &mut env.store;
    let (fls, b0) = (con(st, "False"), st.bvar(0));
    let ty = st.pi(b("h"), fls, fls);
    let val = st.lam(b("h"), fls, b0);
    env.add_declaration(Declaration::theorem(n("fimpf"), vec![], ty, val))
        .unwrap();
}

#[test]
fn theorems_and_opaques_are_never_unfolded_to_make_a_declaration_check() {
    let mut env = env_with_false();
    let st = &mut env.store;
    let (nat, zero, ty1) = (con(st, "Nat"), con(st, "Nat.zero"), sort(st, 1));
    // axiom G : Nat -> Type ; axiom g : G Nat.zero
    let g_ty = st.pi(b("_"), nat, ty1);
    env.add_declaration(Declaration::axiom(n("G"), vec![], g_ty))
        .unwrap();
    let st = &mut env.store;
    let (gc, zero2) = (con(st, "G"), zero);
    let g_zero = st.app(gc, zero2);
    env.add_declaration(Declaration::axiom(n("g"), vec![], g_zero))
        .unwrap();
    // opaque o : Nat := Nat.zero
    env.add_declaration(Declaration::opaque(n("o"), vec![], nat, zero))
        .unwrap();
    // def bad : G o := g   -- would need o ~> Nat.zero, which opaque forbids
    let st = &mut env.store;
    let (gc, o, gv) = (con(st, "G"), con(st, "o"), con(st, "g"));
    let g_o = st.app(gc, o);
    let d = Declaration::definition(n("bad"), vec![], g_o, gv, ReducibilityHints::Abbrev);
    assert!(matches!(
        env.add_declaration(d),
        Err(KernelError::TypeMismatch { .. })
    ));
    // whnf never exposes a theorem body: the constant is returned as is
    let st = &mut env.store;
    let (fls, b0) = (con(st, "False"), st.bvar(0));
    let ty = st.pi(b("h"), fls, fls);
    let val = st.lam(b("h"), fls, b0);
    env.add_declaration(Declaration::theorem(n("fimpf"), vec![], ty, val))
        .unwrap();
    let t = con(&mut env.store, "fimpf");
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.whnf(t).unwrap(), t);
    // Since slice 3a the two proofs of `False -> False` ARE definitionally equal, but by
    // proof irrelevance (the type is a Prop), not by unfolding: whnf above is unchanged.
    assert!(tc.is_def_eq(t, val).unwrap());
    // nor an axiom
    let gcst = con(tc.store(), "g");
    assert_eq!(tc.whnf(gcst).unwrap(), gcst);
}

#[test]
fn level_confusion_is_rejected() {
    let mut env = env_with_false();
    let st = &mut env.store;
    let (nat, zero, prop, ty1) = (
        con(st, "Nat"),
        con(st, "Nat.zero"),
        sort(st, 0),
        sort(st, 1),
    );
    // `Nat : Sort 1` used where `Sort 0` is required
    let d = Declaration::definition(n("c1"), vec![], prop, nat, ReducibilityHints::Abbrev);
    assert!(matches!(
        env.add_declaration(d),
        Err(KernelError::TypeMismatch { .. })
    ));
    // id.{0} Nat zero: the first argument must live in Sort 0
    let st = &mut env.store;
    let sb0 = {
        let (b0, b1) = (st.bvar(0), st.bvar(1));
        let su = st.sort(&Level::param(&n("u")));
        let a = st.pi(b("a"), b0, b1);
        st.pi(b("A"), su, a)
    };
    let id_val = {
        let (b0, su) = (st.bvar(0), st.sort(&Level::param(&n("u"))));
        let inner = st.lam(b("a"), b0, b0);
        st.lam(b("A"), su, inner)
    };
    env.add_declaration(Declaration::definition(
        n("idU"),
        vec![n("u")],
        sb0,
        id_val,
        ReducibilityHints::Abbrev,
    ))
    .unwrap();
    let st = &mut env.store;
    let idc = st.constant(&n("idU"), &[Level::zero()]);
    let bad = st.apps(idc, &[nat, zero]);
    let mut tc = env.type_checker(&[]);
    assert!(matches!(
        tc.infer(bad),
        Err(KernelError::TypeMismatch { .. })
    ));
    // Type : Type (a universe inconsistency) is not derivable: Sort 1 : Sort 1 fails
    let st = tc.store();
    let bad_ty = st.pi(b("x"), ty1, ty1); // fine: Type -> Type : Sort 2
    let s = tc.infer(bad_ty).unwrap();
    assert!(tc.ensure_sort(s).unwrap().is_equiv(&Level::of_nat(2)));
    assert!(!tc.is_def_eq(s, ty1).unwrap());
    // and Prop is not a theorem statement, Type is not a Prop
    drop(tc);
    let d = Declaration::theorem(n("t"), vec![], ty1, nat);
    assert!(matches!(
        env.add_declaration(d),
        Err(KernelError::TheoremNotProp(_))
    ));
}

#[test]
fn universe_levels_must_be_declared_even_when_nested_in_arguments() {
    let mut env = base();
    // axiom a2.{v} : (fun (A : Sort u) => A) Nat   -- `u` is not among the parameters
    let nat = con(&mut env.store, "Nat");
    let b0 = env.store.bvar(0);
    let su = env.store.sort(&Level::param(&n("u")));
    let lam = env.store.lam(b("A"), su, b0);
    let app = env.store.app(lam, nat);
    let d = Declaration::axiom(n("a2"), vec![n("v")], app);
    assert!(env.add_declaration(d).is_err());
    assert!(!env.contains(&n("a2")));
}

#[test]
fn literals_are_not_typed_by_a_constants_name() {
    // `def Nat : Prop := False` makes the *name* Nat mean False; typing `5` as that
    // constant would prove False. Literals are Unsupported until slice 3, whatever the
    // environment holds under those names.
    for is_nat in [true, false] {
        let name = if is_nat { "Nat" } else { "String" };
        let mut env = Environment::new();
        let prop = sort(&mut env.store, 0);
        env.add_declaration(Declaration::axiom(n("False"), vec![], prop))
            .unwrap();
        let fls = con(&mut env.store, "False");
        env.add_declaration(Declaration::definition(
            n(name),
            vec![],
            prop,
            fls,
            ReducibilityHints::Abbrev,
        ))
        .unwrap();
        let v = if is_nat {
            env.store.nat_lit(bignum_core::BigInteger::from_i64(5))
        } else {
            env.store.str_lit("x")
        };
        let d = Declaration::theorem(n("bad"), vec![], fls, v);
        assert!(env.add_declaration(d).is_err(), "{name} literal accepted");
        assert!(!env.contains(&n("bad")));
        assert!(matches!(
            env.type_checker(&[]).infer(v),
            Err(KernelError::Unsupported(_))
        ));
    }
}
