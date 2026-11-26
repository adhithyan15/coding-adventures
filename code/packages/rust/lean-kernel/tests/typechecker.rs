//! Positive and negative tests for the environment, `infer`, `whnf` and `is_def_eq`.

use lean_kernel::{
    Binder, BinderInfo, Declaration, Environment, Expr, ExprStore, KernelError, Level,
    LocalContext, LocalDecl, Name, ReducibilityHints,
};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}
fn b(s: &str) -> Binder {
    Binder {
        name: n(s),
        info: BinderInfo::Default,
    }
}
fn lvl(k: u32) -> Level {
    Level::of_nat(k)
}
fn u() -> Level {
    Level::param(&n("u"))
}
fn con(st: &mut ExprStore, s: &str) -> Expr {
    st.constant(&n(s), &[])
}
fn sort(st: &mut ExprStore, k: u32) -> Expr {
    st.sort(&lvl(k))
}
fn arrow(st: &mut ExprStore, a: Expr, c: Expr) -> Expr {
    // `c` must not mention loose variables, so no lifting is needed
    st.pi(b("_"), a, c)
}

/// `Nat : Type`, `String : Type`, `Nat.zero : Nat`, `id.{u}`, and `p_imp_p`.
fn base() -> Environment {
    let mut env = Environment::new();
    let ty = sort(&mut env.store, 1);
    env.add_declaration(Declaration::axiom(n("Nat"), vec![], ty))
        .unwrap();
    env.add_declaration(Declaration::axiom(n("String"), vec![], ty))
        .unwrap();
    let nat = con(&mut env.store, "Nat");
    env.add_declaration(Declaration::axiom(n("Nat.zero"), vec![], nat))
        .unwrap();
    // id.{u} : {α : Sort u} -> α -> α := fun α a => a
    let st = &mut env.store;
    let su = st.sort(&u());
    let (b0, b1) = (st.bvar(0), st.bvar(1));
    let inner_ty = st.pi(b("a"), b0, b1);
    let id_ty = st.pi(
        Binder {
            name: n("α"),
            info: BinderInfo::Implicit,
        },
        su,
        inner_ty,
    );
    let inner = st.lam(b("a"), b0, b0);
    let id_val = st.lam(b("α"), su, inner);
    env.add_declaration(Declaration::definition(
        n("id"),
        vec![n("u")],
        id_ty,
        id_val,
        ReducibilityHints::Abbrev,
    ))
    .unwrap();
    // theorem p_imp_p : forall p : Prop, p -> p := fun p h => h
    let st = &mut env.store;
    let prop = st.sort(&lvl(0));
    let (b0, b1) = (st.bvar(0), st.bvar(1));
    let inner_ty = st.pi(b("h"), b0, b1);
    let thm_ty = st.pi(b("p"), prop, inner_ty);
    let inner = st.lam(b("h"), b0, b0);
    let thm_val = st.lam(b("p"), prop, inner);
    env.add_declaration(Declaration::theorem(n("p_imp_p"), vec![], thm_ty, thm_val))
        .unwrap();
    env
}

/// `@id.{1} Nat Nat.zero`
fn id_nat_zero(st: &mut ExprStore) -> Expr {
    let id = st.constant(&n("id"), &[lvl(1)]);
    let (nat, zero) = (con(st, "Nat"), con(st, "Nat.zero"));
    st.apps(id, &[nat, zero])
}

fn assert_unchanged_on_err(env: &mut Environment, d: Declaration, name: &str) -> KernelError {
    let before = env.len();
    let err = env.add_declaration(d).unwrap_err();
    assert_eq!(env.len(), before, "environment must be unchanged on error");
    assert!(!env.contains(&n(name)));
    err
}

// ---------------------------------------------------------------- positive

#[test]
fn id_applied_to_nat_zero_has_type_nat() {
    let mut env = base();
    let e = id_nat_zero(&mut env.store);
    let nat = con(&mut env.store, "Nat");
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.infer(e).unwrap(), nat);
    tc.check(e, nat).unwrap();
    // infer is cached and stable
    assert_eq!(tc.infer(e).unwrap(), nat);
}

#[test]
fn theorem_p_imp_p_was_accepted() {
    let env = base();
    assert!(env.contains(&n("p_imp_p")));
    assert!(env.get(&n("p_imp_p")).unwrap().value().is_some());
    assert!(env.get(&n("Nat")).unwrap().value().is_none());
}

#[test]
fn definitions_needing_delta_beta_and_zeta() {
    let mut env = base();
    // delta: def N : Type := Nat ; def z : N := Nat.zero  (needs N ~> Nat)
    let (ty, nat) = (sort(&mut env.store, 1), con(&mut env.store, "Nat"));
    env.add_declaration(Declaration::definition(
        n("N"),
        vec![],
        ty,
        nat,
        ReducibilityHints::Abbrev,
    ))
    .unwrap();
    let (cn, zero) = (con(&mut env.store, "N"), con(&mut env.store, "Nat.zero"));
    env.add_declaration(Declaration::definition(
        n("z"),
        vec![],
        cn,
        zero,
        ReducibilityHints::Regular(1),
    ))
    .unwrap();
    // beta: def z2 : (fun T : Type => T) Nat := Nat.zero
    let st = &mut env.store;
    let (ty, b0) = (sort(st, 1), st.bvar(0));
    let f = st.lam(b("T"), ty, b0);
    let nat = con(st, "Nat");
    let redex = st.app(f, nat);
    let zero = con(st, "Nat.zero");
    env.add_declaration(Declaration::definition(
        n("z2"),
        vec![],
        redex,
        zero,
        ReducibilityHints::Opaque,
    ))
    .unwrap();
    // delta + beta together: def z3 : Nat := id.{1} N z  (id N z : N ~> Nat)
    let st = &mut env.store;
    let id = st.constant(&n("id"), &[lvl(1)]);
    let (cn, cz, nat) = (con(st, "N"), con(st, "z"), con(st, "Nat"));
    let val = st.apps(id, &[cn, cz]);
    env.add_declaration(Declaration::definition(
        n("z3"),
        vec![],
        nat,
        val,
        ReducibilityHints::Regular(2),
    ))
    .unwrap();
    // zeta: def l : Nat := let T : Type := Nat; id.{1} T Nat.zero
    // the body's type is `T`, which equals `Nat` only by unfolding the let variable
    let st = &mut env.store;
    let id = st.constant(&n("id"), &[lvl(1)]);
    let (b0, zero, nat, ty) = (st.bvar(0), con(st, "Nat.zero"), con(st, "Nat"), sort(st, 1));
    let body = st.apps(id, &[b0, zero]);
    let val = st.let_e(b("T"), ty, nat, body);
    env.add_declaration(Declaration::definition(
        n("l"),
        vec![],
        nat,
        val,
        ReducibilityHints::Regular(1),
    ))
    .unwrap();
}

#[test]
fn let_types_keep_or_drop_the_binder() {
    let mut env = base();
    let mut tc = env.type_checker(&[]);
    let st = tc.store();
    // let x : Nat := zero; x   has type  Nat  (x is not in the type: binder dropped)
    let (nat, zero, b0) = (con(st, "Nat"), con(st, "Nat.zero"), st.bvar(0));
    let e = st.let_e(b("x"), nat, zero, b0);
    assert_eq!(tc.infer(e).unwrap(), nat);
    // let T : Type := Nat; fun (a : T) => a  has type  let T := Nat; T -> T  (kept)
    let st = tc.store();
    let (ty, b0, b1) = (sort(st, 1), st.bvar(0), st.bvar(1));
    let lam = st.lam(b("a"), b0, b0);
    let e = st.let_e(b("T"), ty, nat, lam);
    let pi = st.pi(b("a"), b0, b1);
    let want = st.let_e(b("T"), ty, nat, pi);
    assert_eq!(tc.infer(e).unwrap(), want);
    // the let type is definitionally Nat -> Nat
    let st = tc.store();
    let nn = arrow(st, nat, nat);
    assert!(tc.is_def_eq(want, nn).unwrap());
}

#[test]
fn lambda_and_pi_inference() {
    let mut env = base();
    let mut tc = env.type_checker(&[]);
    let st = tc.store();
    let (ty, b0, b1) = (sort(st, 1), st.bvar(0), st.bvar(1));
    // fun (A : Type) (a : A) => a   :   (A : Type) -> A -> A
    let inner = st.lam(b("a"), b0, b0);
    let e = st.lam(b("A"), ty, inner);
    let pi_in = st.pi(b("a"), b0, b1);
    let want = st.pi(b("A"), ty, pi_in);
    assert_eq!(tc.infer(e).unwrap(), want);
    // the Pi is itself well typed: (A : Type) -> A -> A  :  Type 1 (= Sort 2)
    let s = tc.infer(want).unwrap();
    let l = tc.ensure_sort(s).unwrap();
    assert!(l.is_equiv(&lvl(2)));
    // Nat -> Nat : Type
    let st = tc.store();
    let nat = con(st, "Nat");
    let nn = arrow(st, nat, nat);
    let s = tc.infer(nn).unwrap();
    assert!(tc.ensure_sort(s).unwrap().is_equiv(&lvl(1)));
}

#[test]
fn polymorphic_statement_lives_in_prop() {
    let mut env = base();
    let ty = env.get(&n("p_imp_p")).unwrap().ty;
    let mut tc = env.type_checker(&[]);
    let s = tc.infer(ty).unwrap();
    assert!(tc.ensure_sort(s).unwrap().is_equiv(&lvl(0)));
}

#[test]
fn literals_are_unsupported_until_primitive_validation() {
    let mut env = base();
    let lit = env.store.nat_lit(bignum_core::BigInteger::from_i64(5));
    let s = env.store.str_lit("hi");
    let mut tc = env.type_checker(&[]);
    // even with a genuine-looking `Nat` and `String` declared: no primitive validation yet
    assert!(matches!(tc.infer(lit), Err(KernelError::Unsupported(_))));
    assert!(matches!(tc.infer(s), Err(KernelError::Unsupported(_))));
}

#[test]
fn definitional_equality_cases() {
    let mut env = base();
    let mut tc = env.type_checker(&[n("u"), n("v")]);
    let st = tc.store();
    let (lu, lv) = (Level::param(&n("u")), Level::param(&n("v")));
    // Sort (max u v) == Sort (max v u); Sort u != Sort v
    let (a, c) = (
        st.sort(&Level::max(&lu, &lv)),
        st.sort(&Level::max(&lv, &lu)),
    );
    assert!(tc.is_def_eq(a, c).unwrap());
    let st = tc.store();
    let (su, sv) = (st.sort(&lu), st.sort(&lv));
    assert!(!tc.is_def_eq(su, sv).unwrap());
    // binder names are irrelevant; a bound variable is not a constant
    let st = tc.store();
    let (nat, zero, b0) = (con(st, "Nat"), con(st, "Nat.zero"), st.bvar(0));
    let fx = st.lam(b("x"), nat, b0);
    let fy = st.lam(b("y"), nat, b0);
    let fc = st.lam(b("y"), nat, zero);
    assert!(tc.is_def_eq(fx, fy).unwrap());
    assert!(!tc.is_def_eq(fx, fc).unwrap());
    // application congruence, and beta on one side
    let st = tc.store();
    let idc = st.constant(&n("id"), &[lvl(1)]);
    let (nat, zero) = (con(st, "Nat"), con(st, "Nat.zero"));
    let app = st.apps(idc, &[nat, zero]);
    assert!(tc.is_def_eq(app, zero).unwrap());
    assert!(tc.is_def_eq(zero, app).unwrap());
    // constants with different level arguments are different when nothing unfolds
    let st = tc.store();
    let (i1, i2) = (
        st.constant(&n("id"), &[lvl(1)]),
        st.constant(&n("id"), &[lvl(2)]),
    );
    assert!(!tc.is_def_eq(i1, i2).unwrap());
    // Pi domains are compared
    let st = tc.store();
    let (nat, ty) = (con(st, "Nat"), sort(st, 1));
    let (p1, p2) = (arrow(st, nat, nat), arrow(st, ty, ty));
    assert!(!tc.is_def_eq(p1, p2).unwrap());
    // eta needs `f`'s type, and `f` is not declared here, so the rule does not apply and
    // the pair stays unequal (tests/defeq_rules.rs covers eta on a declared `f`).
    let st = tc.store();
    let nat = con(st, "Nat");
    let f = con(st, "f"); // def-eq does not need `f` to be declared: it is simply stuck
    let b0 = st.bvar(0);
    let body = st.app(f, b0);
    let eta = st.lam(b("x"), nat, body);
    assert!(!tc.is_def_eq(eta, f).unwrap());
}

#[test]
fn whnf_unfolds_definitions_only() {
    let mut env = base();
    // opaque and theorem constants, plus an axiom
    let (ty, nat, zero) = (
        sort(&mut env.store, 1),
        con(&mut env.store, "Nat"),
        con(&mut env.store, "Nat.zero"),
    );
    let _ = ty;
    env.add_declaration(Declaration::opaque(n("o"), vec![], nat, zero))
        .unwrap();
    let o = con(&mut env.store, "o");
    let thm = con(&mut env.store, "p_imp_p");
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.whnf(o).unwrap(), o);
    assert_eq!(tc.whnf(thm).unwrap(), thm);
    assert_eq!(tc.whnf(nat).unwrap(), nat);
    // a definition does unfold
    let st = tc.store();
    let e = id_nat_zero(st);
    let zero = con(st, "Nat.zero");
    assert_eq!(tc.whnf(e).unwrap(), zero);
    // whnf_core does not delta
    let st = tc.store();
    let e = id_nat_zero(st);
    assert_eq!(tc.whnf_core(e).unwrap(), e);
    // unknown constants and bound variables are stuck, not errors
    let st = tc.store();
    let ghost = con(st, "ghost");
    let v = st.bvar(0);
    assert_eq!(tc.whnf(ghost).unwrap(), ghost);
    assert_eq!(tc.whnf(v).unwrap(), v);
}

#[test]
fn whnf_partial_beta_and_stuck_heads() {
    let mut env = base();
    let mut tc = env.type_checker(&[]);
    let st = tc.store();
    let (nat, zero, b0, b1) = (con(st, "Nat"), con(st, "Nat.zero"), st.bvar(0), st.bvar(1));
    // (fun x y => x) zero  ~>  fun y => zero
    let inner = st.lam(b("y"), nat, b1);
    let f = st.lam(b("x"), nat, inner);
    let e = st.app(f, zero);
    let want = st.lam(b("y"), nat, zero);
    assert_eq!(tc.whnf(e).unwrap(), want);
    // (fun x => x) zero zero  ~>  zero zero   (extra arguments are kept)
    let st = tc.store();
    let idl = st.lam(b("x"), nat, b0);
    let e = st.apps(idl, &[zero, zero]);
    let want = st.app(zero, zero);
    assert_eq!(tc.whnf(e).unwrap(), want);
    // a let at the head of an application is unfolded first
    let st = tc.store();
    let bodyl = st.lam(b("y"), nat, b0);
    let lt = st.let_e(b("x"), nat, zero, bodyl);
    let e = st.app(lt, zero);
    assert_eq!(tc.whnf(e).unwrap(), zero);
}

#[test]
fn definition_with_wrong_level_count_is_an_error_in_whnf() {
    let mut env = base();
    let bad = env.store.constant(&n("id"), &[]);
    let mut tc = env.type_checker(&[]);
    assert!(matches!(
        tc.whnf(bad),
        Err(KernelError::LevelArityMismatch { .. })
    ));
}

#[test]
fn local_context_rejects_id_reuse() {
    let mut st = ExprStore::new();
    let ty = st.sort(&lvl(0));
    let id = st.fresh_fvar_id();
    let mut lctx = LocalContext::new();
    assert!(lctx.is_empty());
    let decl = LocalDecl {
        name: n("x"),
        ty,
        value: None,
    };
    lctx.insert(id, decl.clone()).unwrap();
    assert_eq!(lctx.len(), 1);
    assert_eq!(lctx.get(id).unwrap().ty, ty);
    assert!(matches!(
        lctx.insert(id, decl),
        Err(KernelError::Internal(_))
    ));
}

#[test]
fn ensure_sort_and_pi_reduce_first() {
    let mut env = base();
    // def T : Type 1 := Type ; ensure_sort (T) sees through the definition
    let (s2, s1) = (sort(&mut env.store, 2), sort(&mut env.store, 1));
    env.add_declaration(Declaration::definition(
        n("T"),
        vec![],
        s2,
        s1,
        ReducibilityHints::Abbrev,
    ))
    .unwrap();
    let t = con(&mut env.store, "T");
    let nat = con(&mut env.store, "Nat");
    let mut tc = env.type_checker(&[]);
    assert!(tc.ensure_sort(t).unwrap().is_equiv(&lvl(1)));
    assert!(matches!(tc.ensure_sort(nat), Err(KernelError::NotASort(_))));
    assert!(matches!(tc.ensure_pi(nat), Err(KernelError::NotAPi(_))));
    let st = tc.store();
    let nn = arrow(st, nat, nat);
    assert_eq!(tc.ensure_pi(nn).unwrap(), nn);
}

// ---------------------------------------------------------------- negative

#[test]
fn ill_typed_application_is_a_type_mismatch() {
    let mut env = base();
    let st = &mut env.store;
    let id = st.constant(&n("id"), &[lvl(1)]);
    let nat = con(st, "Nat");
    let bad = st.apps(id, &[nat, nat]); // second argument must be a Nat, `Nat` is a Type
    let mut tc = env.type_checker(&[]);
    assert!(matches!(
        tc.infer(bad),
        Err(KernelError::TypeMismatch { .. })
    ));
    // check() reports the same for a wrong expected type
    let st = tc.store();
    let (zero, ty) = (con(st, "Nat.zero"), sort(st, 1));
    assert!(matches!(
        tc.check(zero, ty),
        Err(KernelError::TypeMismatch { .. })
    ));
}

#[test]
fn applying_a_non_function_is_not_a_pi() {
    let mut env = base();
    let z = con(&mut env.store, "Nat.zero");
    let bad = env.store.app(z, z);
    let mut tc = env.type_checker(&[]);
    assert!(matches!(tc.infer(bad), Err(KernelError::NotAPi(_))));
}

#[test]
fn wrong_number_of_level_arguments() {
    let mut env = base();
    let none = env.store.constant(&n("id"), &[]);
    let two = env.store.constant(&n("id"), &[lvl(0), lvl(1)]);
    let mut tc = env.type_checker(&[]);
    for bad in [none, two] {
        assert!(matches!(
            tc.infer(bad),
            Err(KernelError::LevelArityMismatch { expected: 1, .. })
        ));
    }
}

#[test]
fn undeclared_level_parameter() {
    let mut env = base();
    let su = env.store.sort(&u());
    let nested = env.store.sort(&Level::max(&lvl(1), &Level::succ(&u())));
    let id_u = env.store.constant(&n("id"), &[Level::imax(&lvl(1), &u())]);
    {
        let mut tc = env.type_checker(&[]);
        for bad in [su, nested, id_u] {
            assert_eq!(
                tc.infer(bad),
                Err(KernelError::UndeclaredLevelParam(n("u")))
            );
        }
    }
    // the same term is fine when `u` is declared
    assert!(env.type_checker(&[n("u")]).infer(su).is_ok());
    // and as a declaration: `axiom A : Sort u` with no parameter list
    let d = Declaration::axiom(n("A"), vec![], su);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "A"),
        KernelError::UndeclaredLevelParam(n("u"))
    );
}

#[test]
fn duplicate_declaration_and_duplicate_level_param() {
    let mut env = base();
    let ty = sort(&mut env.store, 1);
    let d = Declaration::axiom(n("Nat"), vec![], ty);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "Nat.never"),
        KernelError::AlreadyDeclared(n("Nat"))
    );
    let d = Declaration::axiom(n("W"), vec![n("u"), n("v"), n("u")], ty);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "W"),
        KernelError::DuplicateLevelParam(n("u"))
    );
}

#[test]
fn theorem_must_be_a_proposition() {
    let mut env = base();
    // theorem T : Type := Nat   (Type : Sort 2, not Prop)
    let (ty, nat) = (sort(&mut env.store, 1), con(&mut env.store, "Nat"));
    let d = Declaration::theorem(n("T"), vec![], ty, nat);
    assert!(matches!(
        assert_unchanged_on_err(&mut env, d, "T"),
        KernelError::TheoremNotProp(_)
    ));
    // theorem T2 : Nat := Nat.zero   (Nat : Type, so Nat is not a proposition)
    let zero = con(&mut env.store, "Nat.zero");
    let d = Declaration::theorem(n("T2"), vec![], nat, zero);
    assert!(matches!(
        assert_unchanged_on_err(&mut env, d, "T2"),
        KernelError::TheoremNotProp(_)
    ));
}

#[test]
fn definition_value_type_mismatch() {
    let mut env = base();
    let (nat, prop) = (con(&mut env.store, "Nat"), sort(&mut env.store, 0));
    let d = Declaration::definition(n("d"), vec![], nat, prop, ReducibilityHints::Abbrev);
    assert!(matches!(
        assert_unchanged_on_err(&mut env, d, "d"),
        KernelError::TypeMismatch { .. }
    ));
}

#[test]
fn declarations_must_be_closed() {
    let mut env = base();
    let b0 = env.store.bvar(0);
    let d = Declaration::axiom(n("lb"), vec![], b0);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "lb"),
        KernelError::LooseBVar(0)
    );
    let id = env.store.fresh_fvar_id();
    let x = env.store.fvar(id);
    let d = Declaration::axiom(n("fv"), vec![], x);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "fv"),
        KernelError::FreeVarInDeclaration
    );
    // also in the value
    let nat = con(&mut env.store, "Nat");
    let d = Declaration::definition(n("fv2"), vec![], nat, x, ReducibilityHints::Abbrev);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "fv2"),
        KernelError::FreeVarInDeclaration
    );
    // and a free variable the checker never introduced is unknown to `infer`
    let mut tc = env.type_checker(&[]);
    assert_eq!(tc.infer(x), Err(KernelError::UnknownFVar));
    let b3 = tc.store().bvar(3);
    assert_eq!(tc.infer(b3), Err(KernelError::LooseBVar(3)));
}

#[test]
fn unknown_constant_and_no_self_reference() {
    let mut env = base();
    let ghost = con(&mut env.store, "ghost");
    let mut tc = env.type_checker(&[]);
    assert_eq!(
        tc.infer(ghost),
        Err(KernelError::UnknownConstant(n("ghost")))
    );
    drop(tc);
    // def loop : Nat := loop   -- its own name is not in the environment yet
    let (nat, lp) = (con(&mut env.store, "Nat"), con(&mut env.store, "loop"));
    let d = Declaration::definition(n("loop"), vec![], nat, lp, ReducibilityHints::Abbrev);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "loop"),
        KernelError::UnknownConstant(n("loop"))
    );
    // an axiom whose type mentions itself
    let lp = con(&mut env.store, "selfish");
    let d = Declaration::axiom(n("selfish"), vec![], lp);
    assert_eq!(
        assert_unchanged_on_err(&mut env, d, "selfish"),
        KernelError::UnknownConstant(n("selfish"))
    );
}

#[test]
fn binder_type_must_be_a_sort() {
    let mut env = base();
    let st = &mut env.store;
    let (zero, b0) = (con(st, "Nat.zero"), st.bvar(0));
    let lam = st.lam(b("x"), zero, b0); // fun (x : Nat.zero) => x
    let pi = st.pi(b("x"), zero, zero); // (x : Nat.zero) -> Nat.zero
    let nat = con(st, "Nat");
    let body_not_type = st.pi(b("x"), nat, zero); // Nat -> Nat.zero : body is not a type
    let lt = st.let_e(b("x"), zero, zero, b0); // let x : Nat.zero := ..
    let mut tc = env.type_checker(&[]);
    for bad in [lam, pi, body_not_type, lt] {
        assert!(
            matches!(tc.infer(bad), Err(KernelError::NotASort(_))),
            "{bad:?}"
        );
    }
}

#[test]
fn let_value_must_match_its_type() {
    let mut env = base();
    let st = &mut env.store;
    let (nat, ty, b0) = (con(st, "Nat"), sort(st, 1), st.bvar(0));
    let bad = st.let_e(b("x"), nat, ty, b0); // let x : Nat := Type; x
    let mut tc = env.type_checker(&[]);
    assert!(matches!(
        tc.infer(bad),
        Err(KernelError::TypeMismatch { .. })
    ));
}

#[test]
fn projections_are_unsupported() {
    let mut env = base();
    let zero = con(&mut env.store, "Nat.zero");
    let p = env.store.proj(&n("Prod"), 0, zero);
    let mut tc = env.type_checker(&[]);
    assert!(matches!(tc.infer(p), Err(KernelError::Unsupported(_))));
    drop(tc);
    // and as a declaration
    let nat = con(&mut env.store, "Nat");
    let d = Declaration::definition(n("pr"), vec![], nat, p, ReducibilityHints::Abbrev);
    assert!(matches!(
        assert_unchanged_on_err(&mut env, d, "pr"),
        KernelError::Unsupported(_)
    ));
}

#[test]
fn errors_display_and_implement_error() {
    let e: Box<dyn std::error::Error> = Box::new(KernelError::FreeVarInDeclaration);
    assert!(e.to_string().contains("FreeVarInDeclaration"));
}

#[test]
fn application_congruence_failures_and_mixed_binders() {
    let mut env = Environment::default();
    assert!(env.is_empty());
    let mut tc = env.type_checker(&[]);
    let st = tc.store();
    let (f, g, x, y) = (con(st, "f"), con(st, "g"), con(st, "x"), con(st, "y"));
    let (fx, fy, gx) = (st.app(f, x), st.app(f, y), st.app(g, x));
    let fxy = st.apps(f, &[x, y]);
    assert!(!tc.is_def_eq(fx, fy).unwrap()); // different arguments
    assert!(!tc.is_def_eq(fx, gx).unwrap()); // different heads
    assert!(!tc.is_def_eq(fx, fxy).unwrap()); // different spine lengths
    let st = tc.store();
    let ty = sort(st, 1);
    let (lam, pi) = (st.lam(b("a"), ty, ty), st.pi(b("a"), ty, ty));
    assert!(!tc.is_def_eq(lam, pi).unwrap()); // a lambda is not a Pi
                                              // a lambda whose body is a Pi vs one whose body is a lambda: the telescope stops
    let st = tc.store();
    let (lp, ll) = (st.lam(b("a"), ty, pi), st.lam(b("a"), ty, lam));
    assert!(!tc.is_def_eq(lp, ll).unwrap());
    // binder names are invisible, so these are the same handle; the comparison of
    // bodies that differ introduced a fresh local for each telescope above
    let st = tc.store();
    let b0 = st.bvar(0);
    let (l1, l2) = (st.lam(b("a"), ty, b0), st.lam(b("c"), ty, b0));
    assert_eq!(l1, l2);
    assert!(tc.is_def_eq(l1, l2).unwrap());
    assert!(!tc.lctx().is_empty());
}
