//! Negative tests for inductive declarations: one per check, plus atomicity and the
//! "recursors cannot be forged" guard.

mod common;
use common::*;
use lean_kernel::{
    ConstructorInfo, DeclKind, Declaration, Environment, InductiveDecl, InductiveTypeDecl,
    KernelError, Level, RecursorInfo,
};

fn expect_err(env: &mut Environment, d: InductiveDecl) -> KernelError {
    let before = env.len();
    let e = env.add_inductive(d).expect_err("must be rejected");
    assert_eq!(env.len(), before, "a rejected declaration must add nothing");
    e
}

// ---- strict positivity ----------------------------------------------------------------

#[test]
fn a_negative_occurrence_is_rejected() {
    // inductive Bad : Type | mk : (Bad → False) → Bad
    let mut env = std_env();
    let st = &mut env.store;
    let ty = sort(st, 1);
    let (bad, fls) = (c(st, "Bad"), c(st, "False"));
    let neg = arrow(st, bad, fls);
    let mk = arrow(st, neg, bad);
    let e = expect_err(
        &mut env,
        single(&[], 0, "Bad", ty, vec![ctor("Bad.mk", mk)]),
    );
    assert_eq!(e, KernelError::NonPositiveOccurrence(n("Bad.mk")));
}

#[test]
fn a_doubly_negative_occurrence_is_also_rejected() {
    // mk : ((Bad → Nat) → Nat) → Bad: Bad is to the left of an arrow twice
    let mut env = std_env();
    let st = &mut env.store;
    let ty = sort(st, 1);
    let (bad, nat) = (c(st, "Bad"), c(st, "Nat"));
    let inner = arrow(st, bad, nat);
    let outer = arrow(st, inner, nat);
    let mk = arrow(st, outer, bad);
    let e = expect_err(
        &mut env,
        single(&[], 0, "Bad", ty, vec![ctor("Bad.mk", mk)]),
    );
    assert!(matches!(e, KernelError::NonPositiveOccurrence(_)));
}

#[test]
fn a_negative_occurrence_hidden_behind_a_definition_is_rejected() {
    // def Neg : Type → Type := fun X => X → False ;  mk : Neg Bad → Bad
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, fls) = (sort(st, 1), c(st, "False"));
    let neg_v = lam(st, "X", t1, |st, x| arrow(st, x, fls));
    // Neg X := X → False lives in Prop: its honest type is `Type → Prop`.
    let prop = sort(st, 0);
    let neg_ty = arrow(st, t1, prop);
    add_def(&mut env, "Neg", &[], neg_ty, neg_v).unwrap();
    let st = &mut env.store;
    let (bad, neg) = (c(st, "Bad"), c(st, "Neg"));
    let nb = st.app(neg, bad);
    let mk = arrow(st, nb, bad);
    let ty = sort(st, 1);
    let e = expect_err(
        &mut env,
        single(&[], 0, "Bad", ty, vec![ctor("Bad.mk", mk)]),
    );
    assert_eq!(e, KernelError::NonPositiveOccurrence(n("Bad.mk")));
}

#[test]
fn positive_recursion_through_a_function_is_accepted() {
    // inductive W : Type | mk : (Nat → W) → W   (strictly positive)
    let mut env = std_env();
    let st = &mut env.store;
    let ty = sort(st, 1);
    let (w, nat) = (c(st, "W"), c(st, "Nat"));
    let f = arrow(st, nat, w);
    let mk = arrow(st, f, w);
    env.add_inductive(single(&[], 0, "W", ty, vec![ctor("W.mk", mk)]))
        .unwrap();
    // its recursor has an ih that is itself a function
    let d = env.get(&n("W.rec")).unwrap();
    assert!(matches!(&d.kind, DeclKind::Recursor(r) if r.rules.len() == 1));
}

#[test]
fn an_occurrence_as_an_argument_of_a_non_inductive_is_not_positive() {
    // mk : (F Bad) → Bad where F is an axiom, not an inductive: not nested, just wrong
    let mut env = std_env();
    let st = &mut env.store;
    let t1 = sort(st, 1);
    let f_ty = arrow(st, t1, t1);
    env.add_declaration(Declaration::axiom(n("F"), vec![], f_ty))
        .unwrap();
    let st = &mut env.store;
    let (f, bad) = (c(st, "F"), c(st, "Bad"));
    let fb = st.app(f, bad);
    let mk = arrow(st, fb, bad);
    let e = expect_err(
        &mut env,
        single(&[], 0, "Bad", t1, vec![ctor("Bad.mk", mk)]),
    );
    assert_eq!(e, KernelError::NonPositiveOccurrence(n("Bad.mk")));
}

// ---- universes ------------------------------------------------------------------------

#[test]
fn a_field_in_a_bigger_universe_is_rejected() {
    // inductive Big : Type | mk : Type → Big    (a retract of Type: Girard)
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, big) = (sort(st, 1), c(st, "Big"));
    let mk = arrow(st, t1, big);
    let e = expect_err(
        &mut env,
        single(&[], 0, "Big", t1, vec![ctor("Big.mk", mk)]),
    );
    assert_eq!(e, KernelError::UniverseTooLarge(n("Big.mk")));
}

#[test]
fn a_polymorphic_universe_violation_is_rejected() {
    // inductive P.{u} : Type u | mk : Type u → P     (Type u : Type (u+1))
    let mut env = std_env();
    let st = &mut env.store;
    let u = lp("u");
    let tu = sort_l(st, &Level::succ(&u));
    let p = cl(st, "P", std::slice::from_ref(&u));
    let mk = arrow(st, tu, p);
    let e = expect_err(&mut env, single(&["u"], 0, "P", tu, vec![ctor("P.mk", mk)]));
    assert_eq!(e, KernelError::UniverseTooLarge(n("P.mk")));
}

#[test]
fn a_prop_may_store_a_type_but_then_only_eliminates_into_prop() {
    // inductive Ex : Prop | mk : Type → Ex       (allowed: impredicative Prop)
    let mut env = std_env();
    let st = &mut env.store;
    let (prop, t1, ex) = (sort(st, 0), sort(st, 1), c(st, "Ex"));
    let mk = arrow(st, t1, ex);
    env.add_inductive(single(&[], 0, "Ex", prop, vec![ctor("Ex.mk", mk)]))
        .unwrap();
    match &env.get(&n("Ex.rec")).unwrap().kind {
        DeclKind::Recursor(r) => assert!(!r.large_elim),
        k => panic!("{k:?}"),
    }
}

#[test]
fn large_elimination_decisions() {
    let mut env = std_env();
    let st = &mut env.store;
    // EqNat : Nat → Prop | mk (n : Nat) : EqNat n   — the Type field is an index: large
    let (prop, nat) = (sort(st, 0), c(st, "Nat"));
    let ty = arrow(st, nat, prop);
    let eqn = c(st, "EqNat");
    let mk = pi(st, "n", nat, |st, x| st.app(eqn, x));
    env.add_inductive(single(&[], 0, "EqNat", ty, vec![ctor("EqNat.mk", mk)]))
        .unwrap();
    // S.{u} : Sort u | a | b   — might be Prop, two constructors: Prop only
    let st = &mut env.store;
    let u = lp("u");
    let su = sort_l(st, &u);
    let s = cl(st, "S", std::slice::from_ref(&u));
    env.add_inductive(single(
        &["u"],
        0,
        "S",
        su,
        vec![ctor("S.a", s), ctor("S.b", s)],
    ))
    .unwrap();
    // S2.{u} : Sort (u+1) | a | b   — never Prop: large
    let st = &mut env.store;
    let su1 = sort_l(st, &Level::succ(&u));
    let s2 = cl(st, "S2", std::slice::from_ref(&u));
    env.add_inductive(single(
        &["u"],
        0,
        "S2",
        su1,
        vec![ctor("S2.a", s2), ctor("S2.b", s2)],
    ))
    .unwrap();
    let large = |name: &str| match &env.get(&n(name)).unwrap().kind {
        DeclKind::Recursor(r) => r.large_elim,
        k => panic!("{k:?}"),
    };
    assert!(large("EqNat.rec"));
    assert!(!large("S.rec"));
    assert!(large("S2.rec"));
    // Exists.rec accepts only its own universe, never a motive universe
    let st = &mut env.store;
    let ex_rec = cl(st, "Exists.rec", &[lvl(1), lvl(1)]);
    let ty = sort(st, 1);
    assert!(matches!(
        add_def(&mut env, "bad_exists", &[], ty, ex_rec),
        Err(KernelError::LevelArityMismatch {
            expected: 1,
            found: 2,
            ..
        })
    ));
}

// ---- shape ----------------------------------------------------------------------------

#[test]
fn constructor_parameters_must_match_the_types() {
    // Foo (a : Nat) : Type | mk : (a : Bool) → Foo Nat.zero
    let mut env = std_env();
    let st = &mut env.store;
    let (nat, bool_, t1, foo, zero) = (
        c(st, "Nat"),
        c(st, "Bool"),
        sort(st, 1),
        c(st, "Foo"),
        c(st, "Nat.zero"),
    );
    let ty = arrow(st, nat, t1);
    let fz = st.app(foo, zero);
    let mk = arrow(st, bool_, fz);
    let e = expect_err(
        &mut env,
        single(&[], 1, "Foo", ty, vec![ctor("Foo.mk", mk)]),
    );
    assert!(matches!(e, KernelError::InvalidInductive(m) if m.contains("parameters")));
}

#[test]
fn a_constructor_must_use_the_parameter_variables_in_its_result() {
    // Foo (a : Nat) : Type | mk : (a : Nat) → Foo Nat.zero    (result param is not `a`)
    let mut env = std_env();
    let st = &mut env.store;
    let (nat, t1, foo, zero) = (c(st, "Nat"), sort(st, 1), c(st, "Foo"), c(st, "Nat.zero"));
    let ty = arrow(st, nat, t1);
    let fz = st.app(foo, zero);
    let mk = arrow(st, nat, fz);
    let e = expect_err(
        &mut env,
        single(&[], 1, "Foo", ty, vec![ctor("Foo.mk", mk)]),
    );
    assert!(matches!(e, KernelError::InvalidInductive(m) if m.contains("return")));
}

#[test]
fn a_constructor_must_return_the_inductive_type() {
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, nat) = (sort(st, 1), c(st, "Nat"));
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("A.mk", nat)]));
    assert!(matches!(e, KernelError::InvalidInductive(_)));
}

#[test]
fn a_constructor_must_use_the_declared_universe_arguments() {
    // P.{u} : Type | mk : P.{u+1}
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, u) = (sort(st, 1), lp("u"));
    let p_up = cl(st, "P", &[Level::succ(&u)]);
    let e = expect_err(
        &mut env,
        single(&["u"], 0, "P", t1, vec![ctor("P.mk", p_up)]),
    );
    assert!(matches!(e, KernelError::InvalidInductive(_)));
}

#[test]
fn indices_may_not_mention_the_inductive_type() {
    // T : Type → Type | mk : T (T Nat)
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, nat, t) = (sort(st, 1), c(st, "Nat"), c(st, "T"));
    let ty = arrow(st, t1, t1);
    let tn = st.app(t, nat);
    let mk = st.app(t, tn);
    let e = expect_err(&mut env, single(&[], 0, "T", ty, vec![ctor("T.mk", mk)]));
    assert!(matches!(e, KernelError::InvalidInductive(_)));
}

#[test]
fn the_type_must_end_in_a_sort() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let ty = arrow(st, nat, nat);
    let e = expect_err(&mut env, single(&[], 0, "T", ty, vec![]));
    assert!(matches!(e, KernelError::InvalidInductive(m) if m.contains("sort")));
    // not even a type
    let zero = c(&mut env.store, "Nat.zero");
    let e = expect_err(&mut env, single(&[], 0, "T", zero, vec![]));
    assert!(matches!(e, KernelError::NotASort(_)));
}

#[test]
fn the_type_may_not_mention_itself() {
    // Bad : Bad → Type
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, bad) = (sort(st, 1), c(st, "Bad"));
    let ty = arrow(st, bad, t1);
    let e = expect_err(&mut env, single(&[], 0, "Bad", ty, vec![]));
    assert!(matches!(e, KernelError::InvalidInductive(m) if m.contains("itself")));
}

#[test]
fn parameter_count_and_block_shape_are_checked() {
    let mut env = std_env();
    let t1 = sort(&mut env.store, 1);
    let e = expect_err(&mut env, single(&[], 3, "T", t1, vec![]));
    assert!(matches!(e, KernelError::InvalidInductive(m) if m.contains("fewer")));
    let e = expect_err(
        &mut env,
        InductiveDecl {
            level_params: vec![],
            num_params: 0,
            types: vec![],
        },
    );
    assert!(matches!(e, KernelError::InvalidInductive(_)));
    let anon = lean_kernel::Name::anonymous();
    let e = expect_err(
        &mut env,
        InductiveDecl {
            level_params: vec![],
            num_params: 0,
            types: vec![InductiveTypeDecl {
                name: anon,
                ty: t1,
                ctors: vec![],
            }],
        },
    );
    assert!(matches!(e, KernelError::InvalidInductive(_)));
}

// ---- names, universe params, closedness -----------------------------------------------

#[test]
fn duplicate_and_existing_names_are_rejected() {
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, a) = (sort(st, 1), c(st, "A"));
    // two constructors with one name
    let e = expect_err(
        &mut env,
        single(&[], 0, "A", t1, vec![ctor("A.x", a), ctor("A.x", a)]),
    );
    assert_eq!(e, KernelError::AlreadyDeclared(n("A.x")));
    // a constructor name that already exists
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("Nat.zero", a)]));
    assert_eq!(e, KernelError::AlreadyDeclared(n("Nat.zero")));
    // a constructor called `A.rec` collides with the generated recursor
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("A.rec", a)]));
    assert_eq!(e, KernelError::AlreadyDeclared(n("A.rec")));
    // the type name already exists
    let e = expect_err(&mut env, single(&[], 0, "Nat", t1, vec![]));
    assert_eq!(e, KernelError::AlreadyDeclared(n("Nat")));
    // the recursor name already exists (someone declared `A.rec` first)
    let ax_ty = sort(&mut env.store, 0);
    env.add_declaration(Declaration::axiom(n("A.rec"), vec![], ax_ty))
        .unwrap();
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("A.x", a)]));
    assert_eq!(e, KernelError::AlreadyDeclared(n("A.rec")));
    // redeclaring a finished inductive
    let b = c(&mut env.store, "Bool");
    let e = expect_err(
        &mut env,
        single(&[], 0, "Bool", t1, vec![ctor("Bool.z", b)]),
    );
    assert_eq!(e, KernelError::AlreadyDeclared(n("Bool")));
}

#[test]
fn universe_params_must_be_distinct_and_declared() {
    let mut env = std_env();
    let st = &mut env.store;
    let t1 = sort(st, 1);
    let e = expect_err(&mut env, single(&["u", "u"], 0, "A", t1, vec![]));
    assert_eq!(e, KernelError::DuplicateLevelParam(n("u")));
    // a constructor mentioning `v`, which is not declared
    let st = &mut env.store;
    let a = c(st, "A");
    let sv = sort_l(st, &lp("v"));
    let mk = arrow(st, sv, a);
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]));
    assert_eq!(e, KernelError::UndeclaredLevelParam(n("v")));
}

#[test]
fn declarations_must_be_closed_and_owned() {
    let mut env = std_env();
    let st = &mut env.store;
    let t1 = sort(st, 1);
    // a loose bound variable in a constructor
    let a = c(st, "A");
    let loose = st.bvar(0);
    let mk = arrow(st, loose, a);
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]));
    assert!(matches!(e, KernelError::LooseBVar(_)));
    // a free variable
    let id = env.store.fresh_fvar_id();
    let x = env.store.fvar(id);
    let a = c(&mut env.store, "A");
    let mk = arrow(&mut env.store, x, a);
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![ctor("A.mk", mk)]));
    assert_eq!(e, KernelError::FreeVarInDeclaration);
    // a handle from another store
    let mut other = lean_kernel::ExprStore::new();
    let foreign = other.sort(&lvl(1));
    let e = expect_err(&mut env, single(&[], 0, "A", foreign, vec![]));
    assert_eq!(e, KernelError::ForeignExpr);
}

// ---- unsupported (slice 3b) -----------------------------------------------------------

#[test]
fn mutual_blocks_are_unsupported() {
    let mut env = std_env();
    let st = &mut env.store;
    let t1 = sort(st, 1);
    let mk = |name: &str| InductiveTypeDecl {
        name: n(name),
        ty: t1,
        ctors: vec![],
    };
    let e = expect_err(
        &mut env,
        InductiveDecl {
            level_params: vec![],
            num_params: 0,
            types: vec![mk("A"), mk("B")],
        },
    );
    assert!(matches!(e, KernelError::Unsupported(m) if m.contains("mutual")));
}

#[test]
fn nested_inductives_are_unsupported() {
    // inductive Tree | node : List Tree → Tree
    let mut env = std_env();
    add_list(&mut env);
    let st = &mut env.store;
    let (t1, tree) = (sort(st, 1), c(st, "Tree"));
    let list = cl(st, "List", &[lvl(0)]);
    let lt = st.app(list, tree);
    let node = arrow(st, lt, tree);
    let e = expect_err(
        &mut env,
        single(&[], 0, "Tree", t1, vec![ctor("Tree.node", node)]),
    );
    assert!(matches!(e, KernelError::Unsupported(m) if m.contains("nested")));
}

#[test]
fn list_is_accepted_and_has_an_induction_hypothesis() {
    let mut env = std_env();
    add_list(&mut env);
    let (lps, r) = {
        let d = env.get(&n("List.rec")).unwrap();
        match &d.kind {
            DeclKind::Recursor(r) => (d.level_params.len(), r.clone()),
            k => panic!("{k:?}"),
        }
    };
    assert_eq!(
        (lps, r.num_params, r.num_minors, r.rules.len()),
        (2, 1, 2, 2)
    );
    assert!(r.large_elim && !r.k);
}

// ---- atomicity, forging ---------------------------------------------------------------

#[test]
fn a_failed_block_leaves_the_environment_unchanged_and_can_be_retried() {
    let mut env = std_env();
    let before = env.len();
    let st = &mut env.store;
    let (t1, a, nat) = (sort(st, 1), c(st, "A"), c(st, "Nat"));
    let good = arrow(st, nat, a);
    // second constructor is bad, so the first (good) one must not be left behind
    let bad = c(st, "Nat");
    let e = env
        .add_inductive(single(
            &[],
            0,
            "A",
            t1,
            vec![ctor("A.good", good), ctor("A.bad", bad)],
        ))
        .unwrap_err();
    assert!(matches!(e, KernelError::InvalidInductive(_)));
    assert_eq!(env.len(), before);
    for name in ["A", "A.good", "A.bad", "A.rec"] {
        assert!(!env.contains(&n(name)), "{name} leaked");
    }
    // the corrected declaration now succeeds under the same names
    env.add_inductive(single(&[], 0, "A", t1, vec![ctor("A.good", good)]))
        .unwrap();
    assert_eq!(env.len(), before + 3);
}

#[test]
fn inductive_declarations_cannot_be_forged_through_add_declaration() {
    let mut env = std_env();
    let st = &mut env.store;
    let nat = c(st, "Nat");
    let rule = lean_kernel::RecRule {
        ctor: n("Nat.zero"),
        num_fields: 0,
        rhs: nat,
    };
    // a "recursor" that claims anything
    let forged = Declaration {
        name: n("Fake.rec"),
        level_params: vec![],
        ty: nat,
        kind: DeclKind::Recursor(RecursorInfo {
            induct: n("Nat"),
            num_params: 0,
            num_indices: 0,
            num_minors: 0,
            k: true,
            large_elim: true,
            rules: vec![rule],
        }),
    };
    assert!(matches!(
        env.add_declaration(forged),
        Err(KernelError::Unsupported(_))
    ));
    let ctor_decl = Declaration {
        name: n("Fake.mk"),
        level_params: vec![],
        ty: nat,
        kind: DeclKind::Constructor(ConstructorInfo {
            induct: n("Nat"),
            index: 0,
            num_params: 0,
            num_fields: 0,
        }),
    };
    assert!(matches!(
        env.add_declaration(ctor_decl),
        Err(KernelError::Unsupported(_))
    ));
    assert!(!env.contains(&n("Fake.rec")) && !env.contains(&n("Fake.mk")));
}

#[test]
fn anonymous_constructor_names_are_rejected() {
    let mut env = std_env();
    let st = &mut env.store;
    let (t1, a) = (sort(st, 1), c(st, "A"));
    let anon = lean_kernel::ConstructorDecl {
        name: lean_kernel::Name::anonymous(),
        ty: a,
    };
    let e = expect_err(&mut env, single(&[], 0, "A", t1, vec![anon]));
    assert!(matches!(e, KernelError::InvalidInductive(m) if m.contains("anonymous constructor")));
}
