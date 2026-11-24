//! Tests for names and universe levels.
//!
//! The strongest test here is semantic: levels denote natural numbers once the
//! parameters are assigned, so we evaluate levels over a grid of assignments and check
//! that normalisation never changes the value, that `is_equiv` never claims equality of
//! different functions, and that `is_geq` never claims an inequality that fails.

use lean_kernel::{levels_equiv, Level, LevelKind, Name, NamePart};
use std::collections::HashMap;

fn n(s: &str) -> Name {
    Name::from_dotted(s)
}
fn p(s: &str) -> Level {
    Level::param(&n(s))
}
fn z() -> Level {
    Level::zero()
}
fn s(l: &Level) -> Level {
    Level::succ(l)
}
fn mx(a: &Level, b: &Level) -> Level {
    Level::max(a, b)
}
fn imx(a: &Level, b: &Level) -> Level {
    Level::imax(a, b)
}
fn k(v: u32) -> Level {
    Level::of_nat(v)
}

// ---------------------------------------------------------------------------
//  Names
// ---------------------------------------------------------------------------

#[test]
fn names_structure_and_display() {
    let a = Name::anonymous();
    assert!(a.is_anonymous());
    assert_eq!(a.to_string(), "[anonymous]");
    assert!(a.parts().is_empty());
    let nat_add = n("Nat.add");
    assert_eq!(nat_add.to_string(), "Nat.add");
    assert_eq!(format!("{nat_add:?}"), "`Nat.add");
    assert!(!nat_add.is_anonymous());
    assert_eq!(
        nat_add.parts(),
        vec![NamePart::Str("Nat".into()), NamePart::Str("add".into())]
    );
    assert!(n("").is_anonymous());
}

#[test]
fn numeric_and_string_components_differ() {
    let base = n("_uniq");
    let num = Name::num(&base, 3);
    let st = Name::str(&base, "3");
    assert_ne!(num, st);
    assert_eq!(num.to_string(), "_uniq.3");
    assert_eq!(st.to_string(), "_uniq.3"); // they print alike, which is why equality is structural
    assert_eq!(num.parts().last(), Some(&NamePart::Num(3)));
    assert_eq!(num, Name::num(&n("_uniq"), 3));
}

#[test]
fn names_order_and_hash_structurally() {
    use std::collections::HashSet;
    let set: HashSet<Name> = [n("a.b"), n("a.b"), n("a.c")].into_iter().collect();
    assert_eq!(set.len(), 2);
    assert!(n("a") < n("a.b"));
    assert!(n("a.b") < n("a.c"));
}

// ---------------------------------------------------------------------------
//  Basic structure
// ---------------------------------------------------------------------------

#[test]
fn offsets_and_predicates() {
    let u = p("u");
    assert_eq!(k(3).to_offset(), (z(), 3));
    assert_eq!(Level::offset(&u, 2).to_offset(), (u.clone(), 2));
    assert!(k(0).is_zero() && !k(1).is_zero());
    assert!(k(4).is_explicit() && !s(&u).is_explicit());
    assert!(u.has_param() && !k(2).has_param());
    assert!(s(&u).is_never_zero());
    assert!(!u.is_never_zero() && !z().is_never_zero());
    assert!(mx(&u, &k(1)).is_never_zero() && !mx(&u, &p("v")).is_never_zero());
    assert!(imx(&u, &k(1)).is_never_zero() && !imx(&k(1), &u).is_never_zero());
    assert!(matches!(mx(&u, &u).kind(), LevelKind::Max(..)));
}

#[test]
fn structural_equality_hash_and_order() {
    let a = mx(&p("u"), &s(&p("v")));
    let b = mx(&p("u"), &s(&p("v")));
    assert_eq!(a, b);
    assert_eq!(a.hash_value(), b.hash_value());
    assert_ne!(a, mx(&s(&p("v")), &p("u"))); // structural, so order matters here
    assert_ne!(a.hash_value(), imx(&p("u"), &s(&p("v"))).hash_value());
    assert!(z() < s(&z()));
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    assert_eq!(a.partial_cmp(&b), Some(std::cmp::Ordering::Equal));
    let m: HashMap<Level, u8> = [(a.clone(), 1)].into_iter().collect();
    assert_eq!(m[&b], 1);
}

#[test]
fn display_and_debug() {
    assert_eq!(k(2).to_string(), "2");
    assert_eq!(p("u").to_string(), "u");
    assert_eq!(s(&p("u")).to_string(), "u+1");
    assert_eq!(mx(&p("u"), &k(1)).to_string(), "(max u 1)");
    assert_eq!(imx(&p("u"), &p("v")).to_string(), "(imax u v)");
    assert_eq!(s(&mx(&p("u"), &p("v"))).to_string(), "((max u v))+1");
    assert_eq!(format!("{:?}", mx(&p("u"), &k(1))), "(max u 1)");
    assert_eq!(format!("{:?}", z().kind()), "Zero");
    assert_eq!(format!("{:?}", s(&z()).kind()), "Succ(0)");
    assert_eq!(format!("{:?}", mx(&z(), &z()).kind()), "Max(0, 0)");
    assert_eq!(format!("{:?}", imx(&z(), &z()).kind()), "IMax(0, 0)");
    assert_eq!(format!("{:?}", p("u").kind()), "Param(u)");
}

// ---------------------------------------------------------------------------
//  Instantiation
// ---------------------------------------------------------------------------

#[test]
fn instantiate_params() {
    let (u, v) = (n("u"), n("v"));
    let l = mx(&p("u"), &imx(&s(&p("v")), &p("w")));
    let r = l.instantiate(&[u.clone(), v.clone()], &[k(1), p("x")]);
    assert_eq!(r, mx(&k(1), &imx(&s(&p("x")), &p("w")))); // w is not listed, so it stays
                                                          // a parameter-free level is returned unchanged
    assert_eq!(k(3).instantiate(std::slice::from_ref(&u), &[k(9)]), k(3));
    // substitution is simultaneous, not sequential: u := v and v := u swaps
    let swapped = mx(&p("u"), &p("v")).instantiate(&[u.clone(), v.clone()], &[p("v"), p("u")]);
    assert_eq!(swapped, mx(&p("v"), &p("u")));
    // result is not simplified: instantiating with 0 leaves `imax u 0` as written
    assert_eq!(
        imx(&p("u"), &p("v")).instantiate(&[v], &[z()]),
        imx(&p("u"), &z())
    );
}

// ---------------------------------------------------------------------------
//  Normalisation and equivalence: the named edge cases
// ---------------------------------------------------------------------------

fn eq(a: &Level, b: &Level) -> bool {
    let r = a.is_equiv(b);
    assert_eq!(
        r,
        b.is_equiv(a),
        "is_equiv must be symmetric for {a} and {b}"
    );
    r
}

#[test]
fn imax_rules() {
    let (u, v) = (p("u"), p("v"));
    assert!(eq(&imx(&u, &z()), &z())); // imax u 0 = 0
    assert!(eq(&imx(&u, &s(&v)), &mx(&u, &s(&v)))); // imax u (succ v) = max u (succ v)
    assert!(eq(&imx(&z(), &u), &u)); // imax 0 u = u
    assert!(eq(&imx(&u, &u), &u)); // imax u u = u
    assert!(eq(&s(&imx(&u, &z())), &k(1))); // an outer succ lands on the numeral
    assert!(eq(&imx(&u, &mx(&v, &k(1))), &mx(&u, &mx(&v, &k(1)))));
    assert!(eq(&imx(&u, &imx(&v, &z())), &z())); // the inner imax collapses first
                                                 // genuinely different: imax u v is NOT max u v (v could be 0)
    assert!(!eq(&imx(&u, &v), &mx(&u, &v)));
    // an irreducible imax keeps its shape, with normalised arguments
    let n = imx(&mx(&u, &u), &mx(&v, &v)).normalize();
    assert_eq!(n, imx(&u, &v));
    assert_eq!(s(&imx(&u, &v)).normalize(), s(&imx(&u, &v)));
}

#[test]
fn max_rules() {
    let (u, v, w) = (p("u"), p("v"), p("w"));
    assert!(eq(&mx(&u, &u), &u)); // idempotent
    assert!(eq(&mx(&u, &v), &mx(&v, &u))); // commutative
    assert!(eq(&mx(&mx(&u, &v), &w), &mx(&u, &mx(&v, &w)))); // associative
    assert!(eq(&mx(&mx(&u, &v), &mx(&v, &u)), &mx(&u, &v)));
    assert!(eq(&mx(&u, &s(&s(&u))), &s(&s(&u)))); // larger offset of the same base wins
    assert!(eq(&mx(&k(1), &s(&v)), &s(&v))); // v+1 >= 1: numeral dominated
    assert!(eq(&mx(&k(0), &u), &u));
    assert!(eq(&mx(&k(2), &k(3)), &k(3)));
    assert!(eq(&s(&mx(&u, &v)), &mx(&s(&u), &s(&v)))); // succ pushes through max
    assert!(eq(&s(&mx(&k(1), &u)), &mx(&k(2), &s(&u))));
    // not equivalent
    assert!(!eq(&mx(&k(3), &u), &u)); // the numeral is NOT dominated
    assert!(!eq(&mx(&u, &v), &u));
    assert!(!eq(&mx(&k(2), &s(&u)), &s(&u))); // u+1 >= 2 fails when u = 0
    assert!(!eq(&u, &v));
    assert!(!eq(&u, &s(&u)));
    assert!(!eq(&mx(&u, &s(&v)), &mx(&s(&u), &v)));
}

#[test]
fn normalize_is_canonical_shape() {
    let (u, v) = (p("u"), p("v"));
    // sorted, duplicate-free, right-nested; numeral dropped when dominated
    let l = mx(&mx(&v, &k(1)), &mx(&u, &mx(&v, &s(&u)))).normalize();
    assert_eq!(l, mx(&s(&u), &mx(&v, &k(1))).normalize());
    assert!(l.has_param());
    assert_eq!(k(0).normalize(), z());
    assert_eq!(u.normalize(), u);
    assert_eq!(s(&u).normalize(), s(&u));
    assert_eq!(mx(&k(0), &k(0)).normalize(), z());
    // idempotence
    assert_eq!(l.normalize(), l);
}

#[test]
fn levels_equiv_lists() {
    let (u, v) = (p("u"), p("v"));
    assert!(levels_equiv(
        &[mx(&u, &v), z()],
        &[mx(&v, &u), imx(&u, &z())]
    ));
    assert!(!levels_equiv(
        std::slice::from_ref(&u),
        &[u.clone(), v.clone()]
    ));
    assert!(!levels_equiv(
        std::slice::from_ref(&u),
        std::slice::from_ref(&v)
    ));
    assert!(levels_equiv(&[], &[]));
}

// ---------------------------------------------------------------------------
//  is_geq
// ---------------------------------------------------------------------------

#[test]
fn geq_cases() {
    let (u, v) = (p("u"), p("v"));
    assert!(u.is_geq(&u));
    assert!(u.is_geq(&z()) && z().is_geq(&z()));
    assert!(s(&u).is_geq(&u) && !u.is_geq(&s(&u)));
    assert!(k(3).is_geq(&k(2)) && !k(2).is_geq(&k(3)));
    assert!(s(&u).is_geq(&k(1)) && !u.is_geq(&k(1)));
    assert!(mx(&u, &v).is_geq(&u) && mx(&u, &v).is_geq(&v));
    assert!(!u.is_geq(&mx(&u, &v)));
    assert!(mx(&u, &v).is_geq(&mx(&v, &u))); // via normalisation
    assert!(mx(&s(&u), &v).is_geq(&mx(&u, &v)));
    assert!(imx(&u, &v).is_geq(&v) && !imx(&u, &v).is_geq(&u));
    assert!(mx(&u, &v).is_geq(&imx(&u, &v))); // imax <= max
    assert!(s(&u).is_geq(&s(&u)) && s(&mx(&u, &v)).is_geq(&s(&v)));
    assert!(!u.is_geq(&v) && !s(&u).is_geq(&s(&v)));
    assert!(s(&s(&u)).is_geq(&s(&u)));
}

// ---------------------------------------------------------------------------
//  Semantic soundness over a grid of parameter assignments
// ---------------------------------------------------------------------------

fn eval(l: &Level, env: &HashMap<Name, u64>) -> u64 {
    match l.kind() {
        LevelKind::Zero => 0,
        LevelKind::Succ(a) => eval(a, env) + 1,
        LevelKind::Max(a, b) => eval(a, env).max(eval(b, env)),
        LevelKind::IMax(a, b) => {
            let bv = eval(b, env);
            if bv == 0 {
                0
            } else {
                eval(a, env).max(bv)
            }
        }
        LevelKind::Param(name) => env[name],
    }
}

fn grid() -> Vec<HashMap<Name, u64>> {
    let names = [n("u"), n("v"), n("w")];
    let mut out = Vec::new();
    for a in 0..8u64 {
        for b in 0..8u64 {
            for c in 0..8u64 {
                out.push(names.iter().cloned().zip([a, b, c]).collect());
            }
        }
    }
    out
}

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
    fn level(&mut self, depth: u32) -> Level {
        let pick = self.next() % if depth == 0 { 4 } else { 9 };
        match pick {
            0 => z(),
            1 | 2 => p(["u", "v", "w"][(self.next() % 3) as usize]),
            3 => k((self.next() % 4) as u32),
            4 | 5 => s(&self.level(depth - 1)),
            6 | 7 => mx(&self.level(depth - 1), &self.level(depth - 1)),
            _ => imx(&self.level(depth - 1), &self.level(depth - 1)),
        }
    }
}

#[test]
fn normalize_preserves_value_and_is_idempotent() {
    let envs = grid();
    let mut rng = Lcg(7);
    for _ in 0..1500 {
        let l = rng.level(4);
        let nl = l.normalize();
        for env in &envs {
            assert_eq!(eval(&l, env), eval(&nl, env), "normalising {l} gave {nl}");
        }
        assert_eq!(nl.normalize(), nl, "normalize not idempotent on {l}");
    }
}

#[test]
fn equiv_and_geq_are_sound() {
    let envs = grid();
    let mut rng = Lcg(99);
    let (mut equiv_hits, mut geq_hits) = (0, 0);
    for _ in 0..4000 {
        let (a, b) = (rng.level(3), rng.level(3));
        if a.is_equiv(&b) {
            equiv_hits += 1;
            for env in &envs {
                assert_eq!(eval(&a, env), eval(&b, env), "{a} claimed equiv to {b}");
            }
        }
        if a.is_geq(&b) {
            geq_hits += 1;
            for env in &envs {
                assert!(eval(&a, env) >= eval(&b, env), "{a} claimed >= {b}");
            }
        }
    }
    // the generator must actually exercise the positive answers, or the test is vacuous
    assert!(
        equiv_hits > 100 && geq_hits > 500,
        "{equiv_hits} {geq_hits}"
    );
}

#[test]
fn equiv_is_complete_for_max_without_imax() {
    // Without imax, `max`/`succ` levels are determined by (numeral, per-parameter offsets);
    // there the normal form must agree exactly with semantic equality on the grid.
    let envs = grid();
    let mut rng = Lcg(5);
    let gen = |rng: &mut Lcg| loop {
        let l = rng.level(3);
        if !format!("{l}").contains("imax") {
            return l;
        }
    };
    let mut agree_equal = 0;
    for _ in 0..3000 {
        let (a, b) = (gen(&mut rng), gen(&mut rng));
        let sem = envs.iter().all(|e| eval(&a, e) == eval(&b, e));
        assert_eq!(a.is_equiv(&b), sem, "{a} vs {b}");
        agree_equal += sem as u32;
    }
    assert!(agree_equal > 50);
}
