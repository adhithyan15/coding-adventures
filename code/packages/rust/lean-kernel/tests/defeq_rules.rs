//! Proof irrelevance and eta for functions: what they accept, and what they must not.

mod common;
use common::*;
use lean_kernel::Declaration;

#[test]
fn any_two_proofs_of_a_proposition_are_equal() {
    let mut env = std_env();
    let st = &mut env.store;
    let prop = sort(st, 0);
    // ∀ (p : Prop) (h1 h2 : p), h1 = h2   by rfl
    let stmt = pi(st, "p", prop, |st, p| {
        pi(st, "h1", p, |st, h1| {
            pi(st, "h2", p, |st, h2| eq(st, &lvl(0), p, h1, h2))
        })
    });
    let proof = lam(st, "p", prop, |st, p| {
        lam(st, "h1", p, |st, h1| {
            lam(st, "h2", p, |st, _| refl(st, &lvl(0), p, h1))
        })
    });
    add_thm(&mut env, "proof_irrel", &[], stmt, proof).unwrap();
}

#[test]
fn proof_irrelevance_makes_types_that_depend_on_proofs_agree() {
    // ∀ (p : Prop) (P : p → Prop) (h1 h2 : p), P h1 → P h2: needs `P h1 ≡ P h2`
    let mut env = std_env();
    let st = &mut env.store;
    let prop = sort(st, 0);
    let stmt = pi(st, "p", prop, |st, p| {
        let pp = arrow(st, p, prop);
        pi(st, "P", pp, |st, pf| {
            pi(st, "h1", p, |st, h1| {
                pi(st, "h2", p, |st, h2| {
                    let (a, b) = (st.app(pf, h1), st.app(pf, h2));
                    arrow(st, a, b)
                })
            })
        })
    });
    let proof = lam(st, "p", prop, |st, p| {
        let pp = arrow(st, p, prop);
        lam(st, "P", pp, |st, pf| {
            lam(st, "h1", p, |st, h1| {
                lam(st, "h2", p, |st, _| {
                    let a = st.app(pf, h1);
                    lam(st, "x", a, |_, x| x)
                })
            })
        })
    });
    add_thm(&mut env, "dep_irrel", &[], stmt, proof).unwrap();
}

#[test]
fn eta_for_functions_both_directions_and_two_arguments() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let n2n = arrow(st, nat, nat);
    // ∀ f : Nat → Nat, (fun x => f x) = f   and   f = (fun x => f x)
    let stmt1 = pi(st, "f", n2n, |st, f| {
        let e = lam(st, "x", nat, |st, x| st.app(f, x));
        eq(st, &lvl(1), n2n, e, f)
    });
    let proof1 = lam(st, "f", n2n, |st, f| refl(st, &lvl(1), n2n, f));
    add_thm(&mut env, "eta1", &[], stmt1, proof1).unwrap();
    let st = &mut env.store;
    let stmt2 = pi(st, "f", n2n, |st, f| {
        let e = lam(st, "x", nat, |st, x| st.app(f, x));
        eq(st, &lvl(1), n2n, f, e)
    });
    let proof2 = lam(st, "f", n2n, |st, f| refl(st, &lvl(1), n2n, f));
    add_thm(&mut env, "eta2", &[], stmt2, proof2).unwrap();
    // two arguments: fun x y => f x y = f
    let st = &mut env.store;
    let n3 = arrow(st, nat, n2n);
    let stmt3 = pi(st, "f", n3, |st, f| {
        let e = lam(st, "x", nat, |st, x| {
            lam(st, "y", nat, |st, y| app(st, f, &[x, y]))
        });
        eq(st, &lvl(1), n3, e, f)
    });
    let proof3 = lam(st, "f", n3, |st, f| refl(st, &lvl(1), n3, f));
    add_thm(&mut env, "eta3", &[], stmt3, proof3).unwrap();
}

#[test]
fn eta_does_not_equate_different_functions() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let n2n = arrow(st, nat, nat);
    env.add_declaration(Declaration::axiom(n("g"), vec![], n2n))
        .unwrap();
    env.add_declaration(Declaration::axiom(n("f"), vec![], n2n))
        .unwrap();
    let st = &mut env.store;
    let (f, g, zero) = (c(st, "f"), c(st, "g"), c(st, "Nat.zero"));
    let f_eta = lam(st, "x", nat, |st, x| st.app(f, x));
    let const_zero = lam(st, "x", nat, |_, _| zero);
    let mut tc = env.type_checker(&[]);
    assert!(!tc.is_def_eq(f_eta, g).unwrap()); // eta f vs a different function
    assert!(!tc.is_def_eq(g, f_eta).unwrap());
    assert!(!tc.is_def_eq(const_zero, f).unwrap()); // not an eta-expansion of f
    assert!(tc.is_def_eq(f_eta, f).unwrap());
    // a lambda is never equal to a non-function
    let id = {
        let st = tc.store();
        lam(st, "x", nat, |_, x| x)
    };
    assert!(!tc.is_def_eq(id, zero).unwrap());
}
