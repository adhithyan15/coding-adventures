//! Bit conversion helpers — bridges integers and gate-level bit vectors.
//!
//! # Bit ordering: LSB-first
//!
//! All bit vectors are LSB-first: `bits[0]` = bit 0 (value 1), `bits[15]` = bit 15 (value
//! 32768). This matches the `arithmetic` crate's ripple-carry adder, where carry propagates
//! from bit 0 toward bit 15.
//!
//! ```text
//! int_to_bits8(5) → [1, 0, 1, 0, 0, 0, 0, 0]
//!                    ↑ bit0=1(×1)  ↑ bit2=1(×4) → sum = 5
//! ```
//!
//! # Auxiliary carry (AF flag)
//!
//! The 8086 AF flag records the carry out of bit 3 into bit 4 (the low-nibble boundary).
//! It is used by DAA/DAS/AAA/AAS for BCD correction.
//!
//! For ADD/ADC: `AF = carries[3]` — the carry output of the 4th full-adder stage.
//! For SUB/SBB/DEC/NEG/CMP: `AF = nibble_borrow(a, b, borrow)` — a dedicated 4-bit
//! subtractor that correctly reflects nibble borrow, independent of the two's-complement
//! representation used by the main adder.
//!
//! # Overflow detection
//!
//! For signed N-bit addition `A + B = R`:
//! ```text
//! OF = XOR(carry into bit N-1, carry out of bit N-1)
//!    = XOR(carries[N-2], carries[N-1])
//! ```
//! This is a single XOR gate at the MSB of the adder chain.

use arithmetic::adders::full_adder;
use logic_gates::gates::{not_gate, xor_gate};

// ─── Integer ↔ bit vector ──────────────────────────────────────────────────────

/// Convert an 8-bit integer to an 8-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::bits::int_to_bits8;
/// assert_eq!(int_to_bits8(5), vec![1, 0, 1, 0, 0, 0, 0, 0]);
/// ```
pub fn int_to_bits8(value: u8) -> Vec<u8> {
    (0..8).map(|i| (value >> i) & 1).collect()
}

/// Convert a 16-bit integer to a 16-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::bits::int_to_bits16;
/// assert_eq!(int_to_bits16(0x0100)[8], 1); // bit 8 is set
/// ```
pub fn int_to_bits16(value: u16) -> Vec<u8> {
    (0..16).map(|i| ((value >> i) & 1) as u8).collect()
}

/// Convert an LSB-first 8-element bit vector back to a `u8`.
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::bits::bits_to_u8;
/// assert_eq!(bits_to_u8(&[1, 0, 1, 0, 0, 0, 0, 0]), 5);
/// ```
pub fn bits_to_u8(bits: &[u8]) -> u8 {
    bits.iter().take(8).enumerate().fold(0u8, |acc, (i, &b)| acc | (b << i))
}

/// Convert an LSB-first 16-element bit vector back to a `u16`.
pub fn bits_to_u16(bits: &[u8]) -> u16 {
    bits.iter()
        .take(16)
        .enumerate()
        .fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i))
}

// ─── Flag helpers ─────────────────────────────────────────────────────────────

/// Even parity via a 7-gate XOR tree over the low 8 bits, then NOT.
///
/// PF = 1 when the number of 1-bits in bits[0..8] is **even**.
///
/// ```text
/// Stage 1: s0=XOR(b0,b1), s1=XOR(b2,b3), s2=XOR(b4,b5), s3=XOR(b6,b7)
/// Stage 2: t0=XOR(s0,s1), t1=XOR(s2,s3)
/// Stage 3: odd = XOR(t0,t1)
/// Output:  PF = NOT(odd)
/// ```
pub fn compute_parity(bits: &[u8]) -> u8 {
    let s0 = xor_gate(bits[0], bits[1]);
    let s1 = xor_gate(bits[2], bits[3]);
    let s2 = xor_gate(bits[4], bits[5]);
    let s3 = xor_gate(bits[6], bits[7]);
    let t0 = xor_gate(s0, s1);
    let t1 = xor_gate(s2, s3);
    let odd = xor_gate(t0, t1);
    not_gate(odd)
}

/// Zero detection. Returns 1 when all bits are 0 (ZF = 1).
///
/// Hardware: a balanced NOR tree.
pub fn compute_zero(bits: &[u8]) -> u8 {
    u8::from(bits.iter().all(|&b| b == 0))
}

// ─── NOT helpers ──────────────────────────────────────────────────────────────

/// 8 NOT gates in parallel — bitwise complement of an 8-bit value.
///
/// Used for two's complement subtraction: `A - B = A + NOT(B) + 1`.
pub fn invert_8bit(value: u8) -> u8 {
    let bits = int_to_bits8(value);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u8(&inv)
}

/// 16 NOT gates in parallel — bitwise complement of a 16-bit value.
pub fn invert_16bit(value: u16) -> u16 {
    let bits = int_to_bits16(value);
    let inv: Vec<u8> = bits.iter().map(|&b| not_gate(b)).collect();
    bits_to_u16(&inv)
}

// ─── 8-bit addition ──────────────────────────────────────────────────────────

/// 8-bit addition through 8 full-adder stages.
///
/// Returns `(result, carry_out, aux_carry)` where:
/// - `carry_out`  = carry out of bit 7 (CF for ADD)
/// - `aux_carry`  = carry out of bit 3 (AF flag for BCD ops)
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::bits::add_8bit;
/// let (r, cy, af) = add_8bit(0x0F, 0x01, 0);
/// assert_eq!(r, 0x10);
/// assert_eq!(cy, 0);
/// assert_eq!(af, 1); // carry from low nibble → AF=1
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

/// 8-bit addition returning the full carry chain for overflow detection.
///
/// `carries[6]` = carry into bit 7; `carries[7]` = carry out of bit 7.
/// `OF = XOR(carries[6], carries[7])`.
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
/// Returns `(result, carry_out, aux_carry)` where:
/// - `carry_out` = carry out of bit 15 (CF for ADD/ADC)
/// - `aux_carry` = carry out of bit 3 (AF flag)
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::bits::add_16bit;
/// let (r, cy, af) = add_16bit(0xFFFF, 1, 0);
/// assert_eq!(r, 0);
/// assert_eq!(cy, 1);
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
    (bits_to_u16(&sums), carries[15], carries[3])
}

/// 16-bit addition returning the full carry chain.
///
/// `carries[14]` = carry into bit 15; `carries[15]` = carry out of bit 15.
/// `OF = XOR(carries[14], carries[15])`.
pub fn add_16bit_full(a: u16, b: u16, carry_in: u8) -> (u16, Vec<u8>) {
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
    (bits_to_u16(&sums), carries)
}

// ─── Nibble borrow (AF for subtraction) ──────────────────────────────────────

/// Compute whether the low-nibble subtraction `a - b - borrow_in` borrows.
///
/// Returns AF = 1 when a borrow from bit 3 into bit 4 occurs. Used for
/// SUB/SBB/CMP/DEC/NEG on the 8086.
///
/// The adder's carry-out for the low nibble does not directly model AF for
/// subtraction (because two's complement representation shifts the carry meaning).
/// This function uses a dedicated 4-bit two's complement subtractor:
/// ```text
/// 1. 4 NOT gates: NOT_B_nib[i] = NOT(B & 0xF)[i]
/// 2. 4-bit adder: A_nib + NOT_B_nib + NOT(borrow_in)
/// 3. AF = NOT(carry_out_of_4bit_adder)
/// ```
///
/// # Example
/// ```
/// use coding_adventures_intel8086_gatelevel::bits::nibble_borrow;
/// assert_eq!(nibble_borrow(0x00, 0x01, 0), 1); // 0 - 1: borrow
/// assert_eq!(nibble_borrow(0x0F, 0x01, 0), 0); // 0xF - 1: no borrow
/// ```
pub fn nibble_borrow(a: u8, b: u8, borrow_in: u8) -> u8 {
    let a_bits = int_to_bits8(a & 0xF);
    let b_bits = int_to_bits8(b & 0xF);
    let not_b: Vec<u8> = b_bits[..4].iter().map(|&x| not_gate(x)).collect();
    let c_in = not_gate(borrow_in);
    let mut carry = c_in;
    for i in 0..4 {
        let (_, c) = full_adder(a_bits[i], not_b[i], carry);
        carry = c;
    }
    not_gate(carry)
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_8bit() {
        for v in [0u8, 1, 5, 0x0F, 0x80, 0xFF] {
            assert_eq!(bits_to_u8(&int_to_bits8(v)), v);
        }
    }

    #[test]
    fn round_trip_16bit() {
        for v in [0u16, 1, 0xFF, 0x100, 0x1234, 0x8000, 0xFFFF] {
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
    fn add_8bit_basic() {
        let (r, cy, af) = add_8bit(10, 5, 0);
        assert_eq!(r, 15);
        assert_eq!(cy, 0);
        assert_eq!(af, 0);
    }

    #[test]
    fn add_8bit_carry() {
        let (r, cy, _) = add_8bit(0xFF, 1, 0);
        assert_eq!(r, 0);
        assert_eq!(cy, 1);
    }

    #[test]
    fn add_8bit_aux_carry() {
        let (r, cy, af) = add_8bit(0x0F, 0x01, 0);
        assert_eq!(r, 0x10);
        assert_eq!(cy, 0);
        assert_eq!(af, 1);
    }

    #[test]
    fn add_16bit_carry() {
        let (r, cy, _) = add_16bit(0xFFFF, 1, 0);
        assert_eq!(r, 0);
        assert_eq!(cy, 1);
    }

    #[test]
    fn nibble_borrow_cases() {
        assert_eq!(nibble_borrow(0x00, 0x00, 0), 0); // 0 - 0: no borrow
        assert_eq!(nibble_borrow(0x00, 0x01, 0), 1); // 0 - 1: borrow
        assert_eq!(nibble_borrow(0x0F, 0x01, 0), 0); // 0xF - 1 = 0xE: no borrow
        assert_eq!(nibble_borrow(0x10, 0x01, 0), 1); // nibble 0 < nibble 1: borrow
    }

    #[test]
    fn invert_roundtrip() {
        assert_eq!(invert_8bit(0x00), 0xFF);
        assert_eq!(invert_8bit(0xFF), 0x00);
        assert_eq!(invert_8bit(0xAA), 0x55);
        assert_eq!(invert_16bit(0x0000), 0xFFFF);
        assert_eq!(invert_16bit(0xFFFF), 0x0000);
    }
}
