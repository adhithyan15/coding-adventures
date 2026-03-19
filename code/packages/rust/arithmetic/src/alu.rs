//! Arithmetic Logic Unit (ALU) — the computational heart of a CPU.
//!
//! The ALU takes two N-bit inputs and an operation code, produces an N-bit
//! result plus status flags. It is built from adders and logic gates — no
//! magic, just carefully wired hardware.
//!
//! # What an ALU does
//!
//! Every instruction a CPU executes — add, subtract, compare, bitwise AND —
//! ultimately passes through the ALU. It is the core "calculator" inside
//! every processor.
//!
//! # Status flags
//!
//! The ALU produces four status flags alongside its result:
//!
//! - **Zero** — is the result all zeros? Used for equality comparisons.
//! - **Carry** — did the addition overflow the bit width? Used for unsigned arithmetic.
//! - **Negative** — is the MSB set? In two's complement, this means the result is negative.
//! - **Overflow** — did signed arithmetic produce a wrong-sign result?
//!
//! These flags are stored in a "status register" and used by conditional
//! branch instructions (e.g., "jump if zero", "jump if negative").

use logic_gates::gates::{and_gate, not_gate, or_gate, xor_gate};

use crate::adders::ripple_carry_adder_with_carry;

/// ALU operation codes.
///
/// Each variant corresponds to a single operation the ALU can perform.
/// In real hardware, these would be encoded as a few bits on the ALU's
/// control input lines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AluOp {
    /// Addition: A + B
    Add,
    /// Subtraction: A - B (implemented as A + NOT(B) + 1, two's complement)
    Sub,
    /// Bitwise AND: each bit of result = AND(A_bit, B_bit)
    And,
    /// Bitwise OR: each bit of result = OR(A_bit, B_bit)
    Or,
    /// Bitwise XOR: each bit of result = XOR(A_bit, B_bit)
    Xor,
    /// Bitwise NOT: each bit of result = NOT(A_bit). B is ignored.
    Not,
}

/// Result of an ALU operation, including status flags.
///
/// # Flags explained
///
/// ```text
/// Zero:     result == 0                  (all bits are 0)
/// Carry:    unsigned overflow            (result doesn't fit in N bits)
/// Negative: result < 0 in signed        (MSB is 1)
/// Overflow: signed overflow             (sign of result is wrong)
/// ```
///
/// # When does signed overflow occur?
///
/// Overflow happens when the mathematical result cannot be represented
/// in N-bit two's complement:
/// - Adding two positive numbers and getting a negative result
/// - Adding two negative numbers and getting a positive result
/// - Subtracting a negative from a positive and getting negative
/// - Subtracting a positive from a negative and getting positive
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AluResult {
    /// Result bits (LSB first, index 0 = least significant bit).
    pub result: Vec<u8>,
    /// True if all result bits are 0.
    pub zero: bool,
    /// True if addition produced an unsigned overflow (carry out of MSB).
    pub carry: bool,
    /// True if the MSB of the result is 1 (negative in two's complement).
    pub negative: bool,
    /// True if signed overflow occurred.
    pub overflow: bool,
}

/// Execute an ALU operation on two N-bit inputs.
///
/// Both `a` and `b` are bit slices in LSB-first order.
/// For the `Not` operation, `b` is ignored (pass an empty slice or same-length zeros).
///
/// # Panics
///
/// Panics if `a` and `b` have different lengths (except for `Not`),
/// or if either is empty.
///
/// # Example
///
/// ```
/// use arithmetic::alu::{alu, AluOp};
///
/// // 3 + 5 = 8 in 8-bit
/// let a = vec![1, 1, 0, 0, 0, 0, 0, 0]; // 3 LSB-first
/// let b = vec![1, 0, 1, 0, 0, 0, 0, 0]; // 5 LSB-first
/// let result = alu(&a, &b, AluOp::Add);
/// assert_eq!(result.result, vec![0, 0, 0, 1, 0, 0, 0, 0]); // 8
/// assert!(!result.zero);
/// assert!(!result.carry);
/// ```
pub fn alu(a: &[u8], b: &[u8], op: AluOp) -> AluResult {
    assert!(!a.is_empty(), "a must not be empty");

    if op != AluOp::Not {
        assert_eq!(
            a.len(),
            b.len(),
            "a and b must have the same length, got {} and {}",
            a.len(),
            b.len()
        );
    }

    let (result, carry) = match op {
        AluOp::Add => {
            let r = ripple_carry_adder_with_carry(a, b, 0);
            (r.sum, r.carry_out == 1)
        }
        AluOp::Sub => {
            // A - B = A + NOT(B) + 1 (two's complement subtraction)
            //
            // Why this works:
            // In two's complement, -B = NOT(B) + 1
            // So A - B = A + (-B) = A + NOT(B) + 1
            //
            // We implement "+1" by setting carry_in = 1 in the adder.
            let not_b: Vec<u8> = b.iter().map(|&bit| not_gate(bit)).collect();
            let r = ripple_carry_adder_with_carry(a, &not_b, 1);
            (r.sum, r.carry_out == 1)
        }
        AluOp::And => {
            let result: Vec<u8> = a
                .iter()
                .zip(b.iter())
                .map(|(&ai, &bi)| and_gate(ai, bi))
                .collect();
            (result, false)
        }
        AluOp::Or => {
            let result: Vec<u8> = a
                .iter()
                .zip(b.iter())
                .map(|(&ai, &bi)| or_gate(ai, bi))
                .collect();
            (result, false)
        }
        AluOp::Xor => {
            let result: Vec<u8> = a
                .iter()
                .zip(b.iter())
                .map(|(&ai, &bi)| xor_gate(ai, bi))
                .collect();
            (result, false)
        }
        AluOp::Not => {
            let result: Vec<u8> = a.iter().map(|&bit| not_gate(bit)).collect();
            (result, false)
        }
    };

    let zero = result.iter().all(|&bit| bit == 0);
    let negative = if result.is_empty() {
        false
    } else {
        result[result.len() - 1] == 1
    };

    // Signed overflow detection for ADD and SUB
    let overflow = match op {
        AluOp::Add => {
            let a_sign = a[a.len() - 1];
            let b_sign = b[b.len() - 1];
            let r_sign = result[result.len() - 1];
            (a_sign == b_sign) && (r_sign != a_sign)
        }
        AluOp::Sub => {
            // For subtraction, the effective "b_sign" is NOT(b_sign) because
            // we're adding NOT(B)+1.
            let a_sign = a[a.len() - 1];
            let b_sign = not_gate(b[b.len() - 1]);
            let r_sign = result[result.len() - 1];
            (a_sign == b_sign) && (r_sign != a_sign)
        }
        _ => false,
    };

    AluResult {
        result,
        zero,
        carry,
        negative,
        overflow,
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: convert an integer to LSB-first bit vector.
    fn to_bits(val: u8, width: usize) -> Vec<u8> {
        (0..width).map(|i| (val >> i) & 1).collect()
    }

    /// Helper: convert LSB-first bit vector to integer.
    fn from_bits(bits: &[u8]) -> u8 {
        bits.iter()
            .enumerate()
            .fold(0u8, |acc, (i, &b)| acc | (b << i))
    }

    #[test]
    fn test_alu_add_simple() {
        let a = to_bits(3, 8);
        let b = to_bits(5, 8);
        let result = alu(&a, &b, AluOp::Add);
        assert_eq!(from_bits(&result.result), 8);
        assert!(!result.zero);
        assert!(!result.carry);
    }

    #[test]
    fn test_alu_add_zero_flag() {
        let a = to_bits(0, 8);
        let b = to_bits(0, 8);
        let result = alu(&a, &b, AluOp::Add);
        assert!(result.zero);
    }

    #[test]
    fn test_alu_sub_simple() {
        let a = to_bits(10, 8);
        let b = to_bits(3, 8);
        let result = alu(&a, &b, AluOp::Sub);
        assert_eq!(from_bits(&result.result), 7);
    }

    #[test]
    fn test_alu_sub_to_zero() {
        let a = to_bits(5, 8);
        let b = to_bits(5, 8);
        let result = alu(&a, &b, AluOp::Sub);
        assert!(result.zero);
        assert_eq!(from_bits(&result.result), 0);
    }

    #[test]
    fn test_alu_and() {
        let a = to_bits(0b1100, 8);
        let b = to_bits(0b1010, 8);
        let result = alu(&a, &b, AluOp::And);
        assert_eq!(from_bits(&result.result), 0b1000);
    }

    #[test]
    fn test_alu_or() {
        let a = to_bits(0b1100, 8);
        let b = to_bits(0b1010, 8);
        let result = alu(&a, &b, AluOp::Or);
        assert_eq!(from_bits(&result.result), 0b1110);
    }

    #[test]
    fn test_alu_xor() {
        let a = to_bits(0b1100, 8);
        let b = to_bits(0b1010, 8);
        let result = alu(&a, &b, AluOp::Xor);
        assert_eq!(from_bits(&result.result), 0b0110);
    }

    #[test]
    fn test_alu_not() {
        let a = to_bits(0b00001111, 8);
        let b = to_bits(0, 8); // ignored
        let result = alu(&a, &b, AluOp::Not);
        assert_eq!(from_bits(&result.result), 0b11110000);
    }

    #[test]
    fn test_alu_negative_flag() {
        // 1 - 2 in 8-bit two's complement = 255 unsigned, -1 signed
        let a = to_bits(1, 8);
        let b = to_bits(2, 8);
        let result = alu(&a, &b, AluOp::Sub);
        assert!(result.negative, "1 - 2 should be negative");
    }
}
