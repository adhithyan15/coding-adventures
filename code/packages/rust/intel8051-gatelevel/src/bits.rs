//! Bit conversion helpers — bridges integers and gate-level bit vectors.
//!
//! # Bit ordering: LSB-first
//!
//! All bit vectors in this crate are **LSB-first**: `bits[0]` = bit 0
//! (value 1), `bits[7]` = bit 7 (value 128).  This matches the `arithmetic`
//! crate's ripple-carry adder, where the carry propagates from bit 0 toward
//! bit 7.
//!
//! ```text
//! int_to_bits8(5) → [1, 0, 1, 0, 0, 0, 0, 0]
//!                    ↑ bit0=1(×1)  ↑ bit2=1(×4) → 1+4=5
//! ```
//!
//! # Auxiliary carry (AC flag)
//!
//! The 8051 AC flag records the carry out of bit 3 into bit 4 (low-nibble
//! boundary).  It is used by DA A (BCD decimal adjust after ADD) and also
//! set by SUBB.
//!
//! For ADD/ADDC: `AC = carries[3]` — the carry output of the 4th full-adder
//! stage (0-indexed).
//!
//! For SUBB: `AC = NOT(carries[3])` — the nibble *borrow*, since SUBB
//! is implemented as `A + NOT(B) + NOT(borrow_in)`.
//!
//! # Overflow detection
//!
//! For signed 8-bit addition `A + B`:
//! ```text
//! OV = XOR(carry into bit 7, carry out of bit 7)
//!    = XOR(carries[6], carries[7])
//! ```
//!
//! # Parity
//!
//! The 8051 PSW.P flag = 1 when ACC has an **odd** number of set bits.
//! This is implemented as an XOR tree over all 8 bits:
//! XOR(b0, b1, …, b7) = 1 iff the count of 1s is odd.

use arithmetic::adders::full_adder;
use logic_gates::gates::{not_gate, xor_gate};

// ─── Integer ↔ bit vector ──────────────────────────────────────────────────────

/// Convert an 8-bit integer to an 8-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::int_to_bits8;
/// assert_eq!(int_to_bits8(5), [1, 0, 1, 0, 0, 0, 0, 0]);
/// ```
pub fn int_to_bits8(value: u8) -> [u8; 8] {
    let mut bits = [0u8; 8];
    for (i, b) in bits.iter_mut().enumerate() {
        *b = (value >> i) & 1;
    }
    bits
}

/// Convert a 16-bit integer to a 16-element LSB-first bit vector.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::int_to_bits16;
/// let bits = int_to_bits16(0x0100);
/// assert_eq!(bits[8], 1); // bit 8 is set (value 256)
/// ```
pub fn int_to_bits16(value: u16) -> [u8; 16] {
    let mut bits = [0u8; 16];
    for (i, b) in bits.iter_mut().enumerate() {
        *b = ((value >> i) & 1) as u8;
    }
    bits
}

/// Convert an LSB-first 8-element bit vector back to a `u8`.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::bits_to_u8;
/// assert_eq!(bits_to_u8(&[1, 0, 1, 0, 0, 0, 0, 0]), 5);
/// ```
pub fn bits_to_u8(bits: &[u8; 8]) -> u8 {
    let mut val = 0u8;
    for (i, &b) in bits.iter().enumerate() {
        val |= b << i;
    }
    val
}

/// Convert an LSB-first 16-element bit vector back to a `u16`.
pub fn bits_to_u16(bits: &[u8; 16]) -> u16 {
    let mut val = 0u16;
    for (i, &b) in bits.iter().enumerate() {
        val |= (b as u16) << i;
    }
    val
}

// ─── NOT helper ───────────────────────────────────────────────────────────────

/// 8 NOT gates in parallel — bitwise complement of an 8-bit value.
///
/// Used for two's-complement subtraction: `A - B = A + NOT(B) + 1`.
/// In SUBB, `A - B - borrow = A + NOT(B) + NOT(borrow)`.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::invert_8bit;
/// assert_eq!(invert_8bit(0x00), 0xFF);
/// assert_eq!(invert_8bit(0x0F), 0xF0);
/// ```
pub fn invert_8bit(value: u8) -> u8 {
    let bits = int_to_bits8(value);
    let mut inv = [0u8; 8];
    for i in 0..8 {
        inv[i] = not_gate(bits[i]);
    }
    bits_to_u8(&inv)
}

// ─── 8-bit addition ──────────────────────────────────────────────────────────

/// 8-bit addition through 8 full-adder stages, returning the full carry chain.
///
/// Returns `(result, carries)` where `carries[k]` = carry out of adder stage k
/// (i.e., carry out of bit k).
///
/// | Flag | Source |
/// |------|--------|
/// | CY   | carries[7] |
/// | AC   | carries[3] |
/// | OV   | XOR(carries[6], carries[7]) |
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::add_8bit_full;
/// let (result, carries) = add_8bit_full(0x0F, 0x01, 0);
/// assert_eq!(result, 0x10);
/// assert_eq!(carries[3], 1); // nibble carry → AC=1
/// ```
pub fn add_8bit_full(a: u8, b: u8, carry_in: u8) -> (u8, [u8; 8]) {
    let a_bits = int_to_bits8(a);
    let b_bits = int_to_bits8(b);
    let mut carry = carry_in;
    let mut sums = [0u8; 8];
    let mut carries = [0u8; 8];
    for i in 0..8 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums[i] = s;
        carries[i] = c;
        carry = c;
    }
    (bits_to_u8(&sums), carries)
}

// ─── 16-bit addition ─────────────────────────────────────────────────────────

/// 16-bit addition through 16 full-adder stages.
///
/// Returns `(result, carry_out)`.  Used for PC increment and DPTR arithmetic.
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::add_16bit_full;
/// let (result, cy) = add_16bit_full(0xFFFF, 1, 0);
/// assert_eq!(result, 0x0000);
/// assert_eq!(cy, 1);
/// ```
pub fn add_16bit_full(a: u16, b: u16, carry_in: u8) -> (u16, u8) {
    let a_bits = int_to_bits16(a);
    let b_bits = int_to_bits16(b);
    let mut carry = carry_in;
    let mut sums = [0u8; 16];
    for i in 0..16 {
        let (s, c) = full_adder(a_bits[i], b_bits[i], carry);
        sums[i] = s;
        carry = c;
    }
    (bits_to_u16(&sums), carry)
}

// ─── Parity ───────────────────────────────────────────────────────────────────

/// ODD parity via a 7-gate XOR tree — returns 1 when an odd number of bits
/// in `bits[0..8]` are set.
///
/// This is the PSW.P (parity) flag for the 8051: P=1 when ACC has an odd
/// number of 1-bits.  Compare to the Intel 8086 PF flag, which is EVEN
/// parity (opposite sense).
///
/// ```text
/// Stage 1: s0=XOR(b0,b1), s1=XOR(b2,b3), s2=XOR(b4,b5), s3=XOR(b6,b7)
/// Stage 2: t0=XOR(s0,s1), t1=XOR(s2,s3)
/// Stage 3: P = XOR(t0,t1)   ← 1 iff odd count of 1-bits
/// ```
///
/// # Example
/// ```
/// use coding_adventures_intel8051_gatelevel::bits::compute_parity;
/// // 0b00000111 = three 1-bits → odd → P=1
/// let bits = [1, 1, 1, 0, 0, 0, 0, 0];
/// assert_eq!(compute_parity(&bits), 1);
/// // 0b00001111 = four 1-bits → even → P=0
/// let bits2 = [1, 1, 1, 1, 0, 0, 0, 0];
/// assert_eq!(compute_parity(&bits2), 0);
/// ```
pub fn compute_parity(bits: &[u8; 8]) -> u8 {
    let s0 = xor_gate(bits[0], bits[1]);
    let s1 = xor_gate(bits[2], bits[3]);
    let s2 = xor_gate(bits[4], bits[5]);
    let s3 = xor_gate(bits[6], bits[7]);
    let t0 = xor_gate(s0, s1);
    let t1 = xor_gate(s2, s3);
    xor_gate(t0, t1)
}

/// Zero detection. Returns `true` when all bits are 0.
///
/// Hardware: a balanced NOR tree (OR all, then NOT).
pub fn compute_zero(bits: &[u8; 8]) -> bool {
    bits.iter().all(|&b| b == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bits_round_trip_8() {
        for v in 0u8..=255 {
            assert_eq!(bits_to_u8(&int_to_bits8(v)), v);
        }
    }

    #[test]
    fn bits_round_trip_16() {
        for v in [0u16, 1, 0x00FF, 0xFF00, 0xABCD, 0xFFFF] {
            assert_eq!(bits_to_u16(&int_to_bits16(v)), v);
        }
    }

    #[test]
    fn invert_8bit_test() {
        assert_eq!(invert_8bit(0x00), 0xFF);
        assert_eq!(invert_8bit(0xFF), 0x00);
        assert_eq!(invert_8bit(0xAA), 0x55);
        assert_eq!(invert_8bit(0x0F), 0xF0);
    }

    #[test]
    fn add_8bit_carry_and_ac() {
        // 0x0F + 0x01 = 0x10, AC=1 (carry from nibble), CY=0
        let (r, carries) = add_8bit_full(0x0F, 0x01, 0);
        assert_eq!(r, 0x10);
        assert_eq!(carries[3], 1); // AC
        assert_eq!(carries[7], 0); // CY
        // 0xFF + 0x01 = 0x00, CY=1, AC=1
        let (r2, carries2) = add_8bit_full(0xFF, 0x01, 0);
        assert_eq!(r2, 0x00);
        assert_eq!(carries2[7], 1);
        assert_eq!(carries2[3], 1);
    }

    #[test]
    fn add_16bit_wraparound() {
        let (r, cy) = add_16bit_full(0xFFFF, 1, 0);
        assert_eq!(r, 0);
        assert_eq!(cy, 1);
        let (r2, cy2) = add_16bit_full(0x1000, 0x0005, 0);
        assert_eq!(r2, 0x1005);
        assert_eq!(cy2, 0);
    }

    #[test]
    fn parity_odd_even() {
        // 0x01 = 0b00000001 → one 1-bit → odd → P=1
        assert_eq!(compute_parity(&int_to_bits8(0x01)), 1);
        // 0x03 = 0b00000011 → two 1-bits → even → P=0
        assert_eq!(compute_parity(&int_to_bits8(0x03)), 0);
        // 0x07 = 0b00000111 → three 1-bits → odd → P=1
        assert_eq!(compute_parity(&int_to_bits8(0x07)), 1);
        // 0xFF = 0b11111111 → eight 1-bits → even → P=0
        assert_eq!(compute_parity(&int_to_bits8(0xFF)), 0);
    }

    #[test]
    fn compute_zero_test() {
        assert!(compute_zero(&[0; 8]));
        assert!(!compute_zero(&int_to_bits8(1)));
    }
}
