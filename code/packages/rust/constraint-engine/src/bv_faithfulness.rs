//! # Is the `Predicate` -> circuit lowering faithful? (VER10 §9, LH-006a)
//!
//! `bit-blaster`'s audit checked the circuits (`bit_blaster::bv`) against a
//! Rust reference. This file checks the layer on top: **this module's
//! lowering** of `Predicate` terms (operator names, argument order, n-ary
//! folding, indexed identifiers, literals, `ite`, `=`/`distinct`) against an
//! independent reference for SMT-LIB `QF_BV`.
//!
//! ```text
//!   Predicate ──lower──► AIG ──simulate 64 patterns at a time──► bits
//!                                                                 │ compared
//!   SMT-LIB definition, written with plain integers  ◄────────────┘
//! ```
//!
//! Two kinds of check:
//!
//! 1. **Simulation** (cheap, so it can be exhaustive). The lowered AIG is
//!    evaluated directly. Every unary and binary operation is checked on
//!    *every* assignment at widths 1..=8, every ternary form on every
//!    assignment up to width 4 (sampled above), and every operation on
//!    seeded random assignments plus corner values at 16, 32 and 64 bits.
//! 2. **End to end** (through [`BvTactic::solve`]: Tseitin, CDCL, DRAT).
//!    Inputs are pinned by `x = #c` assertions (not substituted, so the
//!    circuit is not folded away). The reference answer must be SAT and the
//!    model must read back the pinned inputs; "the result differs from the
//!    reference" must be UNSAT, which this tactic reports only after
//!    `drat-check` accepted the proof. Exhaustive up to width 3, sampled at
//!    widths 4 and 16.
//!
//! ## The reference semantics (the only definitions used below)
//!
//! Operands are unsigned integers `0 <= x < 2^w`; `signed(x)` reads the two's
//! complement value.
//!
//! | Operation | Definition |
//! |---|---|
//! | `bvadd/bvsub/bvmul` | the exact result reduced mod `2^w` |
//! | `bvshl a s` | `a * 2^s mod 2^w`, and **0 when `s >= w`** |
//! | `bvlshr a s` | `floor(a / 2^s)`, and **0 when `s >= w`** |
//! | `bvashr a s` | `floor(signed(a) / 2^s)`, and **all sign bits when `s >= w`** |
//! | `bvudiv a b` | `floor(a / b)`; **`b = 0`: all ones** |
//! | `bvurem a b` | `a - b*floor(a / b)`; **`b = 0`: `a`** |
//! | `bvsdiv a b` | `signed(a) / signed(b)` truncated toward 0; `b = 0`: `-1` if `a >= 0`, `+1` if `a < 0`; `MIN / -1 = MIN` (overflow wraps) |
//! | `bvsrem a b` | remainder of the truncating division (sign of the dividend); `b = 0`: `a` |
//! | `bvsmod a b` | `signed(a) - signed(b) * floor(signed(a) / signed(b))` (sign of the divisor); `b = 0`: `a` |
//! | `concat a b` | `a * 2^w_b + b` |
//! | `extract hi lo a` | `floor(a / 2^lo) mod 2^(hi-lo+1)` |
//! | `zero_extend k a` / `sign_extend k a` | `a` / `signed(a)`, in `w + k` bits |
//!
//! Division truth table (signed values, 4 bits, so `-8..=7`):
//!
//! ```text
//!    a   b | sdiv  srem  smod
//!   -------+--------------------
//!    7   3 |   2     1     1
//!   -7   3 |  -2    -1     2      srem follows the dividend, smod the divisor
//!    7  -3 |  -2     1    -2
//!   -7  -3 |   2    -1    -1
//!   -8  -1 |  -8     0     0      overflow: -(-8) = 8 wraps to -8 in 4 bits
//!    5   0 |  -1     5     5      by zero: sdiv = -1 (a >= 0), rem/mod = a
//!   -5   0 |   1    -5    -5      by zero: sdiv = +1 (a <  0)
//! ```
//!
//! `smod_srem_sdiv_known_values` pins this table through the tactic.
//!
//! ## What is and is not covered (matrix)
//!
//! | Operation group | w 1..=8 | w 16, 32, 64 | end to end (CNF + DRAT) |
//! |---|---|---|---|
//! | unary, binary, comparisons, `=`, `distinct`, divisions, `bvsmod`, `concat`, `extract`, extensions | **exhaustive** (simulation) | seeded random + corners (simulation) | exhaustive w<=3; sampled w=4, 16 |
//! | ternary (n-ary fold, `ite`) | **exhaustive** w<=4, sampled 5..=8 | seeded random + corners | exhaustive w=3 |
//! | `bvmul` equivalence proofs (commutativity etc.) | see `multiplier_miters` | | per-test work budget, documented there |

use super::*;
use bit_blaster::aig::{edge_value, Node};

// ---------------------------------------------------------------------------
// Reference semantics
// ---------------------------------------------------------------------------

fn mask(w: usize) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

fn signed(x: u128, w: usize) -> i128 {
    if x >> (w - 1) & 1 == 1 {
        x as i128 - (1i128 << w)
    } else {
        x as i128
    }
}

fn wrap(x: i128, w: usize) -> u128 {
    (x as u128) & mask(w)
}

fn shl_ref(a: u128, s: u128, w: usize) -> u128 {
    if s >= w as u128 {
        0
    } else {
        (a << s) & mask(w)
    }
}

fn lshr_ref(a: u128, s: u128, w: usize) -> u128 {
    if s >= w as u128 {
        0
    } else {
        a >> s
    }
}

fn ashr_ref(a: u128, s: u128, w: usize) -> u128 {
    // Arithmetic shift by 127 or more leaves only sign bits, which is also
    // what any shift by >= w does for a w-bit value (w <= 64 here).
    wrap(signed(a, w) >> s.min(127), w)
}

fn udiv_ref(a: u128, b: u128, w: usize) -> u128 {
    a.checked_div(b).unwrap_or(mask(w)) // b = 0: all ones
}

fn urem_ref(a: u128, b: u128) -> u128 {
    if b == 0 {
        a
    } else {
        a % b
    }
}

fn sdiv_ref(a: u128, b: u128, w: usize) -> u128 {
    let (sa, sb) = (signed(a, w), signed(b, w));
    if sb == 0 {
        return if sa >= 0 { mask(w) } else { 1 };
    }
    wrap(sa / sb, w) // Rust `/` truncates toward zero
}

fn srem_ref(a: u128, b: u128, w: usize) -> u128 {
    let (sa, sb) = (signed(a, w), signed(b, w));
    if sb == 0 {
        return a;
    }
    wrap(sa % sb, w) // Rust `%` takes the sign of the dividend
}

/// `a - b * floor(a / b)`: the remainder takes the sign of the divisor.
fn smod_ref(a: u128, b: u128, w: usize) -> u128 {
    let (sa, sb) = (signed(a, w), signed(b, w));
    if sb == 0 {
        return a;
    }
    let trunc = sa / sb;
    let floor = if sa % sb != 0 && ((sa < 0) != (sb < 0)) { trunc - 1 } else { trunc };
    wrap(sa - sb * floor, w)
}

// ---------------------------------------------------------------------------
// The catalogue of operations under test
// ---------------------------------------------------------------------------

fn var(n: &str) -> Predicate {
    Predicate::Var(n.into())
}

fn app(f: &str, args: &[Predicate]) -> Predicate {
    Predicate::Apply { f: f.into(), args: args.to_vec() }
}

/// `((_ name i…) operand)` in the tactic's flattened form.
fn idx(name: &str, indices: &[i128], operand: &Predicate) -> Predicate {
    let mut args = vec![var(name)];
    args.extend(indices.iter().map(|&i| Predicate::Int(i)));
    args.push(operand.clone());
    app("_", &args)
}

fn eq(a: Predicate, b: Predicate) -> Predicate {
    Predicate::Eq(Box::new(a), Box::new(b))
}

fn neq(a: Predicate, b: Predicate) -> Predicate {
    Predicate::NEq(Box::new(a), Box::new(b))
}

/// `(_ bvN w)`.
fn lit(w: usize, v: u128) -> Predicate {
    app("_", &[var(&format!("bv{v}")), Predicate::Int(w as i128)])
}

type BuildFn = dyn Fn(&[Predicate]) -> Predicate;
type RefFn = dyn Fn(&[u128]) -> u128;

/// One operation: how to build its `Predicate`, what it must compute, and
/// the shape of the result.
struct Op {
    name: String,
    /// How many of the free variables `x`, `y`, `z` it reads.
    arity: usize,
    build: Box<BuildFn>,
    /// Reference over the operand values.
    reference: Box<RefFn>,
    /// Result width in bits, or `None` for a Boolean.
    out_width: Option<usize>,
}

fn op(
    name: impl Into<String>,
    arity: usize,
    out_width: Option<usize>,
    build: impl Fn(&[Predicate]) -> Predicate + 'static,
    reference: impl Fn(&[u128]) -> u128 + 'static,
) -> Op {
    Op { name: name.into(), arity, build: Box::new(build), reference: Box::new(reference), out_width }
}

type BinRef = fn(u128, u128, usize) -> u128;

fn fold_ref(name: &str) -> BinRef {
    match name {
        "bvadd" => |a, b, w| a.wrapping_add(b) & mask(w),
        "bvsub" => |a, b, w| a.wrapping_sub(b) & mask(w),
        "bvmul" => |a, b, w| a.wrapping_mul(b) & mask(w),
        "bvand" => |a, b, _| a & b,
        "bvor" => |a, b, _| a | b,
        _ => |a, b, _| a ^ b,
    }
}

/// Every operation the tactic lowers, instantiated at width `w`.
fn catalogue(w: usize) -> Vec<Op> {
    let mut ops = Vec::new();
    let m = mask(w);

    ops.push(op("bvnot", 1, Some(w), |a| app("bvnot", &[a[0].clone()]), move |v| !v[0] & m));
    ops.push(op("bvneg", 1, Some(w), |a| app("bvneg", &[a[0].clone()]), move |v| wrap(-(v[0] as i128), w)));
    // Every extract slice.
    for hi in 0..w {
        for lo in 0..=hi {
            ops.push(op(
                format!("extract {hi} {lo}"),
                1,
                Some(hi - lo + 1),
                move |a| idx("extract", &[hi as i128, lo as i128], &a[0]),
                move |v| (v[0] >> lo) & mask(hi - lo + 1),
            ));
        }
    }
    // Extensions by 0..=3, and by 9 (crosses a byte boundary).
    for k in [0usize, 1, 2, 3, 9] {
        ops.push(op(
            format!("zero_extend {k}"),
            1,
            Some(w + k),
            move |a| idx("zero_extend", &[k as i128], &a[0]),
            |v| v[0],
        ));
        ops.push(op(
            format!("sign_extend {k}"),
            1,
            Some(w + k),
            move |a| idx("sign_extend", &[k as i128], &a[0]),
            move |v| wrap(signed(v[0], w), w + k),
        ));
    }

    let binary: Vec<(&str, BinRef)> = vec![
        ("bvand", fold_ref("bvand")),
        ("bvor", fold_ref("bvor")),
        ("bvxor", fold_ref("bvxor")),
        ("bvadd", fold_ref("bvadd")),
        ("bvsub", fold_ref("bvsub")),
        ("bvmul", fold_ref("bvmul")),
        ("bvshl", shl_ref),
        ("bvlshr", lshr_ref),
        ("bvashr", ashr_ref),
        ("bvudiv", udiv_ref),
        ("bvurem", |a, b, _| urem_ref(a, b)),
        ("bvsdiv", sdiv_ref),
        ("bvsrem", srem_ref),
        ("bvsmod", smod_ref),
    ];
    for (name, f) in binary {
        ops.push(op(name, 2, Some(w), move |a| app(name, &[a[0].clone(), a[1].clone()]), move |v| f(v[0], v[1], w)));
    }

    type Cmp = fn(u128, u128, usize) -> bool;
    let cmps: Vec<(&str, Cmp)> = vec![
        ("bvult", |a, b, _| a < b),
        ("bvule", |a, b, _| a <= b),
        ("bvugt", |a, b, _| a > b),
        ("bvuge", |a, b, _| a >= b),
        ("bvslt", |a, b, w| signed(a, w) < signed(b, w)),
        ("bvsle", |a, b, w| signed(a, w) <= signed(b, w)),
        ("bvsgt", |a, b, w| signed(a, w) > signed(b, w)),
        ("bvsge", |a, b, w| signed(a, w) >= signed(b, w)),
    ];
    for (name, f) in cmps {
        ops.push(op(name, 2, None, move |a| app(name, &[a[0].clone(), a[1].clone()]), move |v| f(v[0], v[1], w) as u128));
    }
    ops.push(op("=", 2, None, |a| eq(a[0].clone(), a[1].clone()), |v| (v[0] == v[1]) as u128));
    ops.push(op("distinct", 2, None, |a| neq(a[0].clone(), a[1].clone()), |v| (v[0] != v[1]) as u128));

    ops.push(op("concat x y", 2, Some(2 * w), |a| app("concat", &[a[0].clone(), a[1].clone()]), move |v| (v[0] << w) | v[1]));
    if w >= 2 {
        // Unequal widths: (extract w-1 1 x) ++ y has width (w-1) + w.
        ops.push(op(
            "concat (extract x) y",
            2,
            Some(2 * w - 1),
            move |a| app("concat", &[idx("extract", &[w as i128 - 1, 1], &a[0]), a[1].clone()]),
            move |v| ((v[0] >> 1) << w) | v[1],
        ));
    }

    // Left-associative n-ary folding, and ite.
    for name in ["bvadd", "bvsub", "bvmul", "bvand", "bvor", "bvxor"] {
        let f = fold_ref(name);
        ops.push(op(
            format!("{name} x y z"),
            3,
            Some(w),
            move |a| app(name, &[a[0].clone(), a[1].clone(), a[2].clone()]),
            move |v| f(f(v[0], v[1], w), v[2], w),
        ));
    }
    ops.push(op(
        "ite (bvult x y) x z",
        3,
        Some(w),
        |a| Predicate::Ite(Box::new(app("bvult", &[a[0].clone(), a[1].clone()])), Box::new(a[0].clone()), Box::new(a[2].clone())),
        |v| if v[0] < v[1] { v[0] } else { v[2] },
    ));
    ops.push(op(
        "ite (= x y) (bvnot z) z",
        3,
        Some(w),
        |a| Predicate::Ite(Box::new(eq(a[0].clone(), a[1].clone())), Box::new(app("bvnot", &[a[2].clone()])), Box::new(a[2].clone())),
        move |v| if v[0] == v[1] { !v[2] & m } else { v[2] },
    ));
    ops
}

// ---------------------------------------------------------------------------
// Check 1: simulate the lowered AIG
// ---------------------------------------------------------------------------

const VARS: [&str; 3] = ["x", "y", "z"];

fn env_of(arity: usize, w: usize) -> SortEnv {
    VARS[..arity].iter().map(|n| (n.to_string(), Sort::BitVec(w as u32))).collect()
}

/// Lower `spec` over `arity` variables of width `w`; return the lowering,
/// the result term, and for each variable its AIG input indices (bit 0 first).
fn lower_op(spec: &Op, w: usize) -> (Lowering, Term, Vec<Vec<usize>>) {
    let mut l = Lowering::new(&env_of(spec.arity, w)).expect("declarations lower");
    let args: Vec<Predicate> = VARS[..spec.arity].iter().map(|n| var(n)).collect();
    let term = l.term(&(spec.build)(&args)).unwrap_or_else(|e| panic!("{} at width {w}: {e}", spec.name));
    let inputs = (0..spec.arity)
        .map(|k| {
            let Term::Bv(b) = &l.vars[l.index[VARS[k]]].1 else { unreachable!() };
            b.bits
                .iter()
                .map(|e| match l.aig.node(e.node()) {
                    Node::Input(i) => i,
                    _ => unreachable!(),
                })
                .collect()
        })
        .collect();
    (l, term, inputs)
}

/// Check `spec` on the given assignments, 64 at a time. Returns how many.
fn simulate_op(spec: &Op, w: usize, assignments: &mut dyn Iterator<Item = [u128; 3]>) -> u64 {
    let (l, term, inputs) = lower_op(spec, w);
    let out: Vec<Edge> = match &term {
        Term::Bool(e) => vec![*e],
        Term::Bv(b) => b.bits.clone(),
    };
    assert_eq!(out.len(), spec.out_width.unwrap_or(1), "{} at width {w}: result width", spec.name);
    let mut count = 0;
    loop {
        let batch: Vec<[u128; 3]> = (&mut *assignments).take(64).collect();
        if batch.is_empty() {
            return count;
        }
        let mut words = vec![0u64; l.aig.num_inputs()];
        for (j, vals) in batch.iter().enumerate() {
            for (k, ins) in inputs.iter().enumerate() {
                for (bit, &i) in ins.iter().enumerate() {
                    words[i] |= ((vals[k] >> bit & 1) as u64) << j;
                }
            }
        }
        let values = l.aig.simulate(&words);
        for (j, vals) in batch.iter().enumerate() {
            let got: u128 = out.iter().enumerate().map(|(i, &e)| ((edge_value(&values, e) >> j & 1) as u128) << i).sum();
            let want = (spec.reference)(&vals[..spec.arity]);
            assert_eq!(
                got,
                want,
                "{} at width {w} on {:?}: lowered {got:#x}, reference {want:#x}",
                spec.name,
                &vals[..spec.arity]
            );
            count += 1;
        }
    }
}

/// All assignments of `arity` variables of width `w`.
fn every_assignment(w: usize, arity: usize) -> impl Iterator<Item = [u128; 3]> {
    let n = 1u128 << (w * arity);
    (0..n).map(move |p| {
        let mut v = [0u128; 3];
        for (k, slot) in v.iter_mut().enumerate().take(arity) {
            *slot = (p >> (w * k)) & mask(w);
        }
        v
    })
}

/// Deterministic xorshift64*.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
}

/// Corner values of width `w`: 0, 1, 2, max, max-1, the sign boundary, and
/// the values around `w` (shift amounts at the overflow threshold).
fn corners(w: usize) -> Vec<u128> {
    let m = mask(w);
    let half = 1u128 << (w - 1);
    let mut v = vec![0, 1, 2, m, m - 1, half, half - 1, half + 1, w as u128 - 1, w as u128, w as u128 + 1];
    v.iter_mut().for_each(|x| *x &= m);
    v.sort_unstable();
    v.dedup();
    v
}

/// Every pair of corners, then `count` seeded random assignments.
fn random_assignments(w: usize, arity: usize, count: usize, seed: u64) -> Vec<[u128; 3]> {
    let mut rng = Rng(seed);
    let c = corners(w);
    let mut out = Vec::new();
    for &a in &c {
        for &b in &c {
            out.push([a, b, c[(rng.next() % c.len() as u64) as usize]]);
        }
    }
    for _ in 0..count {
        let mut v = [0u128; 3];
        for slot in v.iter_mut().take(arity) {
            let r = (rng.next() as u128) << 64 | rng.next() as u128;
            *slot = r & mask(w);
        }
        out.push(v);
    }
    out
}

#[test]
fn widths_1_to_8_unary_and_binary_exhaustive() {
    let mut total = 0;
    for w in 1..=8usize {
        for spec in catalogue(w).iter().filter(|s| s.arity <= 2) {
            total += simulate_op(spec, w, &mut every_assignment(w, spec.arity));
        }
    }
    assert!(total > 2_000_000, "only {total} checks ran");
}

#[test]
fn widths_1_to_4_ternary_exhaustive() {
    for w in 1..=4usize {
        for spec in catalogue(w).iter().filter(|s| s.arity == 3) {
            simulate_op(spec, w, &mut every_assignment(w, 3));
        }
    }
}

#[test]
fn widths_5_to_8_ternary_sampled() {
    for w in 5..=8usize {
        for spec in catalogue(w).iter().filter(|s| s.arity == 3) {
            simulate_op(spec, w, &mut random_assignments(w, 3, 5000, 0xA11CE ^ w as u64).into_iter());
        }
    }
}

/// At 32/64 bits every extract slice is wiring only; check the boundary
/// slices (full, top bit, low bit, byte-crossing ones).
fn wide_extract_is_checked(name: &str, w: usize) -> bool {
    let Some(rest) = name.strip_prefix("extract ") else { return true };
    let mut it = rest.split(' ').map(|s| s.parse::<usize>().unwrap());
    let (hi, lo) = (it.next().unwrap(), it.next().unwrap());
    w == 16 || hi == w - 1 || lo == 0 || hi == lo || (hi % 8 == 7 && lo % 8 == 0)
}

#[test]
fn widths_16_32_64_random_and_corners() {
    for w in [16usize, 32, 64] {
        for spec in catalogue(w).iter().filter(|s| wide_extract_is_checked(&s.name, w)) {
            simulate_op(spec, w, &mut random_assignments(w, spec.arity, 4000, 0xBEEF_0000 + w as u64).into_iter());
        }
    }
}

#[test]
fn wide_vectors_beyond_u128_wire_correctly() {
    // 200 bits is past the u128 reference, so check the wiring structurally.
    let w = 200usize;
    let env: SortEnv = [("x".to_string(), Sort::BitVec(w as u32))].into_iter().collect();
    let mut l = Lowering::new(&env).unwrap();
    // (concat ((_ extract 99 0) x) ((_ extract 199 100) x)) swaps the halves.
    let swapped = app("concat", &[idx("extract", &[99, 0], &var("x")), idx("extract", &[199, 100], &var("x"))]);
    let Term::Bv(b) = l.term(&swapped).unwrap() else { panic!() };
    let Term::Bv(x) = l.vars[0].1.clone() else { panic!() };
    assert_eq!(b.width(), 200);
    for i in 0..100 {
        assert_eq!(b.bits[i], x.bits[100 + i], "low half of the result is x's high half");
        assert_eq!(b.bits[100 + i], x.bits[i], "high half of the result is x's low half");
    }
    let Term::Bv(s) = l.term(&idx("sign_extend", &[28], &var("x"))).unwrap() else { panic!() };
    assert_eq!(s.width(), 228);
    assert!(s.bits[200..].iter().all(|&e| e == x.bits[199]));
    let Term::Bv(z) = l.term(&idx("zero_extend", &[28], &var("x"))).unwrap() else { panic!() };
    assert!(z.bits[200..].iter().all(|&e| e == Edge::FALSE));
}

// ---------------------------------------------------------------------------
// Division by zero, overflow and known values, pinned through the tactic
// ---------------------------------------------------------------------------

/// With the inputs pinned, `f(x, y) = want` must be SAT and `!= want` UNSAT.
fn pinned(w: usize, f: &str, a: u128, b: u128, want: u128) {
    let e = env_of(2, w);
    let pins = [eq(var("x"), lit(w, a)), eq(var("y"), lit(w, b))];
    let r = app(f, &[var("x"), var("y")]);
    let mut yes = pins.to_vec();
    yes.push(eq(r.clone(), lit(w, want)));
    assert!(BvTactic::solve(&yes, &e).model().is_some(), "{f} {a} {b} should be {want}");
    let mut no = pins.to_vec();
    no.push(neq(r, lit(w, want)));
    assert_eq!(BvTactic::solve(&no, &e), SolverResult::Unsat, "{f} {a} {b} is not uniquely {want}");
}

#[test]
fn division_by_zero_follows_smt_lib() {
    for w in [1usize, 4, 8, 16] {
        let m = mask(w);
        let half = 1u128 << (w - 1);
        for a in [0, 1, m, half] {
            pinned(w, "bvudiv", a, 0, m); // x / 0 = all ones
            pinned(w, "bvurem", a, 0, a); // x mod 0 = x
            pinned(w, "bvsrem", a, 0, a);
            pinned(w, "bvsmod", a, 0, a);
            // bvsdiv x 0 = -1 for x >= 0, +1 for x < 0.
            pinned(w, "bvsdiv", a, 0, if a >> (w - 1) & 1 == 0 { m } else { 1 });
        }
    }
}

#[test]
fn signed_overflow_wraps() {
    for w in [2usize, 4, 8, 16] {
        let (m, min) = (mask(w), 1u128 << (w - 1));
        pinned(w, "bvsdiv", min, m, min); // MIN / -1 = MIN
        pinned(w, "bvsrem", min, m, 0); // MIN rem -1 = 0
        pinned(w, "bvsmod", min, m, 0);
    }
}

#[test]
fn smod_srem_sdiv_known_values() {
    let w = 4;
    let v = |x: i128| wrap(x, w);
    for (a, b, sdiv, srem, smod) in [
        (7, 3, 2, 1, 1),
        (-7, 3, -2, -1, 2),
        (7, -3, -2, 1, -2),
        (-7, -3, 2, -1, -1),
        (-8, -1, -8, 0, 0),
        (5, 0, -1, 5, 5),
        (-5, 0, 1, -5, -5),
    ] {
        pinned(w, "bvsdiv", v(a), v(b), v(sdiv));
        pinned(w, "bvsrem", v(a), v(b), v(srem));
        pinned(w, "bvsmod", v(a), v(b), v(smod));
    }
}

// ---------------------------------------------------------------------------
// Check 2: end to end through the tactic
// ---------------------------------------------------------------------------

fn check_end_to_end(spec: &Op, w: usize, assignments: &[[u128; 3]]) {
    let e = env_of(spec.arity, w);
    let args: Vec<Predicate> = VARS[..spec.arity].iter().map(|n| var(n)).collect();
    let term = (spec.build)(&args);
    for vals in assignments {
        let mut pins: Vec<Predicate> = (0..spec.arity).map(|k| eq(var(VARS[k]), lit(w, vals[k]))).collect();
        let want = (spec.reference)(&vals[..spec.arity]);
        let (same, differs) = match spec.out_width {
            None => {
                let (t, f) = (term.clone(), Predicate::Not(Box::new(term.clone())));
                if want == 1 {
                    (t, f)
                } else {
                    (f, t)
                }
            }
            Some(ow) => (eq(term.clone(), lit(ow, want)), neq(term.clone(), lit(ow, want))),
        };
        let mut sat = pins.clone();
        sat.push(same);
        let r = BvTactic::solve(&sat, &e);
        let m = r.model().unwrap_or_else(|| panic!("{} w={w} {vals:?}: reference answer is not SAT: {r:?}", spec.name));
        for k in 0..spec.arity {
            assert_eq!(m.get(VARS[k]).unwrap().as_bv_u128(), Some(vals[k]), "{} model reads back the pinned input", spec.name);
        }
        pins.push(differs);
        assert_eq!(BvTactic::solve(&pins, &e), SolverResult::Unsat, "{} w={w} {vals:?}: a different result is possible", spec.name);
    }
}

#[test]
fn end_to_end_widths_1_to_3_exhaustive() {
    for w in 1..=3usize {
        for spec in catalogue(w).iter().filter(|s| s.arity <= 2) {
            let all: Vec<[u128; 3]> = every_assignment(w, spec.arity).collect();
            check_end_to_end(spec, w, &all);
        }
    }
}

#[test]
fn end_to_end_width_3_ternary_exhaustive() {
    for spec in catalogue(3).iter().filter(|s| s.arity == 3) {
        let all: Vec<[u128; 3]> = every_assignment(3, 3).collect();
        check_end_to_end(spec, 3, &all);
    }
}

#[test]
fn end_to_end_widths_4_and_16_sampled() {
    for w in [4usize, 16] {
        for spec in catalogue(w).iter().filter(|s| wide_extract_is_checked(&s.name, w) && (w == 4 || !s.name.starts_with("extract"))) {
            let sample: Vec<[u128; 3]> = if w == 4 {
                every_assignment(w, spec.arity).step_by(37).collect()
            } else {
                random_assignments(w, spec.arity, 8, 0xE2E ^ w as u64).into_iter().step_by(23).collect()
            };
            check_end_to_end(spec, w, &sample);
        }
    }
}

// ---------------------------------------------------------------------------
// Refusals of the new forms
// ---------------------------------------------------------------------------

#[test]
fn malformed_new_forms_are_unknown() {
    let e: SortEnv = [("x".to_string(), Sort::BitVec(8)), ("y".to_string(), Sort::BitVec(4))].into_iter().collect();
    let unknown = |t: Predicate| match BvTactic::solve(&[eq(t.clone(), t)], &e) {
        SolverResult::Unknown(why) => why,
        other => panic!("expected Unknown, got {other:?}"),
    };
    assert!(unknown(idx("extract", &[8, 0], &var("x"))).contains("needs lo <= hi < width"));
    assert!(unknown(idx("extract", &[2, 3], &var("x"))).contains("needs lo <= hi < width"));
    assert!(unknown(idx("extract", &[-1, 0], &var("x"))).contains("non-negative"));
    assert!(unknown(idx("zero_extend", &[-1], &var("x"))).contains("non-negative"));
    assert!(unknown(idx("zero_extend", &[505], &var("x"))).contains("width 513"));
    assert!(unknown(idx("sign_extend", &[i128::MAX], &var("x"))).contains("bit-vector width"));
    assert!(unknown(idx("rotate_left", &[1], &var("x"))).contains("indexed term"));
    assert!(unknown(app("_", &[Predicate::Int(1)])).contains("indexed term"));
    assert!(unknown(app("bvudiv", &[var("x"), var("y")])).contains("widths 8 and 4"));
    assert!(unknown(app("bvsmod", &[var("x")])).contains("takes 2 arguments"));
    assert!(unknown(app("concat", &[var("x")])).contains("takes 2 arguments"));
    assert!(unknown(app("concat", &[var("x"), Predicate::Bool(true)])).contains("Boolean where"));
    // Concatenation may not exceed the width limit.
    let big: SortEnv = [("a".to_string(), Sort::BitVec(300)), ("b".to_string(), Sort::BitVec(300))].into_iter().collect();
    let c = app("concat", &[var("a"), var("b")]);
    assert!(matches!(BvTactic::solve(&[eq(c.clone(), c)], &big), SolverResult::Unknown(w) if w.contains("width 600")));
}

#[test]
fn wide_division_is_refused_before_it_is_built() {
    // A 512-bit divider would first build millions of gates; the estimate
    // refuses it up front.
    let e = env_of(2, 512);
    for f in ["bvudiv", "bvurem", "bvsdiv", "bvsrem", "bvsmod"] {
        let p = eq(app(f, &[var("x"), var("y")]), var("x"));
        let started = std::time::Instant::now();
        assert!(matches!(BvTactic::solve(&[p], &e), SolverResult::Unknown(w) if w.contains("MAX_AIG_NODES")), "{f}");
        assert!(started.elapsed().as_secs() < 5, "{f} refused slowly");
    }
}

#[test]
fn division_cost_estimate_is_conservative() {
    // The pre-check estimate must be at least the real gate count.
    for w in [1usize, 2, 8, 33, 64, 100] {
        for f in ["bvudiv", "bvurem", "bvsdiv", "bvsrem", "bvsmod"] {
            let mut l = Lowering::new(&env_of(2, w)).unwrap();
            let before = l.aig.num_nodes();
            l.term(&app(f, &[var("x"), var("y")])).unwrap_or_else(|e| panic!("{f} {w}: {e}"));
            let used = l.aig.num_nodes() - before;
            assert!(used <= division_gate_estimate(w), "{f} at {w}: {used} gates > estimate");
        }
    }
}

/// Is `lhs = rhs` valid? `Unsat` of the negation is the proof, and this
/// tactic only reports `Unsat` after `drat-check` accepted its DRAT proof.
fn proved(e: &SortEnv, name: &str, lhs: Predicate, rhs: Predicate) {
    assert_eq!(BvTactic::solve(&[neq(lhs, rhs)], e), SolverResult::Unsat, "{name}: identity not certified");
}

fn x() -> Predicate {
    var("x")
}

fn y() -> Predicate {
    var("y")
}

// ---------------------------------------------------------------------------
// Certified identities, and what the wide multiplier does and does not get
// ---------------------------------------------------------------------------

/// Certified identities at 16, 32 and 64 bits that need no multiplier.
/// Each is a miter whose UNSAT is certified under `drat-check`'s *default*
/// limits (no budget was raised).
#[test]
fn wide_identities_without_a_multiplier_are_certified() {
    for w in [16usize, 32, 64] {
        let e = env_of(2, w);
        let (zero, one) = (lit(w, 0), lit(w, 1));
        let top = w as i128 - 1;
        proved(&e, "a+b = b+a", app("bvadd", &[x(), y()]), app("bvadd", &[y(), x()]));
        proved(&e, "a-b = a+(-b)", app("bvsub", &[x(), y()]), app("bvadd", &[x(), app("bvneg", &[y()])]));
        proved(&e, "-a = ~a + 1", app("bvneg", &[x()]), app("bvadd", &[app("bvnot", &[x()]), one.clone()]));
        proved(
            &e,
            "a xor b = (a|b) & ~(a&b)",
            app("bvxor", &[x(), y()]),
            app("bvand", &[app("bvor", &[x(), y()]), app("bvnot", &[app("bvand", &[x(), y()])])]),
        );
        proved(&e, "a + ~a = -1", app("bvadd", &[x(), app("bvnot", &[x()])]), app("bvnot", std::slice::from_ref(&zero)));
        proved(
            &e,
            "a < b <=> not (b <= a)",
            Predicate::Not(Box::new(app("bvult", &[x(), y()]))),
            app("bvule", &[y(), x()]),
        );
        proved(
            &e,
            "signed negative <=> top bit",
            app("bvslt", &[x(), zero.clone()]),
            eq(idx("extract", &[top, top], &x()), lit(1, 1)),
        );
        proved(
            &e,
            "concat/extract round trip",
            x(),
            app("concat", &[idx("extract", &[top, w as i128 / 2], &x()), idx("extract", &[w as i128 / 2 - 1, 0], &x())]),
        );
        // Division by zero is part of the definition, so it can be proved.
        proved(&e, "x udiv 0 = -1", app("bvudiv", &[x(), zero.clone()]), app("bvnot", std::slice::from_ref(&zero)));
        proved(&e, "x urem 0 = x", app("bvurem", &[x(), zero.clone()]), x());
        proved(&e, "x smod 0 = x", app("bvsmod", &[x(), zero.clone()]), x());
        proved(&e, "x srem 1 = 0", app("bvsrem", &[x(), one.clone()]), zero.clone());
        proved(&e, "x udiv 1 = x", app("bvudiv", &[x(), one.clone()]), x());
        proved(&e, "x sdiv 1 = x", app("bvsdiv", &[x(), one.clone()]), x());
    }
}

/// `bvmul` against constants is a chain of shifted additions. These are
/// certified at 16, 32 and 64 bits, which covers the multiplier circuit on
/// every constant multiplier of the listed shapes.
#[test]
fn wide_multiplication_by_constants_is_certified() {
    for w in [16usize, 32, 64] {
        let e = env_of(2, w);
        let shl = |k: u128| app("bvshl", &[x(), lit(w, k)]);
        proved(&e, "x*0", app("bvmul", &[x(), lit(w, 0)]), lit(w, 0));
        proved(&e, "x*1", app("bvmul", &[x(), lit(w, 1)]), x());
        proved(&e, "x*2", app("bvmul", &[x(), lit(w, 2)]), shl(1));
        proved(&e, "x*3", app("bvmul", &[x(), lit(w, 3)]), app("bvadd", &[x(), shl(1)]));
        proved(&e, "x*5", app("bvmul", &[x(), lit(w, 5)]), app("bvadd", &[x(), shl(2)]));
        proved(&e, "x*7", app("bvmul", &[x(), lit(w, 7)]), app("bvsub", &[shl(3), x()]));
        proved(&e, "x*2^(w-1)", app("bvmul", &[x(), lit(w, 1u128 << (w - 1))]), shl(w as u128 - 1));
        proved(&e, "2^(w/2) * 2^(w/2) = 0", app("bvmul", &[lit(w, 1u128 << (w / 2)), lit(w, 1u128 << (w / 2))]), lit(w, 0));
    }
}

/// **Structural decomposition of the wide multiplier.** The low `h` bits of a
/// `w`-bit product depend only on the low `h` bits of the factors (a carry
/// only travels upward), so
///
/// ```text
///   extract(h-1, 0, a *w b)  =  extract(h-1, 0, a) *h extract(h-1, 0, b)
/// ```
///
/// Each instance is certified here at the default limits. Chained down to
/// width 8 (`64 -> 32 -> 16 -> 8`, or directly `64 -> 8`) with the
/// exhaustive 8-bit simulation, the low bits of the 64-bit multiplier are
/// covered by proof, not sampling. The high bits of a wide product are *not*
/// proved (see the CHANGELOG).
#[test]
fn multiplier_low_halves_decompose() {
    for (w, h) in [(16usize, 8usize), (32, 16), (64, 32), (32, 8), (64, 8)] {
        let e = env_of(2, w);
        let low = |p: Predicate| idx("extract", &[h as i128 - 1, 0], &p);
        proved(
            &e,
            &format!("low {h} bits of the {w}-bit product"),
            low(app("bvmul", &[x(), y()])),
            app("bvmul", &[low(x()), low(y())]),
        );
    }
}

/// Like [`BvTactic::solve`] but with an explicit `drat-check` work budget.
/// Only tests use this; the public tactic always runs at the defaults.
fn solve_with_budget(assertions: &[Predicate], e: &SortEnv, max_work: u64) -> SolverResult {
    let mut l = Lowering::new(e).unwrap();
    let edges = l.lower_all(assertions).unwrap();
    l.decide(&edges, drat_check::Limits { max_work, ..Default::default() })
}

/// `x * -1 = -x`: a multiplier chain against a negation, the hardest
/// identity here that still finishes. Measured proof-check work (release
/// build, literal visits):
///
/// | width | solve + check | check at the default limit (2^32) |
/// |---|---|---|
/// | 8..=12 | under 0.5 s | accepted |
/// | 14 | 2.7 s | accepted |
/// | 16 | 15 s | **rejected: work budget exhausted** (needs 2^34) |
///
/// The default limit is a safety bound on hostile proofs, so the public
/// tactic keeps it and answers `Unknown` (never `Unsat`) at width 16. The
/// first test below pins that refusal on a small case with a deliberately
/// tiny budget; the second certifies the 16-bit instance with a documented
/// per-test budget of 2^34 (four times the default).
fn neg_one_miter(w: usize) -> (SortEnv, Predicate) {
    let p = neq(app("bvmul", &[x(), app("bvnot", &[lit(w, 0)])]), app("bvneg", &[x()]));
    (env_of(2, w), p)
}

#[test]
fn a_work_budget_too_small_gives_unknown_never_unsat() {
    let (e, p) = neg_one_miter(8);
    assert_eq!(solve_with_budget(std::slice::from_ref(&p), &e, drat_check::DEFAULT_MAX_WORK), SolverResult::Unsat);
    match solve_with_budget(&[p], &e, 1 << 10) {
        SolverResult::Unknown(why) => assert!(why.contains("work budget exhausted"), "{why}"),
        other => panic!("a proof the kernel could not finish must be Unknown, got {other:?}"),
    }
}

#[test]
fn multiplier_by_minus_one_certified_up_to_twelve_bits() {
    for w in [9usize, 10, 12] {
        let (e, p) = neg_one_miter(w);
        assert_eq!(BvTactic::solve(&[p], &e), SolverResult::Unsat, "w={w}");
    }
}

/// The 16-bit instance needs a 2^34 work budget (see above). About 80 s
/// unoptimised, 16 s in a release build, so it runs from the BUILD file as
/// `cargo test --release -- --ignored` rather than in the debug run.
#[test]
#[ignore = "slow: runs in the BUILD file under --release --ignored"]
fn multiplier_by_minus_one_at_sixteen_bits_needs_a_larger_work_budget() {
    let (e, p) = neg_one_miter(16);
    match BvTactic::solve(std::slice::from_ref(&p), &e) {
        SolverResult::Unknown(why) => assert!(why.contains("work budget exhausted"), "{why}"),
        other => panic!("expected the default limit to refuse, got {other:?}"),
    }
    assert_eq!(solve_with_budget(&[p], &e, drat_check::DEFAULT_MAX_WORK << 2), SolverResult::Unsat);
}

/// Commutativity of `bvmul`, certified from scratch (the solver has no
/// shortcut here). From width 10 the search exhausts the tactic's fixed
/// conflict budget (measured, release build: 10 -> Unknown after 1M
/// conflicts, about 20 s), so 7 is where this test stops; 8 passes in
/// seconds in release builds but minutes unoptimised.
#[test]
fn multiplier_commutes_certified_up_to_seven_bits() {
    for w in 1..=7usize {
        let e = env_of(2, w);
        proved(&e, "a*b = b*a", app("bvmul", &[x(), y()]), app("bvmul", &[y(), x()]));
    }
}

// ---------------------------------------------------------------------------
// Boolean structure and the refusal paths
// ---------------------------------------------------------------------------

fn bool_term(p: &Predicate, a: bool, b: bool, c: bool) -> bool {
    let e: SortEnv = [("a", Sort::Bool), ("b", Sort::Bool), ("c", Sort::Bool)].into_iter().map(|(n, s)| (n.to_string(), s)).collect();
    let mut l = Lowering::new(&e).unwrap();
    let Term::Bool(out) = l.term(p).unwrap() else { panic!("not Boolean") };
    // Inputs were created in name order: a, b, c.
    l.aig.eval(out, &[a, b, c])
}

#[test]
fn boolean_connectives_match_truth_tables() {
    let (a, b, c) = (var("a"), var("b"), var("c"));
    let bx = |p: &Predicate| Box::new(p.clone());
    for bits in 0..8u8 {
        let (va, vb, vc) = (bits & 1 == 1, bits & 2 == 2, bits & 4 == 4);
        let check = |name: &str, p: Predicate, want: bool| assert_eq!(bool_term(&p, va, vb, vc), want, "{name} on {va} {vb} {vc}");
        check("not", Predicate::Not(bx(&a)), !va);
        check("and", Predicate::And(vec![a.clone(), b.clone(), c.clone()]), va && vb && vc);
        check("or", Predicate::Or(vec![a.clone(), b.clone(), c.clone()]), va || vb || vc);
        check("and []", Predicate::And(vec![]), true);
        check("or []", Predicate::Or(vec![]), false);
        check("implies", Predicate::Implies(bx(&a), bx(&b)), !va || vb);
        check("iff", Predicate::Iff(bx(&a), bx(&b)), va == vb);
        check("= on Bool", eq(a.clone(), b.clone()), va == vb);
        check("distinct on Bool", neq(a.clone(), b.clone()), va != vb);
        check("ite on Bool", Predicate::Ite(bx(&a), bx(&b), bx(&c)), if va { vb } else { vc });
        check("true", Predicate::Bool(true), true);
        check("false", Predicate::Bool(false), false);
    }
}

#[test]
fn ill_formed_terms_are_refused_with_a_reason() {
    let e = env_of(2, 4);
    let unknown = |p: Predicate| match BvTactic::solve(&[p], &e) {
        SolverResult::Unknown(why) => why,
        other => panic!("expected Unknown, got {other:?}"),
    };
    let (u, v) = (x(), y());
    assert!(unknown(eq(app("bvnot", &[u.clone(), v.clone()]), u.clone())).contains("takes 1 argument"));
    assert!(unknown(eq(app("bvneg", &[]), u.clone())).contains("takes 1 argument"));
    for f in ["bvadd", "bvand", "bvor", "bvxor", "bvsub", "bvmul"] {
        assert!(unknown(eq(app(f, std::slice::from_ref(&u)), u.clone())).contains("at least 2 arguments"), "{f}");
        let wide = idx("zero_extend", &[1], &u);
        assert!(unknown(eq(app(f, &[u.clone(), v.clone(), wide]), u.clone())).contains("on widths 4 and 5"), "{f}");
    }
    for f in ["bvshl", "bvlshr", "bvashr", "bvult", "bvule", "bvugt", "bvuge", "bvslt", "bvsle", "bvsgt", "bvsge"] {
        assert!(unknown(app(f, std::slice::from_ref(&u))).contains("takes 2 arguments"), "{f}");
        assert!(unknown(app(f, &[u.clone(), idx("zero_extend", &[1], &v)])).contains("widths 4 and 5"), "{f}");
    }
    assert!(unknown(eq(app("bvfoo", &[u.clone(), v.clone()]), u.clone())).contains("`bvfoo` is not supported"));
    assert!(unknown(eq(app("bvrotl", std::slice::from_ref(&u)), u.clone())).contains("not supported"));
    // Mixed sorts and widths in `=`, `ite`, and Boolean/bit-vector confusion.
    assert!(unknown(eq(u.clone(), Predicate::Bool(true))).contains("different sorts or widths"));
    assert!(unknown(eq(u.clone(), idx("zero_extend", &[1], &v))).contains("different sorts or widths"));
    let ite = |c: Predicate, t: Predicate, f: Predicate| Predicate::Ite(Box::new(c), Box::new(t), Box::new(f));
    assert!(unknown(eq(ite(Predicate::Bool(true), u.clone(), Predicate::Bool(false)), u.clone())).contains("`ite` arms"));
    assert!(unknown(eq(ite(Predicate::Bool(true), u.clone(), idx("zero_extend", &[1], &v)), u.clone())).contains("`ite` arms"));
    assert!(unknown(eq(ite(u.clone(), u.clone(), v.clone()), u.clone())).contains("bit-vector where a Boolean"));
    assert!(unknown(Predicate::And(vec![u.clone()])).contains("bit-vector where a Boolean"));
    assert!(unknown(eq(app("bvnot", &[Predicate::Bool(true)]), u.clone())).contains("Boolean where a bit-vector"));
    assert!(unknown(eq(var("nope"), u.clone())).contains("undeclared `nope`"));
}

/// The node cap keeps every query's CNF inside the kernel's variable limit,
/// so [`Lowering::decide`]'s own guard is unreachable through
/// [`BvTactic::solve`]. Reach it by handing `decide` a circuit built past
/// the caps, to pin that the guard refuses *before* a solve whose UNSAT
/// could never be certified.
#[test]
fn a_cnf_beyond_the_kernels_variable_limit_is_refused_before_solving() {
    let mut l = Lowering::new(&SortEnv::new()).unwrap();
    let mut acc = Edge::TRUE;
    for _ in 0..drat_check::DEFAULT_MAX_VARS / 2 + 1 {
        let input = l.aig.input();
        acc = l.aig.and(acc, input);
    }
    match l.decide(&[acc], drat_check::Limits::default()) {
        SolverResult::Unknown(why) => assert!(why.contains("above drat-check's limit"), "{why}"),
        other => panic!("expected Unknown, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Literals: every `#b`, `#x` and `(_ bvN w)` form
// ---------------------------------------------------------------------------

/// The constant value of a lowered all-constant bit-vector.
fn constant_value(b: &BitVec) -> u128 {
    b.bits
        .iter()
        .enumerate()
        .map(|(i, e)| {
            assert!(e.is_constant(), "literal bit {i} is not a constant");
            if i < 128 {
                ((*e == Edge::TRUE) as u128) << i
            } else {
                assert_eq!(*e, Edge::FALSE, "bits above 128 of a decimal literal are zero");
                0
            }
        })
        .sum()
}

#[test]
fn every_small_literal_reads_back() {
    let mut l = Lowering::new(&SortEnv::new()).unwrap();
    let mut value_of = |p: Predicate| match l.term(&p).unwrap() {
        Term::Bv(b) => (b.width(), constant_value(&b)),
        Term::Bool(_) => panic!("expected a bit-vector"),
    };
    for w in 1..=8usize {
        for v in 0..(1u128 << w) {
            let bin = format!("#b{v:0w$b}");
            assert_eq!(value_of(var(&bin)), (w, v), "{bin}");
            assert_eq!(value_of(lit(w, v)), (w, v), "(_ bv{v} {w})");
        }
    }
    for digits in 1..=2usize {
        for v in 0..(1u128 << (4 * digits)) {
            let hex = format!("#x{v:0digits$x}");
            assert_eq!(value_of(var(&hex)), (4 * digits, v), "{hex}");
            // Upper-case digits are legal SMT-LIB hex.
            let upper = format!("#x{v:0digits$X}");
            assert_eq!(value_of(var(&upper)), (4 * digits, v), "{upper}");
        }
    }
    // The u128 boundary: a 128-bit constant of all ones, and 64-bit corners.
    assert_eq!(value_of(lit(128, u128::MAX)), (128, u128::MAX));
    assert_eq!(value_of(lit(127, mask(127))), (127, mask(127)));
    assert_eq!(value_of(lit(64, u64::MAX as u128)), (64, u64::MAX as u128));
    assert_eq!(value_of(var(&format!("#x{}", "f".repeat(32)))), (128, u128::MAX));
    // Above 128 bits the high bits of a decimal constant are zero.
    assert_eq!(value_of(lit(200, 5)), (200, 5));
}

#[test]
fn a_literal_one_bit_too_wide_is_refused() {
    let e = SortEnv::new();
    for p in [lit(4, 16), lit(1, 2), lit(127, 1u128 << 127)] {
        let q = eq(p.clone(), p);
        assert!(matches!(BvTactic::solve(&[q], &e), SolverResult::Unknown(w) if w.contains("does not fit")));
    }
}
