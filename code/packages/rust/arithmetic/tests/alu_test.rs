//! Integration tests for the ALU module.
//!
//! Tests cover all six operations (ADD, SUB, AND, OR, XOR, NOT) with
//! various inputs, flag behavior, and edge cases.

use arithmetic::alu::*;

// ===========================================================================
// Helper functions
// ===========================================================================

/// Convert an unsigned integer to LSB-first bit vector of given width.
fn to_bits(val: u8, width: usize) -> Vec<u8> {
    (0..width).map(|i| (val >> i) & 1).collect()
}

/// Convert LSB-first bit vector to unsigned integer.
fn from_bits(bits: &[u8]) -> u8 {
    bits.iter()
        .enumerate()
        .fold(0u8, |acc, (i, &b)| acc | (b << i))
}

// ===========================================================================
// ADD operation
// ===========================================================================

#[test]
fn test_alu_add_basic() {
    let cases: Vec<(u8, u8, u8)> = vec![
        (0, 0, 0),
        (1, 1, 2),
        (3, 5, 8),
        (100, 55, 155),
        (10, 20, 30),
    ];

    for (a, b, expected) in cases {
        let result = alu(&to_bits(a, 8), &to_bits(b, 8), AluOp::Add);
        assert_eq!(
            from_bits(&result.result),
            expected,
            "{a} + {b} should equal {expected}"
        );
    }
}

#[test]
fn test_alu_add_zero_flag() {
    let result = alu(&to_bits(0, 8), &to_bits(0, 8), AluOp::Add);
    assert!(result.zero);
    assert!(!result.negative);
    assert!(!result.carry);
}

#[test]
fn test_alu_add_carry_flag() {
    // 200 + 100 = 300, which overflows 8-bit (300 - 256 = 44)
    let result = alu(&to_bits(200, 8), &to_bits(100, 8), AluOp::Add);
    assert!(result.carry, "200 + 100 should carry");
    assert_eq!(from_bits(&result.result), 44);
}

#[test]
fn test_alu_add_signed_overflow() {
    // 127 + 1 = 128, signed overflow (positive + positive = negative)
    let result = alu(&to_bits(127, 8), &to_bits(1, 8), AluOp::Add);
    assert!(result.overflow, "127 + 1 should overflow in signed");
    assert!(result.negative, "Result should be negative (MSB set)");
}

// ===========================================================================
// SUB operation
// ===========================================================================

#[test]
fn test_alu_sub_basic() {
    let cases: Vec<(u8, u8, u8)> = vec![
        (5, 3, 2),
        (10, 1, 9),
        (100, 50, 50),
        (255, 254, 1),
    ];

    for (a, b, expected) in cases {
        let result = alu(&to_bits(a, 8), &to_bits(b, 8), AluOp::Sub);
        assert_eq!(
            from_bits(&result.result),
            expected,
            "{a} - {b} should equal {expected}"
        );
    }
}

#[test]
fn test_alu_sub_to_zero() {
    let result = alu(&to_bits(42, 8), &to_bits(42, 8), AluOp::Sub);
    assert!(result.zero);
    assert_eq!(from_bits(&result.result), 0);
}

#[test]
fn test_alu_sub_negative_result() {
    // 1 - 2 in 8-bit two's complement = 255 unsigned, -1 signed
    let result = alu(&to_bits(1, 8), &to_bits(2, 8), AluOp::Sub);
    assert!(result.negative, "1 - 2 should be negative");
    assert_eq!(from_bits(&result.result), 255); // -1 in two's complement
}

#[test]
fn test_alu_sub_signed_overflow() {
    // -128 - 1 in 8-bit: 0x80 - 0x01
    // -128 = 10000000, -1 -> NOT(00000001)+1 = 11111111
    // 10000000 + 11111111 = 01111111 (127), which is wrong sign -> overflow
    let result = alu(&to_bits(128, 8), &to_bits(1, 8), AluOp::Sub);
    assert!(
        result.overflow,
        "-128 - 1 should overflow in signed 8-bit"
    );
}

// ===========================================================================
// Bitwise AND
// ===========================================================================

#[test]
fn test_alu_and() {
    let result = alu(
        &to_bits(0b11001100, 8),
        &to_bits(0b10101010, 8),
        AluOp::And,
    );
    assert_eq!(from_bits(&result.result), 0b10001000);
    assert!(!result.carry);
    assert!(!result.overflow);
}

#[test]
fn test_alu_and_zero() {
    let result = alu(&to_bits(0xFF, 8), &to_bits(0x00, 8), AluOp::And);
    assert!(result.zero);
}

#[test]
fn test_alu_and_identity() {
    // a AND 0xFF = a
    let result = alu(&to_bits(42, 8), &to_bits(0xFF, 8), AluOp::And);
    assert_eq!(from_bits(&result.result), 42);
}

// ===========================================================================
// Bitwise OR
// ===========================================================================

#[test]
fn test_alu_or() {
    let result = alu(
        &to_bits(0b11001100, 8),
        &to_bits(0b10101010, 8),
        AluOp::Or,
    );
    assert_eq!(from_bits(&result.result), 0b11101110);
}

#[test]
fn test_alu_or_identity() {
    // a OR 0 = a
    let result = alu(&to_bits(42, 8), &to_bits(0, 8), AluOp::Or);
    assert_eq!(from_bits(&result.result), 42);
}

// ===========================================================================
// Bitwise XOR
// ===========================================================================

#[test]
fn test_alu_xor() {
    let result = alu(
        &to_bits(0b11001100, 8),
        &to_bits(0b10101010, 8),
        AluOp::Xor,
    );
    assert_eq!(from_bits(&result.result), 0b01100110);
}

#[test]
fn test_alu_xor_self_is_zero() {
    // a XOR a = 0
    let result = alu(&to_bits(0xAB, 8), &to_bits(0xAB, 8), AluOp::Xor);
    assert!(result.zero);
}

// ===========================================================================
// Bitwise NOT
// ===========================================================================

#[test]
fn test_alu_not() {
    let result = alu(&to_bits(0b00001111, 8), &to_bits(0, 8), AluOp::Not);
    assert_eq!(from_bits(&result.result), 0b11110000);
}

#[test]
fn test_alu_not_all_ones() {
    let result = alu(&to_bits(0xFF, 8), &to_bits(0, 8), AluOp::Not);
    assert!(result.zero, "NOT(0xFF) should be 0x00");
}

#[test]
fn test_alu_not_zero() {
    let result = alu(&to_bits(0, 8), &to_bits(0, 8), AluOp::Not);
    assert_eq!(from_bits(&result.result), 0xFF);
    assert!(result.negative, "NOT(0x00) = 0xFF, MSB is set");
}

// ===========================================================================
// Flag combinations
// ===========================================================================

#[test]
fn test_alu_add_all_flags_clear() {
    let result = alu(&to_bits(1, 8), &to_bits(1, 8), AluOp::Add);
    assert!(!result.zero);
    assert!(!result.carry);
    assert!(!result.negative);
    assert!(!result.overflow);
    assert_eq!(from_bits(&result.result), 2);
}

// ===========================================================================
// 4-bit ALU (smaller width)
// ===========================================================================

#[test]
fn test_alu_4bit_add() {
    let a = to_bits(7, 4);
    let b = to_bits(5, 4);
    let result = alu(&a, &b, AluOp::Add);
    // 7 + 5 = 12
    assert_eq!(from_bits(&result.result), 12);
    assert!(!result.carry);
}

#[test]
fn test_alu_4bit_sub() {
    let a = to_bits(9, 4);
    let b = to_bits(3, 4);
    let result = alu(&a, &b, AluOp::Sub);
    assert_eq!(from_bits(&result.result), 6);
}

// ===========================================================================
// Edge cases
// ===========================================================================

#[test]
#[should_panic(expected = "a must not be empty")]
fn test_alu_empty_input() {
    alu(&[], &[], AluOp::Add);
}

#[test]
#[should_panic(expected = "a and b must have the same length")]
fn test_alu_mismatched_lengths() {
    alu(&[0, 1], &[0, 1, 0], AluOp::Add);
}

#[test]
fn test_alu_1bit() {
    // 1-bit ALU: 1 + 1 = 0 with carry
    let result = alu(&[1], &[1], AluOp::Add);
    assert_eq!(result.result, vec![0]);
    assert!(result.carry);
}
