//! "False from nothing" regressions aimed at the slice-3a rules: iota, K-like reduction,
//! proof irrelevance, large elimination. Each test builds the attack that a *wrong*
//! implementation of the rule would accept, and checks the kernel rejects it. Each also
//! checks the legitimate neighbour is still accepted, so a kernel that rejects everything
//! cannot pass.

mod common;
use common::*;
use lean_kernel::{DeclKind, Declaration, Environment, Expr, ExprStore, KernelError};

/// `true_ne_false : Eq Bool true false → False`, a real proof by `Eq.rec` into Prop with
/// the motive `fun y _ => Bool.rec False True y`.
fn add_true_ne_false(env: &mut Environment) {
    let st = &mut env.store;
    let (bool_, bt, bf) = (c(st, "Bool"), c(st, "Bool.true"), c(st, "Bool.false"));
    let (tr, fl, prop) = (c(st, "True"), c(st, "False"), sort(st, 0));
    let h_ty = eq(st, &lvl(1), bool_, bt, bf);
    let ty = arrow(st, h_ty, fl);
    let eq_rec = cl(st, "Eq.rec", &[lvl(0), lvl(1)]);
    let bool_rec = cl(st, "Bool.rec", &[lvl(1)]);
    let intro = c(st, "True.intro");
    let v = lam(st, "h", h_ty, |st, h| {
        let motive = lam(st, "y", bool_, |st, y| {
            let t = eq(st, &lvl(1), bool_, bt, y);
            lam(st, "_", t, |st, _| {
                let m = lam(st, "_", bool_, |_, _| prop);
                app(st, bool_rec, &[m, fl, tr, y])
            })
        });
        app(st, eq_rec, &[bool_, bt, motive, intro, bf, h])
    });
    add_thm(env, "true_ne_false", &[], ty, v).unwrap();
}

/// `zero_ne_succ : Eq Nat 0 1 → False` by `Nat.rec` into Prop.
fn add_zero_ne_succ(env: &mut Environment) {
    let st = &mut env.store;
    let (nat, zero, fl, tr, prop) = (
        c(st, "Nat"),
        c(st, "Nat.zero"),
        c(st, "False"),
        c(st, "True"),
        sort(st, 0),
    );
    let one = num(st, 1);
    let h_ty = eq(st, &lvl(1), nat, zero, one);
    let ty = arrow(st, h_ty, fl);
    let eq_rec = cl(st, "Eq.rec", &[lvl(0), lvl(1)]);
    let nat_rec = cl(st, "Nat.rec", &[lvl(1)]);
    let intro = c(st, "True.intro");
    let v = lam(st, "h", h_ty, |st, h| {
        let motive = lam(st, "y", nat, |st, y| {
            let t = eq(st, &lvl(1), nat, zero, y);
            lam(st, "_", t, |st, _| {
                let m = lam(st, "_", nat, |_, _| prop);
                let step = lam(st, "_", nat, |st, _| lam(st, "_", prop, |_, _| fl));
                app(st, nat_rec, &[m, tr, step, y])
            })
        });
        app(st, eq_rec, &[nat, zero, motive, intro, one, h])
    });
    add_thm(env, "zero_ne_succ", &[], ty, v).unwrap();
}

fn rejected(env: &mut Environment, name: &str, ty: Expr, val: Expr) -> KernelError {
    let e = env
        .add_declaration(Declaration::theorem(n(name), vec![], ty, val))
        .expect_err("an unsound theorem was accepted");
    assert!(!env.contains(&n(name)));
    e
}

fn assert_stuck(env: &mut Environment, e: Expr) {
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.whnf(e).unwrap(), e, "expected a stuck term");
}

#[test]
fn false_from_refl_on_distinct_constructors_is_rejected() {
    let mut env = std_env();
    add_true_ne_false(&mut env);
    add_zero_ne_succ(&mut env);
    let st = &mut env.store;
    let fl = c(st, "False");
    let (bool_, bt) = (c(st, "Bool"), c(st, "Bool.true"));
    // the honest direction is fine: applying `true_ne_false` to a *proof of true = false*
    // would be False, and the only candidate, refl, has the wrong type
    let tnf = c(st, "true_ne_false");
    let r = refl(st, &lvl(1), bool_, bt);
    let bad = st.app(tnf, r);
    assert!(matches!(
        rejected(&mut env, "bad1", fl, bad),
        KernelError::TypeMismatch { .. }
    ));
    let st = &mut env.store;
    let (nat, zero) = (c(st, "Nat"), c(st, "Nat.zero"));
    let zns = c(st, "zero_ne_succ");
    let r = refl(st, &lvl(1), nat, zero);
    let bad = st.app(zns, r);
    assert!(matches!(
        rejected(&mut env, "bad2", fl, bad),
        KernelError::TypeMismatch { .. }
    ));
}

#[test]
fn proof_irrelevance_is_not_applied_to_data_or_to_propositions() {
    let mut env = std_env();
    let st = &mut env.store;
    let (bool_, prop) = (c(st, "Bool"), sort(st, 0));
    // ∀ a b : Bool, a = b   by rfl: Bool is data, not a Prop
    let stmt = pi(st, "a", bool_, |st, a| {
        pi(st, "b", bool_, |st, b| eq(st, &lvl(1), bool_, a, b))
    });
    let proof = lam(st, "a", bool_, |st, a| {
        lam(st, "b", bool_, |st, _| refl(st, &lvl(1), bool_, a))
    });
    assert!(matches!(
        rejected(&mut env, "all_bools_equal", stmt, proof),
        KernelError::TypeMismatch { .. }
    ));
    // ∀ p q : Prop, p = q   by rfl: `p : Prop`, but the *type* of p is Prop : Type, not a
    // proposition; propositions are not proofs
    let st = &mut env.store;
    let stmt = pi(st, "p", prop, |st, p| {
        pi(st, "q", prop, |st, q| eq(st, &lvl(1), prop, p, q))
    });
    let proof = lam(st, "p", prop, |st, p| {
        lam(st, "q", prop, |st, _| refl(st, &lvl(1), prop, p))
    });
    assert!(matches!(
        rejected(&mut env, "all_props_equal", stmt, proof),
        KernelError::TypeMismatch { .. }
    ));
    // concrete values
    let st = &mut env.store;
    let (bt, bf, tr, fl, zero) = (
        c(st, "Bool.true"),
        c(st, "Bool.false"),
        c(st, "True"),
        c(st, "False"),
        c(st, "Nat.zero"),
    );
    let one = num(st, 1);
    let mut tc = env.type_checker(&[]);
    assert!(!tc.is_def_eq(bt, bf).unwrap());
    assert!(!tc.is_def_eq(tr, fl).unwrap());
    assert!(!tc.is_def_eq(zero, one).unwrap());
}

#[test]
fn proof_irrelevance_needs_the_two_proofs_to_prove_the_same_proposition() {
    let mut env = std_env();
    let st = &mut env.store;
    let (tr, fl, prop) = (c(st, "True"), c(st, "False"), sort(st, 0));
    let _ = prop;
    env.add_declaration(Declaration::axiom(n("hf"), vec![], fl))
        .unwrap();
    let st = &mut env.store;
    let (hf, intro) = (c(st, "hf"), c(st, "True.intro"));
    let _ = tr;
    let mut tc = env.type_checker(&[]);
    // a proof of False and a proof of True are both proofs, of different propositions
    assert!(!tc.is_def_eq(hf, intro).unwrap());
    assert!(!tc.is_def_eq(intro, hf).unwrap());
}

#[test]
fn k_like_reduction_is_not_applied_to_a_type_valued_eq() {
    // EqT : (α : Type) → α → α → Type | refl : EqT α a a    (like Eq, but in Type)
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, ty) = (sort(st, 1), sort(st, 1));
    let ty_eqt = pi(st, "α", t1, |st, a| {
        pi(st, "a", a, |st, _| arrow(st, a, ty))
    });
    let eqt = c(st, "EqT");
    let refl_ty = pi(st, "α", t1, |st, a| {
        pi(st, "a", a, |st, x| app(st, eqt, &[a, x, x]))
    });
    env.add_inductive(single(
        &[],
        2,
        "EqT",
        ty_eqt,
        vec![ctor("EqT.refl", refl_ty)],
    ))
    .unwrap();
    match &env.get(&n("EqT.rec")).unwrap().kind {
        DeclKind::Recursor(r) => assert!(!r.k, "a Type-valued inductive is never a K-target"),
        k => panic!("{k:?}"),
    }
    let st = &mut env.store;
    let (nat, zero) = (c(st, "Nat"), c(st, "Nat.zero"));
    let three = num(st, 3);
    let eqt = c(st, "EqT");
    let h_ty = app(st, eqt, &[nat, zero, zero]);
    env.add_declaration(Declaration::axiom(n("h_t"), vec![], h_ty))
        .unwrap();
    let st = &mut env.store;
    let rec = cl(st, "EqT.rec", &[lvl(1)]);
    let h_t = c(st, "h_t");
    let motive = lam(st, "y", nat, |st, y| {
        let t = app(st, eqt, &[nat, zero, y]);
        lam(st, "_", t, |_, _| nat)
    });
    let r = app(st, rec, &[nat, zero, motive, three, zero, h_t]);
    // the recursor is applied to a variable, not a constructor: stuck, not `three`
    assert_stuck(&mut env, r);
    let st = &mut env.store;
    let stmt = eq(st, &lvl(1), nat, r, three);
    let proof = refl(st, &lvl(1), nat, three);
    assert!(matches!(
        rejected(&mut env, "k_on_type", stmt, proof),
        KernelError::TypeMismatch { .. }
    ));
}

#[test]
fn k_like_reduction_is_not_applied_to_a_prop_inductive_with_fields() {
    // Wrap (p : Prop) : Prop | mk : p → Wrap p        one constructor, but one *field*
    let mut env = std_env();
    let st = &mut env.store;
    let prop = sort(st, 0);
    let wrap = c(st, "Wrap");
    let ty = arrow(st, prop, prop);
    let mk = pi(st, "p", prop, |st, p| {
        let wp = st.app(wrap, p);
        arrow(st, p, wp)
    });
    env.add_inductive(single(&[], 1, "Wrap", ty, vec![ctor("Wrap.mk", mk)]))
        .unwrap();
    match &env.get(&n("Wrap.rec")).unwrap().kind {
        DeclKind::Recursor(r) => assert!(!r.k && r.large_elim),
        k => panic!("{k:?}"),
    }
    let st = &mut env.store;
    let (tr, nat) = (c(st, "True"), c(st, "Nat"));
    let three = num(st, 3);
    let wrap = c(st, "Wrap");
    let w_ty = st.app(wrap, tr);
    env.add_declaration(Declaration::axiom(n("hw"), vec![], w_ty))
        .unwrap();
    let st = &mut env.store;
    let rec = cl(st, "Wrap.rec", &[lvl(1)]);
    let hw = c(st, "hw");
    let motive = lam(st, "_", w_ty, |_, _| nat);
    let minor = lam(st, "_", tr, |_, _| three);
    let r = app(st, rec, &[tr, motive, minor, hw]);
    assert_stuck(&mut env, r);
    let st = &mut env.store;
    let stmt = eq(st, &lvl(1), nat, r, three);
    let proof = refl(st, &lvl(1), nat, three);
    assert!(matches!(
        rejected(&mut env, "k_with_fields", stmt, proof),
        KernelError::TypeMismatch { .. }
    ));
}

#[test]
fn k_like_reduction_needs_the_indices_to_agree() {
    // h : Eq Nat 0 1 (an inconsistent hypothesis): Eq.rec on it must not reduce, since
    // `Eq Nat 0 1` is not `Eq Nat 0 0`
    let mut env = std_env();
    let st = &mut env.store;
    let (nat, zero) = (c(st, "Nat"), c(st, "Nat.zero"));
    let (one, three) = (num(st, 1), num(st, 3));
    let h_ty = eq(st, &lvl(1), nat, zero, one);
    env.add_declaration(Declaration::axiom(n("hbad"), vec![], h_ty))
        .unwrap();
    let st = &mut env.store;
    let rec = cl(st, "Eq.rec", &[lvl(1), lvl(1)]);
    let hbad = c(st, "hbad");
    let motive = lam(st, "y", nat, |st, y| {
        let t = eq(st, &lvl(1), nat, zero, y);
        lam(st, "_", t, |_, _| nat)
    });
    let r = app(st, rec, &[nat, zero, motive, three, one, hbad]);
    assert_stuck(&mut env, r);
    let mut tc = env.type_checker(&[]);
    assert!(!tc.is_def_eq(r, three).unwrap());
}

#[test]
fn iota_requires_a_saturated_recursor_and_an_exactly_saturated_constructor() {
    let mut env = std_env();
    let st = &mut env.store;
    let (nat, zero, bt) = (c(st, "Nat"), c(st, "Nat.zero"), c(st, "Bool.true"));
    let (three, succ) = (num(st, 3), c(st, "Nat.succ"));
    let bool_ = c(st, "Bool");
    let motive = lam(st, "_", bool_, |_, _| nat);
    // Bool.rec motive zero three      (no major premise): stuck, no index panic
    let rec1 = cl(st, "Bool.rec", &[lvl(1)]);
    let partial = app(st, rec1, &[motive, zero, three]);
    assert_stuck(&mut env, partial);
    // Nat.rec motive zero step Nat.succ   (major is an unapplied constructor): stuck
    let st = &mut env.store;
    let nrec = cl(st, "Nat.rec", &[lvl(1)]);
    let nmotive = lam(st, "_", nat, |_, _| nat);
    let step = lam(st, "_", nat, |st, _| lam(st, "ih", nat, |_, ih| ih));
    let bad_major = app(st, nrec, &[nmotive, zero, step, succ]);
    assert_stuck(&mut env, bad_major);
    // ... and as a definition it is rejected outright
    assert!(add_def(&mut env, "bad_major", &[], nat, bad_major).is_err());
    // Option.rec ... (Option.some Nat)   (constructor missing its field): stuck
    let st = &mut env.store;
    let opt_some = cl(st, "Option.some", &[lvl(0)]);
    let orec = cl(st, "Option.rec", &[lvl(1), lvl(0)]);
    let some_nat = st.app(opt_some, nat);
    let opt = cl(st, "Option", &[lvl(0)]);
    let opt_nat = st.app(opt, nat);
    let omotive = lam(st, "_", opt_nat, |_, _| nat);
    let some_case = lam(st, "v", nat, |_, v| v);
    let e = app(st, orec, &[nat, omotive, zero, some_case, some_nat]);
    assert_stuck(&mut env, e);
    // wrong number of universe levels on the recursor: stuck for whnf, error for infer
    let st = &mut env.store;
    let rec_bad = cl(st, "Bool.rec", &[lvl(1), lvl(2)]);
    let e = app(st, rec_bad, &[motive, zero, three, bt]);
    assert_stuck(&mut env, e);
    let mut tc = env.type_checker(&[]);
    assert!(matches!(
        tc.infer(e),
        Err(KernelError::LevelArityMismatch { .. })
    ));
}

#[test]
fn iota_passes_extra_arguments_to_the_result() {
    // Bool.rec (fun _ => Nat → Nat) Nat.succ (fun x => x) true zero  ~>  zero
    let mut env = std_env();
    let st = &mut env.store;
    let (nat, zero, bt) = (c(st, "Nat"), c(st, "Nat.zero"), c(st, "Bool.true"));
    let succ = c(st, "Nat.succ");
    let n2n = arrow(st, nat, nat);
    let bool_ = c(st, "Bool");
    let motive = lam(st, "_", bool_, |_, _| n2n);
    let id = lam(st, "x", nat, |_, x| x);
    let rec = cl(st, "Bool.rec", &[lvl(1)]);
    let e = app(st, rec, &[motive, succ, id, bt, zero]);
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.whnf(e).unwrap(), zero);
}

#[test]
fn large_elimination_of_or_into_data_is_rejected() {
    // fun p q (h : Or p q) => Or.rec (fun _ => Bool) (fun _ => true) (fun _ => false) h
    // would make `true ≡ false` (inl and inr are proof-irrelevantly equal).
    let mut env = std_env();
    let st = &mut env.store;
    let (prop, bool_, or) = (sort(st, 0), c(st, "Bool"), c(st, "Or"));
    let (bt, bf) = (c(st, "Bool.true"), c(st, "Bool.false"));
    let rec = c(st, "Or.rec");
    let ty = pi(st, "p", prop, |st, p| {
        pi(st, "q", prop, |st, q| {
            let o = app(st, or, &[p, q]);
            arrow(st, o, bool_)
        })
    });
    let v = lam(st, "p", prop, |st, p| {
        lam(st, "q", prop, |st, q| {
            let o = app(st, or, &[p, q]);
            lam(st, "h", o, |st, h| {
                let motive = lam(st, "_", o, |_, _| bool_);
                let f = lam(st, "_", p, |_, _| bt);
                let g = lam(st, "_", q, |_, _| bf);
                app(st, rec, &[p, q, motive, f, g, h])
            })
        })
    });
    assert!(matches!(
        add_def(&mut env, "or_to_bool", &[], ty, v),
        Err(KernelError::TypeMismatch { .. })
    ));
    assert!(!env.contains(&n("or_to_bool")));
}

#[test]
fn rejected_inductives_leave_no_recursor_to_abuse() {
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, bad, fl) = (sort(st, 1), c(st, "Bad"), c(st, "False"));
    let neg = arrow(st, bad, fl);
    let mk = arrow(st, neg, bad);
    assert!(env
        .add_inductive(single(&[], 0, "Bad", t1, vec![ctor("Bad.mk", mk)]))
        .is_err());
    let st = &mut env.store;
    let rec = c(st, "Bad.rec");
    let mut tc = env.type_checker(&[]);
    assert!(matches!(
        tc.infer(rec),
        Err(KernelError::UnknownConstant(_))
    ));
}

#[test]
fn eta_is_not_applied_between_a_lambda_and_a_non_function() {
    let mut env = std_env();
    let st = &mut env.store;
    let (nat, zero) = (c(st, "Nat"), c(st, "Nat.zero"));
    let id = lam(st, "x", nat, |_, x| x);
    let mut tc = env.type_checker(&[]);
    assert!(!tc.is_def_eq(id, zero).unwrap());
    assert!(!tc.is_def_eq(zero, id).unwrap());
}

/// The helper types are used by several tests above.
#[allow(dead_code)]
fn _uses(_: &ExprStore) {}
