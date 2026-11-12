//! # Bit-blasting: fixed-width integers as circuits
//!
//! A `w`-bit integer is `w` Boolean signals. Every operation on it becomes
//! the circuit a hardware designer would draw. This is exact: no
//! approximation, no overflow guesswork. A 32-bit `bvadd` *is* a 32-bit
//! ripple-carry adder, and it wraps modulo 2³² because the carry out of the
//! top bit has nowhere to go.
//!
//! A [`BitVec`] is a vector of AIG edges, **least significant bit first**:
//!
//! ```text
//!   value 6 in 4 bits = 0110₂   →   bits[0] = 0, bits[1] = 1, bits[2] = 1, bits[3] = 0
//! ```
//!
//! Semantics are SMT-LIB `QF_BV`'s, including its edge cases (VER10 §4):
//! shifts by the width or more give all zeros (all copies of the sign bit
//! for `ashr`), and everything wraps. Both operands of a binary operation
//! must have the same width; a mismatch panics, because it is a bug in the
//! caller. The QF_BV tactic in `constraint-engine` checks widths before it gets here,
//! so hostile input never reaches the panic.
//!
//! ## The adder, the heart of it
//!
//! A **full adder** adds two bits and a carry-in:
//!
//! ```text
//!   sum   = a ⊕ b ⊕ cin
//!   cout  = (a ∧ b) ∨ (cin ∧ (a ⊕ b))
//!
//!   a b cin │ sum cout
//!   0 0 0   │  0   0
//!   0 1 0   │  1   0
//!   1 1 0   │  0   1
//!   1 1 1   │  1   1
//! ```
//!
//! Chain `w` of them, each one's carry-out into the next one's carry-in,
//! and you have a ripple-carry adder. Subtraction reuses it:
//! `a − b = a + ¬b + 1` in two's complement, so the `+ 1` is just a carry-in
//! of 1. The carry out of the top bit of `a + ¬b + 1` is 1 exactly when
//! `a ≥ b` unsigned, which is how comparisons are built.

use crate::aig::{Aig, Edge};

/// A fixed-width bit-vector: AIG edges, least significant bit first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitVec {
    /// The bits, `bits[0]` least significant.
    pub bits: Vec<Edge>,
}

impl BitVec {
    /// `width` fresh inputs.
    pub fn input(aig: &mut Aig, width: usize) -> BitVec {
        BitVec { bits: (0..width).map(|_| aig.input()).collect() }
    }

    /// The constant `value` truncated to `width` bits (bits above 128 are 0).
    pub fn constant(width: usize, value: u128) -> BitVec {
        BitVec { bits: (0..width).map(|i| Edge::constant(i < 128 && value >> i & 1 == 1)).collect() }
    }

    /// A constant from explicit bits, least significant first.
    pub fn from_bools(bits: &[bool]) -> BitVec {
        BitVec { bits: bits.iter().map(|&b| Edge::constant(b)).collect() }
    }

    /// The width in bits.
    pub fn width(&self) -> usize {
        self.bits.len()
    }

    /// The most significant (sign) bit.
    pub fn msb(&self) -> Edge {
        *self.bits.last().expect("bit-vectors have width ≥ 1")
    }
}

fn same_width(a: &BitVec, b: &BitVec) {
    assert_eq!(a.width(), b.width(), "bit-vector width mismatch");
}

/// Apply a two-input gate bit by bit.
fn bitwise(aig: &mut Aig, a: &BitVec, b: &BitVec, gate: fn(&mut Aig, Edge, Edge) -> Edge) -> BitVec {
    same_width(a, b);
    BitVec { bits: a.bits.iter().zip(&b.bits).map(|(&x, &y)| gate(aig, x, y)).collect() }
}

/// Bitwise NOT: free, just inverted edges.
pub fn not(a: &BitVec) -> BitVec {
    BitVec { bits: a.bits.iter().map(|&x| !x).collect() }
}

/// Bitwise AND.
pub fn and(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    bitwise(aig, a, b, Aig::and)
}

/// Bitwise OR.
pub fn or(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    bitwise(aig, a, b, Aig::or)
}

/// Bitwise XOR.
pub fn xor(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    bitwise(aig, a, b, Aig::xor)
}

/// `a + b + carry_in`, returning the sum and the carry out of the top bit.
fn add_with_carry(aig: &mut Aig, a: &BitVec, b: &BitVec, carry_in: Edge) -> (BitVec, Edge) {
    same_width(a, b);
    let mut carry = carry_in;
    let mut bits = Vec::with_capacity(a.width());
    for (&x, &y) in a.bits.iter().zip(&b.bits) {
        let half = aig.xor(x, y);
        bits.push(aig.xor(half, carry));
        let both = aig.and(x, y);
        let propagated = aig.and(carry, half);
        carry = aig.or(both, propagated);
    }
    (BitVec { bits }, carry)
}

/// `a + b` modulo 2^w (ripple carry).
pub fn add(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    add_with_carry(aig, a, b, Edge::FALSE).0
}

/// `a − b` modulo 2^w, as `a + ¬b + 1`.
pub fn sub(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    add_with_carry(aig, a, &not(b), Edge::TRUE).0
}

/// `−a` modulo 2^w, as `0 − a`.
pub fn neg(aig: &mut Aig, a: &BitVec) -> BitVec {
    sub(aig, &BitVec::constant(a.width(), 0), a)
}

/// `a · b` modulo 2^w: shift-and-add, keeping the low `w` bits.
///
/// Row `i` is `a << i`, kept only where `b[i]` is 1; the rows are summed.
/// That is `w` adders of `w` bits: quadratic, the price of exactness.
pub fn mul(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    same_width(a, b);
    let w = a.width();
    let mut acc = BitVec::constant(w, 0);
    for (i, &bit) in b.bits.iter().enumerate() {
        let shifted = shl_const(a, i);
        let row = BitVec {
            bits: shifted.bits.iter().map(|&x| aig.and(x, bit)).collect(),
        };
        acc = add(aig, &acc, &row);
    }
    acc
}

/// `a << k` for a constant `k`: rewiring, no gates. `k ≥ w` gives zero.
pub fn shl_const(a: &BitVec, k: usize) -> BitVec {
    let w = a.width();
    BitVec { bits: (0..w).map(|i| if i >= k { a.bits[i - k] } else { Edge::FALSE }).collect() }
}

/// Logical `a >> k` for a constant `k` (zeros shift in).
pub fn lshr_const(a: &BitVec, k: usize) -> BitVec {
    let w = a.width();
    BitVec { bits: (0..w).map(|i| if i.saturating_add(k) < w { a.bits[i + k] } else { Edge::FALSE }).collect() }
}

/// Arithmetic `a >> k` for a constant `k` (copies of the sign bit shift in).
pub fn ashr_const(a: &BitVec, k: usize) -> BitVec {
    let w = a.width();
    let sign = a.msb();
    BitVec { bits: (0..w).map(|i| if i.saturating_add(k) < w { a.bits[i + k] } else { sign }).collect() }
}

/// A shift by a variable amount: a **barrel shifter**.
///
/// The amount is a bit-vector `s`. Stage `k` shifts by `2^k` when `s[k]` is
/// 1, so the stages together shift by exactly `s`. Bits of `s` whose weight
/// `2^k` is at least `w` mean "shifted out entirely"; they force the
/// overflow value (zero, or all sign bits).
///
/// ```text
///   s = 0b101 (5) on 8 bits:  stage 0 shifts by 1, stage 1 skipped, stage 2 shifts by 4
/// ```
fn barrel(
    aig: &mut Aig,
    a: &BitVec,
    s: &BitVec,
    by_const: fn(&BitVec, usize) -> BitVec,
    overflow_bit: Edge,
) -> BitVec {
    same_width(a, s);
    let w = a.width();
    let mut x = a.clone();
    let mut overflow = Edge::FALSE;
    for (k, &bit) in s.bits.iter().enumerate() {
        if k < usize::BITS as usize && (1usize << k) < w {
            let shifted = by_const(&x, 1 << k);
            x = BitVec { bits: x.bits.iter().zip(&shifted.bits).map(|(&keep, &moved)| aig.mux(bit, moved, keep)).collect() };
        } else {
            overflow = aig.or(overflow, bit);
        }
    }
    BitVec { bits: x.bits.iter().map(|&b| aig.mux(overflow, overflow_bit, b)).collect() }
}

/// `a << s` (SMT-LIB `bvshl`).
pub fn shl(aig: &mut Aig, a: &BitVec, s: &BitVec) -> BitVec {
    barrel(aig, a, s, shl_const, Edge::FALSE)
}

/// Logical `a >> s` (SMT-LIB `bvlshr`).
pub fn lshr(aig: &mut Aig, a: &BitVec, s: &BitVec) -> BitVec {
    barrel(aig, a, s, lshr_const, Edge::FALSE)
}

/// Arithmetic `a >> s` (SMT-LIB `bvashr`).
pub fn ashr(aig: &mut Aig, a: &BitVec, s: &BitVec) -> BitVec {
    let sign = a.msb();
    barrel(aig, a, s, ashr_const, sign)
}

/// `a = b`: every bit equal.
pub fn eq(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    same_width(a, b);
    let same: Vec<Edge> = a.bits.iter().zip(&b.bits).map(|(&x, &y)| aig.xnor(x, y)).collect();
    aig.and_all(&same)
}

/// Unsigned `a < b`: no carry out of `a + ¬b + 1` (it borrowed).
pub fn ult(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    !add_with_carry(aig, a, &not(b), Edge::TRUE).1
}

/// Unsigned `a ≤ b`, as `¬(b < a)`.
pub fn ule(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    !ult(aig, b, a)
}

/// Unsigned `a > b`.
pub fn ugt(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    ult(aig, b, a)
}

/// Unsigned `a ≥ b`.
pub fn uge(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    ule(aig, b, a)
}

/// Flip the sign bit. Signed order on `w` bits is unsigned order after
/// adding 2^(w−1), which flips the top bit: −128..127 becomes 0..255.
fn flip_sign(a: &BitVec) -> BitVec {
    assert!(a.width() >= 1, "bit-vectors have width ≥ 1");
    let mut bits = a.bits.clone();
    let top = bits.len() - 1;
    bits[top] = !bits[top];
    BitVec { bits }
}

/// Signed `a < b`.
pub fn slt(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    ult(aig, &flip_sign(a), &flip_sign(b))
}

/// Signed `a ≤ b`.
pub fn sle(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    ule(aig, &flip_sign(a), &flip_sign(b))
}

/// Signed `a > b`.
pub fn sgt(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    slt(aig, b, a)
}

/// Signed `a ≥ b`.
pub fn sge(aig: &mut Aig, a: &BitVec, b: &BitVec) -> Edge {
    sle(aig, b, a)
}

/// `if c then t else e`, bit by bit.
pub fn ite(aig: &mut Aig, c: Edge, t: &BitVec, e: &BitVec) -> BitVec {
    same_width(t, e);
    BitVec { bits: t.bits.iter().zip(&e.bits).map(|(&x, &y)| aig.mux(c, x, y)).collect() }
}

/// Restoring long division: `(⌊a / b⌋, a mod b)` for unsigned `a`, `b`.
///
/// This is schoolbook division in binary. Walk the dividend from its top
/// bit down, keeping a running remainder `r`. At each step shift the next
/// dividend bit into `r`; if `r ≥ b`, subtract `b` and write a 1 in the
/// quotient, otherwise write a 0.
///
/// ```text
///   13 ÷ 3 in 4 bits (a = 1101, b = 0011):
///   bit 3: r = 1          < 3  → q3 = 0
///   bit 2: r = 11  (3)   ≥ 3  → q2 = 1, r = 0
///   bit 1: r = 0          < 3  → q1 = 0
///   bit 0: r = 1          < 3  → q0 = 0
///   quotient 0100 (4), remainder 1
/// ```
///
/// `r` is kept in `w + 1` bits because `2r + bit` can reach `2^(w+1) − 2`
/// before the subtraction; after it, `r < b < 2^w` again.
///
/// **Division by zero** needs no special case and lands exactly on SMT-LIB's
/// total definition: `r ≥ 0` always holds, so every quotient bit is 1
/// (`bvudiv a 0 = 2^w − 1`) and nothing is ever subtracted, so the
/// remainder is `a` itself (`bvurem a 0 = a`).
fn udivrem(aig: &mut Aig, a: &BitVec, b: &BitVec) -> (BitVec, BitVec) {
    same_width(a, b);
    let w = a.width();
    assert!(w >= 1, "bit-vectors have width ≥ 1");
    let b_wide = zext(b, 1);
    let mut r = BitVec::constant(w + 1, 0);
    let mut q = vec![Edge::FALSE; w];
    for i in (0..w).rev() {
        // r ← (r << 1) | a[i]; the old top bit is always 0 (see above).
        let mut bits = Vec::with_capacity(w + 1);
        bits.push(a.bits[i]);
        bits.extend_from_slice(&r.bits[..w]);
        let shifted = BitVec { bits };
        let ge = uge(aig, &shifted, &b_wide);
        let diff = sub(aig, &shifted, &b_wide);
        r = ite(aig, ge, &diff, &shifted);
        q[i] = ge;
    }
    (BitVec { bits: q }, extract(&r, w - 1, 0))
}

/// Unsigned `a / b` (SMT-LIB `bvudiv`; `a / 0 = 2^w − 1`).
pub fn udiv(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    udivrem(aig, a, b).0
}

/// Unsigned `a mod b` (SMT-LIB `bvurem`; `a mod 0 = a`).
pub fn urem(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    udivrem(aig, a, b).1
}

/// `|a|` read as an unsigned value: `−a` if the sign bit is set. The most
/// negative number maps to itself, which read unsigned is `2^(w−1)`: right.
fn abs(aig: &mut Aig, a: &BitVec) -> BitVec {
    negate_if(aig, a.msb(), a)
}

/// `if flip then −x else x`.
fn negate_if(aig: &mut Aig, flip: Edge, x: &BitVec) -> BitVec {
    let negated = neg(aig, x);
    ite(aig, flip, &negated, x)
}

/// Signed `a / b`, truncating toward zero (SMT-LIB `bvsdiv`).
///
/// Divide the magnitudes, then negate when the signs differ. `a / 0` is
/// `−1` for `a ≥ 0` and `1` for `a < 0`, which this construction yields
/// without a special case (see [`udiv`]).
pub fn sdiv(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    let (abs_a, abs_b) = (abs(aig, a), abs(aig, b));
    let q = udiv(aig, &abs_a, &abs_b);
    let differ = aig.xor(a.msb(), b.msb());
    negate_if(aig, differ, &q)
}

/// Signed remainder; the sign follows the dividend (SMT-LIB `bvsrem`).
pub fn srem(aig: &mut Aig, a: &BitVec, b: &BitVec) -> BitVec {
    let (abs_a, abs_b) = (abs(aig, a), abs(aig, b));
    let r = urem(aig, &abs_a, &abs_b);
    negate_if(aig, a.msb(), &r)
}

/// Bits `hi` down to `lo` of `a` (SMT-LIB `(_ extract hi lo)`), width
/// `hi − lo + 1`.
///
/// # Panics
///
/// Panics unless `lo ≤ hi < a.width()`.
pub fn extract(a: &BitVec, hi: usize, lo: usize) -> BitVec {
    assert!(lo <= hi && hi < a.width(), "extract needs lo ≤ hi < width");
    BitVec { bits: a.bits[lo..=hi].to_vec() }
}

/// `hi · 2^w_lo + lo` (SMT-LIB `concat`): `lo` supplies the low bits.
pub fn concat(hi: &BitVec, lo: &BitVec) -> BitVec {
    BitVec { bits: lo.bits.iter().chain(&hi.bits).copied().collect() }
}

/// `a` widened by `extra` zero bits (SMT-LIB `zero_extend`).
pub fn zext(a: &BitVec, extra: usize) -> BitVec {
    let mut bits = a.bits.clone();
    bits.extend(std::iter::repeat_n(Edge::FALSE, extra));
    BitVec { bits }
}

/// `a` widened by `extra` copies of its sign bit (SMT-LIB `sign_extend`).
pub fn sext(a: &BitVec, extra: usize) -> BitVec {
    let sign = a.msb();
    let mut bits = a.bits.clone();
    bits.extend(std::iter::repeat_n(sign, extra));
    BitVec { bits }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Evaluate a bit-vector on one assignment of the AIG's inputs.
    fn value(aig: &Aig, v: &BitVec, inputs: &[bool]) -> u64 {
        v.bits.iter().enumerate().map(|(i, &e)| (aig.eval(e, inputs) as u64) << i).sum()
    }

    #[test]
    fn constants() {
        let g = Aig::new();
        let c = BitVec::constant(4, 6);
        assert_eq!(value(&g, &c, &[]), 6);
        assert_eq!(BitVec::constant(130, u128::MAX).bits[129], Edge::FALSE);
        assert_eq!(BitVec::from_bools(&[true, false]), BitVec::constant(2, 1));
        assert_eq!(c.msb(), Edge::FALSE);
    }

    #[test]
    #[should_panic(expected = "width mismatch")]
    fn width_mismatch_panics() {
        let mut g = Aig::new();
        let a = BitVec::input(&mut g, 3);
        let b = BitVec::input(&mut g, 4);
        add(&mut g, &a, &b);
    }

    #[test]
    fn a_few_values_by_hand() {
        let mut g = Aig::new();
        let a = BitVec::input(&mut g, 4);
        let b = BitVec::input(&mut g, 4);
        let s = add(&mut g, &a, &b);
        let d = sub(&mut g, &a, &b);
        let lt = slt(&mut g, &a, &b);
        // a = 0b1110 (14 / −2), b = 0b0011 (3)
        let ins = [false, true, true, true, true, true, false, false];
        assert_eq!(value(&g, &s, &ins), (14 + 3) % 16);
        assert_eq!(value(&g, &d, &ins), 11);
        assert!(g.eval(lt, &ins), "−2 < 3 signed");
    }

    #[test]
    fn huge_constant_shifts_are_total() {
        let mut g = Aig::new();
        let a = BitVec::input(&mut g, 4);
        assert_eq!(shl_const(&a, usize::MAX), BitVec::constant(4, 0));
        assert_eq!(lshr_const(&a, usize::MAX), BitVec::constant(4, 0));
        assert_eq!(ashr_const(&a, usize::MAX).bits, vec![a.msb(); 4]);
    }
}
