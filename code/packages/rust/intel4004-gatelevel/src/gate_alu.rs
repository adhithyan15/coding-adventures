//! 4-bit ALU -- the arithmetic heart of the Intel 4004.
//!
//! # How the real 4004's ALU worked
//!
//! The Intel 4004 had a 4-bit ALU that could add, subtract, and perform
//! logical operations on 4-bit values. It used a ripple-carry adder built
//! from full adders, which were themselves built from AND, OR, and XOR gates.
//!
//! This module wraps the arithmetic crate's `alu()` function to provide
//! the exact operations the 4004 needs. Every addition and subtraction
//! physically routes through the gate chain:
//!
//! ```text
//! XOR -> AND -> OR -> full_adder -> ripple_carry_adder -> ALU
//! ```
//!
//! That's real hardware simulation -- not behavioral shortcuts.
//!
//! # Subtraction via complement-add
//!
//! The 4004 doesn't have a dedicated subtractor. Instead, it uses the
//! ones' complement method:
//!
//! ```text
//! A - B = A + NOT(B) + borrow_in
//! ```
//!
//! where borrow_in = 0 if carry_flag else 1 (inverted carry semantics).
//! The ALU's SUB operation does this internally using NOT gates to
//! complement B, then feeding through the same adder.

use arithmetic::alu::{alu, AluOp};

use crate::bits::{bits_to_int, int_to_bits};

/// 4-bit ALU for the Intel 4004 gate-level simulator.
///
/// All operations route through real logic gates via the arithmetic
/// crate's `alu()` function. No behavioral shortcuts.
///
/// The ALU provides:
/// - `add(a, b, carry_in)` -> `(result, carry_out)`
/// - `subtract(a, b, borrow_in)` -> `(result, carry_out)`
/// - `complement(a)` -> result (4-bit NOT)
/// - `increment(a)` -> `(result, carry_out)`
/// - `decrement(a)` -> `(result, borrow_out)`
pub struct GateALU {
    /// Bit width of this ALU (always 4 for the Intel 4004).
    bit_width: usize,
}

impl GateALU {
    /// Create a 4-bit ALU using real logic gates.
    pub fn new() -> Self {
        Self { bit_width: 4 }
    }

    /// Add two 4-bit values with carry.
    ///
    /// Routes through: XOR -> AND -> OR -> full_adder x 4 -> ripple_carry
    ///
    /// Returns `(result, carry_out)` where result is 4-bit (0-15).
    pub fn add(&self, a: u8, b: u8, carry_in: u8) -> (u8, bool) {
        let a_bits = int_to_bits(a as u16, self.bit_width);
        let b_bits = int_to_bits(b as u16, self.bit_width);

        if carry_in != 0 {
            // Add carry_in by first adding a+b, then adding 1.
            // This simulates the carry input to the LSB full adder.
            let result1 = alu(&a_bits, &b_bits, AluOp::Add);
            let one_bits = int_to_bits(1, self.bit_width);
            let result2 = alu(&result1.result, &one_bits, AluOp::Add);
            // Carry is set if either addition overflowed.
            let carry = result1.carry || result2.carry;
            (bits_to_int(&result2.result) as u8, carry)
        } else {
            let result = alu(&a_bits, &b_bits, AluOp::Add);
            (bits_to_int(&result.result) as u8, result.carry)
        }
    }

    /// Subtract using complement-add: A + NOT(B) + borrow_in.
    ///
    /// The 4004's carry flag semantics for subtraction:
    /// - `carry=true`  -> no borrow (result >= 0)
    /// - `carry=false` -> borrow occurred
    ///
    /// Returns `(result, carry_out)` where carry_out=true means no borrow.
    pub fn subtract(&self, a: u8, b: u8, borrow_in: u8) -> (u8, bool) {
        // Complement b using NOT gates
        let b_bits = int_to_bits(b as u16, self.bit_width);
        let b_comp = alu(&b_bits, &b_bits, AluOp::Not);
        // A + NOT(B) + borrow_in
        self.add(a, bits_to_int(&b_comp.result) as u8, borrow_in)
    }

    /// 4-bit NOT: invert all bits using NOT gates.
    pub fn complement(&self, a: u8) -> u8 {
        let a_bits = int_to_bits(a as u16, self.bit_width);
        let result = alu(&a_bits, &a_bits, AluOp::Not);
        bits_to_int(&result.result) as u8
    }

    /// Increment by 1 using the adder. Returns `(result, carry)`.
    pub fn increment(&self, a: u8) -> (u8, bool) {
        self.add(a, 1, 0)
    }

    /// Decrement by 1 using complement-add.
    ///
    /// `A - 1 = A + NOT(1) + 1 = A + 14 + 1 = A + 15`.
    /// carry=true if A > 0 (no borrow), false if A == 0.
    pub fn decrement(&self, a: u8) -> (u8, bool) {
        self.subtract(a, 1, 1)
    }

    /// 4-bit AND using AND gates.
    pub fn bitwise_and(&self, a: u8, b: u8) -> u8 {
        let a_bits = int_to_bits(a as u16, self.bit_width);
        let b_bits = int_to_bits(b as u16, self.bit_width);
        let result = alu(&a_bits, &b_bits, AluOp::And);
        bits_to_int(&result.result) as u8
    }

    /// 4-bit OR using OR gates.
    pub fn bitwise_or(&self, a: u8, b: u8) -> u8 {
        let a_bits = int_to_bits(a as u16, self.bit_width);
        let b_bits = int_to_bits(b as u16, self.bit_width);
        let result = alu(&a_bits, &b_bits, AluOp::Or);
        bits_to_int(&result.result) as u8
    }

    /// Estimated gate count for a 4-bit ALU.
    ///
    /// Each full adder: 5 gates (2 XOR + 2 AND + 1 OR).
    /// 4-bit ripple carry: 4 x 5 = 20 gates.
    /// SUB complement: 4 NOT gates.
    /// Control muxing: ~8 gates.
    /// Total: ~32 gates.
    pub fn gate_count(&self) -> usize {
        32
    }
}

impl Default for GateALU {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_simple() {
        let alu = GateALU::new();
        let (result, carry) = alu.add(3, 5, 0);
        assert_eq!(result, 8);
        assert!(!carry);
    }

    #[test]
    fn test_add_overflow() {
        let alu = GateALU::new();
        let (result, carry) = alu.add(15, 1, 0);
        assert_eq!(result, 0);
        assert!(carry);
    }

    #[test]
    fn test_add_with_carry() {
        let alu = GateALU::new();
        let (result, carry) = alu.add(3, 5, 1);
        assert_eq!(result, 9);
        assert!(!carry);
    }

    #[test]
    fn test_subtract_simple() {
        let alu = GateALU::new();
        let (result, carry) = alu.subtract(5, 3, 1);
        assert_eq!(result, 2);
        assert!(carry); // no borrow
    }

    #[test]
    fn test_subtract_borrow() {
        let alu = GateALU::new();
        let (result, carry) = alu.subtract(3, 5, 1);
        // 3 - 5 wraps in 4-bit: 3 + NOT(5) + 1 = 3 + 10 + 1 = 14
        assert_eq!(result, 14);
        assert!(!carry); // borrow occurred
    }

    #[test]
    fn test_complement() {
        let alu = GateALU::new();
        assert_eq!(alu.complement(0), 15);
        assert_eq!(alu.complement(15), 0);
        assert_eq!(alu.complement(5), 10);
    }

    #[test]
    fn test_increment() {
        let alu = GateALU::new();
        let (result, carry) = alu.increment(7);
        assert_eq!(result, 8);
        assert!(!carry);
    }

    #[test]
    fn test_increment_overflow() {
        let alu = GateALU::new();
        let (result, carry) = alu.increment(15);
        assert_eq!(result, 0);
        assert!(carry);
    }

    #[test]
    fn test_decrement() {
        let alu = GateALU::new();
        let (result, carry) = alu.decrement(5);
        assert_eq!(result, 4);
        assert!(carry); // no borrow
    }

    #[test]
    fn test_decrement_underflow() {
        let alu = GateALU::new();
        let (result, carry) = alu.decrement(0);
        assert_eq!(result, 15);
        assert!(!carry); // borrow
    }

    #[test]
    fn test_bitwise_and() {
        let alu = GateALU::new();
        assert_eq!(alu.bitwise_and(0b1100, 0b1010), 0b1000);
    }

    #[test]
    fn test_bitwise_or() {
        let alu = GateALU::new();
        assert_eq!(alu.bitwise_or(0b1100, 0b1010), 0b1110);
    }
}
