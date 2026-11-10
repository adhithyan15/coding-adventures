//! VER04j WX-B / WX-C evidence generation, tested the way VER04h tests the v0
//! generator: by *compiling and running* what it emits.  The generated harness
//! (`tests/generated/wx_evidence.rs`, checked in and drift-tested by
//! `tests/wx_regen.rs`) is included as a module and run against a tiny subject
//! (`wx_subject`) whose exports are the native Rust operations the
//! specification says Warrant must agree with.

use std::collections::BTreeSet;

use coding_adventures_test_bootstrap::run::Options;
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::fixed::{self, FixedTy};

#[allow(dead_code, unused_variables, unused_parens, clippy::all)]
mod harness {
    /// The subject under test: the real native operations.  The generated code
    /// reaches it through `use wx_subject::*;`.
    pub mod wx_subject {
        pub fn add32(a: u32, b: u32) -> u32 {
            a.wrapping_add(b)
        }
        pub fn sub8(a: u8, b: u8) -> u8 {
            a.wrapping_sub(b)
        }
        pub fn mul16(a: u16, b: u16) -> u16 {
            a.wrapping_mul(b)
        }
        pub fn lt_i32(a: i32, b: i32) -> bool {
            a < b
        }
        pub fn try_u8(x: i16) -> Option<u8> {
            u8::try_from(x).ok()
        }
        pub fn shl16(a: u16, n: u32) -> u16 {
            a.checked_shl(n).unwrap_or(0)
        }
        pub fn sar8(a: i8, n: u32) -> i8 {
            a.checked_shr(n).unwrap_or(if a < 0 { -1 } else { 0 })
        }
        pub fn rotl32(a: u32, n: u32) -> u32 {
            a.rotate_left(n % 32)
        }
        pub fn rotr16(a: u16, n: u32) -> u16 {
            a.rotate_right(n % 16)
        }
        pub fn popcnt8(a: i8) -> u32 {
            a.count_ones()
        }
        pub fn narrow8(x: i32) -> i8 {
            i8::try_from(x).expect("out of range")
        }
    }
    include!("generated/wx_evidence.rs");
}
use harness::{Fault, Ty, F, V};

fn f(t: FixedTy) -> F {
    match t {
        FixedTy::U8 => F::U8,
        FixedTy::U16 => F::U16,
        FixedTy::U32 => F::U32,
        FixedTy::U64 => F::U64,
        FixedTy::I8 => F::I8,
        FixedTy::I16 => F::I16,
        FixedTy::I32 => F::I32,
        FixedTy::I64 => F::I64,
    }
}

fn is_fail(v: &Verdict, id: &str) -> bool {
    matches!(v, Verdict::Fail { failure_id, .. } if *failure_id == id)
}

/// A deterministic xorshift stream (the tests must not depend on `rand`).
struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// A value of `t`: 3/4 uniform, 1/4 from the edge set.
    fn value(&mut self, t: FixedTy) -> i128 {
        if self.next().is_multiple_of(4) {
            let e = fixed::edge_domain(t);
            e[(self.next() % e.len() as u64) as usize]
        } else {
            t.canon(u128::from(self.next()))
        }
    }
}

// ---------------------------------------------------------------------------
// The whole generated suite, including every negative control
// ---------------------------------------------------------------------------

#[test]
fn the_full_generated_suite_passes_and_every_fault_is_caught() {
    // The subject panics on purpose (narrow8 out of range); the generated
    // harness silences the hook in `run_main`, so do the same here.
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| ()));
    let suite = harness::build_suite(Fault::None);
    let mut tap = Vec::new();
    let (summary, outcomes) = suite
        .run_recorded(&Options::default(), &mut tap)
        .expect("run");
    std::panic::set_hook(previous);
    let tap = String::from_utf8(tap).unwrap();
    assert!(
        summary.failed == 0 && !summary.bailed,
        "failures:\n{}",
        tap.lines()
            .filter(|l| l.starts_with("not ok"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    // Examples, three suites per claim, and one negative control per
    // (suite kind, operation, fault).
    let negctl: Vec<_> = outcomes
        .iter()
        .filter(|o| o.id.contains("/negctl/"))
        .collect();
    assert!(negctl.iter().all(|o| o.passed()));
    // 16 distinct faults are bound; each is attacked by up to four suite kinds.
    let faults: BTreeSet<&str> = negctl
        .iter()
        .map(|o| o.id.rsplit('/').next().unwrap())
        .collect();
    assert_eq!(faults.len(), 16, "{faults:?}");

    // The manifest of that very run records the v1 additions.
    let out = std::env::temp_dir().join(format!("wx_manifest_{}.json", std::process::id()));
    std::env::set_var("WARRANT_EVIDENCE_OUT", &out);
    harness::write_manifest_for(&outcomes);
    std::env::remove_var("WARRANT_EVIDENCE_OUT");
    let manifest = std::fs::read_to_string(&out).expect("manifest written");
    let _ = std::fs::remove_file(&out);
    assert!(
        manifest.contains("\"warrant_version\": \"v1\""),
        "{manifest}"
    );
    // mul16: a U16 x U16 claim at the reference budget demotes both binders,
    // leftmost first (65536 * 65536 -> 281 * 65536 -> 281 * 281).
    assert!(
        manifest.contains("a: U16 edge(281); b: U16 edge(281)"),
        "{manifest}"
    );
    // shl16: only the U16 binder is demoted; the shift count is a count domain.
    assert!(
        manifest.contains("a: U16 edge(281); n: Nat count(73)"),
        "{manifest}"
    );
    // sub8: U8 binders are never demoted (65536 cases fit the budget).
    assert!(
        manifest.contains("a: U8 full(256); b: U8 full(256)"),
        "{manifest}"
    );
    let flat = manifest.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flat.contains("\"demoted\": [ \"a\", \"b\" ]"), "{manifest}");
    assert!(flat.contains("\"demoted\": [ \"a\" ]"), "{manifest}");
    assert!(flat.contains("\"demoted\": [],"), "{manifest}");
}

#[test]
fn each_fault_of_sections_2_7_and_3_7_is_caught_with_a_registered_failure_id() {
    use Fault::*;
    // (fault, the exhaustive suite that must catch it)
    let cases: Vec<(Fault, Verdict)> = vec![
        (
            SaturateAdd,
            harness::exh_add32_is_wrapping_add(SaturateAdd).0,
        ),
        (
            TrapAsZeroAdd,
            harness::exh_add32_is_wrapping_add(TrapAsZeroAdd).0,
        ),
        (
            NarrowWidth,
            harness::exh_add32_is_wrapping_add(NarrowWidth).0,
        ),
        (
            SaturateSub,
            harness::exh_sub8_is_wrapping_sub(SaturateSub).0,
        ),
        (
            TrapAsZeroSub,
            harness::exh_sub8_is_wrapping_sub(TrapAsZeroSub).0,
        ),
        (
            SaturateMul,
            harness::exh_mul16_is_wrapping_mul(SaturateMul).0,
        ),
        (
            TrapAsZeroMul,
            harness::exh_mul16_is_wrapping_mul(TrapAsZeroMul).0,
        ),
        (
            SignConfuse,
            harness::exh_lt_i32_is_signed_order(SignConfuse).0,
        ),
        (OffByOneRange, harness::exh_try_u8_is_try(OffByOneRange).0),
        (ShiftMask, harness::exh_shl16_is_total_shift(ShiftMask).0),
        (ShiftUb, harness::exh_shl16_is_total_shift(ShiftUb).0),
        (SarAsShr, harness::exh_sar8_is_arithmetic(SarAsShr).0),
        (
            RotateAsShiftLeft,
            harness::exh_rotl32_is_rotate(RotateAsShiftLeft).0,
        ),
        (
            RotateAsShiftRight,
            harness::exh_rotr16_is_rotate(RotateAsShiftRight).0,
        ),
        (
            PopcountOff,
            harness::exh_popcnt8_counts_the_pattern(PopcountOff).0,
        ),
        (
            MissingError,
            harness::exh_narrow8_is_try_into_i8(MissingError).0,
        ),
    ];
    for (fault, verdict) in &cases {
        assert!(
            is_fail(verdict, "property-counterexample"),
            "{fault:?}: {verdict:?}"
        );
    }
    // The property and fuzz suites catch the common ones too (random draws
    // reach the boundary through the edge-biased generator).
    assert!(is_fail(
        &harness::prop_add32_is_wrapping_add(SaturateAdd).0,
        "property-counterexample"
    ));
    assert!(is_fail(
        &harness::prop_sub8_is_wrapping_sub(SaturateSub).0,
        "property-counterexample"
    ));
    assert!(is_fail(
        &harness::prop_rotl32_is_rotate(RotateAsShiftLeft).0,
        "property-counterexample"
    ));
    assert!(is_fail(
        &harness::fuz_sub8_is_wrapping_sub(SaturateSub, 400).0,
        "fuzz-crash"
    ));
    // Without a fault every suite passes.
    for v in [
        harness::exh_add32_is_wrapping_add(None).0,
        harness::exh_mul16_is_wrapping_mul(None).0,
        harness::exh_shl16_is_total_shift(None).0,
        harness::exh_narrow8_is_try_into_i8(None).0,
        harness::prop_try_u8_is_try(None).0,
        harness::fuz_popcnt8_counts_the_pattern(None, 200).0,
    ] {
        assert_eq!(v, Verdict::Pass);
    }
    assert!(!harness::example_add32_wraps(SaturateAdd));
    assert!(!harness::example_try_u8_rejects_256(OffByOneRange));
    assert!(!harness::example_popcnt8_of_minus_one(PopcountOff));
}

#[test]
fn fault_names_round_trip() {
    for name in [
        "saturate-add",
        "saturate-sub",
        "saturate-mul",
        "trap-as-zero-add",
        "trap-as-zero-sub",
        "trap-as-zero-mul",
        "narrow-width",
        "sign-confuse",
        "off-by-one-range",
        "shift-mask",
        "shift-ub",
        "rotate-as-shift-left",
        "rotate-as-shift-right",
        "sar-as-shr",
        "popcount-off",
        "missing-error",
    ] {
        let fault = Fault::parse(name).unwrap_or_else(|| panic!("{name} is not registered"));
        assert_eq!(fault.name(), name);
    }
    assert_eq!(
        Fault::parse("saturate"),
        None,
        "operator-less names are not faults"
    );
}

// ---------------------------------------------------------------------------
// Domain descriptors (section 2.6, 3.6)
// ---------------------------------------------------------------------------

/// The edge set of section 2.6 computed straight from the formula, with
/// native casts, independently of `fixed::edge_domain` and the prelude.
fn expected_edge(t: FixedTy) -> Vec<i128> {
    let w = t.bits();
    let umax: u128 = if w == 64 {
        u64::MAX as u128
    } else {
        (1u128 << w) - 1
    };
    let mut u: Vec<u128> = (0..=255u128).filter(|v| *v <= umax).collect();
    for i in 8..w {
        u.push(1u128 << i);
    }
    u.push(umax);
    for i in 0..w {
        u.push(umax ^ (1u128 << i));
    }
    if !t.signed() {
        return u.into_iter().map(|x| x as i128).collect();
    }
    let signed = |x: u128| -> i128 {
        match w {
            8 => x as u8 as i8 as i128,
            16 => x as u16 as i16 as i128,
            32 => x as u32 as i32 as i128,
            _ => x as u64 as i64 as i128,
        }
    };
    let mut s: BTreeSet<i128> = u.into_iter().map(signed).collect();
    for k in 1..=255i128 {
        if -k >= t.min() {
            s.insert(-k);
        }
    }
    s.into_iter().collect()
}

#[test]
fn domain_sizes_and_ends_follow_section_2_6() {
    // (type, size of the default domain, first, last)
    let full = |t: FixedTy| (1usize << t.bits(), t.min(), t.max());
    for t in [FixedTy::U8, FixedTy::I8, FixedTy::U16, FixedTy::I16] {
        let d = harness::domain(Ty::Fx(f(t)));
        let (n, first, last) = full(t);
        assert_eq!(d.len(), n, "{t:?}");
        assert_eq!(d.first(), Some(&V::Fixed(f(t), first)));
        assert_eq!(d.last(), Some(&V::Fixed(f(t), last)));
        assert_eq!(harness::domain_size(Ty::Fx(f(t))), Some(n as u64));
    }
    // The edge sets: sizes fixed by the specification, contents by formula.
    for (t, n) in [
        (FixedTy::U16, 281),
        (FixedTy::U32, 313),
        (FixedTy::U64, 377),
        (FixedTy::I16, 527),
        (FixedTy::I32, 559),
        (FixedTy::I64, 623),
    ] {
        let want = expected_edge(t);
        assert_eq!(want.len(), n, "{t:?} formula");
        assert_eq!(fixed::edge_domain(t), want, "{t:?} fixed::edge_domain");
        // Wide types default to the edge set; U16/I16 reach it by demotion.
        let ty = if t.bits() <= 16 {
            Ty::FxEdge(f(t))
        } else {
            Ty::Fx(f(t))
        };
        let got: Vec<i128> = harness::domain(ty)
            .into_iter()
            .map(|v| match v {
                V::Fixed(_, x) => x,
                other => panic!("{other:?}"),
            })
            .collect();
        assert_eq!(got, want, "{t:?} prelude domain");
        assert_eq!(harness::domain_size(ty), Some(n as u64));
    }
    // v0's U64 edge set is exactly the unsigned formula for width 64.
    assert_eq!(fixed::edge_domain(FixedTy::U64).len(), 256 + 56 + 1 + 64);
    // Signed sets ascend; unsigned keep the formula's order.
    let i32e = fixed::edge_domain(FixedTy::I32);
    assert!(i32e.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(i32e.first(), Some(&(i32::MIN as i128)));
    assert_eq!(i32e.last(), Some(&(i32::MAX as i128)));
    // The count-aware Nat domain adds 65 127 128 129 255 256 65535 65536.
    let counts: Vec<i128> = harness::domain(Ty::NatCount)
        .into_iter()
        .map(|v| match v {
            V::Int(x) => x,
            other => panic!("{other:?}"),
        })
        .collect();
    assert_eq!(counts.len(), 73);
    assert_eq!(&counts[..65], (0..=64i128).collect::<Vec<_>>().as_slice());
    assert_eq!(&counts[65..], &[65, 127, 128, 129, 255, 256, 65535, 65536]);
    assert_eq!(harness::domain_size(Ty::Nat), Some(65));
}

#[test]
fn edge_sets_contain_the_boundaries_the_faults_need() {
    for t in FixedTy::ALL {
        let e: BTreeSet<i128> = if t.bits() == 8 {
            fixed::full_domain(t).into_iter().collect()
        } else {
            fixed::edge_domain(t).into_iter().collect()
        };
        for must in [t.max(), t.min(), 0, 1] {
            assert!(e.contains(&must), "{t:?} lacks {must}");
        }
        if t.signed() {
            assert!(e.contains(&-1), "{t:?} lacks -1");
        } else {
            assert!(e.contains(&(t.max() - 1)), "{t:?} lacks max - 1");
        }
    }
    // `try<T>(x)` of the next-wider type: max + 1 and min - 1 of every target
    // that has a wider source in the same family are edge values.
    for (target, source) in [
        (FixedTy::U8, FixedTy::I32),
        (FixedTy::U16, FixedTy::I32),
        (FixedTy::U32, FixedTy::I64),
        (FixedTy::I8, FixedTy::I16),
        (FixedTy::I16, FixedTy::I32),
        (FixedTy::I32, FixedTy::I64),
    ] {
        let e: BTreeSet<i128> = fixed::edge_domain(source).into_iter().collect();
        assert!(
            e.contains(&(target.max() + 1)),
            "{source:?} lacks {target:?}.max + 1"
        );
        assert!(
            e.contains(&(target.min() - 1)),
            "{source:?} lacks {target:?}.min - 1"
        );
    }
}

#[test]
fn the_prelude_budget_check_is_checked_arithmetic_for_v1_types() {
    use harness::{domain_size, within_budget};
    // 65536 * 65536 is far over the reference budget; 281 * 281 is not.
    assert!(!within_budget(&[Ty::Fx(F::U16), Ty::Fx(F::U16)]));
    assert!(within_budget(&[Ty::FxEdge(F::U16), Ty::FxEdge(F::U16)]));
    assert!(within_budget(&[Ty::Fx(F::U8), Ty::Fx(F::U8)]));
    assert!(!within_budget(&[
        Ty::Fx(F::U8),
        Ty::Fx(F::U8),
        Ty::Fx(F::U8)
    ]));
    assert!(within_budget(&[Ty::Fx(F::U32), Ty::Fx(F::U32)]));
    assert_eq!(domain_size(Ty::U8), None, "v1 documents bind U8 as Ty::Fx");
    for ty in [
        Ty::Fx(F::U8),
        Ty::Fx(F::I16),
        Ty::FxEdge(F::I16),
        Ty::Fx(F::U32),
        Ty::Fx(F::I64),
        Ty::NatCount,
        Ty::Nat,
        Ty::Bool,
    ] {
        assert_eq!(
            domain_size(ty),
            Some(harness::domain(ty).len() as u64),
            "{ty:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Differential tests of the generated model code (the prelude's `V`)
// ---------------------------------------------------------------------------

fn fx(t: FixedTy, v: i128) -> V {
    V::Fixed(f(t), v)
}

fn int(v: i128) -> V {
    V::Int(v)
}

fn unwrap_fx(v: V) -> i128 {
    match v {
        V::Fixed(_, x) => x,
        V::Int(x) => x,
        other => panic!("{other:?}"),
    }
}

/// Every value a type's sweep visits: all of an 8-bit type, the edge set
/// otherwise, plus `random` extra draws.
fn sweep(t: FixedTy, random: usize, rng: &mut Xs) -> Vec<i128> {
    let mut v = if t.bits() == 8 {
        fixed::full_domain(t)
    } else {
        fixed::edge_domain(t)
    };
    v.extend((0..random).map(|_| rng.value(t)));
    v
}

#[test]
fn prelude_arithmetic_and_bitwise_agree_with_the_model_everywhere_it_is_swept() {
    let mut rng = Xs(0x9e37_79b9_7f4a_7c15);
    for t in FixedTy::ALL {
        let vals = sweep(t, 300, &mut rng);
        let pairs: Vec<(i128, i128)> = if t.bits() == 8 {
            vals.iter()
                .flat_map(|a| vals.iter().map(move |b| (*a, *b)))
                .collect()
        } else {
            let mut p: Vec<(i128, i128)> =
                (0..40_000).map(|_| (rng.value(t), rng.value(t))).collect();
            let e = fixed::edge_domain(t);
            for a in e.iter().step_by(7) {
                for b in e.iter().step_by(5) {
                    p.push((*a, *b));
                }
            }
            p
        };
        for (a, b) in pairs {
            let (va, vb) = (fx(t, a), fx(t, b));
            assert_eq!(
                unwrap_fx(V::arith(&va, &vb, '+')),
                fixed::add(t, a, b),
                "{t:?} {a}+{b}"
            );
            assert_eq!(
                unwrap_fx(V::arith(&va, &vb, '-')),
                fixed::sub(t, a, b),
                "{t:?} {a}-{b}"
            );
            assert_eq!(
                unwrap_fx(V::arith(&va, &vb, '*')),
                fixed::mul(t, a, b),
                "{t:?} {a}*{b}"
            );
            assert_eq!(unwrap_fx(V::bitop('&', &va, &vb)), fixed::bit_and(t, a, b));
            assert_eq!(unwrap_fx(V::bitop('|', &va, &vb)), fixed::bit_or(t, a, b));
            assert_eq!(unwrap_fx(V::bitop('^', &va, &vb)), fixed::bit_xor(t, a, b));
            for (op, want) in [
                ('+', fixed::checked('+', t, a, b)),
                ('-', fixed::checked('-', t, a, b)),
                ('*', fixed::checked('*', t, a, b)),
            ] {
                let got = V::checked(op, &va, &vb);
                let expect = V::Opt(want.map(|x| Box::new(fx(t, x))));
                assert_eq!(got, expect, "{t:?} checked {op}");
            }
            let cmp = V::compare(&va, &vb, "<");
            assert_eq!(cmp, V::Bool(a < b));
            if t.signed() {
                let q = V::divrem_trunc(true, &va, &vb);
                assert_eq!(
                    q,
                    V::Opt(fixed::div_trunc(t, a, b).map(|x| Box::new(fx(t, x))))
                );
                let r = V::divrem_trunc(false, &va, &vb);
                assert_eq!(
                    r,
                    V::Opt(fixed::rem_trunc(t, a, b).map(|x| Box::new(fx(t, x))))
                );
            } else {
                let q = V::divrem(true, &va, &vb);
                assert_eq!(
                    q,
                    V::Opt(fixed::div_unsigned(a, b).map(|x| Box::new(fx(t, x))))
                );
                let r = V::divrem(false, &va, &vb);
                assert_eq!(
                    r,
                    V::Opt(fixed::rem_unsigned(a, b).map(|x| Box::new(fx(t, x))))
                );
            }
        }
        for a in &vals {
            let va = fx(t, *a);
            assert_eq!(unwrap_fx(V::bit_not(&va)), fixed::bit_not(t, *a));
            if t.signed() {
                assert_eq!(unwrap_fx(V::neg(&va)), fixed::neg(t, *a));
            }
            assert_eq!(
                unwrap_fx(V::bitcount("popcount", &va)),
                fixed::popcount(t, *a) as i128
            );
            assert_eq!(
                unwrap_fx(V::bitcount("clz", &va)),
                fixed::clz(t, *a) as i128
            );
            assert_eq!(
                unwrap_fx(V::bitcount("ctz", &va)),
                fixed::ctz(t, *a) as i128
            );
            let counts: Vec<i128> = if t.bits() == 8 {
                (0..=300).collect()
            } else {
                (0..=130).chain([255, 256, 65535, 65536]).collect()
            };
            for n in counts {
                let vn = int(n);
                let cnt = n as u128;
                assert_eq!(
                    unwrap_fx(V::shift("shl", &va, &vn)),
                    fixed::shl(t, *a, cnt),
                    "{t:?} shl {a} {n}"
                );
                assert_eq!(
                    unwrap_fx(V::shift("shr", &va, &vn)),
                    fixed::shr(t, *a, cnt),
                    "{t:?} shr {a} {n}"
                );
                assert_eq!(
                    unwrap_fx(V::shift("rotl", &va, &vn)),
                    fixed::rotl(t, *a, cnt),
                    "{t:?} rotl {a} {n}"
                );
                assert_eq!(
                    unwrap_fx(V::shift("rotr", &va, &vn)),
                    fixed::rotr(t, *a, cnt),
                    "{t:?} rotr {a} {n}"
                );
                if t.signed() {
                    assert_eq!(
                        unwrap_fx(V::shift("sar", &va, &vn)),
                        fixed::sar(t, *a, cnt),
                        "{t:?} sar {a} {n}"
                    );
                }
                assert_eq!(V::bit(&va, &vn), V::Bool(fixed::bit(t, *a, cnt)));
            }
            for (big, bytes) in [
                (true, fixed::be_bytes(t, *a)),
                (false, fixed::le_bytes(t, *a)),
            ] {
                let got = V::endian_bytes(big, &va);
                assert_eq!(got, V::Bytes(bytes.clone()));
                assert_eq!(V::from_endian(big, f(t), &got), va);
            }
        }
    }
}

#[test]
fn prelude_conversions_agree_with_the_model_over_the_whole_matrix() {
    let mut rng = Xs(0x0123_4567_89ab_cdef);
    for src in FixedTy::ALL {
        let vals = sweep(src, 200, &mut rng);
        for dst in FixedTy::ALL {
            for v in &vals {
                let x = fx(src, *v);
                assert_eq!(
                    V::wrap(f(dst), &x),
                    fx(dst, dst.wrap(*v)),
                    "wrap {src:?}->{dst:?}"
                );
                assert_eq!(
                    V::try_(f(dst), &x),
                    V::Opt(dst.try_from_int(*v).map(|y| Box::new(fx(dst, y)))),
                    "try {src:?}->{dst:?}"
                );
                if src.widens_to(dst) {
                    assert_eq!(V::widen(f(dst), &x), fx(dst, *v), "widen {src:?}->{dst:?}");
                }
            }
        }
        // Int and Nat sources.
        for v in [
            -300i128,
            -129,
            -128,
            -1,
            0,
            1,
            127,
            128,
            255,
            256,
            65535,
            65536,
            1 << 40,
        ] {
            for dst in FixedTy::ALL {
                assert_eq!(V::wrap(f(dst), &int(v)), fx(dst, dst.wrap(v)));
                assert_eq!(
                    V::try_(f(dst), &int(v)),
                    V::Opt(dst.try_from_int(v).map(|y| Box::new(fx(dst, y))))
                );
            }
        }
        assert_eq!(V::to_int(&fx(src, src.max())), int(src.max()));
        assert_eq!(V::max_value(f(src)), fx(src, src.max()));
        assert_eq!(V::min_value(f(src)), fx(src, src.min()));
    }
    // The boundary case of section 2.5: `to_int(U64) * to_int(U64)` can leave
    // i128; the model panics (incomplete checking), it never wraps.
    let m = u64::MAX as i128;
    assert_eq!(V::arith(&int(m), &int(1 << 62), '*'), int(m << 62));
    assert!(std::panic::catch_unwind(|| V::arith(&int(m), &int(m), '*')).is_err());
    // -min_value<I64>() wraps to min_value<I64>().
    assert_eq!(V::neg(&V::min_value(F::I64)), V::min_value(F::I64));
    // unwrap_or and Int floor division/modulo.
    assert_eq!(
        V::unwrap_or(&V::Opt(None), &fx(FixedTy::U8, 9)),
        fx(FixedTy::U8, 9)
    );
    assert_eq!(
        V::unwrap_or(&V::try_(F::U8, &int(5)), &fx(FixedTy::U8, 9)),
        fx(FixedTy::U8, 5)
    );
    assert_eq!(
        V::divrem(true, &int(-7), &int(2)),
        V::Opt(Some(Box::new(int(-4))))
    );
    assert_eq!(
        V::divrem(true, &int(7), &int(-2)),
        V::Opt(Some(Box::new(int(-4))))
    );
    assert_eq!(
        V::divrem(false, &int(-7), &int(2)),
        V::Opt(Some(Box::new(int(1))))
    );
    assert_eq!(
        V::divrem(false, &int(7), &int(-2)),
        V::Opt(Some(Box::new(int(-1))))
    );
    assert_eq!(V::divrem(false, &int(7), &int(0)), V::Opt(None));
}

#[test]
fn u8_arithmetic_in_the_generated_model_wraps_wd_013() {
    // The v0 generator evaluated U8 as an exact integer; in a v1 document the
    // same `+ - *` wrap (VER04 section 3, VER04j section 1.4).
    let u8v = |n: i128| fx(FixedTy::U8, n);
    assert_eq!(V::arith(&u8v(255), &u8v(1), '+'), u8v(0));
    assert_eq!(V::arith(&u8v(0), &u8v(1), '-'), u8v(255));
    assert_eq!(V::arith(&u8v(200), &u8v(2), '*'), u8v(144));
    assert!(harness::example_sub8_wraps_below_zero(Fault::None));
}
