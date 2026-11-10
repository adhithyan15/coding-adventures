//! VER04j sections 2.5, 3.5 and 10.3: the meaning of the fixed-width and
//! bitwise operations, held against native Rust.
//!
//! Three layers are compared:
//!
//! 1. native Rust (`wrapping_*`, `checked_*`, `rotate_*`, `count_ones`,
//!    `leading_zeros`, `to_be_bytes`, ...) against `fixed.rs`, the model that
//!    the checker, the ground evaluator and the generator tests share: *every*
//!    value of `U8`/`I8` (all pairs, every shift count `0..=300`) and a large
//!    fixed-seed sweep of the edge domain plus random pairs for `U16 U32 U64
//!    I16 I32 I64`;
//! 2. `fixed.rs` against the ground evaluator (`codegen::Evaluator`), over the
//!    same `U8`/`I8` domain and a sample of the wide ones;
//! 3. `fixed.rs` against the generated model code, in `tests/wx_evidence.rs`.
//!
//! The mutant evaluators of section 10.3 (non-wrapping `U8`, saturating add,
//! arithmetic `shr`, masked shift count, `popcount` of the sign-extended word,
//! floor-based `rem_trunc`) are reproduced below and each must be *detected*
//! by the same sweep, so the suite is known to be capable of failing.

use std::collections::HashMap;

use coding_adventures_warrant_grammar::codegen::{
    CodegenLimits, DiagnosticCode as CgCode, EvalValue, Evaluator,
};
use coding_adventures_warrant_grammar::fixed::{self, FixedTy};
use coding_adventures_warrant_grammar::layout::{capture_layout, LayoutLimits};
use coding_adventures_warrant_grammar::parser::{
    parse_document, BinaryOp, Declaration, Expression, ParserLimits, WarrantFile,
};
use coding_adventures_warrant_grammar::source::{SourceDocument, SourceLimits};

struct Xs(u64);
impl Xs {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn value(&mut self, t: FixedTy) -> i128 {
        if self.next().is_multiple_of(4) {
            let e = fixed::edge_domain(t);
            e[(self.next() % e.len() as u64) as usize]
        } else {
            t.canon(u128::from(self.next()))
        }
    }
}

/// The values a sweep of `t` visits: all of an 8-bit type, else the edge set
/// plus `extra` seeded draws.
fn values(t: FixedTy, extra: usize, rng: &mut Xs) -> Vec<i128> {
    let mut v = if t.bits() == 8 {
        fixed::full_domain(t)
    } else {
        fixed::edge_domain(t)
    };
    v.extend((0..extra).map(|_| rng.value(t)));
    v
}

/// Operand pairs: every pair of an 8-bit type, else `E x E` (thinned to keep
/// the debug-build run short) plus `random` seeded pairs.
fn pairs(t: FixedTy, random: usize, rng: &mut Xs) -> Vec<(i128, i128)> {
    if t.bits() == 8 {
        let all = fixed::full_domain(t);
        return all
            .iter()
            .flat_map(|a| all.iter().map(move |b| (*a, *b)))
            .collect();
    }
    let e = fixed::edge_domain(t);
    let step = if t.bits() == 16 { 2 } else { 3 };
    let mut p = Vec::new();
    for a in e.iter().step_by(step) {
        for b in e.iter().step_by(step) {
            p.push((*a, *b));
        }
    }
    p.extend((0..random).map(|_| (rng.value(t), rng.value(t))));
    p
}

fn counts(t: FixedTy) -> Vec<u128> {
    if t.bits() == 8 {
        (0..=300).collect()
    } else {
        (0..=300).chain([65_535, 65_536, 1 << 40]).collect()
    }
}

// ---------------------------------------------------------------------------
// Layer 1: native Rust <-> the model
// ---------------------------------------------------------------------------

macro_rules! native_agrees {
    ($name:ident, $t:ty, $u:ty, $ft:expr, $random:expr) => {
        #[test]
        fn $name() {
            let ft: FixedTy = $ft;
            let bits = <$t>::BITS;
            let mut rng = Xs(0x2545_f491_4f6c_dd1d ^ u64::from(bits));
            for (a, b) in pairs(ft, $random, &mut rng) {
                let (x, y) = (a as $t, b as $t);
                assert_eq!(
                    fixed::add(ft, a, b),
                    x.wrapping_add(y) as i128,
                    "{ft:?} {a}+{b}"
                );
                assert_eq!(
                    fixed::sub(ft, a, b),
                    x.wrapping_sub(y) as i128,
                    "{ft:?} {a}-{b}"
                );
                assert_eq!(
                    fixed::mul(ft, a, b),
                    x.wrapping_mul(y) as i128,
                    "{ft:?} {a}*{b}"
                );
                assert_eq!(fixed::bit_and(ft, a, b), (x & y) as i128);
                assert_eq!(fixed::bit_or(ft, a, b), (x | y) as i128);
                assert_eq!(fixed::bit_xor(ft, a, b), (x ^ y) as i128);
                assert_eq!(
                    fixed::checked('+', ft, a, b),
                    x.checked_add(y).map(|v| v as i128)
                );
                assert_eq!(
                    fixed::checked('-', ft, a, b),
                    x.checked_sub(y).map(|v| v as i128)
                );
                assert_eq!(
                    fixed::checked('*', ft, a, b),
                    x.checked_mul(y).map(|v| v as i128)
                );
                let (div, rem) = if ft.signed() {
                    (fixed::div_trunc(ft, a, b), fixed::rem_trunc(ft, a, b))
                } else {
                    (fixed::div_unsigned(a, b), fixed::rem_unsigned(a, b))
                };
                assert_eq!(div, x.checked_div(y).map(|v| v as i128), "{ft:?} {a}/{b}");
                assert_eq!(rem, x.checked_rem(y).map(|v| v as i128), "{ft:?} {a}%{b}");
                assert_eq!(a < b, x < y, "signed compare is by signed value");
            }
            for a in values(ft, 2000, &mut rng) {
                let x = a as $t;
                assert_eq!(fixed::neg(ft, a), x.wrapping_neg() as i128);
                assert_eq!(fixed::bit_not(ft, a), (!x) as i128);
                assert_eq!(fixed::popcount(ft, a), u128::from(x.count_ones()));
                assert_eq!(fixed::clz(ft, a), u128::from(x.leading_zeros()));
                assert_eq!(fixed::ctz(ft, a), u128::from(x.trailing_zeros()));
                assert_eq!(fixed::be_bytes(ft, a), x.to_be_bytes().to_vec());
                assert_eq!(fixed::le_bytes(ft, a), x.to_le_bytes().to_vec());
                assert_eq!(fixed::from_be(ft, &x.to_be_bytes()), Some(a));
                assert_eq!(fixed::from_le(ft, &x.to_le_bytes()), Some(a));
                for n in counts(ft) {
                    let k = u32::try_from(n).unwrap_or(u32::MAX);
                    // checked_shl/shr return None for k >= bits: normalised to 0.
                    assert_eq!(
                        fixed::shl(ft, a, n),
                        x.checked_shl(k).unwrap_or(0) as i128,
                        "{ft:?} shl {a} {n}"
                    );
                    assert_eq!(
                        fixed::shr(ft, a, n),
                        (x as $u).checked_shr(k).unwrap_or(0) as $t as i128,
                        "{ft:?} shr {a} {n}"
                    );
                    if ft.signed() {
                        let fill: $t = if (x as i128) < 0 { !0 } else { 0 };
                        assert_eq!(
                            fixed::sar(ft, a, n),
                            x.checked_shr(k).unwrap_or(fill) as i128,
                            "{ft:?} sar {a} {n}"
                        );
                    }
                    let r = (n % u128::from(bits)) as u32;
                    assert_eq!(
                        fixed::rotl(ft, a, n),
                        x.rotate_left(r) as i128,
                        "{ft:?} rotl {a} {n}"
                    );
                    assert_eq!(
                        fixed::rotr(ft, a, n),
                        x.rotate_right(r) as i128,
                        "{ft:?} rotr {a} {n}"
                    );
                    let bit = n < u128::from(bits) && ((x as $u) >> n) & 1 == 1;
                    assert_eq!(fixed::bit(ft, a, n), bit);
                }
            }
            // Conversions from every width: wrap is `as`, try is `try_from`.
            for v in [
                i128::from(i64::MIN),
                -300,
                -129,
                -128,
                -1,
                0,
                1,
                127,
                128,
                255,
                256,
                32_767,
                32_768,
                65_535,
                65_536,
                i128::from(i32::MIN),
                i128::from(i32::MAX),
                i128::from(u32::MAX),
                i128::from(u32::MAX) + 1,
                i128::from(i64::MAX),
                i128::from(u64::MAX),
            ] {
                assert_eq!(ft.wrap(v), (v as $t) as i128, "{ft:?} wrap {v}");
                assert_eq!(
                    ft.try_from_int(v),
                    <$t>::try_from(v).ok().map(|x| x as i128),
                    "{ft:?} try {v}"
                );
            }
            assert_eq!(ft.min(), <$t>::MIN as i128);
            assert_eq!(ft.max(), <$t>::MAX as i128);
        }
    };
}

native_agrees!(
    u8_model_equals_native_rust_on_every_pair,
    u8,
    u8,
    FixedTy::U8,
    0
);
native_agrees!(
    i8_model_equals_native_rust_on_every_pair,
    i8,
    u8,
    FixedTy::I8,
    0
);
native_agrees!(u16_model_equals_native_rust, u16, u16, FixedTy::U16, 60_000);
native_agrees!(i16_model_equals_native_rust, i16, u16, FixedTy::I16, 60_000);
native_agrees!(
    u32_model_equals_native_rust,
    u32,
    u32,
    FixedTy::U32,
    120_000
);
native_agrees!(
    i32_model_equals_native_rust,
    i32,
    u32,
    FixedTy::I32,
    120_000
);
native_agrees!(
    u64_model_equals_native_rust,
    u64,
    u64,
    FixedTy::U64,
    120_000
);
native_agrees!(
    i64_model_equals_native_rust,
    i64,
    u64,
    FixedTy::I64,
    120_000
);

#[test]
fn widening_matches_native_lossless_conversion() {
    // `widen<D>(x: S)` is admitted exactly when every value of S is a value of
    // D; the extremes decide it, and native `From` agrees on which pairs exist.
    macro_rules! lossless {
        ($($s:ty => $d:ty),* $(,)?) => {$(
            let s = FixedTy::from_suffix(stringify!($s)).unwrap();
            let d = FixedTy::from_suffix(stringify!($d)).unwrap();
            assert!(s.widens_to(d), "{s:?} -> {d:?} has a native From");
            for v in [s.min(), s.max(), 0, 1] {
                assert_eq!(<$d>::from(v as $s) as i128, v);
            }
        )*};
    }
    lossless!(u8 => u16, u8 => u32, u8 => u64, u8 => i16, u8 => i32, u8 => i64, u16 => u32,
        u16 => u64, u16 => i32, u16 => i64, u32 => u64, u32 => i64, i8 => i16, i8 => i32,
        i8 => i64, i16 => i32, i16 => i64, i32 => i64);
    let native_from = [
        (
            "u8",
            ["u8", "u16", "u32", "u64", "i16", "i32", "i64"].as_slice(),
        ),
        ("u16", ["u16", "u32", "u64", "i32", "i64"].as_slice()),
        ("u32", ["u32", "u64", "i64"].as_slice()),
        ("u64", ["u64"].as_slice()),
        ("i8", ["i8", "i16", "i32", "i64"].as_slice()),
        ("i16", ["i16", "i32", "i64"].as_slice()),
        ("i32", ["i32", "i64"].as_slice()),
        ("i64", ["i64"].as_slice()),
    ];
    for s in FixedTy::ALL {
        for d in FixedTy::ALL {
            let table = native_from
                .iter()
                .find(|(n, _)| *n == s.suffix())
                .unwrap()
                .1;
            assert_eq!(
                s.widens_to(d),
                table.contains(&d.suffix()),
                "{s:?} -> {d:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The mutants of section 10.3 must be detected
// ---------------------------------------------------------------------------

/// Does `mutant` disagree with the model anywhere in the sweep of `t`?
fn detected(
    t: FixedTy,
    mutant: impl Fn(FixedTy, i128, i128) -> i128,
    model: impl Fn(FixedTy, i128, i128) -> i128,
) -> bool {
    let mut rng = Xs(77);
    pairs(t, 2000, &mut rng)
        .into_iter()
        .any(|(a, b)| mutant(t, a, b) != model(t, a, b))
}

#[test]
fn every_mutant_evaluator_of_section_10_3_is_killed() {
    // Non-wrapping U8 (the actual v0 defect): exact result, no reduction.
    assert!(detected(FixedTy::U8, |_, a, b| a + b, fixed::add));
    assert!(detected(FixedTy::U8, |_, a, b| a * b, fixed::mul));
    assert!(detected(FixedTy::U8, |_, a, b| a - b, fixed::sub));
    // Saturating add.
    assert!(detected(
        FixedTy::U32,
        |t, a, b| (a + b).clamp(t.min(), t.max()),
        fixed::add
    ));
    assert!(detected(
        FixedTy::I8,
        |t, a, b| (a + b).clamp(t.min(), t.max()),
        fixed::add
    ));
    // Arithmetic shr where the model says logical (visible on negatives).
    assert!(detected(
        FixedTy::I16,
        |t, a, b| fixed::sar(t, a, (b & 31) as u128),
        |t, a, b| fixed::shr(t, a, (b & 31) as u128)
    ));
    // The shift count masked modulo the width (x86): visible once n >= w.
    for t in FixedTy::ALL {
        let w = u128::from(t.bits());
        let found = values(t, 50, &mut Xs(3))
            .into_iter()
            .any(|a| (0..=300u128).any(|n| fixed::shl(t, a, n % w) != fixed::shl(t, a, n)));
        assert!(found, "{t:?}: masked-count shl not detected");
    }
    // popcount of the sign-extended host word instead of the width-w pattern.
    assert!(detected(
        FixedTy::I8,
        |_, a, _| a.count_ones() as i128 & 0xff, // i128 sign extension
        |t, a, _| fixed::popcount(t, a) as i128
    ));
    // Floor-based rem_trunc (Python/Ruby) where the model truncates.
    assert!(detected(
        FixedTy::I8,
        |_, a, b| if b == 0 { 0 } else { a.rem_euclid(b) },
        |t, a, b| fixed::rem_trunc(t, a, b).unwrap_or(0)
    ));
    // ... and the host-shift-by-width mutant: a count >= w left unguarded.
    assert!(detected(
        FixedTy::U8,
        |t, a, b| t.canon(t.pattern(a).wrapping_shl((b & 255) as u32)),
        |t, a, b| fixed::shl(t, a, (b & 255) as u128)
    ));
    // Sanity: the model itself is not flagged by an identical copy.
    assert!(!detected(FixedTy::U8, fixed::add, fixed::add));
}

// ---------------------------------------------------------------------------
// Layer 2: the ground evaluator
// ---------------------------------------------------------------------------

fn with_doc<R>(src: &str, f: impl FnOnce(&WarrantFile) -> R) -> R {
    let doc = SourceDocument::admit(src.as_bytes(), SourceLimits::default()).expect("admit");
    let layout = capture_layout(&doc, LayoutLimits::default()).expect("layout");
    let file = parse_document(&layout, &doc, ParserLimits::default()).expect("parse");
    f(&file)
}

/// The left-hand side expressions of a document of `example`s `<expr> == <expr>`.
fn lhs_expressions<'a>(file: &'a WarrantFile<'a>) -> Vec<&'a Expression<'a>> {
    file.declarations
        .iter()
        .filter_map(|d| match d {
            Declaration::Example(e) => match &e.body {
                Expression::Binary {
                    op: BinaryOp::Eq,
                    left,
                    ..
                } => Some(&**left),
                _ => None,
            },
            _ => None,
        })
        .collect()
}

fn doc_of(exprs: &[String]) -> String {
    let mut s = String::from("warrant sweep v1\n");
    for (i, e) in exprs.iter().enumerate() {
        s.push_str(&format!("  example e{i}:\n    {e} == {e}\n"));
    }
    s
}

fn opt(v: Option<i128>, t: FixedTy) -> EvalValue {
    EvalValue::Option(v.map(|x| Box::new(EvalValue::from_fixed(t, x))))
}

#[test]
fn the_ground_evaluator_agrees_with_the_model_on_every_operation() {
    let mut rng = Xs(0xdead_beef_cafe_f00d);
    for t in FixedTy::ALL {
        let n = t.name();
        let signed = t.signed();
        let mut exprs: Vec<String> = [
            "a + b",
            "a - b",
            "a * b",
            "bit_and(a, b)",
            "bit_or(a, b)",
            "bit_xor(a, b)",
            "bit_not(a)",
            "shl(a, n)",
            "shr(a, n)",
            "rotl(a, n)",
            "rotr(a, n)",
            "popcount(a)",
            "clz(a)",
            "ctz(a)",
            "bit(a, n)",
            "be_bytes(a)",
            "le_bytes(a)",
            "checked_add(a, b)",
            "checked_sub(a, b)",
            "checked_mul(a, b)",
            "to_int(a)",
            "-a",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if !signed {
            exprs.remove(exprs.len() - 1); // unary minus is signed-only
        }
        exprs.push(
            if signed {
                "div_trunc(a, b)"
            } else {
                "div(a, b)"
            }
            .into(),
        );
        exprs.push(
            if signed {
                "rem_trunc(a, b)"
            } else {
                "rem(a, b)"
            }
            .into(),
        );
        if signed {
            exprs.push("sar(a, n)".into());
        }
        exprs.extend([
            format!("from_be<{n}>(be_bytes(a))"),
            format!("from_le<{n}>(le_bytes(a))"),
            format!("wrap<{n}>(to_int(a))"),
            format!("try<{n}>(to_int(a))"),
            format!("max_value<{n}>()"),
            format!("min_value<{n}>()"),
            format!("unwrap_or(try<{n}>(to_int(a)), b)"),
        ]);
        let src = doc_of(&exprs);
        with_doc(&src, |file| {
            let lhs = lhs_expressions(file);
            assert_eq!(lhs.len(), exprs.len());
            let mut ev = Evaluator::new(file, CodegenLimits::default());
            let ctx = |a: i128, b: i128, c: u128| -> HashMap<&'static str, EvalValue> {
                HashMap::from([
                    ("a", EvalValue::from_fixed(t, a)),
                    ("b", EvalValue::from_fixed(t, b)),
                    ("n", EvalValue::Nat(c as u64)),
                ])
            };
            let bool_ = EvalValue::Bool;
            let nat = |v: u128| EvalValue::Nat(v as u64);
            let fx = |v: i128| EvalValue::from_fixed(t, v);
            let mut eval = |i: usize, env: &HashMap<&'static str, EvalValue>| -> EvalValue {
                ev.reset_work();
                ev.eval(lhs[i], env)
                    .unwrap_or_else(|e| panic!("{}: {e}", exprs[i]))
            };
            let index = |text: &str| exprs.iter().position(|e| e == text).unwrap();
            let sample: Vec<(i128, i128)> = if t.bits() == 8 {
                pairs(t, 0, &mut rng)
            } else {
                pairs(t, 300, &mut rng).into_iter().step_by(37).collect()
            };
            for (a, b) in sample {
                let env = ctx(a, b, 0);
                assert_eq!(
                    eval(index("a + b"), &env),
                    fx(fixed::add(t, a, b)),
                    "{n} {a}+{b}"
                );
                assert_eq!(
                    eval(index("a - b"), &env),
                    fx(fixed::sub(t, a, b)),
                    "{n} {a}-{b}"
                );
                assert_eq!(
                    eval(index("a * b"), &env),
                    fx(fixed::mul(t, a, b)),
                    "{n} {a}*{b}"
                );
                assert_eq!(
                    eval(index("bit_and(a, b)"), &env),
                    fx(fixed::bit_and(t, a, b))
                );
                assert_eq!(
                    eval(index("bit_or(a, b)"), &env),
                    fx(fixed::bit_or(t, a, b))
                );
                assert_eq!(
                    eval(index("bit_xor(a, b)"), &env),
                    fx(fixed::bit_xor(t, a, b))
                );
                assert_eq!(
                    eval(index("checked_add(a, b)"), &env),
                    opt(fixed::checked('+', t, a, b), t)
                );
                assert_eq!(
                    eval(index("checked_sub(a, b)"), &env),
                    opt(fixed::checked('-', t, a, b), t)
                );
                assert_eq!(
                    eval(index("checked_mul(a, b)"), &env),
                    opt(fixed::checked('*', t, a, b), t)
                );
                let (dv, rm, dname, rname) = if signed {
                    (
                        fixed::div_trunc(t, a, b),
                        fixed::rem_trunc(t, a, b),
                        "div_trunc(a, b)",
                        "rem_trunc(a, b)",
                    )
                } else {
                    (
                        fixed::div_unsigned(a, b),
                        fixed::rem_unsigned(a, b),
                        "div(a, b)",
                        "rem(a, b)",
                    )
                };
                assert_eq!(eval(index(dname), &env), opt(dv, t), "{n} {a}/{b}");
                assert_eq!(eval(index(rname), &env), opt(rm, t), "{n} {a}%{b}");
            }
            for a in values(t, 100, &mut rng) {
                let env = ctx(a, 0, 0);
                assert_eq!(eval(index("bit_not(a)"), &env), fx(fixed::bit_not(t, a)));
                if signed {
                    assert_eq!(eval(index("-a"), &env), fx(fixed::neg(t, a)), "{n} -{a}");
                }
                assert_eq!(eval(index("popcount(a)"), &env), nat(fixed::popcount(t, a)));
                assert_eq!(eval(index("clz(a)"), &env), nat(fixed::clz(t, a)));
                assert_eq!(eval(index("ctz(a)"), &env), nat(fixed::ctz(t, a)));
                assert_eq!(eval(index("to_int(a)"), &env), EvalValue::Int(a));
                assert_eq!(
                    eval(index("be_bytes(a)"), &env),
                    EvalValue::Bytes(fixed::be_bytes(t, a))
                );
                assert_eq!(
                    eval(index("le_bytes(a)"), &env),
                    EvalValue::Bytes(fixed::le_bytes(t, a))
                );
                assert_eq!(
                    eval(index(&format!("from_be<{n}>(be_bytes(a))")), &env),
                    fx(a)
                );
                assert_eq!(
                    eval(index(&format!("from_le<{n}>(le_bytes(a))")), &env),
                    fx(a)
                );
                assert_eq!(eval(index(&format!("wrap<{n}>(to_int(a))")), &env), fx(a));
                assert_eq!(
                    eval(index(&format!("try<{n}>(to_int(a))")), &env),
                    opt(Some(a), t)
                );
                assert_eq!(
                    eval(index(&format!("unwrap_or(try<{n}>(to_int(a)), b)")), &env),
                    fx(a)
                );
                let cs: Vec<u128> = if t.bits() == 8 {
                    (0..=300).collect()
                } else {
                    (0..=70).chain([127, 128, 129, 255, 256, 65_536]).collect()
                };
                for c in cs {
                    let env = ctx(a, 0, c);
                    assert_eq!(
                        eval(index("shl(a, n)"), &env),
                        fx(fixed::shl(t, a, c)),
                        "{n} shl {a} {c}"
                    );
                    assert_eq!(
                        eval(index("shr(a, n)"), &env),
                        fx(fixed::shr(t, a, c)),
                        "{n} shr {a} {c}"
                    );
                    assert_eq!(
                        eval(index("rotl(a, n)"), &env),
                        fx(fixed::rotl(t, a, c)),
                        "{n} rotl {a} {c}"
                    );
                    assert_eq!(
                        eval(index("rotr(a, n)"), &env),
                        fx(fixed::rotr(t, a, c)),
                        "{n} rotr {a} {c}"
                    );
                    assert_eq!(eval(index("bit(a, n)"), &env), bool_(fixed::bit(t, a, c)));
                    if signed {
                        assert_eq!(
                            eval(index("sar(a, n)"), &env),
                            fx(fixed::sar(t, a, c)),
                            "{n} sar {a} {c}"
                        );
                    }
                }
            }
            let env = ctx(0, 0, 0);
            assert_eq!(eval(index(&format!("max_value<{n}>()")), &env), fx(t.max()));
            assert_eq!(eval(index(&format!("min_value<{n}>()")), &env), fx(t.min()));
        });
    }
}

#[test]
fn the_ground_evaluator_follows_the_specification_tables() {
    // (expression, expected) with the literal forms of section 3.2's table.
    let rows: &[(&str, EvalValue)] = &[
        ("shl(0x81u8, 1)", EvalValue::U8(0x02)),
        ("shl(1u8, 7)", EvalValue::U8(128)),
        ("shl(1u8, 8)", EvalValue::U8(0)),
        ("shr(0x81u8, 1)", EvalValue::U8(0x40)),
        ("sar(-128i8, 1)", EvalValue::Fixed(FixedTy::I8, -64)),
        ("sar(-128i8, 8)", EvalValue::Fixed(FixedTy::I8, -1)),
        ("sar(100i8, 9)", EvalValue::Fixed(FixedTy::I8, 0)),
        ("shr(-1i8, 1)", EvalValue::Fixed(FixedTy::I8, 127)),
        ("rotl(0x81u8, 1)", EvalValue::U8(0x03)),
        ("rotl(0x81u8, 9)", EvalValue::U8(0x03)),
        ("rotr(0x03u8, 1)", EvalValue::U8(0x81)),
        ("popcount(0xffu8)", EvalValue::Nat(8)),
        ("popcount(-1i8)", EvalValue::Nat(8)),
        ("clz(0u8)", EvalValue::Nat(8)),
        ("ctz(0x80u8)", EvalValue::Nat(7)),
        ("bit(5u8, 8)", EvalValue::Bool(false)),
        ("bit_not(0u8)", EvalValue::U8(255)),
        ("bit_not(0i8)", EvalValue::Fixed(FixedTy::I8, -1)),
        (
            "be_bytes(0x01020304u32)",
            EvalValue::Bytes(vec![1, 2, 3, 4]),
        ),
        ("le_bytes(-2i16)", EvalValue::Bytes(vec![0xfe, 0xff])),
        ("wrap<I8>(255u8)", EvalValue::Fixed(FixedTy::I8, -1)),
        ("wrap<U8>(-1)", EvalValue::U8(255)),
        ("max_value<U32>() + 1u32", EvalValue::Fixed(FixedTy::U32, 0)),
        ("-min_value<I8>()", EvalValue::Fixed(FixedTy::I8, -128)),
        (
            "-min_value<I64>()",
            EvalValue::Fixed(FixedTy::I64, i64::MIN as i128),
        ),
        ("widen<U32>(200u8)", EvalValue::Fixed(FixedTy::U32, 200)),
        (
            "to_int(18446744073709551615u64)",
            EvalValue::Int(u64::MAX as i128),
        ),
        ("to_nat(255u8)", EvalValue::Nat(255)),
        ("200u8 + 100u8", EvalValue::U8(44)),
        ("0u8 - 1u8", EvalValue::U8(255)),
        (
            "18446744073709551615u64 * 18446744073709551615u64",
            EvalValue::U64(1),
        ),
        ("-128i8", EvalValue::Fixed(FixedTy::I8, -128)),
        ("div_trunc(-128i8, -1i8)", EvalValue::Option(None)),
        ("rem_trunc(-128i8, -1i8)", EvalValue::Option(None)),
        (
            "div_trunc(-7i8, 2i8)",
            EvalValue::Option(Some(Box::new(EvalValue::Fixed(FixedTy::I8, -3)))),
        ),
        (
            "rem(7, -2)",
            EvalValue::Option(Some(Box::new(EvalValue::Int(-1)))),
        ),
        (
            "div(7, -2)",
            EvalValue::Option(Some(Box::new(EvalValue::Int(-4)))),
        ),
        (
            "rem(-7, 2)",
            EvalValue::Option(Some(Box::new(EvalValue::Int(1)))),
        ),
        ("try<U8>(256)", EvalValue::Option(None)),
        (
            "try<I8>(-128)",
            EvalValue::Option(Some(Box::new(EvalValue::Fixed(FixedTy::I8, -128)))),
        ),
        ("unwrap_or(try<U8>(300), 9u8)", EvalValue::U8(9)),
        (
            "from_be<U16>(b\"0102\")",
            EvalValue::Fixed(FixedTy::U16, 0x0102),
        ),
        (
            "from_le<U16>(b\"0102\")",
            EvalValue::Fixed(FixedTy::U16, 0x0201),
        ),
    ];
    let exprs: Vec<String> = rows.iter().map(|(e, _)| e.to_string()).collect();
    with_doc(&doc_of(&exprs), |file| {
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for (lhs, (text, want)) in lhs_expressions(file).into_iter().zip(rows) {
            ev.reset_work();
            let got = ev
                .eval(lhs, &HashMap::new())
                .unwrap_or_else(|e| panic!("{text}: {e}"));
            assert_eq!(&got, want, "{text}");
        }
    });
}

#[test]
fn numeric_equality_in_a_v1_document_compares_by_value() {
    let src = "warrant t v1\n  example e:\n    popcount(0xffu8) == 8\n  example f:\n    div(6, 3) == Some(2)\n  example g:\n    popcount(0xffu8) != 9\n";
    with_doc(src, |file| {
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for d in &file.declarations {
            if let Declaration::Example(e) = d {
                ev.reset_work();
                assert_eq!(
                    ev.eval(&e.body, &HashMap::new()).unwrap(),
                    EvalValue::Bool(true),
                    "{}",
                    e.name
                );
            }
        }
    });
    // In a v0 document `==` stays structural (a Nat is not an Int literal).
    let src = "warrant t\n  example e:\n    len(b\"00\") == 1\n";
    with_doc(src, |file| {
        let Declaration::Example(e) = &file.declarations[0] else {
            panic!()
        };
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        assert_eq!(
            ev.eval(&e.body, &HashMap::new()).unwrap(),
            EvalValue::Bool(false)
        );
    });
}

#[test]
fn int_overflow_is_w0703_in_v1_and_never_a_wrapped_value() {
    let big = "to_int(18446744073709551615u64) * to_int(18446744073709551615u64)";
    let src = format!("warrant t v1\n  example e:\n    {big} == {big}\n");
    with_doc(&src, |file| {
        let Declaration::Example(e) = &file.declarations[0] else {
            panic!()
        };
        let Expression::Binary { left, .. } = &e.body else {
            panic!()
        };
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let err = ev.eval(left, &HashMap::new()).unwrap_err();
        assert_eq!(err.code, CgCode::W0703, "{err}");
    });
    // v0 keeps W0702 for the same event (an Int product beyond i128).
    let v0 = "warrant t\n  example e:\n    170141183460469231731687303715884105727 + 1 == 0\n";
    let _ = v0; // literals above u64 are not expressible in v0; see tests/codegen.rs
}

#[test]
fn work_is_charged_per_node_and_shifts_charge_by_width() {
    // `shl(a, n)` on a u64 charges ceil(64/8) = 8 units in total for the call
    // node (one of them by `eval` itself), on a u8 just 1.
    let work = |src: &str| -> usize {
        with_doc(&doc_of(&[src.to_string()]), |file| {
            let lhs = lhs_expressions(file)[0];
            // Run with a budget of exactly `k` units and find the smallest k
            // that completes: the work the expression costs.
            for k in 1..200 {
                let limits = CodegenLimits {
                    max_work: k,
                    ..CodegenLimits::default()
                };
                let mut ev = Evaluator::new(file, limits);
                if ev.eval(lhs, &HashMap::new()).is_ok() {
                    return k;
                }
            }
            panic!("never completes");
        })
    };
    // call node + two literal operands = 3 base units.
    assert_eq!(work("shl(1u8, 1)"), 3);
    assert_eq!(work("shl(1u64, 1)"), 3 + 7);
    assert_eq!(work("popcount(1u32)"), 2 + 3);
    assert_eq!(work("1u8 + 1u8"), 3);
    assert_eq!(work("bit_xor(1u64, 2u64)"), 3);
}

#[test]
fn ill_typed_calls_that_bypass_the_checker_are_reported_not_panicked() {
    // The evaluator is also reachable on parsed-but-unchecked text; every
    // shape error is a W0702 diagnostic naming the built-in.
    let rows = [
        ("widen<U8>(300)", "widen"),
        ("widen(5u8)", "static type"),
        ("widen<Q>(5u8)", "static type"),
        ("to_nat(-1i8)", "to_nat"),
        ("to_nat(5i8)", "to_nat"),
        ("div(-4i8, 2i8)", "signed operands"),
        ("unwrap_or(5u8, 1u8)", "Option operand"),
        ("sar(5u8, 1)", "sar of an unsigned value"),
        ("shl(5u8, -1)", "count must be a natural number"),
        ("from_be<U8>(5u8)", "Bytes operand"),
        ("from_be<U32>(b\"0102\")", "wrong byte width"),
        ("shl(true, 1)", "fixed-width operand"),
        ("wrap<U8>(true)", "integer operand"),
        ("checked_add(1, 2)", "fixed-width operand"),
    ];
    let exprs: Vec<String> = rows.iter().map(|(e, _)| e.to_string()).collect();
    with_doc(&doc_of(&exprs), |file| {
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        for (lhs, (text, needle)) in lhs_expressions(file).into_iter().zip(rows) {
            ev.reset_work();
            let err = ev.eval(lhs, &HashMap::new()).expect_err(text);
            assert_eq!(err.code, CgCode::W0702, "{text}");
            assert!(format!("{err:?}").contains(needle), "{text}: {err:?}");
        }
    });
}

#[test]
fn value_equality_recurses_through_option_result_and_list() {
    let rows = [
        ("[1, 2] == [1, 2]", true),
        ("[1, 2] == [1, 3]", false),
        ("[1] == [1, 2]", false),
        ("Some(1) == Some(1)", true),
        ("Some(1) == Some(2)", false),
        ("Some(1) == None", false),
        ("None == Some(1)", false),
        ("Ok(3) == Ok(3)", true),
        ("Ok(3) == Ok(4)", false),
        ("true == true", true),
        ("true != false", true),
        ("b\"01\" == b\"01\"", true),
        ("[Some(1)] == [Some(1)]", true),
        ("popcount(0xffu8) == len(b\"0000000000000000\") + 0", true),
    ];
    let mut src = String::from("warrant t v1\n");
    for (i, (e, _)) in rows.iter().enumerate() {
        src.push_str(&format!("  example e{i}:\n    {e}\n"));
    }
    with_doc(&src, |file| {
        let mut ev = Evaluator::new(file, CodegenLimits::default());
        let decls: Vec<_> = file
            .declarations
            .iter()
            .filter_map(|d| match d {
                Declaration::Example(e) => Some(e),
                _ => None,
            })
            .collect();
        for (ex, (text, want)) in decls.iter().zip(rows) {
            ev.reset_work();
            assert_eq!(
                ev.eval(&ex.body, &HashMap::new()).unwrap(),
                EvalValue::Bool(want),
                "{text}"
            );
        }
    });
}

#[test]
fn fixed_values_convert_and_encode_like_the_other_integers() {
    let v = EvalValue::Fixed(FixedTy::I16, -5);
    assert_eq!(v.as_i128(), Some(-5));
    assert_eq!(v.as_u64(), None, "a negative value is not a natural number");
    assert_eq!(v.as_fixed(), Some((FixedTy::I16, -5)));
    assert_eq!(v.encode_tfv(), "I-5");
    let u = EvalValue::Fixed(FixedTy::U32, 7);
    assert_eq!(u.as_u64(), Some(7));
    assert_eq!(u.encode_tfv(), "I7");
    assert_eq!(EvalValue::U8(9).as_fixed(), Some((FixedTy::U8, 9)));
    assert_eq!(EvalValue::U64(9).as_fixed(), Some((FixedTy::U64, 9)));
    assert_eq!(EvalValue::Bool(true).as_fixed(), None);
    // from_fixed keeps U8 and U64 in their v0 variants.
    assert_eq!(EvalValue::from_fixed(FixedTy::U8, 3), EvalValue::U8(3));
    assert_eq!(EvalValue::from_fixed(FixedTy::U64, 3), EvalValue::U64(3));
    assert_eq!(
        EvalValue::from_fixed(FixedTy::I8, -3),
        EvalValue::Fixed(FixedTy::I8, -3)
    );
}
