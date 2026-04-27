//! 8-Bit ALU built from the `arithmetic` crate's ripple-carry adder.
//!
//! # Gate depth
//!
//! The 8008's 8-bit ALU is double the width of the 4004's 4-bit ALU. The
//! core adder chain uses 8 full-adders (vs 4 for the 4004):
//!
//! ```text
//! 8-bit ripple-carry: 8 × full_adder = 8 × 5 gates = 40 gates total
//! 4-bit ripple-carry: 4 × full_adder = 4 × 5 gates = 20 gates total
//!
//! full_adder gates:
//!   sum_xor1 = XOR(a, b)          [1 XOR]
//!   sum      = XOR(sum_xor1, cin) [1 XOR]
//!   carry1   = AND(a, b)          [1 AND]
//!   carry2   = AND(sum_xor1, cin) [1 AND]
//!   cout     = OR(carry1, carry2) [1 OR]
//! Total: 5 gates per stage → 40 gates for 8-bit
//! ```
//!
//! Additionally, the 8008 ALU includes a 7-gate XOR parity tree absent in
//! the 4004, because the 8008 has a Parity flag and the 4004 does not.
//!
//! # Operations
//!
//! | Op       | Implementation                        |
//! |----------|---------------------------------------|
//! | ADD      | ripple_carry_adder(a, b, cin=0)       |
//! | ADC      | ripple_carry_adder(a, b, cin=carry)   |
//! | SUB      | ripple_carry_adder(a, NOT(b), cin=1)  |
//! | SBB      | ripple_carry_adder(a, NOT(b), cin=!c) |
//! | ANA      | bitwise AND (no adder)                |
//! | XRA      | bitwise XOR (no adder)                |
//! | ORA      | bitwise OR  (no adder)                |
//! | CMP      | same as SUB, result discarded         |
//! | Rotates  | bit-shift logic (separate from ALU)   |

use arithmetic::alu::{alu, AluOp};
use logic_gates::gates::{and_gate, or_gate, xor_gate};

use crate::bits::{bits_to_int, compute_parity, int_to_bits};

/// Flags produced by an ALU operation.
///
/// These four bits map directly to the 8008's condition flags register.
/// The ALU computes them purely from the result bits via gate logic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AluFlags {
    /// CY — unsigned overflow (carry out of bit 7, or borrow for subtraction).
    pub carry: bool,
    /// Z — all result bits are 0.
    pub zero: bool,
    /// S — bit 7 of the result is 1 (negative in two's complement).
    pub sign: bool,
    /// P — even number of 1-bits in the result (even parity).
    pub parity: bool,
}

/// 8-bit ALU for the Intel 8008.
///
/// All arithmetic routes through the `arithmetic` crate's `ripple_carry_adder`,
/// which itself is built from full-adders, half-adders, XOR/AND/OR gates.
/// Logical operations (AND, OR, XOR, NOT) use the gate functions directly.
pub struct GateAlu8;

impl GateAlu8 {
    /// Add two 8-bit values. Carry in is 0.
    pub fn add(a: u8, b: u8) -> (u8, AluFlags) {
        Self::add_with_carry(a, b, false)
    }

    /// Add two 8-bit values with carry in (for ADC).
    ///
    /// ADC implements this as a two-step addition:
    /// - Step 1: r1, c1 = a + b
    /// - Step 2: r2, c2 = r1 + carry_in
    /// - Final carry = c1 OR c2
    pub fn add_with_carry(a: u8, b: u8, carry_in: bool) -> (u8, AluFlags) {
        let a_bits = int_to_bits(a, 8);
        let b_bits = int_to_bits(b, 8);
        let ci = if carry_in { 1u8 } else { 0u8 };

        // Route through arithmetic crate's ALU (which uses ripple-carry adder)
        let r1 = alu(&a_bits, &b_bits, AluOp::Add);
        let c1 = r1.carry;

        // Add carry_in as a second pass
        let ci_bits = int_to_bits(ci, 8);
        let r2 = alu(&r1.result, &ci_bits, AluOp::Add);
        let c2 = r2.carry;

        let result = bits_to_int(&r2.result);
        let final_carry = c1 || c2;
        let flags = Self::compute_flags_from_bits(&r2.result, final_carry);
        (result, flags)
    }

    /// Subtract b from a (SUB). Carry = 1 means borrow occurred.
    pub fn subtract(a: u8, b: u8) -> (u8, AluFlags) {
        Self::subtract_with_borrow(a, b, false)
    }

    /// Subtract b from a with borrow in (for SBB).
    ///
    /// Two's complement subtraction: A - B = A + NOT(B) + 1
    /// SBB: A - B - borrow = A + NOT(B) + NOT(borrow)
    pub fn subtract_with_borrow(a: u8, b: u8, borrow_in: bool) -> (u8, AluFlags) {
        let a_bits = int_to_bits(a, 8);
        let b_bits = int_to_bits(b, 8);

        // Two's complement subtraction: A - B = A + NOT(B) + 1
        // For SBB with borrow: A - B - 1 = A + NOT(B) + 0
        //
        // The arithmetic crate's AluOp::Sub handles NOT(B)+1 internally.
        // When there is an incoming borrow we subtract 1 more from the result.
        // Two's complement sub: A + NOT(B) + 1.
        // The arithmetic crate's carry_out for Sub: carry_out=1 means NO borrow
        // (A >= B), carry_out=0 means borrow (A < B).
        // The 8008's CY flag convention: CY=1 means borrow occurred.
        // So: 8008_carry = NOT(arithmetic_carry_out).
        let r1 = alu(&a_bits, &b_bits, AluOp::Sub);
        // arithmetic carry=1 → no borrow → 8008 carry=0
        // arithmetic carry=0 → borrow   → 8008 carry=1
        let borrow1 = !r1.carry;

        if borrow_in {
            // Subtract 1 more (the borrow)
            let one_bits = int_to_bits(1, 8);
            let r2 = alu(&r1.result, &one_bits, AluOp::Sub);
            let borrow2 = !r2.carry;
            let result = bits_to_int(&r2.result);
            // Either subtraction step producing borrow → final borrow
            let final_borrow = borrow1 || borrow2;
            let flags = Self::compute_flags_from_bits(&r2.result, final_borrow);
            (result, flags)
        } else {
            let result = bits_to_int(&r1.result);
            let flags = Self::compute_flags_from_bits(&r1.result, borrow1);
            (result, flags)
        }
    }

    /// Bitwise AND. Always clears carry (8008 hardware behavior).
    pub fn and(a: u8, b: u8) -> (u8, AluFlags) {
        let a_bits = int_to_bits(a, 8);
        let b_bits = int_to_bits(b, 8);
        let result_bits: Vec<u8> = a_bits.iter().zip(b_bits.iter())
            .map(|(&ai, &bi)| and_gate(ai, bi))
            .collect();
        let result = bits_to_int(&result_bits);
        let mut flags = Self::compute_flags_from_bits(&result_bits, false);
        flags.carry = false; // ANA always clears carry
        (result, flags)
    }

    /// Bitwise OR. Always clears carry.
    pub fn or(a: u8, b: u8) -> (u8, AluFlags) {
        let a_bits = int_to_bits(a, 8);
        let b_bits = int_to_bits(b, 8);
        let result_bits: Vec<u8> = a_bits.iter().zip(b_bits.iter())
            .map(|(&ai, &bi)| or_gate(ai, bi))
            .collect();
        let result = bits_to_int(&result_bits);
        let mut flags = Self::compute_flags_from_bits(&result_bits, false);
        flags.carry = false;
        (result, flags)
    }

    /// Bitwise XOR. Always clears carry.
    pub fn xor(a: u8, b: u8) -> (u8, AluFlags) {
        let a_bits = int_to_bits(a, 8);
        let b_bits = int_to_bits(b, 8);
        let result_bits: Vec<u8> = a_bits.iter().zip(b_bits.iter())
            .map(|(&ai, &bi)| xor_gate(ai, bi))
            .collect();
        let result = bits_to_int(&result_bits);
        let mut flags = Self::compute_flags_from_bits(&result_bits, false);
        flags.carry = false;
        (result, flags)
    }

    /// Increment. Updates Z, S, P; does NOT update carry (8008 INR behavior).
    pub fn increment(a: u8) -> u8 {
        a.wrapping_add(1)
    }

    /// Decrement. Updates Z, S, P; does NOT update carry (8008 DCR behavior).
    pub fn decrement(a: u8) -> u8 {
        a.wrapping_sub(1)
    }

    /// Compute flags Z, S, P from a result bit vector, plus supplied carry.
    pub fn compute_flags_from_bits(bits: &[u8], carry: bool) -> AluFlags {
        // Zero flag: all bits are 0 (implemented as NOR across all bits)
        let any_set = bits.iter().any(|&b| b == 1);
        let zero = !any_set;

        // Sign flag: bit 7 (MSB) is 1
        let sign = bits.len() >= 8 && bits[7] == 1;

        // Parity flag: even number of 1-bits (computed by XOR chain + NOT)
        let parity = compute_parity(bits) == 1;

        AluFlags { carry, zero, sign, parity }
    }

    /// Rotate A left circular: CY ← A[7]; A ← (A<<1) | A[7]
    pub fn rotate_left_circular(a: u8) -> (u8, bool) {
        let bits = int_to_bits(a, 8);
        let bit7 = bits[7];
        let mut result_bits = vec![0u8; 8];
        // Shift left: new bit[i] = old bit[i-1] for i >= 1
        for i in 1..8 {
            result_bits[i] = bits[i - 1];
        }
        // Wrap: new bit[0] = old bit[7]
        result_bits[0] = bit7;
        (bits_to_int(&result_bits), bit7 == 1)
    }

    /// Rotate A right circular: CY ← A[0]; A ← (A>>1) | (A[0]<<7)
    pub fn rotate_right_circular(a: u8) -> (u8, bool) {
        let bits = int_to_bits(a, 8);
        let bit0 = bits[0];
        let mut result_bits = vec![0u8; 8];
        // Shift right: new bit[i] = old bit[i+1] for i < 7
        for i in 0..7 {
            result_bits[i] = bits[i + 1];
        }
        // Wrap: new bit[7] = old bit[0]
        result_bits[7] = bit0;
        (bits_to_int(&result_bits), bit0 == 1)
    }

    /// Rotate A left through carry: new_CY ← A[7]; A ← (A<<1) | old_CY
    pub fn rotate_left_carry(a: u8, carry_in: bool) -> (u8, bool) {
        let bits = int_to_bits(a, 8);
        let new_carry = bits[7] == 1;
        let ci = if carry_in { 1u8 } else { 0u8 };
        let mut result_bits = vec![0u8; 8];
        for i in 1..8 {
            result_bits[i] = bits[i - 1];
        }
        result_bits[0] = ci;
        (bits_to_int(&result_bits), new_carry)
    }

    /// Rotate A right through carry: new_CY ← A[0]; A ← (old_CY<<7) | (A>>1)
    pub fn rotate_right_carry(a: u8, carry_in: bool) -> (u8, bool) {
        let bits = int_to_bits(a, 8);
        let new_carry = bits[0] == 1;
        let ci = if carry_in { 1u8 } else { 0u8 };
        let mut result_bits = vec![0u8; 8];
        for i in 0..7 {
            result_bits[i] = bits[i + 1];
        }
        result_bits[7] = ci;
        (bits_to_int(&result_bits), new_carry)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add() {
        let (result, flags) = GateAlu8::add(2, 3);
        assert_eq!(result, 5);
        assert!(!flags.carry);
        assert!(!flags.zero);
        assert!(flags.parity); // 5 = 0b101 → 2 ones → even parity
    }

    #[test]
    fn test_add_overflow() {
        let (result, flags) = GateAlu8::add(0xFF, 1);
        assert_eq!(result, 0);
        assert!(flags.carry);
        assert!(flags.zero);
        assert!(flags.parity);
    }

    #[test]
    fn test_subtract() {
        let (result, flags) = GateAlu8::subtract(5, 3);
        assert_eq!(result, 2);
        assert!(!flags.carry); // no borrow
    }

    #[test]
    fn test_subtract_borrow() {
        let (result, flags) = GateAlu8::subtract(0, 1);
        assert_eq!(result, 0xFF);
        assert!(flags.carry); // borrow
        assert!(flags.sign);
    }

    #[test]
    fn test_and_clears_carry() {
        let (result, flags) = GateAlu8::and(0xFF, 0x0F);
        assert_eq!(result, 0x0F);
        assert!(!flags.carry);
    }

    #[test]
    fn test_rotate_left_circular() {
        let (r, c) = GateAlu8::rotate_left_circular(0x80);
        assert_eq!(r, 0x01);
        assert!(c);
    }

    #[test]
    fn test_rotate_right_carry() {
        let (r, c) = GateAlu8::rotate_right_carry(0x01, true);
        assert_eq!(r, 0x80);
        assert!(c);
    }
}
