//! Integration tests for the adders module.
//!
//! Tests cover half adder, full adder, and ripple-carry adder with various
//! bit widths, edge cases (overflow, carry), and arithmetic correctness.

use arithmetic::adders::*;

// ===========================================================================
// Helper functions
// ===========================================================================

/// Convert an unsigned integer to LSB-first bit vector of given width.
fn to_bits(val: u16, width: usize) -> Vec<u8> {
    (0..width).map(|i| ((val >> i) & 1) as u8).collect()
}

/// Convert LSB-first bit vector to unsigned integer.
fn from_bits(bits: &[u8]) -> u16 {
    bits.iter()
        .enumerate()
        .fold(0u16, |acc, (i, &b)| acc | ((b as u16) << i))
}

// ===========================================================================
// Half adder
// ===========================================================================

#[test]
fn test_half_adder_exhaustive() {
    assert_eq!(half_adder(0, 0), (0, 0));
    assert_eq!(half_adder(0, 1), (1, 0));
    assert_eq!(half_adder(1, 0), (1, 0));
    assert_eq!(half_adder(1, 1), (0, 1));
}

// ===========================================================================
// Full adder
// ===========================================================================

#[test]
fn test_full_adder_exhaustive() {
    // All 8 combinations
    let expected = [
        (0, 0, 0, 0, 0), // 0+0+0 = 0, carry 0
        (0, 0, 1, 1, 0), // 0+0+1 = 1, carry 0
        (0, 1, 0, 1, 0), // 0+1+0 = 1, carry 0
        (0, 1, 1, 0, 1), // 0+1+1 = 2, carry 1
        (1, 0, 0, 1, 0), // 1+0+0 = 1, carry 0
        (1, 0, 1, 0, 1), // 1+0+1 = 2, carry 1
        (1, 1, 0, 0, 1), // 1+1+0 = 2, carry 1
        (1, 1, 1, 1, 1), // 1+1+1 = 3, carry 1
    ];
    for &(a, b, cin, exp_sum, exp_carry) in &expected {
        let (sum, carry) = full_adder(a, b, cin);
        assert_eq!(
            (sum, carry),
            (exp_sum, exp_carry),
            "full_adder({a}, {b}, {cin})"
        );
    }
}

// ===========================================================================
// Ripple-carry adder — 4-bit
// ===========================================================================

#[test]
fn test_ripple_carry_4bit_simple_cases() {
    // 0 + 0 = 0
    let r = ripple_carry_adder(&to_bits(0, 4), &to_bits(0, 4));
    assert_eq!(from_bits(&r.sum), 0);
    assert_eq!(r.carry_out, 0);

    // 1 + 1 = 2
    let r = ripple_carry_adder(&to_bits(1, 4), &to_bits(1, 4));
    assert_eq!(from_bits(&r.sum), 2);
    assert_eq!(r.carry_out, 0);

    // 7 + 1 = 8
    let r = ripple_carry_adder(&to_bits(7, 4), &to_bits(1, 4));
    assert_eq!(from_bits(&r.sum), 8);
    assert_eq!(r.carry_out, 0);
}

#[test]
fn test_ripple_carry_4bit_overflow() {
    // 15 + 1 = 0 with carry
    let r = ripple_carry_adder(&to_bits(15, 4), &to_bits(1, 4));
    assert_eq!(from_bits(&r.sum), 0);
    assert_eq!(r.carry_out, 1);

    // 15 + 15 = 30, but 4-bit can only hold 0-15
    let r = ripple_carry_adder(&to_bits(15, 4), &to_bits(15, 4));
    assert_eq!(from_bits(&r.sum), 14); // 30 mod 16 = 14
    assert_eq!(r.carry_out, 1);
}

// ===========================================================================
// Ripple-carry adder — 8-bit
// ===========================================================================

#[test]
fn test_ripple_carry_8bit_various() {
    let cases: Vec<(u16, u16, u16, u8)> = vec![
        (0, 0, 0, 0),
        (1, 1, 2, 0),
        (100, 55, 155, 0),
        (200, 100, 44, 1), // 300 mod 256 = 44, carry = 1
        (255, 1, 0, 1),    // overflow
        (128, 128, 0, 1),  // 256 mod 256 = 0, carry = 1
    ];

    for (a, b, expected_sum, expected_carry) in cases {
        let r = ripple_carry_adder(&to_bits(a, 8), &to_bits(b, 8));
        assert_eq!(
            from_bits(&r.sum),
            expected_sum,
            "{a} + {b} = sum should be {expected_sum}"
        );
        assert_eq!(
            r.carry_out, expected_carry,
            "{a} + {b}: carry should be {expected_carry}"
        );
    }
}

#[test]
fn test_ripple_carry_8bit_commutative() {
    // Addition is commutative: a + b == b + a
    for a in (0..256).step_by(17) {
        for b in (0..256).step_by(23) {
            let r1 = ripple_carry_adder(&to_bits(a as u16, 8), &to_bits(b as u16, 8));
            let r2 = ripple_carry_adder(&to_bits(b as u16, 8), &to_bits(a as u16, 8));
            assert_eq!(r1.sum, r2.sum, "Commutativity: {a} + {b}");
            assert_eq!(r1.carry_out, r2.carry_out);
        }
    }
}

#[test]
fn test_ripple_carry_identity() {
    // a + 0 == a
    for a in 0..=255u16 {
        let r = ripple_carry_adder(&to_bits(a, 8), &to_bits(0, 8));
        assert_eq!(from_bits(&r.sum), a, "{a} + 0 should equal {a}");
        assert_eq!(r.carry_out, 0);
    }
}

// ===========================================================================
// Ripple-carry adder — 16-bit
// ===========================================================================

#[test]
fn test_ripple_carry_16bit() {
    // 1000 + 2000 = 3000
    let r = ripple_carry_adder(&to_bits(1000, 16), &to_bits(2000, 16));
    assert_eq!(from_bits(&r.sum), 3000);
    assert_eq!(r.carry_out, 0);
}

// ===========================================================================
// Signed overflow
// ===========================================================================

#[test]
fn test_ripple_carry_signed_overflow() {
    // In 8-bit two's complement, range is -128 to 127.
    // 127 + 1 = 128, which overflows (result looks negative).
    // 127 = 0111_1111, 1 = 0000_0001
    let r = ripple_carry_adder(&to_bits(127, 8), &to_bits(1, 8));
    assert!(r.overflow, "127 + 1 should overflow in signed 8-bit");
    assert_eq!(from_bits(&r.sum), 128); // Unsigned view: 128
}

#[test]
fn test_ripple_carry_no_signed_overflow() {
    // 64 + 63 = 127, which fits in signed 8-bit
    let r = ripple_carry_adder(&to_bits(64, 8), &to_bits(63, 8));
    assert!(!r.overflow);
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
#[should_panic(expected = "a and b must have the same length")]
fn test_ripple_carry_mismatched_lengths() {
    ripple_carry_adder(&[0, 1], &[0, 1, 0]);
}

#[test]
#[should_panic(expected = "bit lists must not be empty")]
fn test_ripple_carry_empty() {
    ripple_carry_adder(&[], &[]);
}

#[test]
fn test_ripple_carry_1bit() {
    let r = ripple_carry_adder(&[1], &[1]);
    assert_eq!(r.sum, vec![0]);
    assert_eq!(r.carry_out, 1);
}
