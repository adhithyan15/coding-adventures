//! Regressions for the denial-of-service paths found in review of slice 2: huge or deep
//! universe levels, node-count blow-up inside one store traversal, and foreign handles.
//!
//! Every test runs on a 64 MiB thread stack, far more than the checker should ever use,
//! so a pass means "an error was returned", not "the recursion happened to fit".

use lean_kernel::{
    Binder, BinderInfo, Declaration, Environment, ExprStore, KernelError, Level, Limits, Name,
    ReducibilityHints,
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
fn big_stack(f: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(f)
        .unwrap()
        .join()
        .unwrap();
}
fn succs(l: &Level, k: usize) -> Level {
    (0..k).fold(l.clone(), |acc, _| Level::succ(&acc))
}
fn base() -> Environment {
    let mut env = Environment::new();
    let ty = env.store.sort(&Level::of_nat(1));
    env.add_declaration(Declaration::axiom(n("Nat"), vec![], ty))
        .unwrap();
    let nat = env.store.constant(&n("Nat"), &[]);
    env.add_declaration(Declaration::axiom(n("Nat.zero"), vec![], nat))
        .unwrap();
    env
}
fn is_limit<T: std::fmt::Debug>(r: Result<T, KernelError>) -> bool {
    matches!(r, Err(KernelError::ResourceLimit(_)))
}

#[test]
fn huge_level_offsets_are_a_resource_limit_not_a_panic() {
    big_stack(|| {
        let (u, v) = (Level::param(&n("u")), Level::param(&n("v")));
        let a = succs(&Level::max(&u, &v), 70_000);
        let c = succs(&Level::max(&succs(&u, 40_000), &v), 40_000);
        for l in [a, c] {
            let mut env = base();
            let sort = env.store.sort(&l);
            let zero = env.store.constant(&n("Nat.zero"), &[]);
            let d = Declaration::theorem(n("t"), vec![n("u"), n("v")], sort, zero);
            assert!(is_limit(env.add_declaration(d)));
            assert!(!env.contains(&n("t")));
        }
    });
}

#[test]
fn level_operations_never_panic_on_oversized_levels() {
    big_stack(|| {
        let (u, v) = (Level::param(&n("u")), Level::param(&n("v")));
        let l = succs(&Level::max(&succs(&u, 40_000), &succs(&v, 40_000)), 40_000);
        // No panic; and equivalence is only claimed when the levels are structurally equal.
        let _ = l.normalize();
        assert!(l.is_equiv(&l.clone()));
        assert!(!l.is_equiv(&Level::zero()));
        let _ = l.is_geq(&u);
        let _ = l.is_never_zero();
    });
}

#[test]
fn deeply_nested_levels_are_a_resource_limit_not_a_crash() {
    big_stack(|| {
        let v = Level::param(&n("v"));
        let build = || (0..1_000_000).fold(v.clone(), |acc, _| Level::max(&acc, &v));
        let (l1, l2) = (build(), build());
        // Hash-consing two equal, separately built deep levels must not recurse.
        let mut st = ExprStore::new();
        let (s1, s2) = (st.sort(&l1), st.sort(&l2));
        assert_eq!(s1, s2);
        // The other level operations are stack-safe too.
        assert!(!l1.is_never_zero());
        assert_eq!(l1.cmp(&l2), std::cmp::Ordering::Equal);
        assert!(!format!("{l1}").is_empty());
        let inst = l1.instantiate(&[n("v")], &[Level::zero()]).unwrap();
        assert!(!inst.has_param());
        let _ = l1.normalize();
        // In a declaration, in Sort and in Const arguments:
        let mut env = base();
        let sort = env.store.sort(&l1);
        let d = Declaration::axiom(n("A"), vec![n("v")], sort);
        assert!(is_limit(env.add_declaration(d)));
        let nat = env.store.constant(&n("Nat"), &[]);
        let ty = env.store.sort(&Level::of_nat(1));
        let id = env.store.constant(&n("Nat"), std::slice::from_ref(&l2));
        let d = Declaration::definition(n("B"), vec![n("v")], ty, id, ReducibilityHints::Abbrev);
        let _ = nat;
        assert!(env.add_declaration(d).is_err());
        let mut tc = env.type_checker(&[n("v")]);
        assert!(is_limit(tc.infer(sort)));
    });
}

#[test]
fn a_shared_level_dag_does_not_take_exponential_time() {
    big_stack(|| {
        // L_k = max(L_{k-1}, L_{k-1}) with a shared child: 2^100 paths, 100 nodes.
        let u = Level::param(&n("u"));
        let l = (0..100).fold(u, |acc, _| Level::max(&acc, &acc));
        let l2 = (0..100).fold(Level::param(&n("u")), |acc, _| Level::max(&acc, &acc));
        assert_eq!(l.cmp(&l2), std::cmp::Ordering::Equal);
        assert!(!l.is_never_zero());
        let mut env = base();
        let sort = env.store.sort(&l);
        let d = Declaration::axiom(n("A"), vec![n("u")], sort);
        assert!(is_limit(env.add_declaration(d)));
    });
}

#[test]
fn one_traversal_cannot_intern_unbounded_nodes() {
    big_stack(|| {
        // T_k = App(Lam(_, A, T_{k-1}), T_{k-1}) over a free variable: abstracting it
        // creates a distinct node per (subterm, binder depth), about k^2 / 2 of them.
        let mut st = ExprStore::new();
        let id = st.fresh_fvar_id();
        let x = st.fvar(id);
        let a = st.sort(&Level::zero());
        let mut t = x;
        for _ in 0..300 {
            let lam = st.lam(b("_"), a, t);
            t = st.app(lam, t);
        }
        let cap = st.len() + 1_000;
        st.set_node_cap(Some(cap));
        let _ = st.abstract_fvars(t, &[id]);
        assert!(st.budget_exceeded());
        // Unwinding rebuilds at most one parent per stack frame (depth ~600), so the
        // overshoot is linear in the depth, not the ~45,000 nodes an unchecked run makes.
        let over = st.len() - cap;
        assert!(
            over < 2_000,
            "traversal ran far past its budget: {over} over"
        );
        st.set_node_cap(None);
        assert!(!st.budget_exceeded());
    });
}

#[test]
fn a_checker_reports_node_growth_inside_a_traversal_and_releases_the_cap() {
    big_stack(|| {
        let mut env = base();
        let st = &mut env.store;
        let id = st.fresh_fvar_id();
        let x = st.fvar(id);
        let a = st.sort(&Level::zero());
        let zero = st.constant(&n("Nat.zero"), &[]);
        let mut t = x;
        for _ in 0..300 {
            let lam = st.lam(b("_"), a, t);
            t = st.app(lam, t);
        }
        let mut tc = env.type_checker(&[]).with_limits(Limits {
            max_new_nodes: 50,
            ..Limits::default()
        });
        // a traversal that blows the budget poisons the next checker step
        let _ = tc.store().abstract_fvars(t, &[id]);
        assert!(is_limit(tc.infer(zero)));
        drop(tc);
        // after the checker is gone the store is unrestricted again
        assert!(!env.store.budget_exceeded());
        let _ = env.store.abstract_fvars(t, &[id]);
        assert!(!env.store.budget_exceeded());
    });
}

#[test]
fn foreign_expressions_are_an_error_not_a_panic() {
    let mut env1 = base();
    let mut env2 = base();
    let foreign_ty = env1.store.sort(&Level::of_nat(1));
    let foreign_val = env1.store.constant(&n("Nat.zero"), &[]);
    let own_ty = env2.store.constant(&n("Nat"), &[]);
    let d = Declaration::axiom(n("A"), vec![], foreign_ty);
    assert_eq!(env2.add_declaration(d), Err(KernelError::ForeignExpr));
    let d = Declaration::definition(
        n("B"),
        vec![],
        own_ty,
        foreign_val,
        ReducibilityHints::Abbrev,
    );
    assert_eq!(env2.add_declaration(d), Err(KernelError::ForeignExpr));
    assert!(!env2.contains(&n("A")) && !env2.contains(&n("B")));
    let mut tc = env2.type_checker(&[]);
    assert_eq!(tc.infer(foreign_val), Err(KernelError::ForeignExpr));
    assert_eq!(tc.whnf(foreign_val), Err(KernelError::ForeignExpr));
    assert_eq!(tc.ensure_sort(foreign_ty), Err(KernelError::ForeignExpr));
    assert_eq!(tc.ensure_pi(foreign_ty), Err(KernelError::ForeignExpr));
    assert_eq!(
        tc.is_def_eq(foreign_ty, foreign_val),
        Err(KernelError::ForeignExpr)
    );
}
