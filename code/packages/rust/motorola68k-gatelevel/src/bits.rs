//! Bit-vector conversion and ripple-carry adder helpers for the 68000 simulator.
//!
//! ## Representation
//!
//! All bit arrays use **LSB-first** ordering: `bits[0]` is the least-significant
//! bit, `bits[N-1]` is the most-significant bit.  This matches the arithmetic
//! crate's `full_adder` convention and makes ripple-carry wiring natural: stage
//! `i` receives `carry_in` from stage `i-1`.
//!
//! ```text
//! u8 value 0b10110101 (181)
//!   bits[0] = 1  (2^0 = 1)
//!   bits[1] = 0  (2^1 = 2)
//!   bits[2] = 1  (2^2 = 4)
//!   bits[3] = 0  (2^3 = 8)
//!   bits[4] = 1  (2^4 = 16)
//!   bits[5] = 1  (2^5 = 32)
//!   bits[6] = 0  (2^6 = 64)
//!   bits[7] = 1  (2^7 = 128)
//! ```
//!
//! ## Ripple-carry adder
//!
//! ```text
//! Bit 0: full_adder(a[0], b[0], cin)   → (sum[0], carry[0])
//! Bit 1: full_adder(a[1], b[1], carry[0]) → (sum[1], carry[1])
//! ...
//! Bit N-1: full_adder(a[N-1], b[N-1], carry[N-2]) → (sum[N-1], carry[N-1])
//!
//! carry[N-1] = carry out of MSB stage (used for C flag in ADD)
//! overflow   = XOR(carry[N-2], carry[N-1])  (detects signed wrap)
//! ```

use arithmetic::adders::full_adder;
use logic_gates::gates::{not_gate, or_gate, xor_gate};

// ── Integer → LSB-first bit vector ───────────────────────────────────────────

/// Convert a `u8` into an 8-element LSB-first bit vector.
///
/// ```
/// use coding_adventures_motorola68k_gatelevel::bits::int_to_bits8;
/// let bits = int_to_bits8(0b10110101);
/// assert_eq!(bits[0], 1); // LSB
/// assert_eq!(bits[7], 1); // MSB
/// ```
pub fn int_to_bits8(val: u8) -> Vec<u8> {
    (0..8).map(|i| (val >> i) & 1).collect()
}

/// Convert a `u16` into a 16-element LSB-first bit vector.
pub fn int_to_bits16(val: u16) -> Vec<u8> {
    (0..16).map(|i| ((val >> i) & 1) as u8).collect()
}

/// Convert a `u32` into a 32-element LSB-first bit vector.
pub fn int_to_bits32(val: u32) -> Vec<u8> {
    (0..32).map(|i| ((val >> i) & 1) as u8).collect()
}

// ── LSB-first bit vector → integer ───────────────────────────────────────────

/// Reconstruct a `u8` from an 8-element LSB-first bit vector.
pub fn bits_to_u8(bits: &[u8]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0u8, |acc, (i, &b)| acc | (b << i))
}

/// Reconstruct a `u16` from a 16-element LSB-first bit vector.
pub fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .enumerate()
        .fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i))
}

/// Reconstruct a `u32` from a 32-element LSB-first bit vector.
pub fn bits_to_u32(bits: &[u8]) -> u32 {
    bits.iter()
        .enumerate()
        .fold(0u32, |acc, (i, &b)| acc | ((b as u32) << i))
}

// ── Ripple-carry adders ───────────────────────────────────────────────────────

/// 8-stage ripple-carry adder.  Returns `(result, carries)`.
///
/// `carries[7]` is the carry out of the MSB stage (used for carry flag).
/// Overflow = `XOR(carries[6], carries[7])`.
///
/// `cin` = 0 for plain ADD; 1 for two's-complement SUB (invert `b` first).
pub fn add_8bit_full(a: u8, b: u8, cin: u8) -> (u8, Vec<u8>) {
    let a = int_to_bits8(a);
    let b = int_to_bits8(b);
    let mut carries = Vec::with_capacity(8);
    let mut sum_bits = Vec::with_capacity(8);
    let mut carry = cin;
    for i in 0..8 {
        let (s, co) = full_adder(a[i], b[i], carry);
        sum_bits.push(s);
        carries.push(co);
        carry = co;
    }
    (bits_to_u8(&sum_bits), carries)
}

/// 16-stage ripple-carry adder.  Returns `(result, carries)`.
///
/// `carries[15]` = carry out of MSB.  Overflow = `XOR(carries[14], carries[15])`.
pub fn add_16bit_full(a: u16, b: u16, cin: u8) -> (u16, Vec<u8>) {
    let a = int_to_bits16(a);
    let b = int_to_bits16(b);
    let mut carries = Vec::with_capacity(16);
    let mut sum_bits = Vec::with_capacity(16);
    let mut carry = cin;
    for i in 0..16 {
        let (s, co) = full_adder(a[i], b[i], carry);
        sum_bits.push(s);
        carries.push(co);
        carry = co;
    }
    (bits_to_u16(&sum_bits), carries)
}

/// 32-stage ripple-carry adder.  Returns `(result, carries)`.
///
/// `carries[31]` = carry out of MSB.  Overflow = `XOR(carries[30], carries[31])`.
pub fn add_32bit_full(a: u32, b: u32, cin: u8) -> (u32, Vec<u8>) {
    let a = int_to_bits32(a);
    let b = int_to_bits32(b);
    let mut carries = Vec::with_capacity(32);
    let mut sum_bits = Vec::with_capacity(32);
    let mut carry = cin;
    for i in 0..32 {
        let (s, co) = full_adder(a[i], b[i], carry);
        sum_bits.push(s);
        carries.push(co);
        carry = co;
    }
    (bits_to_u32(&sum_bits), carries)
}

// ── Flag helpers ──────────────────────────────────────────────────────────────

/// Compute overflow flag: `XOR(carries[N-2], carries[N-1])`.
///
/// This single XOR gate at the MSB position detects signed overflow.
/// If the carry INTO the sign bit differs from the carry OUT of the sign bit,
/// the sign of the result is wrong (overflow occurred).
///
/// Precondition: `carries.len() >= 2`.
pub fn compute_v_from_carries(carries: &[u8]) -> u8 {
    let n = carries.len();
    xor_gate(carries[n - 2], carries[n - 1])
}

/// Compute the N (negative) flag: the MSB of an 8-bit result.
pub fn compute_n8(result: u8) -> u8 {
    (result >> 7) & 1
}

/// Compute the N (negative) flag: the MSB of a 16-bit result.
pub fn compute_n16(result: u16) -> u8 {
    ((result >> 15) & 1) as u8
}

/// Compute the N (negative) flag: the MSB of a 32-bit result.
pub fn compute_n32(result: u32) -> u8 {
    ((result >> 31) & 1) as u8
}

/// Compute the Z (zero) flag from an LSB-first bit vector.
///
/// Zero flag = NOR of all bits (all bits must be 0).
/// Gate-level: a chain of OR gates that feeds into a NOT.
///
/// ```
/// use coding_adventures_motorola68k_gatelevel::bits::compute_z;
/// assert_eq!(compute_z(&[0, 0, 0, 0, 0, 0, 0, 0]), 1); // zero
/// assert_eq!(compute_z(&[1, 0, 0, 0, 0, 0, 0, 0]), 0); // nonzero
/// ```
pub fn compute_z(bits: &[u8]) -> u8 {
    let any_one = bits.iter().fold(0u8, |acc, &b| or_gate(acc, b));
    not_gate(any_one)
}

/// Compute the Z flag for a 32-bit value using compute_z on its bit vector.
pub fn compute_z32(val: u32) -> u8 {
    compute_z(&int_to_bits32(val))
}

/// Compute the Z flag for a 16-bit value.
pub fn compute_z16(val: u16) -> u8 {
    compute_z(&int_to_bits16(val))
}

/// Compute the Z flag for an 8-bit value.
pub fn compute_z8(val: u8) -> u8 {
    compute_z(&int_to_bits8(val))
}

// ── Bitwise NOT helpers ───────────────────────────────────────────────────────

/// Bitwise NOT of a `u8` — routes every bit through a NOT gate.
///
/// Used to produce the two's-complement addend for SUB: `NOT(b)` with carry-in=1.
///
/// ```
/// use coding_adventures_motorola68k_gatelevel::bits::not_8bit;
/// assert_eq!(not_8bit(0xFF), 0x00);
/// assert_eq!(not_8bit(0x00), 0xFF);
/// assert_eq!(not_8bit(0xAA), 0x55);
/// ```
pub fn not_8bit(val: u8) -> u8 {
    let bits = int_to_bits8(val);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u8(&inv)
}

/// Bitwise NOT of a `u16`.
pub fn not_16bit(val: u16) -> u16 {
    let bits = int_to_bits16(val);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u16(&inv)
}

/// Bitwise NOT of a `u32`.
pub fn not_32bit(val: u32) -> u32 {
    let bits = int_to_bits32(val);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u32(&inv)
}

// ── Parity helper (unused by 68K but included for completeness) ───────────────

/// Compute even parity over a bit slice via a XOR tree.
/// Returns 1 if the count of 1-bits is even, 0 if odd.
/// (Not used by the 68000, which has no PF flag.)
pub fn compute_parity(bits: &[u8]) -> u8 {
    let xor_all = bits.iter().fold(0u8, |acc, &b| xor_gate(acc, b));
    not_gate(xor_all) // 1 = even parity
}

// ── NEG-specific flag helpers ─────────────────────────────────────────────────

/// For the NEG instruction: carry = NOT(zero) i.e. C=1 when result is nonzero.
///
/// Gate-level: C = OR-reduction of all result bits.
/// The 68000 special case: `NEG src` sets C = (result != 0).
pub fn compute_c_neg(result_bits: &[u8]) -> u8 {
    result_bits.iter().fold(0u8, |acc, &b| or_gate(acc, b))
}

/// For the NEG instruction: overflow = (src == MSB-only pattern).
///
/// Overflow occurs only when negating the most-negative value (0x80, 0x8000,
/// or 0x80000000), because `-(-128) = -128` in two's complement.
///
/// Gate-level: AND(src[MSB]=1, NOT(src[MSB-1])=1, ..., NOT(src[0])=1).
/// `src_bits` must be LSB-first; `bits` is the width (8, 16, or 32).
pub fn compute_v_neg(src_bits: &[u8], bits: usize) -> u8 {
    // MSB is src_bits[bits-1]; lower bits must all be 0.
    let msb = src_bits[bits - 1];
    let lower_all_zero = src_bits[..bits - 1]
        .iter()
        .fold(1u8, |acc, &b| {
            // acc = 1 so far means "all zero"; stays 1 only if current bit is 0
            // AND(acc, NOT(b))
            let inv = not_gate(b);
            // acc & inv
            let one_bit = not_gate(not_gate(acc)); // identity (just acc, but makes gate explicit)
            // Use: 1 AND NOT(b) = NOT(OR(NOT(1), b)) ... simplify: just and_gate
            use logic_gates::gates::and_gate;
            and_gate(one_bit, inv)
        });
    use logic_gates::gates::and_gate;
    and_gate(msb, lower_all_zero)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_u8() {
        for v in [0u8, 1, 127, 128, 255] {
            assert_eq!(bits_to_u8(&int_to_bits8(v)), v);
        }
    }

    #[test]
    fn round_trip_u32() {
        for v in [0u32, 1, 0x7FFF_FFFF, 0x8000_0000, 0xFFFF_FFFF] {
            assert_eq!(bits_to_u32(&int_to_bits32(v)), v);
        }
    }

    #[test]
    fn add_8bit_basic() {
        let (r, c) = add_8bit_full(10, 20, 0);
        assert_eq!(r, 30);
        assert_eq!(c[7], 0); // no carry
    }

    #[test]
    fn add_8bit_overflow_carry() {
        let (r, c) = add_8bit_full(0xFF, 1, 0);
        assert_eq!(r, 0);
        assert_eq!(c[7], 1); // carry out
    }

    #[test]
    fn add_32bit_basic() {
        let (r, c) = add_32bit_full(0x7FFF_FFFF, 1, 0);
        assert_eq!(r, 0x8000_0000);
        assert_eq!(c[31], 0); // no unsigned carry
        assert_eq!(compute_v_from_carries(&c), 1); // but signed overflow!
    }

    #[test]
    fn not_8bit_correct() {
        assert_eq!(not_8bit(0xAA), 0x55);
        assert_eq!(not_8bit(0xFF), 0x00);
        assert_eq!(not_8bit(0x00), 0xFF);
    }

    #[test]
    fn compute_z_zero() {
        let bits = int_to_bits8(0);
        assert_eq!(compute_z(&bits), 1);
        let bits = int_to_bits8(1);
        assert_eq!(compute_z(&bits), 0);
    }

    #[test]
    fn sub_via_twos_complement_8() {
        // 5 - 3 = 2: invert b, add 1 as carry-in
        let a = 5u8;
        let b = not_8bit(3u8);
        let (result, carries) = add_8bit_full(a, b, 1);
        assert_eq!(result, 2);
        assert_eq!(carries[7], 1); // carry out → C flag = NOT(carry) = 0 (no borrow)
    }

    #[test]
    fn sub_via_twos_complement_8_borrow() {
        // 3 - 5: borrow expected
        let a = 3u8;
        let b = not_8bit(5u8);
        let (result, carries) = add_8bit_full(a, b, 1);
        assert_eq!(result, 0xFEu8); // 3 - 5 = -2 = 0xFE
        assert_eq!(carries[7], 0); // no carry out → C flag = NOT(carry) = 1 (borrow)
    }

    #[test]
    fn compute_v_neg_byte() {
        // NEG 0x80 → overflow (most-negative byte)
        let src = int_to_bits8(0x80);
        assert_eq!(compute_v_neg(&src, 8), 1);
        // NEG 0x01 → no overflow
        let src = int_to_bits8(0x01);
        assert_eq!(compute_v_neg(&src, 8), 0);
    }
}
