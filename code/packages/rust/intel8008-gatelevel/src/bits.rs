//! Bit conversion helpers — integer ↔ LSB-first bit vectors.
//!
//! # Why LSB-first?
//!
//! The `arithmetic` crate's adders use LSB-first bit ordering: bit[0] is the
//! least significant bit (value 1), bit[7] is the most significant bit
//! (value 128). This matches the natural carry propagation direction in a
//! ripple-carry adder — carries ripple from bit 0 towards bit 7.
//!
//! All gate-level modules in this crate use the same LSB-first convention.
//!
//! # Parity computation
//!
//! The 8008 Parity flag is set when the result has an even number of 1-bits
//! (even parity). The hardware implements this as a chain of XOR gates:
//!
//! ```text
//! XOR_chain = bit[0] XOR bit[1] XOR ... XOR bit[7]
//! P = NOT(XOR_chain)   -- P=1 means even parity
//! ```
//!
//! The `xor_n` function from `logic-gates` implements the XOR chain.
//! We wrap it here with the NOT inversion to match the 8008 convention.

use logic_gates::gates::{not_gate, xor_n};

/// Convert an 8-bit integer to an 8-element LSB-first bit vector.
///
/// ```text
/// int_to_bits(5, 8) → [1, 0, 1, 0, 0, 0, 0, 0]
///                       ↑   ↑
///                      bit0 bit2 (values 1 and 4 sum to 5)
/// ```
///
/// # Example
///
/// ```
/// use coding_adventures_intel8008_gatelevel::bits::int_to_bits;
/// let bits = int_to_bits(0b1010_0101, 8);
/// assert_eq!(bits[0], 1); // bit 0 = 1
/// assert_eq!(bits[1], 0); // bit 1 = 0
/// assert_eq!(bits[7], 1); // bit 7 = 1
/// ```
pub fn int_to_bits(value: u8, width: usize) -> Vec<u8> {
    (0..width).map(|i| (value >> i) & 1).collect()
}

/// Convert an LSB-first bit vector back to an integer.
///
/// ```text
/// bits_to_int([1, 0, 1, 0, 0, 0, 0, 0]) → 5
/// ```
///
/// # Example
///
/// ```
/// use coding_adventures_intel8008_gatelevel::bits::bits_to_int;
/// assert_eq!(bits_to_int(&[1, 0, 1, 0, 0, 0, 0, 0]), 5);
/// assert_eq!(bits_to_int(&[1, 1, 1, 1, 1, 1, 1, 1]), 255);
/// ```
pub fn bits_to_int(bits: &[u8]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0u8, |acc, (i, &b)| acc | (b << i))
}

/// Compute the 8008 Parity flag from an 8-bit result's bit vector.
///
/// Returns 1 (flag set) when the number of 1-bits is **even** (even parity).
/// Returns 0 (flag clear) when the number of 1-bits is **odd** (odd parity).
///
/// Implementation uses `xor_n` (a chain of XOR gates) followed by NOT:
///
/// ```text
/// xor_chain = XOR(bit[0], XOR(bit[1], XOR(bit[2], ...)))
/// P = NOT(xor_chain)   -- flip: XOR=0 (even) → P=1
/// ```
///
/// # Why XOR gives parity
///
/// XOR outputs 1 when an odd number of inputs are 1. Chaining XOR across
/// all 8 bits produces 1 for odd parity and 0 for even parity. The 8008
/// convention is P=1 for EVEN parity, so we invert the XOR result.
///
/// This is a 7-gate XOR chain (7 two-input XOR gates), matching the real
/// 8008's parity tree.
///
/// # Example
///
/// ```
/// use coding_adventures_intel8008_gatelevel::bits::{compute_parity, int_to_bits};
/// // 0x03 = 0b00000011 → 2 ones → even parity → P=1
/// let bits = int_to_bits(0x03, 8);
/// assert_eq!(compute_parity(&bits), 1);
/// // 0x01 = 0b00000001 → 1 one → odd parity → P=0
/// let bits = int_to_bits(0x01, 8);
/// assert_eq!(compute_parity(&bits), 0);
/// ```
pub fn compute_parity(bits: &[u8]) -> u8 {
    // xor_n requires at least 2 inputs. For safety, handle degenerate cases.
    if bits.is_empty() {
        return 1; // even parity of empty set (0 ones = even)
    }
    if bits.len() == 1 {
        return not_gate(bits[0]); // single bit: even only if bit is 0
    }
    not_gate(xor_n(bits))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round_trip() {
        for v in 0u8..=255 {
            let bits = int_to_bits(v, 8);
            assert_eq!(bits_to_int(&bits), v, "round-trip failed for {v}");
        }
    }

    #[test]
    fn test_parity_even() {
        // 0x00 = 0 ones → even parity → 1
        assert_eq!(compute_parity(&int_to_bits(0x00, 8)), 1);
        // 0x03 = 2 ones → even parity → 1
        assert_eq!(compute_parity(&int_to_bits(0x03, 8)), 1);
        // 0xFF = 8 ones → even parity → 1
        assert_eq!(compute_parity(&int_to_bits(0xFF, 8)), 1);
    }

    #[test]
    fn test_parity_odd() {
        // 0x01 = 1 one → odd parity → 0
        assert_eq!(compute_parity(&int_to_bits(0x01, 8)), 0);
        // 0x80 = 1 one → odd parity → 0
        assert_eq!(compute_parity(&int_to_bits(0x80, 8)), 0);
        // 0x07 = 3 ones → odd parity → 0
        assert_eq!(compute_parity(&int_to_bits(0x07, 8)), 0);
    }
}
