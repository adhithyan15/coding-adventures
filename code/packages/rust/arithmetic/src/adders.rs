//! Adder circuits built from logic gates.
//!
//! # From gates to addition
//!
//! How do you add two numbers using only AND, OR, and XOR gates? The key
//! insight is that binary addition of two single bits works exactly like
//! XOR (for the sum) and AND (for the carry):
//!
//! ```text
//! 0 + 0 = 0, carry 0   ->  XOR(0,0)=0, AND(0,0)=0
//! 0 + 1 = 1, carry 0   ->  XOR(0,1)=1, AND(0,1)=0
//! 1 + 0 = 1, carry 0   ->  XOR(1,0)=1, AND(1,0)=0
//! 1 + 1 = 0, carry 1   ->  XOR(1,1)=0, AND(1,1)=1
//! ```
//!
//! This is the **half adder**. To handle carry-in from a previous column,
//! we chain two half adders into a **full adder**. To add N-bit numbers,
//! we chain N full adders into a **ripple-carry adder**.
//!
//! # Why "ripple carry"?
//!
//! The carry "ripples" from the least significant bit to the most significant
//! bit, one full adder at a time. This is simple but slow for large bit
//! widths because each adder must wait for the previous one's carry output.
//! Real CPUs use faster designs (carry-lookahead, carry-select) but they
//! all build on the same fundamental principles shown here.

use logic_gates::gates::{and_gate, or_gate, xor_gate};

/// Half adder — adds two single bits.
///
/// Returns `(sum, carry)`.
///
/// The half adder is the simplest adding circuit. It handles the addition
/// of two bits but cannot accept a carry-in from a previous addition.
///
/// # Circuit
///
/// ```text
/// a --+--[XOR]-- sum
///     |
/// b --+--[AND]-- carry
/// ```
///
/// # Truth table
///
/// ```text
/// A  B  | Sum  Carry
/// ------+-----------
/// 0  0  |  0     0     (0 + 0 = 0)
/// 0  1  |  1     0     (0 + 1 = 1)
/// 1  0  |  1     0     (1 + 0 = 1)
/// 1  1  |  0     1     (1 + 1 = 10 in binary)
/// ```
///
/// # Example
///
/// ```
/// use arithmetic::adders::half_adder;
/// assert_eq!(half_adder(1, 1), (0, 1)); // 1 + 1 = 10 binary
/// assert_eq!(half_adder(1, 0), (1, 0)); // 1 + 0 = 1
/// ```
#[inline]
pub fn half_adder(a: u8, b: u8) -> (u8, u8) {
    (xor_gate(a, b), and_gate(a, b))
}

/// Full adder — adds two bits plus a carry-in from a previous addition.
///
/// Returns `(sum, carry_out)`.
///
/// Built from two half adders and an OR gate:
/// 1. Half-add `a` and `b` -> `partial_sum`, `partial_carry`
/// 2. Half-add `partial_sum` and `carry_in` -> `sum`, `carry2`
/// 3. `carry_out = OR(partial_carry, carry2)`
///
/// # Circuit
///
/// ```text
/// a --------+
///           |--[HA]-- partial_sum --+
/// b --------+         |             |--[HA]-- sum
///                     |  carry_in --+         |
///                     |                       |
///           partial_carry            carry2   |
///                     |                |      |
///                     +--[OR]----------+      |
///                         |                   |
///                     carry_out                |
/// ```
///
/// # Example
///
/// ```
/// use arithmetic::adders::full_adder;
/// assert_eq!(full_adder(1, 1, 0), (0, 1)); // 1+1+0 = 10
/// assert_eq!(full_adder(1, 1, 1), (1, 1)); // 1+1+1 = 11
/// ```
#[inline]
pub fn full_adder(a: u8, b: u8, carry_in: u8) -> (u8, u8) {
    let (partial_sum, partial_carry) = half_adder(a, b);
    let (sum, carry2) = half_adder(partial_sum, carry_in);
    let carry_out = or_gate(partial_carry, carry2);
    (sum, carry_out)
}

/// Result of a ripple-carry addition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RippleCarryResult {
    /// Sum bits (LSB first, index 0 = least significant bit).
    pub sum: Vec<u8>,
    /// Carry out of the most significant bit position.
    pub carry_out: u8,
    /// Signed overflow: set when adding two positive numbers produces a
    /// negative result, or two negative numbers produces positive.
    /// Only meaningful when interpreting bits as two's complement.
    pub overflow: bool,
}

/// Ripple-carry adder — adds two N-bit numbers using a chain of full adders.
///
/// Each full adder handles one bit position, passing its carry output to
/// the next position. The carry "ripples" from LSB to MSB.
///
/// # Arguments
///
/// - `a` — first number as bits, LSB first (index 0 = least significant)
/// - `b` — second number as bits, LSB first
///
/// Both slices must have the same length and be non-empty.
///
/// # Carry-in
///
/// The initial carry-in defaults to 0. For subtraction (A - B), you'd
/// invert B and set carry-in to 1 (two's complement negation). The ALU
/// module handles this automatically.
///
/// # Example
///
/// ```
/// use arithmetic::adders::ripple_carry_adder;
///
/// // 5 + 3 = 8
/// // 5 = 0101 (LSB first: [1, 0, 1, 0])
/// // 3 = 0011 (LSB first: [1, 1, 0, 0])
/// // 8 = 1000 (LSB first: [0, 0, 0, 1])
/// let result = ripple_carry_adder(&[1, 0, 1, 0], &[1, 1, 0, 0]);
/// assert_eq!(result.sum, vec![0, 0, 0, 1]);
/// assert_eq!(result.carry_out, 0);
/// ```
pub fn ripple_carry_adder(a: &[u8], b: &[u8]) -> RippleCarryResult {
    assert_eq!(
        a.len(),
        b.len(),
        "a and b must have the same length, got {} and {}",
        a.len(),
        b.len()
    );
    assert!(!a.is_empty(), "bit lists must not be empty");

    let mut sum = Vec::with_capacity(a.len());
    let mut carry: u8 = 0;

    for i in 0..a.len() {
        let (s, c) = full_adder(a[i], b[i], carry);
        sum.push(s);
        carry = c;
    }

    // Signed overflow detection: overflow occurs when the carry INTO the
    // sign bit differs from the carry OUT of the sign bit. Equivalently,
    // when adding two positive numbers gives negative, or vice versa.
    let n = a.len();
    let a_sign = a[n - 1];
    let b_sign = b[n - 1];
    let result_sign = sum[n - 1];
    let overflow = (a_sign == b_sign) && (result_sign != a_sign);

    RippleCarryResult {
        sum,
        carry_out: carry,
        overflow,
    }
}

/// Ripple-carry adder with explicit carry-in, used internally by the ALU
/// for subtraction (where carry_in = 1).
pub fn ripple_carry_adder_with_carry(a: &[u8], b: &[u8], carry_in: u8) -> RippleCarryResult {
    assert_eq!(
        a.len(),
        b.len(),
        "a and b must have the same length, got {} and {}",
        a.len(),
        b.len()
    );
    assert!(!a.is_empty(), "bit lists must not be empty");

    let mut sum = Vec::with_capacity(a.len());
    let mut carry = carry_in;

    for i in 0..a.len() {
        let (s, c) = full_adder(a[i], b[i], carry);
        sum.push(s);
        carry = c;
    }

    let n = a.len();
    let a_sign = a[n - 1];
    let b_sign = b[n - 1];
    let result_sign = sum[n - 1];
    let overflow = (a_sign == b_sign) && (result_sign != a_sign);

    RippleCarryResult {
        sum,
        carry_out: carry,
        overflow,
    }
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_half_adder_truth_table() {
        assert_eq!(half_adder(0, 0), (0, 0));
        assert_eq!(half_adder(0, 1), (1, 0));
        assert_eq!(half_adder(1, 0), (1, 0));
        assert_eq!(half_adder(1, 1), (0, 1));
    }

    #[test]
    fn test_full_adder_truth_table() {
        // All 8 combinations of (a, b, carry_in)
        assert_eq!(full_adder(0, 0, 0), (0, 0)); // 0+0+0 = 0
        assert_eq!(full_adder(0, 0, 1), (1, 0)); // 0+0+1 = 1
        assert_eq!(full_adder(0, 1, 0), (1, 0)); // 0+1+0 = 1
        assert_eq!(full_adder(0, 1, 1), (0, 1)); // 0+1+1 = 10
        assert_eq!(full_adder(1, 0, 0), (1, 0)); // 1+0+0 = 1
        assert_eq!(full_adder(1, 0, 1), (0, 1)); // 1+0+1 = 10
        assert_eq!(full_adder(1, 1, 0), (0, 1)); // 1+1+0 = 10
        assert_eq!(full_adder(1, 1, 1), (1, 1)); // 1+1+1 = 11
    }

    #[test]
    fn test_ripple_carry_5_plus_3() {
        // 5 + 3 = 8 in 4-bit
        // In signed 4-bit two's complement, range is -8 to 7.
        // 5 + 3 = 8 overflows because the result (1000) has MSB=1
        // while both inputs had MSB=0 (positive + positive = negative).
        let result = ripple_carry_adder(&[1, 0, 1, 0], &[1, 1, 0, 0]);
        assert_eq!(result.sum, vec![0, 0, 0, 1]); // 8 in LSB-first
        assert_eq!(result.carry_out, 0);
        assert!(result.overflow); // Signed overflow in 4-bit
    }

    #[test]
    fn test_ripple_carry_overflow() {
        // 255 + 1 in 8-bit: carries out
        let a = vec![1, 1, 1, 1, 1, 1, 1, 1]; // 255
        let b = vec![1, 0, 0, 0, 0, 0, 0, 0]; // 1
        let result = ripple_carry_adder(&a, &b);
        assert_eq!(result.sum, vec![0, 0, 0, 0, 0, 0, 0, 0]); // 0
        assert_eq!(result.carry_out, 1);
    }
}
