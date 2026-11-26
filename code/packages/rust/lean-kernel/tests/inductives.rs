//! Positive tests for inductive types, derived recursors and iota reduction: the first
//! real proofs the kernel can check (the logic-gate truth tables).

mod common;
use common::*;
use lean_kernel::{DeclKind, Environment, Expr, ExprStore, KernelError, Level};

fn rec_info(env: &Environment, name: &str) -> (usize, lean_kernel::RecursorInfo) {
    let d = env.get(&n(name)).expect("recursor exists");
    match &d.kind {
        DeclKind::Recursor(r) => (d.level_params.len(), r.clone()),
        k => panic!("{name} is not a recursor: {k:?}"),
    }
}

#[test]
fn standard_types_are_accepted_with_the_right_recursor_shapes() {
    let env = std_env();
    // (universe params of the recursor, k, large_elim, rules)
    let expect = [
        ("Bool.rec", 1, false, true, 2),
        ("Nat.rec", 1, false, true, 2),
        ("False.rec", 1, false, true, 0),
        ("True.rec", 1, true, true, 1),
        ("Eq.rec", 2, true, true, 1),
        ("And.rec", 1, false, true, 1),
        ("Or.rec", 0, false, false, 2), // Prop only: no motive universe
        ("Option.rec", 2, false, true, 2),
        ("Exists.rec", 1, false, false, 1), // only the inductive's own `u`
    ];
    for (name, lps, k, large, rules) in expect {
        let (got_lps, r) = rec_info(&env, name);
        assert_eq!(got_lps, lps, "{name} universe params");
        assert_eq!(r.k, k, "{name} k");
        assert_eq!(r.large_elim, large, "{name} large_elim");
        assert_eq!(r.rules.len(), rules, "{name} rules");
        assert_eq!(r.num_minors, rules);
    }
    // the type and constructors are recorded too
    assert!(matches!(
        env.get(&n("Nat")).unwrap().kind,
        DeclKind::Inductive(_)
    ));
    match &env.get(&n("Nat.succ")).unwrap().kind {
        DeclKind::Constructor(c) => {
            assert_eq!((c.index, c.num_params, c.num_fields), (1, 0, 1));
            assert_eq!(c.induct, n("Nat"));
        }
        k => panic!("{k:?}"),
    }
    let (_, eq_rec) = rec_info(&env, "Eq.rec");
    assert_eq!((eq_rec.num_params, eq_rec.num_indices), (2, 1));
}

#[test]
fn motive_universe_name_avoids_a_clash() {
    // An inductive that already has a universe parameter called `u`: the motive's
    // universe is `u_1`, so the two are different parameters.
    let mut env = Environment::new();
    add_option(&mut env);
    let d = env.get(&n("Option.rec")).unwrap();
    assert_eq!(d.level_params, vec![n("u_1"), n("u")]);
}

// -------------------------------------------------------------------------------------
// Boolean gates
// -------------------------------------------------------------------------------------

fn bool_rec(st: &mut ExprStore, u: u32, motive: Expr, f: Expr, t: Expr, major: Expr) -> Expr {
    let r = cl(st, "Bool.rec", &[lvl(u)]);
    app(st, r, &[motive, f, t, major])
}

/// `Bool.not`, `Bool.and`, `Bool.or`, all by `Bool.rec`.
fn add_gates(env: &mut Environment) {
    let st = &mut env.store;
    let bool_ = c(st, "Bool");
    let (bf, bt) = (c(st, "Bool.false"), c(st, "Bool.true"));
    let motive = lam(st, "_", bool_, |_, _| bool_);
    let b1 = arrow(st, bool_, bool_);
    let b2 = arrow(st, bool_, b1);
    let not_v = lam(st, "a", bool_, |st, a| bool_rec(st, 1, motive, bt, bf, a));
    let and_v = lam(st, "a", bool_, |st, a| {
        lam(st, "b", bool_, |st, b| bool_rec(st, 1, motive, bf, b, a))
    });
    let or_v = lam(st, "a", bool_, |st, a| {
        lam(st, "b", bool_, |st, b| bool_rec(st, 1, motive, b, bt, a))
    });
    add_def(env, "Bool.not", &[], b1, not_v).unwrap();
    add_def(env, "Bool.and", &[], b2, and_v).unwrap();
    add_def(env, "Bool.or", &[], b2, or_v).unwrap();
}

/// `∀ a b : Bool, P a b` and a proof by four-way case split, with `leaf a b` proving
/// each closed case `P a b` (a, b among `false`, `true`).
fn forall_bool2(
    st: &mut ExprStore,
    p: &dyn Fn(&mut ExprStore, Expr, Expr) -> Expr,
    leaf: &dyn Fn(&mut ExprStore, Expr, Expr) -> Expr,
) -> (Expr, Expr) {
    let bool_ = c(st, "Bool");
    let (bf, bt) = (c(st, "Bool.false"), c(st, "Bool.true"));
    let rec0 = cl(st, "Bool.rec", &[lvl(0)]);
    let stmt = pi(st, "a", bool_, |st, a| {
        pi(st, "b", bool_, |st, b| p(st, a, b))
    });
    let proof = lam(st, "a", bool_, |st, a| {
        let motive_a = lam(st, "a", bool_, |st, a2| {
            pi(st, "b", bool_, |st, b| p(st, a2, b))
        });
        let mut cases = vec![];
        for av in [bf, bt] {
            cases.push(lam(st, "b", bool_, |st, b| {
                let motive_b = lam(st, "b", bool_, |st, b2| p(st, av, b2));
                let (l0, l1) = (leaf(st, av, bf), leaf(st, av, bt));
                app(st, rec0, &[motive_b, l0, l1, b])
            }));
        }
        app(st, rec0, &[motive_a, cases[0], cases[1], a])
    });
    (stmt, proof)
}

#[test]
fn gates_compute_by_iota() {
    let mut env = std_env();
    add_gates(&mut env);
    let st = &mut env.store;
    let (bf, bt) = (c(st, "Bool.false"), c(st, "Bool.true"));
    let (and, or, not) = (c(st, "Bool.and"), c(st, "Bool.or"), c(st, "Bool.not"));
    let mut tc = env.type_checker(&[]);
    for (f, a, b, want) in [
        (and, bt, bt, bt),
        (and, bt, bf, bf),
        (and, bf, bt, bf),
        (or, bf, bf, bf),
        (or, bf, bt, bt),
        (or, bt, bf, bt),
    ] {
        let e = tc.store().apps(f, &[a, b]);
        assert_eq!(tc.whnf(e).unwrap(), want);
    }
    let e = tc.store().app(not, bt);
    assert_eq!(tc.whnf(e).unwrap(), bf);
}

#[test]
fn and_is_commutative_on_all_inputs() {
    let mut env = std_env();
    add_gates(&mut env);
    let st = &mut env.store;
    let bool_ = c(st, "Bool");
    let and = c(st, "Bool.and");
    let (stmt, proof) = forall_bool2(
        st,
        &|st, a, b| {
            let (x, y) = (app(st, and, &[a, b]), app(st, and, &[b, a]));
            let bool_ = c(st, "Bool");
            eq(st, &lvl(1), bool_, x, y)
        },
        &|st, a, b| {
            let bool_ = c(st, "Bool");
            let v = app(st, and, &[a, b]);
            refl(st, &lvl(1), bool_, v)
        },
    );
    let _ = bool_;
    add_thm(&mut env, "and_comm", &[], stmt, proof).unwrap();
}

#[test]
fn false_is_not_provable_for_wrong_truth_table() {
    // ∀ a b, and a b = or a b has a counterexample, so the same proof shape fails.
    let mut env = std_env();
    add_gates(&mut env);
    let st = &mut env.store;
    let (and, or) = (c(st, "Bool.and"), c(st, "Bool.or"));
    let (stmt, proof) = forall_bool2(
        st,
        &|st, a, b| {
            let (x, y) = (app(st, and, &[a, b]), app(st, or, &[a, b]));
            let bool_ = c(st, "Bool");
            eq(st, &lvl(1), bool_, x, y)
        },
        &|st, a, b| {
            let bool_ = c(st, "Bool");
            let v = app(st, and, &[a, b]);
            refl(st, &lvl(1), bool_, v)
        },
    );
    assert!(matches!(
        add_thm(&mut env, "bad", &[], stmt, proof),
        Err(KernelError::TypeMismatch { .. })
    ));
}

#[test]
fn nand_built_xor_equals_xor_on_all_inputs() {
    let mut env = std_env();
    add_gates(&mut env);
    let st = &mut env.store;
    let bool_ = c(st, "Bool");
    let (and, not) = (c(st, "Bool.and"), c(st, "Bool.not"));
    let b1 = arrow(st, bool_, bool_);
    let b2 = arrow(st, bool_, b1);
    // nand a b := not (and a b)
    let nand_v = lam(st, "a", bool_, |st, a| {
        lam(st, "b", bool_, |st, b| {
            let ab = app(st, and, &[a, b]);
            app(st, not, &[ab])
        })
    });
    add_def(&mut env, "nand", &[], b2, nand_v).unwrap();
    let st = &mut env.store;
    let nand = c(st, "nand");
    // xor from four NANDs: t = nand a b; nand (nand a t) (nand b t)
    let xor_nand_v = lam(st, "a", bool_, |st, a| {
        lam(st, "b", bool_, |st, b| {
            let t = app(st, nand, &[a, b]);
            let l = app(st, nand, &[a, t]);
            let r = app(st, nand, &[b, t]);
            app(st, nand, &[l, r])
        })
    });
    // reference xor, directly by recursion: false => b ; true => not b
    let motive = lam(st, "_", bool_, |_, _| bool_);
    let xor_ref_v = lam(st, "a", bool_, |st, a| {
        lam(st, "b", bool_, |st, b| {
            let nb = app(st, not, &[b]);
            bool_rec(st, 1, motive, b, nb, a)
        })
    });
    add_def(&mut env, "xor_nand", &[], b2, xor_nand_v).unwrap();
    add_def(&mut env, "xor_ref", &[], b2, xor_ref_v).unwrap();
    let st = &mut env.store;
    let (xn, xr) = (c(st, "xor_nand"), c(st, "xor_ref"));
    let (stmt, proof) = forall_bool2(
        st,
        &|st, a, b| {
            let (x, y) = (app(st, xn, &[a, b]), app(st, xr, &[a, b]));
            let bool_ = c(st, "Bool");
            eq(st, &lvl(1), bool_, x, y)
        },
        &|st, a, b| {
            let bool_ = c(st, "Bool");
            let v = app(st, xr, &[a, b]);
            refl(st, &lvl(1), bool_, v)
        },
    );
    add_thm(&mut env, "xor_nand_correct", &[], stmt, proof).unwrap();
}

// -------------------------------------------------------------------------------------
// Nat
// -------------------------------------------------------------------------------------

fn add_nat_add(env: &mut Environment) {
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let succ = c(st, "Nat.succ");
    let rec = cl(st, "Nat.rec", &[lvl(1)]);
    let motive = lam(st, "_", nat, |_, _| nat);
    let step = lam(st, "k", nat, |st, _| {
        lam(st, "ih", nat, |st, ih| st.app(succ, ih))
    });
    let v = lam(st, "n", nat, |st, nn| {
        lam(st, "m", nat, |st, m| app(st, rec, &[motive, nn, step, m]))
    });
    let a1 = arrow(st, nat, nat);
    let ty = arrow(st, nat, a1);
    add_def(env, "Nat.add", &[], ty, v).unwrap();
}

#[test]
fn nat_add_two_two_is_four_by_iota() {
    let mut env = std_env();
    add_nat_add(&mut env);
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let add = c(st, "Nat.add");
    let (two, four) = (num(st, 2), num(st, 4));
    let lhs = app(st, add, &[two, two]);
    let stmt = eq(st, &lvl(1), nat, lhs, four);
    let proof = refl(st, &lvl(1), nat, four);
    add_thm(&mut env, "two_add_two", &[], stmt, proof).unwrap();
    // ... and 2 + 2 = 5 is rejected
    let st = &mut env.store;
    let five = num(st, 5);
    let stmt = eq(st, &lvl(1), nat, lhs, five);
    let proof = refl(st, &lvl(1), nat, five);
    assert!(matches!(
        add_thm(&mut env, "two_add_two_five", &[], stmt, proof),
        Err(KernelError::TypeMismatch { .. })
    ));
}

#[test]
fn nat_add_zero_right_is_definitional_but_zero_left_is_not() {
    let mut env = std_env();
    add_nat_add(&mut env);
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let add = c(st, "Nat.add");
    let zero = c(st, "Nat.zero");
    // ∀ n, n + 0 = n   by rfl: iota on the second argument
    let stmt = pi(st, "n", nat, |st, nn| {
        let l = app(st, add, &[nn, zero]);
        eq(st, &lvl(1), nat, l, nn)
    });
    let proof = lam(st, "n", nat, |st, nn| refl(st, &lvl(1), nat, nn));
    add_thm(&mut env, "add_zero", &[], stmt, proof).unwrap();
    // ∀ n, 0 + n = n   is stuck on the variable, so rfl fails (needs induction)
    let st = &mut env.store;
    let stmt = pi(st, "n", nat, |st, nn| {
        let l = app(st, add, &[zero, nn]);
        eq(st, &lvl(1), nat, l, nn)
    });
    let proof = lam(st, "n", nat, |st, nn| refl(st, &lvl(1), nat, nn));
    assert!(add_thm(&mut env, "zero_add", &[], stmt, proof).is_err());
}

// -------------------------------------------------------------------------------------
// Eq: symm, trans, K
// -------------------------------------------------------------------------------------

fn add_eq_lemmas(env: &mut Environment) {
    let st = &mut env.store;
    let u = lp("u");
    let su = sort_l(st, &u);
    let rec = cl(st, "Eq.rec", &[lvl(0), u.clone()]);
    // Eq.symm
    let stmt = pi(st, "α", su, |st, al| {
        pi(st, "a", al, |st, a| {
            pi(st, "b", al, |st, b| {
                let h = eq(st, &u, al, a, b);
                let r = eq(st, &u, al, b, a);
                arrow(st, h, r)
            })
        })
    });
    let proof = lam(st, "α", su, |st, al| {
        lam(st, "a", al, |st, a| {
            lam(st, "b", al, |st, b| {
                let h_ty = eq(st, &u, al, a, b);
                lam(st, "h", h_ty, |st, h| {
                    let motive = lam(st, "y", al, |st, y| {
                        let t = eq(st, &u, al, a, y);
                        lam(st, "_", t, |st, _| eq(st, &u, al, y, a))
                    });
                    let r = refl(st, &u, al, a);
                    app(st, rec, &[al, a, motive, r, b, h])
                })
            })
        })
    });
    add_thm(env, "Eq.symm", &["u"], stmt, proof).unwrap();
    // Eq.trans
    let st = &mut env.store;
    let stmt = pi(st, "α", su, |st, al| {
        pi(st, "a", al, |st, a| {
            pi(st, "b", al, |st, b| {
                pi(st, "c", al, |st, cc| {
                    let h1 = eq(st, &u, al, a, b);
                    let h2 = eq(st, &u, al, b, cc);
                    let r = eq(st, &u, al, a, cc);
                    let inner = arrow(st, h2, r);
                    arrow(st, h1, inner)
                })
            })
        })
    });
    let proof = lam(st, "α", su, |st, al| {
        lam(st, "a", al, |st, a| {
            lam(st, "b", al, |st, b| {
                lam(st, "c", al, |st, cc| {
                    let h1_ty = eq(st, &u, al, a, b);
                    lam(st, "h1", h1_ty, |st, h1| {
                        let h2_ty = eq(st, &u, al, b, cc);
                        lam(st, "h2", h2_ty, |st, h2| {
                            let motive = lam(st, "y", al, |st, y| {
                                let t = eq(st, &u, al, b, y);
                                lam(st, "_", t, |st, _| eq(st, &u, al, a, y))
                            });
                            app(st, rec, &[al, b, motive, h1, cc, h2])
                        })
                    })
                })
            })
        })
    });
    add_thm(env, "Eq.trans", &["u"], stmt, proof).unwrap();
}

#[test]
fn eq_symm_and_trans_via_eq_rec() {
    let mut env = std_env();
    add_eq_lemmas(&mut env);
    // use Eq.symm on a concrete equation: 1 = 1 -> 1 = 1 (just type-checks and applies)
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let one = num(st, 1);
    let symm = cl(st, "Eq.symm", &[lvl(1)]);
    let r = refl(st, &lvl(1), nat, one);
    let used = app(st, symm, &[nat, one, one, r]);
    let want = eq(st, &lvl(1), nat, one, one);
    let mut tc = env.type_checker(&[]);
    let ty = tc.infer(used).unwrap();
    assert!(tc.is_def_eq(ty, want).unwrap());
}

#[test]
fn k_like_reduction_fires_on_a_variable_proof_of_a_refl_type() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let zero = c(st, "Nat.zero");
    let three = num(st, 3);
    let rec = cl(st, "Eq.rec", &[lvl(1), lvl(1)]);
    // ∀ h : Eq Nat 0 0, (Eq.rec (fun _ _ => Nat) 3 h) = 3   by rfl: needs K
    let h_ty = eq(st, &lvl(1), nat, zero, zero);
    let stmt = pi(st, "h", h_ty, |st, h| {
        let motive = lam(st, "y", nat, |st, y| {
            let t = eq(st, &lvl(1), nat, zero, y);
            lam(st, "_", t, |_, _| nat)
        });
        let r = app(st, rec, &[nat, zero, motive, three, zero, h]);
        eq(st, &lvl(1), nat, r, three)
    });
    let proof = lam(st, "h", h_ty, |st, _| refl(st, &lvl(1), nat, three));
    add_thm(&mut env, "k_reduces", &[], stmt, proof).unwrap();
}

// -------------------------------------------------------------------------------------
// Large elimination
// -------------------------------------------------------------------------------------

#[test]
fn ex_falso_into_type_is_allowed_for_false() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let fls = c(st, "False");
    let rec = cl(st, "False.rec", &[lvl(1)]);
    let motive = lam(st, "_", fls, |_, _| nat);
    let ty = arrow(st, fls, nat);
    let v = lam(st, "h", fls, |st, h| app(st, rec, &[motive, h]));
    add_def(&mut env, "absurd_nat", &[], ty, v).unwrap();
}

#[test]
fn and_eliminates_into_type_and_or_only_into_prop() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let prop = sort(st, 0);
    let zero = c(st, "Nat.zero");
    let and = c(st, "And");
    // And.rec into Type: fun a b (h : And a b) => (0 : Nat)
    let rec = cl(st, "And.rec", &[lvl(1)]);
    let ty = pi(st, "a", prop, |st, a| {
        pi(st, "b", prop, |st, b| {
            let ab = app(st, and, &[a, b]);
            arrow(st, ab, nat)
        })
    });
    let v = lam(st, "a", prop, |st, a| {
        lam(st, "b", prop, |st, b| {
            let ab = app(st, and, &[a, b]);
            lam(st, "h", ab, |st, h| {
                let motive = lam(st, "_", ab, |_, _| nat);
                let minor = lam(st, "x", a, |st, _| lam(st, "y", b, |_, _| zero));
                app(st, rec, &[a, b, motive, minor, h])
            })
        })
    });
    add_def(&mut env, "and_to_nat", &[], ty, v).unwrap();
    // Or.rec into Prop is fine: Or.elim
    let st = &mut env.store;
    let or = c(st, "Or");
    let rec = c(st, "Or.rec");
    let ty = pi(st, "a", prop, |st, a| {
        pi(st, "b", prop, |st, b| {
            pi(st, "r", prop, |st, r| {
                let ab = app(st, or, &[a, b]);
                let f = arrow(st, a, r);
                let g = arrow(st, b, r);
                let tail = arrow(st, g, r);
                let mid = arrow(st, f, tail);
                arrow(st, ab, mid)
            })
        })
    });
    let v = lam(st, "a", prop, |st, a| {
        lam(st, "b", prop, |st, b| {
            lam(st, "r", prop, |st, r| {
                let ab = app(st, or, &[a, b]);
                lam(st, "h", ab, |st, h| {
                    let f_ty = arrow(st, a, r);
                    lam(st, "f", f_ty, |st, f| {
                        let g_ty = arrow(st, b, r);
                        lam(st, "g", g_ty, |st, g| {
                            let motive = lam(st, "_", ab, |_, _| r);
                            app(st, rec, &[a, b, motive, f, g, h])
                        })
                    })
                })
            })
        })
    });
    add_def(&mut env, "or_elim", &[], ty, v).unwrap();
    // Or.rec into Type: `Or.rec.{1}` is a universe-arity error (Or.rec has none)
    let st = &mut env.store;
    let rec1 = cl(st, "Or.rec", &[lvl(1)]);
    let ty = sort(st, 1);
    assert!(matches!(
        add_def(&mut env, "bad_or", &[], ty, rec1),
        Err(KernelError::LevelArityMismatch {
            expected: 0,
            found: 1,
            ..
        })
    ));
}

#[test]
fn option_computes() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let zero = c(st, "Nat.zero");
    let two = num(st, 2);
    let opt = cl(st, "Option", &[lvl(0)]);
    let (none, some) = (
        cl(st, "Option.none", &[lvl(0)]),
        cl(st, "Option.some", &[lvl(0)]),
    );
    let rec = cl(st, "Option.rec", &[lvl(1), lvl(0)]);
    let opt_nat = st.app(opt, nat);
    let get_d_ty = arrow(st, opt_nat, nat);
    let motive = lam(st, "_", opt_nat, |_, _| nat);
    let some_case = lam(st, "v", nat, |_, v| v);
    let get_d = lam(st, "o", opt_nat, |st, o| {
        app(st, rec, &[nat, motive, zero, some_case, o])
    });
    add_def(&mut env, "getD", &[], get_d_ty, get_d).unwrap();
    let st = &mut env.store;
    let get_d = c(st, "getD");
    let s2 = app(st, some, &[nat, two]);
    let n0 = app(st, none, &[nat]);
    let (a, b) = (st.app(get_d, s2), st.app(get_d, n0));
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.whnf(a).unwrap(), tc.whnf(two).unwrap());
    assert_eq!(tc.whnf(b).unwrap(), zero);
}

#[test]
fn exists_and_prop_only_flag() {
    let env = std_env();
    // Exists has a field in Type that is not an index: Prop-only elimination.
    let (_, r) = rec_info(&env, "Exists.rec");
    assert!(!r.large_elim);
    let _ = Level::zero();
}
