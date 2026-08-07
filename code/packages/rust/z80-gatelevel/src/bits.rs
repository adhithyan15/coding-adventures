//! Bit conversion helpers for the Z80 gate-level simulator.
//!
//! # Bit ordering: LSB-first
//!
//! All bit vectors use LSB-first ordering: `bits[0]` = least significant bit
//! (value 1), `bits[7]` = most significant bit (value 128). This matches the
//! `arithmetic` crate's ripple-carry adder, where carry propagates from bit 0
//! upward.
//!
//! ```text
//! int_to_bits8(6) → [0, 1, 1, 0, 0, 0, 0, 0]
//!                    ↑ bit0=0 (×1)  ↑ bit2=1 (×4) ↑ bit1=1 (×2) → sum = 6
//! ```
//!
//! # Half-carry in the Z80
//!
//! Unlike the 6502, the Z80 has a half-carry flag (H) that records the carry
//! out of bit 3 (into bit 4) for addition, or the borrow from bit 4 into bit 3
//! for subtraction. Hardware-wise: H_sub = NOT(adder_half_carry).
//!
//! # Parity
//!
//! P/V in logical mode (AND/OR/XOR) is even parity: 1 if the result has an
//! even number of 1-bits. Implemented as an XOR tree: XOR all 8 bits together
//! then NOT (even parity = NOT(odd parity)).

use arithmetic::adders::full_adder;
use logic_gates::gates::{not_gate, xor_gate};

// ─── 8-bit helpers ──────────────────────────────────────────────────────────

/// Convert an 8-bit integer to an 8-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_z80_gatelevel::bits::int_to_bits8;
/// assert_eq!(int_to_bits8(6), vec![0, 1, 1, 0, 0, 0, 0, 0]);
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
/// use coding_adventures_z80_gatelevel::bits::bits_to_u8;
/// assert_eq!(bits_to_u8(&[0, 1, 1, 0, 0, 0, 0, 0]), 6);
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

/// Zero detection: returns 1 when all bits are 0 (NOR tree).
pub fn compute_zero(bits: &[u8]) -> u8 {
    u8::from(bits.iter().all(|&b| b == 0))
}

/// Even parity: returns 1 if the count of 1-bits is even (XOR tree + NOT).
///
/// Z80 P/V flag for logical operations (AND/OR/XOR): 1 = even parity.
/// Hardware: XOR all 8 bits → odd-parity bit; NOT → even-parity bit.
///
/// # Example
/// ```
/// use coding_adventures_z80_gatelevel::bits::compute_parity;
/// assert_eq!(compute_parity(&[0, 0, 0, 0, 0, 0, 0, 0]), 1); // 0 ones → even
/// assert_eq!(compute_parity(&[1, 0, 0, 0, 0, 0, 0, 0]), 0); // 1 one → odd
/// assert_eq!(compute_parity(&[1, 1, 0, 0, 0, 0, 0, 0]), 1); // 2 ones → even
/// ```
pub fn compute_parity(bits: &[u8]) -> u8 {
    let odd = bits.iter().take(8).fold(0u8, |acc, &b| xor_gate(acc, b));
    not_gate(odd)
}

// ─── Bitwise inversion ───────────────────────────────────────────────────────

/// Bitwise NOT of an 8-bit value (8 NOT gates in parallel).
///
/// Used by sub8: A - B = A + NOT(B) + 1 (two's complement).
pub fn invert_8bit(value: u8) -> u8 {
    let bits = int_to_bits8(value);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u8(&inv)
}

/// Bitwise NOT of a 16-bit value (16 NOT gates in parallel).
///
/// Used by sbc16: HL - rp = HL + NOT(rp) + 1.
pub fn invert_16bit(value: u16) -> u16 {
    let bits = int_to_bits16(value);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u16(&inv)
}

// ─── 8-bit addition ──────────────────────────────────────────────────────────

/// 8-bit addition through 8 full-adder stages.
///
/// Returns `(result, carry_out, half_carry)` where:
/// - `carry_out` = carry out of bit 7 (C flag for ADD)
/// - `half_carry` = carry out of bit 3 (H flag for ADD; inverted for SUB)
///
/// # Example
/// ```
/// use coding_adventures_z80_gatelevel::bits::add_8bit;
/// let (r, c, h) = add_8bit(0x0F, 0x01, 0);
/// assert_eq!(r, 0x10);
/// assert_eq!(c, 0); // no carry out
/// assert_eq!(h, 1); // half-carry: bit 3 → bit 4
/// ```
pub fn add_8bit(a: u8, b: u8, carry_in: u8) -> (u8, u8, u8) {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);

    let mut carry = carry_in;
    let mut sums = Vec::with_capacity(8);
    let mut carries = Vec::with_capacity(8);

    for i in 0..8 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums.push(s);
        carries.push(c);
        carry = c;
    }

    (bits_to_u8(&sums), carries[7], carries[3])
}

/// 8-bit addition returning full carry chain (for overflow detection).
///
/// Returns `(result, carries)` — `carries[i]` is carry out of stage i.
/// - `carries[6]` = carry into bit 7
/// - `carries[7]` = carry out of bit 7
///
/// Overflow = XOR(carries[6], carries[7]).
pub fn add_8bit_full(a: u8, b: u8, carry_in: u8) -> (u8, Vec<u8>) {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);

    let mut carry = carry_in;
    let mut sums = Vec::with_capacity(8);
    let mut carries = Vec::with_capacity(8);

    for i in 0..8 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums.push(s);
        carries.push(c);
        carry = c;
    }

    (bits_to_u8(&sums), carries)
}

// ─── 16-bit addition ─────────────────────────────────────────────────────────

/// 16-bit addition through 16 full-adder stages.
///
/// Returns `(result, carry_out, half_carry_16)` where:
/// - `carry_out` = carry out of bit 15 (C flag)
/// - `half_carry_16` = carry out of bit 11 (H flag for 16-bit ops)
///
/// The Z80 16-bit half-carry is at bit 11 (the "boundary" between the low
/// and high bytes, shifted by 4 bits because of 16-bit arithmetic).
///
/// # Example
/// ```
/// use coding_adventures_z80_gatelevel::bits::add_16bit;
/// let (r, c, h) = add_16bit(0x0FFF, 0x0001, 0);
/// assert_eq!(r, 0x1000);
/// assert_eq!(c, 0);
/// assert_eq!(h, 1); // carry out of bit 11
/// ```
pub fn add_16bit(a: u16, b: u16, carry_in: u8) -> (u16, u8, u8) {
    let a_bits = int_to_bits16(a);
    let b_bits = int_to_bits16(b);

    let mut carry = carry_in;
    let mut sums = Vec::with_capacity(16);
    let mut carries = Vec::with_capacity(16);

    for i in 0..16 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums.push(s);
        carries.push(c);
        carry = c;
    }

    (bits_to_u16(&sums), carries[15], carries[11])
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
        for v in [0u16, 1, 255, 256, 0x0FFF, 0x1000, 0xFFFF] {
            assert_eq!(bits_to_u16(&int_to_bits16(v)), v);
        }
    }

    #[test]
    fn zero_detection() {
        assert_eq!(compute_zero(&[0, 0, 0, 0, 0, 0, 0, 0]), 1);
        assert_eq!(compute_zero(&[1, 0, 0, 0, 0, 0, 0, 0]), 0);
        assert_eq!(compute_zero(&[0, 0, 0, 0, 0, 0, 0, 1]), 0);
    }

    #[test]
    fn parity() {
        assert_eq!(compute_parity(&[0, 0, 0, 0, 0, 0, 0, 0]), 1); // 0 ones → even
        assert_eq!(compute_parity(&[1, 0, 0, 0, 0, 0, 0, 0]), 0); // 1 one → odd
        assert_eq!(compute_parity(&[1, 1, 0, 0, 0, 0, 0, 0]), 1); // 2 ones → even
        assert_eq!(compute_parity(&[1, 1, 1, 0, 0, 0, 0, 0]), 0); // 3 ones → odd
        // 0b10110110 = 0xB6: bits 1,2,4,5,7 → 5 ones → odd parity → P=0
        let bits = int_to_bits8(0xB6);
        assert_eq!(compute_parity(&bits), 0);
    }

    #[test]
    fn invert_8bit_test() {
        assert_eq!(invert_8bit(0x00), 0xFF);
        assert_eq!(invert_8bit(0xFF), 0x00);
        assert_eq!(invert_8bit(0xAA), 0x55);
        assert_eq!(invert_8bit(0x0F), 0xF0);
    }

    #[test]
    fn invert_16bit_test() {
        assert_eq!(invert_16bit(0x0000), 0xFFFF);
        assert_eq!(invert_16bit(0xFFFF), 0x0000);
        assert_eq!(invert_16bit(0xABCD), 0x5432);
    }

    #[test]
    fn add_8bit_basic() {
        assert_eq!(add_8bit(10, 5, 0), (15, 0, 0));
        assert_eq!(add_8bit(0xFF, 1, 0), (0, 1, 1)); // carry + half-carry
        assert_eq!(add_8bit(0, 0, 1), (1, 0, 0));
    }

    #[test]
    fn add_8bit_half_carry() {
        // 0x0F + 0x01 = 0x10: carry from bit 3 to bit 4
        let (r, c, h) = add_8bit(0x0F, 0x01, 0);
        assert_eq!(r, 0x10);
        assert_eq!(c, 0);
        assert_eq!(h, 1);
    }

    #[test]
    fn add_16bit_half_carry() {
        // 0x0FFF + 0x0001 = 0x1000: carry out of bit 11
        let (r, c, h) = add_16bit(0x0FFF, 0x0001, 0);
        assert_eq!(r, 0x1000);
        assert_eq!(c, 0);
        assert_eq!(h, 1);

        // 0xFFFF + 0x0001: full carry, bit-11 carry
        let (r2, c2, _) = add_16bit(0xFFFF, 0x0001, 0);
        assert_eq!(r2, 0x0000);
        assert_eq!(c2, 1);
    }
}
