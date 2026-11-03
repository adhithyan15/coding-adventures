//! # Every operation, every width up to 8, every pair of operands
//!
//! The circuits in `bv.rs` are checked against plain Rust integer
//! arithmetic, the most independent oracle available. For widths 1–8 the
//! check is **exhaustive**: all 2^(2w) operand pairs, up to 65 536 per
//! operation and width, evaluated 64 at a time by the AIG's bit-parallel
//! simulator. Widths 9–64 get random operands.
//!
//! SMT-LIB semantics are spelled out in the reference functions below:
//! wrap-around modulo 2^w, shifts of `w` or more give 0 (or all sign bits),
//! signed comparisons in two's complement.

use bit_blaster::aig::{edge_value, Aig, Edge};
use bit_blaster::bv::{self, BitVec};

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// Two's-complement reading of a `w`-bit value.
fn signed(x: u128, w: u32) -> i128 {
    if x >> (w - 1) & 1 == 1 {
        x as i128 - (1i128 << w)
    } else {
        x as i128
    }
}

fn shift_amount(s: u128, w: u32) -> Option<u32> {
    (s < w as u128).then_some(s as u32)
}

type VecOp = fn(&mut Aig, &BitVec, &BitVec) -> BitVec;
type BoolOp = fn(&mut Aig, &BitVec, &BitVec) -> Edge;
type Reference = fn(u128, u128, u32) -> u128;

/// The operations returning bit-vectors, with their reference semantics.
fn vector_ops() -> Vec<(&'static str, VecOp, Reference)> {
    vec![
        ("and", bv::and, |a, b, _| a & b),
        ("or", bv::or, |a, b, _| a | b),
        ("xor", bv::xor, |a, b, _| a ^ b),
        ("not", |_, a, _| bv::not(a), |a, _, w| !a & mask(w)),
        ("add", bv::add, |a, b, w| a.wrapping_add(b) & mask(w)),
        ("sub", bv::sub, |a, b, w| a.wrapping_sub(b) & mask(w)),
        ("neg", |g, a, _| bv::neg(g, a), |a, _, w| a.wrapping_neg() & mask(w)),
        ("mul", bv::mul, |a, b, w| a.wrapping_mul(b) & mask(w)),
        ("shl", bv::shl, |a, s, w| shift_amount(s, w).map_or(0, |k| (a << k) & mask(w))),
        ("lshr", bv::lshr, |a, s, w| shift_amount(s, w).map_or(0, |k| a >> k)),
        ("ashr", bv::ashr, |a, s, w| {
            let k = shift_amount(s, w).unwrap_or(w - 1).min(w - 1);
            let shifted = if shift_amount(s, w).is_none() { signed(a, w) >> (w - 1) } else { signed(a, w) >> k };
            shifted as u128 & mask(w)
        }),
        ("ite(a<b)", |g, a, b| {
            let c = bv::ult(g, a, b);
            bv::ite(g, c, a, b)
        }, |a, b, _| a.min(b)),
    ]
}

/// The operations returning Booleans.
fn bool_ops() -> Vec<(&'static str, BoolOp, Reference)> {
    vec![
        ("eq", bv::eq, |a, b, _| (a == b) as u128),
        ("ult", bv::ult, |a, b, _| (a < b) as u128),
        ("ule", bv::ule, |a, b, _| (a <= b) as u128),
        ("ugt", bv::ugt, |a, b, _| (a > b) as u128),
        ("uge", bv::uge, |a, b, _| (a >= b) as u128),
        ("slt", bv::slt, |a, b, w| (signed(a, w) < signed(b, w)) as u128),
        ("sle", bv::sle, |a, b, w| (signed(a, w) <= signed(b, w)) as u128),
        ("sgt", bv::sgt, |a, b, w| (signed(a, w) > signed(b, w)) as u128),
        ("sge", bv::sge, |a, b, w| (signed(a, w) >= signed(b, w)) as u128),
    ]
}

/// Build the circuit for one operation at width `w`; returns the AIG and
/// the output edges (one edge for Boolean operations).
fn build(w: u32, vec_op: Option<VecOp>, bool_op: Option<BoolOp>) -> (Aig, Vec<Edge>) {
    let mut g = Aig::new();
    let a = BitVec::input(&mut g, w as usize);
    let b = BitVec::input(&mut g, w as usize);
    let out = match (vec_op, bool_op) {
        (Some(f), _) => f(&mut g, &a, &b).bits,
        (_, Some(f)) => vec![f(&mut g, &a, &b)],
        _ => unreachable!(),
    };
    (g, out)
}

/// Evaluate the circuit on a batch of up to 64 operand pairs.
fn run_batch(g: &Aig, out: &[Edge], w: u32, pairs: &[(u128, u128)]) -> Vec<u128> {
    let mut inputs = vec![0u64; 2 * w as usize];
    for (p, &(a, b)) in pairs.iter().enumerate() {
        for i in 0..w as usize {
            inputs[i] |= ((a >> i & 1) as u64) << p;
            inputs[w as usize + i] |= ((b >> i & 1) as u64) << p;
        }
    }
    let values = g.simulate(&inputs);
    (0..pairs.len())
        .map(|p| out.iter().enumerate().map(|(i, &e)| ((edge_value(&values, e) >> p & 1) as u128) << i).sum())
        .collect()
}

fn check_all(w: u32, pairs: &[(u128, u128)]) -> usize {
    let mut checked = 0;
    let cases: Vec<(&str, Option<VecOp>, Option<BoolOp>, Reference)> = vector_ops()
        .into_iter()
        .map(|(n, f, r)| (n, Some(f), None, r))
        .chain(bool_ops().into_iter().map(|(n, f, r)| (n, None, Some(f), r)))
        .collect();
    for (name, vf, bf, reference) in cases {
        let (g, out) = build(w, vf, bf);
        for batch in pairs.chunks(64) {
            let got = run_batch(&g, &out, w, batch);
            for (&(a, b), &v) in batch.iter().zip(&got) {
                let want = reference(a, b, w);
                assert_eq!(v, want, "{name} at width {w}: a={a} b={b}");
                checked += 1;
            }
        }
    }
    checked
}

#[test]
fn widths_one_to_eight_exhaustively() {
    let mut checked = 0;
    for w in 1..=8u32 {
        let n = 1u128 << w;
        let pairs: Vec<(u128, u128)> = (0..n).flat_map(|a| (0..n).map(move |b| (a, b))).collect();
        checked += check_all(w, &pairs);
    }
    // 21 operations × Σ 4^w for w = 1..8.
    assert_eq!(checked, 21 * (1..=8).map(|w| 4usize.pow(w)).sum::<usize>());
}

#[test]
fn wide_widths_on_random_operands() {
    let mut state = 0x0123_4567_89AB_CDEFu64;
    let mut next = || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state as u128
    };
    for w in [9u32, 12, 16, 17, 31, 32, 33, 48, 63, 64] {
        let pairs: Vec<(u128, u128)> = (0..256)
            .map(|i| {
                let a = next() & mask(w);
                // Small shift amounts and equal operands need to appear too.
                let b = match i % 4 {
                    0 => next() % (w as u128 + 2),
                    1 => a,
                    _ => next() & mask(w),
                };
                (a, b)
            })
            .collect();
        check_all(w, &pairs);
    }
}
