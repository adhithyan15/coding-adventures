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
//! # No half-carry on the 6502
//!
//! Unlike the Intel 8080 or Z80, the 6502 has NO half-carry (AC) flag.
//! BCD correction uses nibble-overflow detection directly in the ALU.
//!
//! # Zero detection
//!
//! The Z flag is 1 when ALL result bits are 0. Hardware: balanced NOR tree.
//! Stage 1: OR pairs → Stage 2: OR pairs → Stage 3: NOR.

use arithmetic::adders::full_adder;
use logic_gates::gates::not_gate;

// ─── 8-bit helpers ──────────────────────────────────────────────────────────

/// Convert an 8-bit integer to an 8-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_mos6502_gatelevel::bits::int_to_bits8;
/// assert_eq!(int_to_bits8(5), vec![1, 0, 1, 0, 0, 0, 0, 0]);
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
/// use coding_adventures_mos6502_gatelevel::bits::bits_to_u8;
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

/// Zero detection: returns 1 when all bits are 0 (NOR tree).
pub fn compute_zero(bits: &[u8]) -> u8 {
    u8::from(bits.iter().all(|&b| b == 0))
}

/// Bitwise NOT of an 8-bit value (8 NOT gates in parallel).
///
/// Used by SBC: A - M - borrow = A + NOT(M) + C.
pub fn not8(value: u8) -> u8 {
    let bits = int_to_bits8(value);
    let inverted: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u8(&inverted)
}

// ─── 8-bit addition ──────────────────────────────────────────────────────────

/// 8-bit addition through 8 full-adder stages.
///
/// Returns `(result, carries)` where `carries[i]` is the carry out of stage i.
/// - `carries[7]` = carry out of bit 7 (C flag for addition)
/// - `carries[6]` = carry into bit 7 (used for overflow detection)
///
/// Overflow = XOR(carries[6], carries[7]) — one XOR gate.
///
/// # Example
/// ```
/// use coding_adventures_mos6502_gatelevel::bits::add_8bit_full;
/// let (r, carries) = add_8bit_full(0x7F, 0x01, 0);
/// assert_eq!(r, 0x80);
/// assert_eq!(carries[7], 0); // no carry out
/// // overflow: carry_into_bit7=0, carry_out_of_bit7=0 → no overflow? Actually:
/// // 0x7F + 0x01 = 0x80: carry_into_bit7=1, carry_out=0 → V=1
/// ```
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

/// 8-bit addition returning just (result, carry_out).
///
/// Convenience wrapper used by stack-pointer arithmetic and address computation.
pub fn add_8bit(a: u8, b: u8, carry_in: u8) -> (u8, u8) {
    let (result, carries) = add_8bit_full(a, b, carry_in);
    (result, carries[7])
}

// ─── 16-bit addition ─────────────────────────────────────────────────────────

/// 16-bit addition through 16 full-adder stages.
///
/// Used for PC increment, indirect address computation, and indexed addressing.
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
    fn zero_detection() {
        assert_eq!(compute_zero(&[0, 0, 0, 0, 0, 0, 0, 0]), 1);
        assert_eq!(compute_zero(&[1, 0, 0, 0, 0, 0, 0, 0]), 0);
        assert_eq!(compute_zero(&[0, 0, 0, 0, 0, 0, 0, 1]), 0);
    }

    #[test]
    fn not8_inverts() {
        assert_eq!(not8(0x00), 0xFF);
        assert_eq!(not8(0xFF), 0x00);
        assert_eq!(not8(0xAA), 0x55);
        assert_eq!(not8(0x0F), 0xF0);
    }

    #[test]
    fn add_8bit_basic() {
        assert_eq!(add_8bit(10, 5, 0), (15, 0));
        assert_eq!(add_8bit(0xFF, 1, 0), (0, 1)); // overflow + carry
        assert_eq!(add_8bit(0, 0, 1), (1, 0));    // carry_in propagates
    }

    #[test]
    fn add_8bit_full_overflow_detection() {
        // 0x7F + 0x01 = 0x80: signed overflow (positive + positive = negative)
        // carry_into_bit7=1, carry_out_of_bit7=0 → XOR=1 → V=1
        let (r, carries) = add_8bit_full(0x7F, 0x01, 0);
        assert_eq!(r, 0x80);
        assert_eq!(carries[6], 1); // carry INTO bit 7
        assert_eq!(carries[7], 0); // carry OUT of bit 7
        // overflow = XOR(1, 0) = 1

        // 0xFF + 0xFF = 0xFE + carry: both negative, result negative → no overflow
        let (r2, carries2) = add_8bit_full(0xFF, 0xFF, 0);
        assert_eq!(r2, 0xFE);
        assert_eq!(carries2[6], 1);
        assert_eq!(carries2[7], 1);
        // overflow = XOR(1, 1) = 0 → no overflow ✓
    }

    #[test]
    fn add_16bit_basic() {
        assert_eq!(add_16bit(0x1234, 1, 0), (0x1235, 0));
        assert_eq!(add_16bit(0xFFFF, 1, 0), (0, 1));
        assert_eq!(add_16bit(0x01FF, 0x0001, 0), (0x0200, 0));
    }

    #[test]
    fn sbc_via_not_and_add() {
        // 6502 SBC: A - M - (1-C) = A + NOT(M) + C
        // 10 - 3 with C=1 (no borrow): result=7, C=1
        let not_m = not8(3);
        let (result, _carry) = add_8bit_full(10, not_m, 1);
        assert_eq!(result, 7);
    }
}
