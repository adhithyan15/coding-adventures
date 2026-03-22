//! Bit conversion helpers -- the bridge between integers and gate-level bits.
//!
//! # Why this module exists
//!
//! The gate-level simulator operates on individual bits (vectors of 0s and 1s),
//! because that's what real hardware does. But the outside world (test programs,
//! the behavioral simulator) works with integers. This module converts between
//! the two representations.
//!
//! # Bit ordering: LSB first
//!
//! All bit vectors use LSB-first ordering, matching the logic-gates and arithmetic
//! crates. Index 0 is the least significant bit.
//!
//! ```text
//! int_to_bits(5, 4)  =>  [1, 0, 1, 0]
//!   bit0=1(x1) + bit1=0(x2) + bit2=1(x4) + bit3=0(x8) = 5
//! ```
//!
//! This convention is used throughout the computing stack because it maps
//! naturally to how adders chain: bit 0 feeds the first full adder, bit 1
//! feeds the second, and so on.

/// Convert an integer to a vector of bits (LSB first).
///
/// Each element in the returned vector is 0 or 1. The vector has exactly
/// `width` elements, with the least significant bit at index 0.
///
/// # Examples
///
/// ```
/// use intel4004_gatelevel::bits::int_to_bits;
/// assert_eq!(int_to_bits(5, 4), vec![1, 0, 1, 0]);
/// assert_eq!(int_to_bits(0, 4), vec![0, 0, 0, 0]);
/// assert_eq!(int_to_bits(15, 4), vec![1, 1, 1, 1]);
/// ```
pub fn int_to_bits(value: u16, width: usize) -> Vec<u8> {
    // Mask to width to handle oversized values.
    // We use u16 because the program counter is 12 bits wide.
    let mask = if width >= 16 { 0xFFFF } else { (1u16 << width) - 1 };
    let masked = value & mask;
    (0..width).map(|i| ((masked >> i) & 1) as u8).collect()
}

/// Convert a vector of bits (LSB first) to an integer.
///
/// # Examples
///
/// ```
/// use intel4004_gatelevel::bits::bits_to_int;
/// assert_eq!(bits_to_int(&[1, 0, 1, 0]), 5);
/// assert_eq!(bits_to_int(&[0, 0, 0, 0]), 0);
/// assert_eq!(bits_to_int(&[1, 1, 1, 1]), 15);
/// ```
pub fn bits_to_int(bits: &[u8]) -> u16 {
    let mut result: u16 = 0;
    for (i, &bit) in bits.iter().enumerate() {
        result |= (bit as u16) << i;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_int_to_bits_zero() {
        assert_eq!(int_to_bits(0, 4), vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_int_to_bits_five() {
        assert_eq!(int_to_bits(5, 4), vec![1, 0, 1, 0]);
    }

    #[test]
    fn test_int_to_bits_fifteen() {
        assert_eq!(int_to_bits(15, 4), vec![1, 1, 1, 1]);
    }

    #[test]
    fn test_int_to_bits_12bit() {
        // 0xABC = 2748
        assert_eq!(
            int_to_bits(0xABC, 12),
            vec![0, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 1]
        );
    }

    #[test]
    fn test_bits_to_int_roundtrip() {
        for val in 0..=15u16 {
            assert_eq!(bits_to_int(&int_to_bits(val, 4)), val);
        }
    }

    #[test]
    fn test_bits_to_int_12bit_roundtrip() {
        for val in [0u16, 1, 255, 1023, 4095] {
            assert_eq!(bits_to_int(&int_to_bits(val, 12)), val);
        }
    }
}
