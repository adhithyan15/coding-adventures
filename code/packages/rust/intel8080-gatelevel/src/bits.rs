//! Bit conversion helpers — the bridge between integers and gate-level bit vectors.
//!
//! # Bit ordering: LSB-first
//!
//! All bit vectors are LSB-first: `bits[0]` is the least significant bit (value 1),
//! `bits[7]` is the most significant bit (value 128). This matches the `arithmetic`
//! crate's ripple-carry adder, where carry propagates from bit 0 toward bit 7.
//!
//! ```text
//! int_to_bits8(5) → [1, 0, 1, 0, 0, 0, 0, 0]
//!                    ↑ bit0=1 (×1)  ↑ bit2=1 (×4) → sum = 5
//! ```
//!
//! # Auxiliary carry (AC)
//!
//! The 8080 has an Auxiliary Carry flag that records the carry out of bit 3
//! (the low nibble) into bit 4 (the high nibble). It is used by the DAA
//! (Decimal Adjust Accumulator) instruction.
//!
//! In the ripple-carry adder, `carries[3]` is the carry OUT of stage 3, which
//! is exactly the carry into bit 4. We capture this as `ac` in `add_8bit`.

use arithmetic::adders::full_adder;
use logic_gates::gates::{not_gate, xor_n};

// ─── 8-bit helpers ──────────────────────────────────────────────────────────

/// Convert an 8-bit integer to an 8-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_intel8080_gatelevel::bits::int_to_bits8;
/// let b = int_to_bits8(5);
/// assert_eq!(b, vec![1, 0, 1, 0, 0, 0, 0, 0]);
/// ```
pub fn int_to_bits8(value: u8) -> Vec<u8> {
    (0..8u8).map(|i| (value >> i) & 1).collect()
}

/// Convert a 16-bit integer to a 16-element LSB-first bit vector.
pub fn int_to_bits16(value: u16) -> Vec<u8> {
    (0..16u8).map(|i| ((value >> i) & 1) as u8).collect()
}

/// Convert an LSB-first 8-element bit vector back to a `u8`.
///
/// # Example
/// ```
/// use coding_adventures_intel8080_gatelevel::bits::bits_to_u8;
/// assert_eq!(bits_to_u8(&[1, 0, 1, 0, 0, 0, 0, 0]), 5);
/// ```
pub fn bits_to_u8(bits: &[u8]) -> u8 {
    bits.iter()
        .take(8)
        .enumerate()
        .fold(0u8, |acc, (i, &b)| acc | (b << i))
}

/// Convert an LSB-first 16-element bit vector back to a `u16`.
pub fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .take(16)
        .enumerate()
        .fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i))
}

// ─── Flag helpers ────────────────────────────────────────────────────────────

/// Compute even parity via a 7-gate XOR chain + NOT.
///
/// Returns 1 when the number of 1-bits is **even** (even parity = P flag set).
///
/// ```text
/// xor_chain = XOR(b[0], XOR(b[1], ... XOR(b[6], b[7])))
/// P = NOT(xor_chain)   ← 0 from even-count XOR → P = 1
/// ```
pub fn compute_parity(bits: &[u8]) -> u8 {
    if bits.is_empty() { return 1; }
    if bits.len() == 1 { return not_gate(bits[0]); }
    not_gate(xor_n(bits))
}

/// Zero detection: returns 1 when all bits are 0.
///
/// In hardware this is a NOR tree; here we use a fold.
pub fn compute_zero(bits: &[u8]) -> u8 {
    u8::from(bits.iter().all(|&b| b == 0))
}

// ─── 8-bit addition ──────────────────────────────────────────────────────────

/// 8-bit addition through 8 full-adder stages.
///
/// Returns `(result, carry_out, aux_carry)` where:
/// - `result` is the 8-bit sum
/// - `carry_out` is the carry out of bit 7 (CY flag for addition)
/// - `aux_carry` is the carry out of bit 3 (AC flag)
///
/// # Example
/// ```
/// use coding_adventures_intel8080_gatelevel::bits::add_8bit;
/// let (r, cy, ac) = add_8bit(0x0F, 0x01, 0);
/// assert_eq!(r, 0x10);
/// assert_eq!(cy, 0);
/// assert_eq!(ac, 1); // carry from low nibble into high nibble
/// ```
pub fn add_8bit(a: u8, b: u8, carry_in: u8) -> (u8, u8, u8) {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);

    let mut carry = carry_in;
    let mut sums = Vec::with_capacity(8);
    let mut carries = Vec::with_capacity(8);

    // Bit 0: half_adder when carry_in=0, but use full_adder uniformly for simplicity
    for i in 0..8 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums.push(s);
        carries.push(c);
        carry = c;
    }

    let result = bits_to_u8(&sums);
    let carry_out = carries[7];
    let aux_carry = carries[3];
    (result, carry_out, aux_carry)
}

/// 8-bit subtraction: `a - b - borrow_in` via two's complement.
///
/// Returns `(result, borrow_out, aux_borrow)` where each flag uses
/// the 8080 borrow convention: borrow=1 means "no carry / underflow".
///
/// Implementation: `a + NOT(b) + NOT(borrow_in)`.
/// The 8080 sets CY=1 when borrow occurred (i.e., adder carry = 0).
pub fn sub_8bit(a: u8, b: u8, borrow_in: u8) -> (u8, u8, u8) {
    let not_b = !b;  // bitwise NOT (8 NOT gates in parallel)
    let cin = 1 - borrow_in; // NOT(borrow_in): borrow=0 → cin=1
    let (result, adder_carry, adder_ac) = add_8bit(a, not_b, cin);
    // 8080 subtraction convention: CY = NOT(adder_carry), AC = NOT(adder_ac)
    let borrow_out = 1 - adder_carry;
    let aux_borrow = 1 - adder_ac;
    (result, borrow_out, aux_borrow)
}

// ─── 16-bit addition ─────────────────────────────────────────────────────────

/// 16-bit addition through 16 full-adder stages.
///
/// Used for PC increment, SP arithmetic, DAD (double add), INX/DCX.
/// Returns `(result, carry_out)`.
pub fn add_16bit(a: u16, b: u16, carry_in: u8) -> (u16, u8) {
    let a_bits = int_to_bits16(a);
    let b_bits = int_to_bits16(b);

    let mut carry = carry_in;
    let mut sums = Vec::with_capacity(16);

    for i in 0..16 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums.push(s);
        carry = c;
    }

    (bits_to_u16(&sums), carry)
}

/// 16-bit subtraction: `a - b` through the adder chain.
///
/// Used for DCX (decrement register pair by 1).
pub fn sub_16bit(a: u16, b: u16) -> (u16, u8) {
    // a - b = a + NOT(b) + 1
    let not_b = !b;
    add_16bit(a, not_b, 1)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_8bit() {
        for v in 0u8..=255 {
            assert_eq!(bits_to_u8(&int_to_bits8(v)), v);
        }
    }

    #[test]
    fn round_trip_16bit() {
        for v in [0u16, 1, 255, 256, 0x1234, 0xFFFF] {
            assert_eq!(bits_to_u16(&int_to_bits16(v)), v);
        }
    }

    #[test]
    fn parity_even() {
        assert_eq!(compute_parity(&int_to_bits8(0x00)), 1); // 0 ones
        assert_eq!(compute_parity(&int_to_bits8(0x03)), 1); // 2 ones
        assert_eq!(compute_parity(&int_to_bits8(0xFF)), 1); // 8 ones
    }

    #[test]
    fn parity_odd() {
        assert_eq!(compute_parity(&int_to_bits8(0x01)), 0); // 1 one
        assert_eq!(compute_parity(&int_to_bits8(0x07)), 0); // 3 ones
    }

    #[test]
    fn add_basic() {
        assert_eq!(add_8bit(10, 5, 0), (15, 0, 0));
    }

    #[test]
    fn add_overflow() {
        let (r, cy, _ac) = add_8bit(0xFF, 1, 0);
        assert_eq!(r, 0);
        assert_eq!(cy, 1);
    }

    #[test]
    fn add_aux_carry() {
        let (r, cy, ac) = add_8bit(0x0F, 0x01, 0);
        assert_eq!(r, 0x10);
        assert_eq!(cy, 0);
        assert_eq!(ac, 1); // carry out of low nibble
    }

    #[test]
    fn sub_basic() {
        let (r, borrow, _ac) = sub_8bit(10, 5, 0);
        assert_eq!(r, 5);
        assert_eq!(borrow, 0); // no borrow
    }

    #[test]
    fn sub_borrow() {
        let (r, borrow, _ac) = sub_8bit(5, 10, 0);
        assert_eq!(r, 0xFB); // 5 - 10 = -5 → 0xFB in two's complement
        assert_eq!(borrow, 1); // borrow occurred
    }

    #[test]
    fn add_16_basic() {
        assert_eq!(add_16bit(0x1234, 1, 0), (0x1235, 0));
        assert_eq!(add_16bit(0xFFFF, 1, 0), (0, 1));
    }
}
