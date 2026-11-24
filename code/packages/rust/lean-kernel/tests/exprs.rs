//! Tests for hash-consed expressions and the de Bruijn operations.
//!
//! Besides hand-built cases for each classic bug (capture, off-by-one shifting, binder
//! names leaking into equality), the operations are checked against deliberately naive
//! reference implementations on randomly generated terms: no caching, no loose-range
//! shortcuts, one explicit shift function. Agreement means the shortcuts are sound.

use bignum_core::BigInteger;
use lean_kernel::{Binder, BinderInfo, Expr, ExprKind, ExprStore, FVarId, Level, Literal, Name};

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}
fn b(name: &str) -> Binder {
    Binder {
        name: n(name),
        info: BinderInfo::Default,
    }
}
fn ty(st: &mut ExprStore) -> Expr {
    let l = Level::of_nat(1);
    st.sort(&l)
}
fn lam(st: &mut ExprStore, body: Expr) -> Expr {
    let t = ty(st);
    st.lam(b("x"), t, body)
}

// ---------------------------------------------------------------------------
//  Hash-consing and metadata
// ---------------------------------------------------------------------------

#[test]
fn equal_structure_is_one_node() {
    let mut st = ExprStore::new();
    assert!(st.is_empty());
    let (c1, c2) = (st.constant(&n("Nat"), &[]), st.constant(&n("Nat"), &[]));
    assert_eq!(c1, c2);
    let a1 = st.app(c1, c2);
    let before = st.len();
    let a2 = st.app(c2, c1);
    assert_eq!(a1, a2);
    assert_eq!(
        st.len(),
        before,
        "re-building an existing term must not grow the store"
    );
    let other = st.constant(&n("Int"), &[]);
    assert_ne!(st.app(c1, other), a1);
    // levels in constants take part in identity
    let u = Level::param(&n("u"));
    assert_ne!(
        st.constant(&n("List"), std::slice::from_ref(&u)),
        st.constant(&n("List"), &[])
    );
    assert_eq!(
        st.constant(&n("List"), std::slice::from_ref(&u)),
        st.constant(&n("List"), &[u])
    );
    let (f3, f4) = (st.fresh_fvar_id(), st.fresh_fvar_id());
    assert_eq!(st.fvar(f3), st.fvar(f3));
    assert_ne!(st.fvar(f3), st.fvar(f4));
    assert_ne!(st.bvar(0), st.bvar(1));
}

#[test]
fn binder_names_and_info_never_affect_identity() {
    let mut st = ExprStore::new();
    let t = ty(&mut st);
    let body = st.bvar(0);
    let x = st.lam(b("x"), t, body);
    let y = st.lam(b("y"), t, body);
    let implicit = st.lam(
        Binder {
            name: n("z"),
            info: BinderInfo::Implicit,
        },
        t,
        body,
    );
    assert_eq!(x, y);
    assert_eq!(x, implicit);
    let p1 = st.pi(b("a"), t, body);
    assert_eq!(
        p1,
        st.pi(
            Binder {
                name: n("b"),
                info: BinderInfo::InstImplicit
            },
            t,
            body
        )
    );
    assert_ne!(p1, x, "Pi and Lam with equal parts are different terms");
    let v = st.nat_lit(BigInteger::from_u64(1));
    let l1 = st.let_e(b("a"), t, v, body);
    assert_eq!(l1, st.let_e(b("b"), t, v, body));
    // the first spelling is the one kept, for printing
    if let ExprKind::Lam(m, _, _) = st.kind(y) {
        assert_eq!(m.0.name, n("x"));
        assert_eq!(m.0.info, BinderInfo::Default);
    } else {
        panic!("expected a lambda");
    }
}

#[test]
fn hash_is_structural_and_insertion_order_independent() {
    let mk = |reverse: bool| {
        let mut st = ExprStore::new();
        let (a, c) = (st.constant(&n("a"), &[]), st.constant(&n("c"), &[]));
        let e = if reverse {
            let _noise = st.constant(&n("noise"), &[]);
            let inner = st.app(c, a);
            lam(&mut st, inner)
        } else {
            let inner = st.app(c, a);
            lam(&mut st, inner)
        };
        st.hash_value(e)
    };
    assert_eq!(mk(false), mk(true));
    let mut st = ExprStore::new();
    let (a, c) = (st.constant(&n("a"), &[]), st.constant(&n("c"), &[]));
    assert_ne!(st.hash_value(a), st.hash_value(c));
    let (ac, ca) = (st.app(a, c), st.app(c, a));
    assert_ne!(st.hash_value(ac), st.hash_value(ca));
}

#[test]
#[should_panic(expected = "did not create it")]
fn handle_from_another_store_is_rejected() {
    let mut s1 = ExprStore::new();
    let s2 = ExprStore::default();
    let e = s1.bvar(0);
    let _ = s2.kind(e);
}

#[test]
fn cached_facts() {
    let mut st = ExprStore::new();
    let (b0, b1, b3) = (st.bvar(0), st.bvar(1), st.bvar(3));
    assert_eq!(
        [
            st.loose_bvar_range(b0),
            st.loose_bvar_range(b1),
            st.loose_bvar_range(b3)
        ],
        [1, 2, 4]
    );
    let c = st.constant(&n("c"), &[]);
    assert!(!st.has_loose_bvars(c));
    // under a binder: #0 is bound, #1 is loose with range 1
    let closed = lam(&mut st, b0);
    let open = lam(&mut st, b1);
    assert_eq!(
        (st.loose_bvar_range(closed), st.loose_bvar_range(open)),
        (0, 1)
    );
    // a loose bvar in the *type* position is not shielded by the binder itself
    let t_open = st.bvar(0);
    let in_type = st.lam(b("x"), t_open, b0);
    assert_eq!(st.loose_bvar_range(in_type), 1);
    let v = st.bvar(2);
    let l = st.let_e(b("x"), c, v, b1);
    assert_eq!(st.loose_bvar_range(l), 3); // val #2 is loose: range 3; body #1 -> 1
    let pr = st.proj(&n("Prod"), 0, b3);
    assert_eq!(st.loose_bvar_range(pr), 4);
    let ap = st.app(b3, b0);
    assert_eq!(st.loose_bvar_range(ap), 4);
    // fvars and level params
    let fv0 = st.fresh_fvar_id();
    let fv = st.fvar(fv0);
    let u = Level::param(&n("u"));
    let srt = st.sort(&u);
    let cu = st.constant(&n("List"), &[Level::zero(), u]);
    let plain = st.sort(&Level::of_nat(2));
    assert!(st.has_fvar(fv) && !st.has_fvar(srt) && !st.has_fvar(plain));
    assert!(st.has_level_param(srt) && st.has_level_param(cu) && !st.has_level_param(plain));
    let mix = st.apps(fv, &[srt]);
    assert!(st.has_fvar(mix) && st.has_level_param(mix));
    let wrapped = lam(&mut st, mix);
    let pw = st.proj(&n("S"), 1, wrapped);
    assert!(st.has_fvar(pw) && st.has_level_param(pw));
}

#[test]
fn literals_and_spines() {
    let mut st = ExprStore::new();
    let a = st.nat_lit(BigInteger::from_u64(7));
    assert_eq!(a, st.nat_lit(BigInteger::from_u64(7)));
    assert_ne!(a, st.nat_lit(BigInteger::from_u64(8)));
    let s7 = st.str_lit("7");
    assert_ne!(a, s7);
    assert!(matches!(st.kind(s7), ExprKind::Lit(Literal::Str(x)) if x == "7"));
    let f = st.constant(&n("f"), &[]);
    let (ix, iy) = (st.fresh_fvar_id(), st.fresh_fvar_id());
    let (x, y) = (st.fvar(ix), st.fvar(iy));
    let app = st.apps(f, &[x, y]);
    assert_eq!(st.get_app_fn(app), f);
    assert_eq!(st.get_app_args(app), vec![x, y]);
    assert_eq!(st.get_app_args(f), vec![]);
    assert_eq!(st.get_app_fn(f), f);
    let (i1, i2) = (st.fresh_fvar_id(), st.fresh_fvar_id());
    assert_ne!(i1, i2);
}

// ---------------------------------------------------------------------------
//  instantiate: the classic bugs, one by one
// ---------------------------------------------------------------------------

#[test]
fn instantiate_basic_and_shifting() {
    let mut st = ExprStore::new();
    let c = st.constant(&n("c"), &[]);
    let (b0, b1, b2) = (st.bvar(0), st.bvar(1), st.bvar(2));
    assert_eq!(st.instantiate1(b0, c), c);
    // other loose indices drop by one: the binder being instantiated is gone
    assert_eq!(st.instantiate1(b1, c), b0);
    assert_eq!(st.instantiate1(b2, c), b1);
    // a closed term is returned as the very same node
    assert_eq!(st.instantiate1(c, b0), c);
    // empty substitution is the identity
    assert_eq!(st.instantiate(b2, &[]), b2);
    // `#0 #1` with [c, d]: instantiate is positional, instantiate_rev is reversed
    let d = st.constant(&n("d"), &[]);
    let app = st.app(b0, b1);
    assert_eq!(st.instantiate(app, &[c, d]), st.app(c, d));
    assert_eq!(st.instantiate_rev(app, &[c, d]), st.app(d, c));
    // beyond the substitution, indices shift down by its length
    assert_eq!(st.instantiate(b2, &[c, d]), b0);
}

#[test]
fn instantiate_does_not_capture() {
    // body = fun x => #1   (the #1 refers to the binder outside `body`)
    // substituting the loose variable #0 for it must give  fun x => #1, NOT fun x => #0
    // (which would be captured by the inner binder).
    let mut st = ExprStore::new();
    let b1 = st.bvar(1);
    let body = lam(&mut st, b1);
    let outer = st.bvar(0);
    let result = st.instantiate1(body, outer);
    assert_eq!(
        result, body,
        "the substituted #0 must be lifted to #1 under the lambda"
    );
    // two binders deep: fun _ => fun _ => #2 refers to the same outside variable, and
    // substituting #0 for it must lift by two, giving the same term back.
    let b2 = st.bvar(2);
    let inner = lam(&mut st, b2);
    let nested = lam(&mut st, inner);
    assert_eq!(st.instantiate1(nested, outer), nested);
}

#[test]
fn instantiate_substitutes_under_binders_at_the_right_index() {
    // (fun x => #1 #0) with #0 := c  ==>  fun x => c #0
    let mut st = ExprStore::new();
    let (b0, b1) = (st.bvar(0), st.bvar(1));
    let c = st.constant(&n("c"), &[]);
    let app = st.app(b1, b0);
    let body = lam(&mut st, app);
    let got = st.instantiate1(body, c);
    let inner = st.app(c, b0);
    let want = lam(&mut st, inner);
    assert_eq!(got, want);
    // a bound #0 inside the binder is left alone, not replaced (off-by-one guard)
    let id = lam(&mut st, b0);
    assert_eq!(st.instantiate1(id, c), id);
    // Pi, Let and Proj positions: val/type at the outer depth, body one deeper
    let t = st.bvar(0);
    let pi = st.pi(b("y"), t, b1); // (y : #0) -> #1
    let got = st.instantiate1(pi, c);
    let want = st.pi(b("y"), c, c);
    assert_eq!(got, want);
    let l = st.let_e(b("z"), t, t, b1); // let z : #0 := #0; #1
    let got = st.instantiate1(l, c);
    assert_eq!(got, st.let_e(b("z"), c, c, c));
    let pr = st.proj(&n("S"), 2, b0);
    assert_eq!(st.instantiate1(pr, c), st.proj(&n("S"), 2, c));
}

#[test]
fn lift_and_lower_loose_bvars() {
    let mut st = ExprStore::new();
    let (b0, b1, b2) = (st.bvar(0), st.bvar(1), st.bvar(2));
    assert_eq!(st.lift_loose_bvars(b1, 0, 3), st.bvar(4));
    assert_eq!(
        st.lift_loose_bvars(b0, 1, 3),
        b0,
        "indices below s are untouched"
    );
    assert_eq!(st.lift_loose_bvars(b1, 1, 3), st.bvar(4));
    assert_eq!(st.lift_loose_bvars(b2, 0, 0), b2);
    // under a binder the cut-off moves with the depth: fun _ => #0 #1  (+1 at s=0) => fun _ => #0 #2
    let app = st.app(b0, b1);
    let l = lam(&mut st, app);
    let lifted = st.lift_loose_bvars(l, 0, 1);
    let want_app = st.app(b0, b2);
    let want = lam(&mut st, want_app);
    assert_eq!(lifted, want);
    assert_eq!(st.lower_loose_bvars(lifted, 1, 1), l);
    assert_eq!(st.lower_loose_bvars(b2, 2, 2), b0);
    assert_eq!(st.lower_loose_bvars(b0, 1, 1), b0);
    assert_eq!(st.lower_loose_bvars(b2, 1, 0), b2);
}

#[test]
#[should_panic(expected = "s must be at least d")]
fn lower_rejects_underflowing_cut() {
    let mut st = ExprStore::new();
    let e = st.bvar(0);
    let _ = st.lower_loose_bvars(e, 0, 1);
}

#[test]
fn has_loose_bvar_queries() {
    let mut st = ExprStore::new();
    let (b0, b1) = (st.bvar(0), st.bvar(1));
    let c = st.constant(&n("c"), &[]);
    assert!(st.has_loose_bvar(b0, 0) && !st.has_loose_bvar(b0, 1));
    let app = st.app(c, b1);
    assert!(st.has_loose_bvar(app, 1) && !st.has_loose_bvar(app, 0));
    // under a binder the queried index shifts: fun _ => #1 mentions loose #0
    let l = lam(&mut st, b1);
    assert!(st.has_loose_bvar(l, 0) && !st.has_loose_bvar(l, 1));
    let t = st.bvar(0);
    let lt = st.lam(b("x"), t, c); // loose #0 in the binder type
    assert!(st.has_loose_bvar(lt, 0));
    let let_e = st.let_e(b("x"), c, c, b1);
    assert!(st.has_loose_bvar(let_e, 0));
    let let_v = st.let_e(b("x"), c, b0, c);
    assert!(st.has_loose_bvar(let_v, 0));
    let pr = st.proj(&n("S"), 0, b1);
    assert!(st.has_loose_bvar(pr, 1));
    assert!(!st.has_loose_bvar(c, 0));
    // shared subterm queried at the same index twice is fine (visited set)
    let sh = st.app(app, app);
    assert!(st.has_loose_bvar(sh, 1));
}

// ---------------------------------------------------------------------------
//  Level parameters
// ---------------------------------------------------------------------------

#[test]
fn instantiate_level_params_in_exprs() {
    let mut st = ExprStore::new();
    let (u, v) = (n("u"), n("v"));
    let lu = Level::param(&u);
    let sort_u = st.sort(&lu);
    let list_u = st.constant(&n("List"), &[lu.clone(), Level::param(&v)]);
    let plain = st.constant(&n("Nat"), &[]);
    let body = st.apps(list_u, &[sort_u, plain]);
    let e = lam(&mut st, body);
    let got = st
        .instantiate_level_params(e, std::slice::from_ref(&u), &[Level::of_nat(1)])
        .unwrap();
    let want_sort = st.sort(&Level::of_nat(1));
    let want_list = st.constant(&n("List"), &[Level::of_nat(1), Level::param(&v)]);
    let want_body = st.apps(want_list, &[want_sort, plain]);
    let want = lam(&mut st, want_body);
    assert_eq!(got, want);
    // nothing to do: same handle
    assert_eq!(
        st.instantiate_level_params(plain, std::slice::from_ref(&u), &[Level::zero()])
            .unwrap(),
        plain
    );
    assert_eq!(st.instantiate_level_params(e, &[], &[]).unwrap(), e);
    assert_eq!(
        st.instantiate_level_params(e, &[n("w")], &[Level::zero()])
            .unwrap(),
        e
    );
    // reaches through Pi / Let / Proj
    let pr = st.proj(&n("S"), 0, sort_u);
    let pi = st.pi(b("x"), pr, sort_u);
    let l = st.let_e(b("x"), pi, sort_u, sort_u);
    let out = st
        .instantiate_level_params(l, &[u], &[Level::zero()])
        .unwrap();
    assert!(!st.has_level_param(out));
}

// ---------------------------------------------------------------------------
//  abstract / instantiate round trips
// ---------------------------------------------------------------------------

#[test]
fn abstract_fvars_numbering() {
    let mut st = ExprStore::new();
    let (ix, iy) = (st.fresh_fvar_id(), st.fresh_fvar_id());
    let (x, y) = (st.fvar(ix), st.fvar(iy));
    let app = st.app(x, y);
    // the LAST variable is the innermost: #0
    let got = st.abstract_fvars(app, &[ix, iy]);
    let (b0, b1) = (st.bvar(0), st.bvar(1));
    assert_eq!(got, st.app(b1, b0));
    // under a binder the indices are offset by the depth
    let l = lam(&mut st, app);
    let got = st.abstract_fvars(l, &[ix, iy]);
    let (b2, b1) = (st.bvar(2), st.bvar(1));
    let inner = st.app(b2, b1);
    assert_eq!(got, lam(&mut st, inner));
    // other free variables and closed terms are left alone
    let other = st.fresh_fvar_id();
    let o = st.fvar(other);
    assert_eq!(st.abstract_fvars(o, &[ix, iy]), o);
    let c = st.constant(&n("c"), &[]);
    assert_eq!(st.abstract_fvars(c, &[ix]), c);
    assert_eq!(st.abstract_fvars(app, &[]), app);
    // abstract then instantiate with the same variable
    let abs = st.abstract_fvars(app, &[ix]);
    let back = st.instantiate1(abs, x);
    assert_eq!(back, app);
}

#[test]
fn binding_an_fvar_and_applying_is_substitution() {
    // (fun x => body[x]) c  beta-reduces to body[c]: abstract then instantiate
    let mut st = ExprStore::new();
    let id = st.fresh_fvar_id();
    let x = st.fvar(id);
    let f = st.constant(&n("f"), &[]);
    let body = st.app(f, x);
    let abs = st.abstract_fvars(body, &[id]);
    let c = st.constant(&n("c"), &[]);
    assert_eq!(st.instantiate1(abs, c), st.app(f, c));
}

// ---------------------------------------------------------------------------
//  Randomised agreement with naive references
// ---------------------------------------------------------------------------

struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let x = (self.0 ^ (self.0 >> 31)).wrapping_mul(0x2545_F491_4F6C_DD1D);
        x ^ (x >> 29)
    }
    fn below(&mut self, m: u64) -> u64 {
        self.next() % m
    }
}

/// A random term. Bound variables are drawn from `depth + loose` indices, so `loose`
/// extra indices are left dangling (loose); free variables come from `fvars`.
fn gen(
    st: &mut ExprStore,
    rng: &mut Lcg,
    size: u32,
    depth: u32,
    loose: u32,
    fvars: &[FVarId],
) -> Expr {
    let leaf = size == 0 || rng.below(6) == 0;
    if leaf {
        return match rng.below(5) {
            0 if depth + loose > 0 => st.bvar(rng.below((depth + loose) as u64) as u32),
            1 if !fvars.is_empty() => st.fvar(fvars[rng.below(fvars.len() as u64) as usize]),
            2 => st.sort(&Level::of_nat(rng.below(3) as u32)),
            3 => st.nat_lit(BigInteger::from_u64(rng.below(3))),
            _ => st.constant(&n(["a", "b", "c"][rng.below(3) as usize]), &[]),
        };
    }
    let pick = rng.below(6);
    let sub = |st: &mut ExprStore, rng: &mut Lcg, d: u32| gen(st, rng, size - 1, d, loose, fvars);
    match pick {
        0 | 1 => {
            let (f, a) = (sub(st, rng, depth), sub(st, rng, depth));
            st.app(f, a)
        }
        2 => {
            let (t, bd) = (sub(st, rng, depth), sub(st, rng, depth + 1));
            st.lam(b("x"), t, bd)
        }
        3 => {
            let (t, bd) = (sub(st, rng, depth), sub(st, rng, depth + 1));
            st.pi(b("y"), t, bd)
        }
        4 => {
            let (t, v, bd) = (
                sub(st, rng, depth),
                sub(st, rng, depth),
                sub(st, rng, depth + 1),
            );
            st.let_e(b("z"), t, v, bd)
        }
        _ => {
            let x = sub(st, rng, depth);
            st.proj(&n("S"), 1, x)
        }
    }
}

/// Naive reference: add `d` to every loose index >= `s`, one node at a time.
fn ref_lift(st: &mut ExprStore, e: Expr, s: u32, d: u32, depth: u32) -> Expr {
    match st.kind(e).clone() {
        ExprKind::BVar(i) => st.bvar(if i >= s + depth { i + d } else { i }),
        ExprKind::App(f, a) => {
            let (f, a) = (ref_lift(st, f, s, d, depth), ref_lift(st, a, s, d, depth));
            st.app(f, a)
        }
        ExprKind::Lam(m, t, bd) => {
            let (t, bd) = (
                ref_lift(st, t, s, d, depth),
                ref_lift(st, bd, s, d, depth + 1),
            );
            st.lam(m.0, t, bd)
        }
        ExprKind::Pi(m, t, bd) => {
            let (t, bd) = (
                ref_lift(st, t, s, d, depth),
                ref_lift(st, bd, s, d, depth + 1),
            );
            st.pi(m.0, t, bd)
        }
        ExprKind::Let(m, t, v, bd) => {
            let (t, v) = (ref_lift(st, t, s, d, depth), ref_lift(st, v, s, d, depth));
            let bd = ref_lift(st, bd, s, d, depth + 1);
            st.let_e(m.0, t, v, bd)
        }
        ExprKind::Proj(sn, i, x) => {
            let x = ref_lift(st, x, s, d, depth);
            st.proj(&sn, i, x)
        }
        _ => e,
    }
}

/// Naive reference for `instantiate1`: replace `#depth` by `v` lifted by `depth`, and
/// shift the indices above it down by one.
fn ref_inst(st: &mut ExprStore, e: Expr, v: Expr, depth: u32) -> Expr {
    match st.kind(e).clone() {
        ExprKind::BVar(i) if i == depth => ref_lift(st, v, 0, depth, 0),
        ExprKind::BVar(i) if i > depth => st.bvar(i - 1),
        ExprKind::App(f, a) => {
            let (f, a) = (ref_inst(st, f, v, depth), ref_inst(st, a, v, depth));
            st.app(f, a)
        }
        ExprKind::Lam(m, t, bd) => {
            let (t, bd) = (ref_inst(st, t, v, depth), ref_inst(st, bd, v, depth + 1));
            st.lam(m.0, t, bd)
        }
        ExprKind::Pi(m, t, bd) => {
            let (t, bd) = (ref_inst(st, t, v, depth), ref_inst(st, bd, v, depth + 1));
            st.pi(m.0, t, bd)
        }
        ExprKind::Let(m, t, val, bd) => {
            let (t, val) = (ref_inst(st, t, v, depth), ref_inst(st, val, v, depth));
            let bd = ref_inst(st, bd, v, depth + 1);
            st.let_e(m.0, t, val, bd)
        }
        ExprKind::Proj(sn, i, x) => {
            let x = ref_inst(st, x, v, depth);
            st.proj(&sn, i, x)
        }
        _ => e,
    }
}

#[test]
fn lift_lower_and_instantiate_agree_with_references() {
    let mut st = ExprStore::new();
    let mut rng = Lcg(2024);
    let mut nontrivial = 0;
    for _ in 0..1500 {
        let e = gen(&mut st, &mut rng, 5, 0, 3, &[]);
        let (s, d) = (rng.below(3) as u32, 1 + rng.below(3) as u32);
        let lifted = st.lift_loose_bvars(e, s, d);
        assert_eq!(lifted, ref_lift(&mut st, e, s, d, 0));
        if lifted != e {
            nontrivial += 1;
        }
        // lower is the inverse of lift when cut at s + d
        assert_eq!(st.lower_loose_bvars(lifted, s + d, d), e);
        let v = gen(&mut st, &mut rng, 3, 0, 2, &[]);
        let inst = st.instantiate1(e, v);
        assert_eq!(inst, ref_inst(&mut st, e, v, 0));
    }
    assert!(
        nontrivial > 300,
        "generator produced too few terms with loose variables"
    );
}

#[test]
fn simultaneous_instantiate_equals_sequential_for_closed_values() {
    let mut st = ExprStore::new();
    let mut rng = Lcg(77);
    for _ in 0..800 {
        let e = gen(&mut st, &mut rng, 5, 0, 2, &[]);
        let v0 = gen(&mut st, &mut rng, 2, 0, 0, &[]); // closed
        let v1 = gen(&mut st, &mut rng, 2, 0, 0, &[]); // closed
        let simultaneous = st.instantiate(e, &[v0, v1]);
        // #0 := v0 first, then the old #1 (now #0) := v1
        let step = st.instantiate1(e, v0);
        let sequential = st.instantiate1(step, v1);
        assert_eq!(simultaneous, sequential);
        assert_eq!(st.instantiate_rev(e, &[v1, v0]), simultaneous);
    }
}

#[test]
fn abstract_then_instantiate_rev_is_identity() {
    let mut st = ExprStore::new();
    let mut rng = Lcg(31337);
    let ids: Vec<FVarId> = (0..3).map(|_| st.fresh_fvar_id()).collect();
    let fvs: Vec<Expr> = ids.iter().map(|&i| st.fvar(i)).collect();
    let mut with_fvars = 0;
    for _ in 0..1500 {
        // closed except for free variables: no loose bvars
        let e = gen(&mut st, &mut rng, 5, 0, 0, &ids);
        assert!(!st.has_loose_bvars(e));
        let abs = st.abstract_fvars(e, &ids);
        if st.has_fvar(e) {
            with_fvars += 1;
            assert!(
                st.has_loose_bvars(abs),
                "abstracting must introduce bound variables"
            );
            assert!(!st.has_fvar(abs));
            assert!(st.loose_bvar_range(abs) <= 3);
        }
        assert_eq!(st.instantiate_rev(abs, &fvs), e, "round trip failed");
        let rev: Vec<Expr> = fvs.iter().rev().copied().collect();
        assert_eq!(st.instantiate(abs, &rev), e);
        // abstracting a subset only: the others stay free
        let abs_one = st.abstract_fvars(e, &ids[..1]);
        assert_eq!(st.instantiate1(abs_one, fvs[0]), e);
    }
    assert!(with_fvars > 400);
}

#[test]
fn instantiating_with_open_values_never_captures() {
    // Close a term over one fvar, then instantiate it with a value that itself has a
    // loose bvar, placed under extra binders: the result must equal wrapping first and
    // substituting later, which is the definition of capture-avoiding substitution.
    let mut st = ExprStore::new();
    let mut rng = Lcg(4242);
    let id = st.fresh_fvar_id();
    for _ in 0..800 {
        let e = gen(&mut st, &mut rng, 5, 0, 0, &[id]);
        let abs = st.abstract_fvars(e, &[id]);
        let v = gen(&mut st, &mut rng, 3, 0, 2, &[]); // loose variables: the outer context
        let direct = st.instantiate1(abs, v);
        let reference = ref_inst(&mut st, abs, v, 0);
        assert_eq!(direct, reference);
    }
}
