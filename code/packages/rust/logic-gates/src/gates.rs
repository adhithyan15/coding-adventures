//! Logic Gates — the seven fundamental gates, NAND-derived gates, and multi-input variants.
//!
//! # Transistor-backed implementations
//!
//! Each of the seven primitive gate functions now delegates its digital evaluation to a
//! CMOS transistor model from the `transistors` crate. The public API is unchanged —
//! inputs and outputs remain `u8` values of 0 or 1 — but the computation now routes
//! through a physical transistor simulation:
//!
//! | Gate | CMOS model              | Transistors |
//! |------|-------------------------|-------------|
//! | NOT  | `CMOSInverter`          | 2           |
//! | NAND | `CMOSNand`              | 4           |
//! | NOR  | `CMOSNor`               | 4           |
//! | AND  | `CMOSAnd` (NAND+NOT)    | 6           |
//! | OR   | `CMOSOr`  (NOR+NOT)     | 6           |
//! | XOR  | `CMOSXor` (4 NANDs)     | 16          |
//! | XNOR | `CMOSXnor` (XOR+NOT)     | 8           |
//!
//! # What is a logic gate?
//!
//! A logic gate is the simplest possible decision-making element. It takes
//! one or two inputs, each either 0 or 1, and produces a single output
//! that is also 0 or 1. The output is entirely determined by the inputs —
//! there is no randomness, no hidden state, no memory.
//!
//! # Why only 0 and 1?
//!
//! Computers use binary (base-2) because transistors are most reliable as
//! on/off switches. A transistor that is "on" (conducting electricity)
//! represents 1. A transistor that is "off" (blocking electricity) represents 0.
//! You could theoretically build a computer using base-3 or base-10, but the
//! error margins for distinguishing between voltage levels would make it
//! unreliable. Binary gives us two clean, easily distinguishable states.

use transistors::cmos_gates::{CMOSAnd, CMOSInverter, CMOSNand, CMOSNor, CMOSOr, CMOSXnor, CMOSXor};

// ===========================================================================
// Input validation
// ===========================================================================

/// Panics if the value is not a valid binary bit (0 or 1).
///
/// In Rust, we use `debug_assert!` so that validation runs in debug builds
/// (during development and testing) but is compiled away in release builds
/// for maximum performance. This is the Rust idiom for "trust but verify."
#[inline]
fn validate_bit(value: u8, name: &str) {
    debug_assert!(
        value == 0 || value == 1,
        "{name} must be 0 or 1, got {value}"
    );
}

// ===========================================================================
// THE FOUR FUNDAMENTAL GATES
// ===========================================================================
// These are the building blocks. NOT, AND, OR, and XOR are the four gates
// from which all other gates (and all of digital logic) can be constructed.
//
// Each gate is defined by its "truth table" — an exhaustive listing of
// every possible input combination and the corresponding output. Since each
// input can only be 0 or 1, a two-input gate has exactly 4 possible input
// combinations (2 x 2 = 4), making it easy to verify correctness.

/// The NOT gate (also called an "inverter").
///
/// NOT is the simplest gate — it has one input and flips it.
/// If the input is 0, the output is 1. If the input is 1, the output is 0.
///
/// Think of it like a light switch: if the light is off (0), flipping the
/// switch turns it on (1), and vice versa.
///
/// # Truth table
///
/// ```text
/// Input | Output
/// ------+-------
///   0   |   1
///   1   |   0
/// ```
///
/// # Circuit symbol
///
/// ```text
/// a -->o-- output
/// (the small circle o means "invert")
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::not_gate;
/// assert_eq!(not_gate(0), 1);
/// assert_eq!(not_gate(1), 0);
/// ```
#[inline]
pub fn not_gate(a: u8) -> u8 {
    validate_bit(a, "a");
    // Delegate to the CMOS inverter (2 transistors: 1 PMOS + 1 NMOS).
    // evaluate_digital returns Ok(0) or Ok(1); unwrap is safe because we've
    // already validated that a is 0 or 1.
    CMOSInverter::new(None, None, None)
        .evaluate_digital(a)
        .unwrap()
}

/// The AND gate — returns 1 only if both inputs are 1.
///
/// Think of two switches wired in series (one after the other): electric
/// current can only flow through if both switches are closed (both = 1).
///
/// # Truth table
///
/// ```text
/// A  B  | Output
/// ------+-------
/// 0  0  |   0      Neither is 1 -> 0
/// 0  1  |   0      Only B is 1  -> 0
/// 1  0  |   0      Only A is 1  -> 0
/// 1  1  |   1      Both are 1   -> 1
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::and_gate;
/// assert_eq!(and_gate(1, 1), 1);
/// assert_eq!(and_gate(1, 0), 0);
/// ```
#[inline]
pub fn and_gate(a: u8, b: u8) -> u8 {
    validate_bit(a, "a");
    validate_bit(b, "b");
    // Delegate to the CMOS AND gate (NAND + inverter = 6 transistors).
    CMOSAnd::new(None)
        .evaluate_digital(a, b)
        .unwrap()
}

/// The OR gate — returns 1 if either input is 1 (or both).
///
/// Think of two switches wired in parallel (side by side): current flows
/// if either switch is closed.
///
/// # Truth table
///
/// ```text
/// A  B  | Output
/// ------+-------
/// 0  0  |   0      Neither is 1 -> 0
/// 0  1  |   1      B is 1       -> 1
/// 1  0  |   1      A is 1       -> 1
/// 1  1  |   1      Both are 1   -> 1
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::or_gate;
/// assert_eq!(or_gate(0, 0), 0);
/// assert_eq!(or_gate(0, 1), 1);
/// ```
#[inline]
pub fn or_gate(a: u8, b: u8) -> u8 {
    validate_bit(a, "a");
    validate_bit(b, "b");
    // Delegate to the CMOS OR gate (NOR + inverter = 6 transistors).
    CMOSOr::new(None)
        .evaluate_digital(a, b)
        .unwrap()
}

/// The XOR gate (Exclusive OR) — returns 1 if the inputs are different.
///
/// Unlike OR, XOR outputs 0 when both inputs are 1. The name "exclusive"
/// means: one or the other, but NOT both.
///
/// # Why XOR matters for arithmetic
///
/// In binary addition, 1 + 1 = 10 (that's "one-zero" in binary, which
/// equals 2 in decimal). The sum digit is 0 and the carry is 1.
/// Notice that the sum digit (0) is exactly what XOR(1, 1) produces!
///
/// ```text
/// 0 + 0 = 0  ->  XOR(0, 0) = 0
/// 0 + 1 = 1  ->  XOR(0, 1) = 1
/// 1 + 0 = 1  ->  XOR(1, 0) = 1
/// 1 + 1 = 0  ->  XOR(1, 1) = 0  (carry the 1 separately)
/// ```
///
/// This is why XOR is the key gate in building adder circuits.
///
/// # Truth table
///
/// ```text
/// A  B  | Output
/// ------+-------
/// 0  0  |   0      Same      -> 0
/// 0  1  |   1      Different -> 1
/// 1  0  |   1      Different -> 1
/// 1  1  |   0      Same      -> 0
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::xor_gate;
/// assert_eq!(xor_gate(1, 0), 1);
/// assert_eq!(xor_gate(1, 1), 0);
/// ```
#[inline]
pub fn xor_gate(a: u8, b: u8) -> u8 {
    validate_bit(a, "a");
    validate_bit(b, "b");
    // Delegate to the CMOS XOR gate (4 NAND gates = 16 transistors).
    CMOSXor::new(None)
        .evaluate_digital(a, b)
        .unwrap()
}

// ===========================================================================
// COMPOSITE GATES
// ===========================================================================
// These gates are built by combining fundamental gates. They are included
// because they appear frequently in digital circuits and have useful properties.

/// The NAND gate (NOT AND) — the universal gate.
///
/// NAND is the inverse of AND: it outputs 1 in every case EXCEPT when both
/// inputs are 1.
///
/// # Why NAND is special — Functional Completeness
///
/// NAND has a remarkable property: you can build EVERY other gate using
/// only NAND gates. This means if you had a factory that could only
/// produce one type of gate, you'd pick NAND — because from NAND alone,
/// you can construct NOT, AND, OR, XOR, and any other logic function.
///
/// This property is called "functional completeness" and it's why real
/// chip manufacturers often build entire processors from NAND gates —
/// they're the cheapest and simplest to manufacture.
///
/// # Truth table
///
/// ```text
/// A  B  | Output
/// ------+-------
/// 0  0  |   1
/// 0  1  |   1
/// 1  0  |   1
/// 1  1  |   0      <- the only 0 output
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::nand_gate;
/// assert_eq!(nand_gate(1, 1), 0);
/// assert_eq!(nand_gate(1, 0), 1);
/// ```
#[inline]
pub fn nand_gate(a: u8, b: u8) -> u8 {
    // Delegate to the CMOS NAND gate (4 transistors — the natural CMOS primitive).
    CMOSNand::new(None, None, None)
        .evaluate_digital(a, b)
        .unwrap()
}

/// The NOR gate (NOT OR) — outputs 1 only when both inputs are 0.
///
/// Like NAND, NOR is also functionally complete — you can build every
/// other gate from NOR alone.
///
/// # Truth table
///
/// ```text
/// A  B  | Output
/// ------+-------
/// 0  0  |   1      <- the only 1 output
/// 0  1  |   0
/// 1  0  |   0
/// 1  1  |   0
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::nor_gate;
/// assert_eq!(nor_gate(0, 0), 1);
/// assert_eq!(nor_gate(0, 1), 0);
/// ```
#[inline]
pub fn nor_gate(a: u8, b: u8) -> u8 {
    // Delegate to the CMOS NOR gate (4 transistors — the other natural CMOS primitive).
    CMOSNor::new(None, None, None)
        .evaluate_digital(a, b)
        .unwrap()
}

/// The XNOR gate (Exclusive NOR, also called "equivalence gate").
///
/// XNOR is the inverse of XOR: it outputs 1 when the inputs are the SAME.
/// This makes it useful as an equality comparator — XNOR(a, b) = 1 means
/// a and b have the same value.
///
/// # Truth table
///
/// ```text
/// A  B  | Output
/// ------+-------
/// 0  0  |   1      Same      -> 1
/// 0  1  |   0      Different -> 0
/// 1  0  |   0      Different -> 0
/// 1  1  |   1      Same      -> 1
/// ```
///
/// # Example
///
/// ```
/// use logic_gates::gates::xnor_gate;
/// assert_eq!(xnor_gate(1, 1), 1);
/// assert_eq!(xnor_gate(1, 0), 0);
/// ```
#[inline]
pub fn xnor_gate(a: u8, b: u8) -> u8 {
    // Delegate to the dedicated CMOSXnor gate (XOR + Inverter = 8 transistors).
    CMOSXnor::new(None).evaluate_digital(a, b).unwrap()
}

// ===========================================================================
// NAND-DERIVED GATES — Proving Functional Completeness
// ===========================================================================
// The functions below prove that NAND is functionally complete by building
// NOT, AND, OR, and XOR using ONLY the NAND gate. No other gate is used.
//
// This is not just an academic exercise. In real chip manufacturing, the
// ability to build everything from one gate type dramatically simplifies
// the fabrication process. The first commercially successful logic family
// (TTL 7400 series, introduced in 1966) was built around NAND gates.

/// NOT built entirely from NAND gates.
///
/// # Construction
///
/// `NOT(a) = NAND(a, a)`
///
/// # Why this works
///
/// NAND outputs 0 only when both inputs are 1.
/// If we feed the same value to both inputs:
/// - `NAND(0, 0) = 1` (neither is 1, so NOT 0 = 1)
/// - `NAND(1, 1) = 0` (both are 1, so NOT 1 = 0)
///
/// # Circuit
///
/// ```text
/// a --+--+
///     |  |D--o-- output
///     +--+
/// (both inputs of the NAND come from the same wire)
/// ```
#[inline]
pub fn nand_not(a: u8) -> u8 {
    nand_gate(a, a)
}

/// AND built entirely from NAND gates.
///
/// # Construction
///
/// `AND(a, b) = NOT(NAND(a, b)) = NAND(NAND(a, b), NAND(a, b))`
///
/// NAND is the opposite of AND. So if we invert NAND's output (using
/// our nand_not trick above), we get AND back.
///
/// # Circuit (2 NAND gates)
///
/// ```text
/// a --+
///     |D--o--+--+
/// b --+      |  |D--o-- output
///            +--+
/// ```
#[inline]
pub fn nand_and(a: u8, b: u8) -> u8 {
    nand_not(nand_gate(a, b))
}

/// OR built entirely from NAND gates.
///
/// # Construction
///
/// `OR(a, b) = NAND(NOT(a), NOT(b)) = NAND(NAND(a,a), NAND(b,b))`
///
/// This uses De Morgan's Law: `A OR B = NOT(NOT(A) AND NOT(B)) = NAND(NOT(A), NOT(B))`
///
/// De Morgan's Law is a fundamental identity in Boolean algebra, discovered by
/// Augustus De Morgan in the 1800s — long before electronic computers existed!
///
/// # Circuit (3 NAND gates)
///
/// ```text
/// a --+--+
///     |  |D--o--+
///     +--+      |
///               |D--o-- output
/// b --+--+      |
///     |  |D--o--+
///     +--+
/// ```
#[inline]
pub fn nand_or(a: u8, b: u8) -> u8 {
    nand_gate(nand_not(a), nand_not(b))
}

/// XOR built entirely from NAND gates.
///
/// # Construction
///
/// ```text
/// Let N = NAND(a, b)
/// XOR(a, b) = NAND(NAND(a, N), NAND(b, N))
/// ```
///
/// This is the most complex NAND construction — it uses 4 NAND gates.
/// The intermediate value N = NAND(a, b) is reused twice, which is why
/// XOR is more "expensive" in hardware than AND or OR.
///
/// # Proof by truth table
///
/// ```text
/// a=0, b=0: N=NAND(0,0)=1, NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0
/// a=0, b=1: N=NAND(0,1)=1, NAND(0,1)=1, NAND(1,1)=0, NAND(1,0)=1
/// a=1, b=0: N=NAND(1,0)=1, NAND(1,1)=0, NAND(0,1)=1, NAND(0,1)=1
/// a=1, b=1: N=NAND(1,1)=0, NAND(1,0)=1, NAND(1,0)=1, NAND(1,1)=0
/// ```
#[inline]
pub fn nand_xor(a: u8, b: u8) -> u8 {
    let nab = nand_gate(a, b);
    nand_gate(nand_gate(a, nab), nand_gate(b, nab))
}

// ===========================================================================
// MULTI-INPUT GATES
// ===========================================================================
// In practice, you often need to AND or OR more than two values together.
// For example, "are ALL four conditions true?" requires a 4-input AND.
//
// Multi-input gates work by chaining 2-input gates. For AND:
//   AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)
//
// Rust's Iterator::fold does exactly this: it takes a collection and
// repeatedly applies a 2-argument function from left to right, just
// like Python's functools.reduce.

/// AND with N inputs — returns 1 only if ALL inputs are 1.
///
/// This chains 2-input AND gates together using fold:
///
/// ```text
/// AND_N(a, b, c, d) = AND(AND(AND(a, b), c), d)
/// ```
///
/// In hardware, this would be a chain of AND gates:
///
/// ```text
/// a --+
///     |D-- r1 --+
/// b --+         |D-- r2 --+
///          c ---+         |D-- output
///                   d ---+
/// ```
///
/// # Panics
///
/// Panics if fewer than 2 inputs are provided.
///
/// # Example
///
/// ```
/// use logic_gates::gates::and_n;
/// assert_eq!(and_n(&[1, 1, 1, 1]), 1);
/// assert_eq!(and_n(&[1, 1, 0, 1]), 0);
/// ```
pub fn and_n(inputs: &[u8]) -> u8 {
    assert!(inputs.len() >= 2, "and_n requires at least 2 inputs");
    inputs.iter().copied().reduce(and_gate).unwrap()
}

/// OR with N inputs — returns 1 if ANY input is 1.
///
/// This chains 2-input OR gates together using fold:
///
/// ```text
/// OR_N(a, b, c, d) = OR(OR(OR(a, b), c), d)
/// ```
///
/// # Panics
///
/// Panics if fewer than 2 inputs are provided.
///
/// # Example
///
/// ```
/// use logic_gates::gates::or_n;
/// assert_eq!(or_n(&[0, 0, 0, 0]), 0);
/// assert_eq!(or_n(&[0, 0, 1, 0]), 1);
/// ```
pub fn or_n(inputs: &[u8]) -> u8 {
    assert!(inputs.len() >= 2, "or_n requires at least 2 inputs");
    inputs.iter().copied().reduce(or_gate).unwrap()
}

/// N-input XOR gate — reduces a slice of bits via XOR (parity checker).
///
/// Returns 1 if an **odd** number of inputs are 1 (odd parity).
/// Returns 0 if an **even** number of inputs are 1 (even parity).
///
/// This chains 2-input XOR gates left-to-right using fold:
///
/// ```text
/// XOR_N(a, b, c, d) = XOR(XOR(XOR(a, b), c), d)
/// ```
///
/// # Why XOR gives parity
///
/// XOR is 1 when the count of 1-inputs is odd. Chaining XOR gates over
/// all bits counts parity: each new 1-bit flips the running total between
/// odd and even. This is exactly how real hardware implements parity checks —
/// a chain of XOR gates is the most area-efficient parity detector.
///
/// # Use in the Intel 8008
///
/// The 8008 Parity flag P is set when the result has **even** parity
/// (an even number of 1-bits). So the hardware computes:
///
/// ```text
/// P = NOT(XOR_N(result_bits))
/// ```
///
/// P=1 means "even number of 1s" — the inverse of the raw XOR parity.
///
/// # Example
///
/// ```
/// use logic_gates::gates::xor_n;
/// // Three inputs = odd count → odd parity → XOR result = 1
/// assert_eq!(xor_n(&[1, 0, 0]), 1);
/// assert_eq!(xor_n(&[1, 1, 0]), 0); // two 1s = even parity → 0
/// assert_eq!(xor_n(&[1, 1, 1]), 1); // three 1s = odd parity → 1
/// assert_eq!(xor_n(&[0, 0, 0, 0]), 0); // all zeros = even parity → 0
/// ```
pub fn xor_n(inputs: &[u8]) -> u8 {
    assert!(inputs.len() >= 2, "xor_n requires at least 2 inputs");
    inputs.iter().copied().reduce(xor_gate).unwrap()
}

// ===========================================================================
// Inline unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // --- NOT gate ---
    #[test]
    fn test_not_gate() {
        assert_eq!(not_gate(0), 1);
        assert_eq!(not_gate(1), 0);
    }

    // --- AND gate ---
    #[test]
    fn test_and_gate_truth_table() {
        assert_eq!(and_gate(0, 0), 0);
        assert_eq!(and_gate(0, 1), 0);
        assert_eq!(and_gate(1, 0), 0);
        assert_eq!(and_gate(1, 1), 1);
    }

    // --- OR gate ---
    #[test]
    fn test_or_gate_truth_table() {
        assert_eq!(or_gate(0, 0), 0);
        assert_eq!(or_gate(0, 1), 1);
        assert_eq!(or_gate(1, 0), 1);
        assert_eq!(or_gate(1, 1), 1);
    }

    // --- XOR gate ---
    #[test]
    fn test_xor_gate_truth_table() {
        assert_eq!(xor_gate(0, 0), 0);
        assert_eq!(xor_gate(0, 1), 1);
        assert_eq!(xor_gate(1, 0), 1);
        assert_eq!(xor_gate(1, 1), 0);
    }

    // --- NAND gate ---
    #[test]
    fn test_nand_gate_truth_table() {
        assert_eq!(nand_gate(0, 0), 1);
        assert_eq!(nand_gate(0, 1), 1);
        assert_eq!(nand_gate(1, 0), 1);
        assert_eq!(nand_gate(1, 1), 0);
    }

    // --- NOR gate ---
    #[test]
    fn test_nor_gate_truth_table() {
        assert_eq!(nor_gate(0, 0), 1);
        assert_eq!(nor_gate(0, 1), 0);
        assert_eq!(nor_gate(1, 0), 0);
        assert_eq!(nor_gate(1, 1), 0);
    }

    // --- XNOR gate ---
    #[test]
    fn test_xnor_gate_truth_table() {
        assert_eq!(xnor_gate(0, 0), 1);
        assert_eq!(xnor_gate(0, 1), 0);
        assert_eq!(xnor_gate(1, 0), 0);
        assert_eq!(xnor_gate(1, 1), 1);
    }

    // --- NAND-derived gates match their originals ---
    #[test]
    fn test_nand_not_matches_not() {
        for a in 0..=1u8 {
            assert_eq!(nand_not(a), not_gate(a), "nand_not({a}) != not_gate({a})");
        }
    }

    #[test]
    fn test_nand_and_matches_and() {
        for a in 0..=1u8 {
            for b in 0..=1u8 {
                assert_eq!(nand_and(a, b), and_gate(a, b));
            }
        }
    }

    #[test]
    fn test_nand_or_matches_or() {
        for a in 0..=1u8 {
            for b in 0..=1u8 {
                assert_eq!(nand_or(a, b), or_gate(a, b));
            }
        }
    }

    #[test]
    fn test_nand_xor_matches_xor() {
        for a in 0..=1u8 {
            for b in 0..=1u8 {
                assert_eq!(nand_xor(a, b), xor_gate(a, b));
            }
        }
    }

    // --- Multi-input gates ---
    #[test]
    fn test_and_n() {
        assert_eq!(and_n(&[1, 1]), 1);
        assert_eq!(and_n(&[1, 0]), 0);
        assert_eq!(and_n(&[1, 1, 1, 1]), 1);
        assert_eq!(and_n(&[1, 1, 0, 1]), 0);
    }

    #[test]
    fn test_or_n() {
        assert_eq!(or_n(&[0, 0]), 0);
        assert_eq!(or_n(&[0, 1]), 1);
        assert_eq!(or_n(&[0, 0, 0, 0]), 0);
        assert_eq!(or_n(&[0, 0, 1, 0]), 1);
    }

    #[test]
    #[should_panic(expected = "and_n requires at least 2 inputs")]
    fn test_and_n_panics_on_too_few_inputs() {
        and_n(&[1]);
    }

    #[test]
    #[should_panic(expected = "or_n requires at least 2 inputs")]
    fn test_or_n_panics_on_too_few_inputs() {
        or_n(&[0]);
    }

    // --- XOR_N gate ---
    #[test]
    fn test_xor_n_parity() {
        // Zero 1-bits → even parity → 0
        assert_eq!(xor_n(&[0, 0]), 0);
        assert_eq!(xor_n(&[0, 0, 0, 0]), 0);
        // One 1-bit → odd parity → 1
        assert_eq!(xor_n(&[1, 0]), 1);
        assert_eq!(xor_n(&[0, 1]), 1);
        assert_eq!(xor_n(&[1, 0, 0]), 1);
        // Two 1-bits → even parity → 0
        assert_eq!(xor_n(&[1, 1]), 0);
        assert_eq!(xor_n(&[1, 1, 0]), 0);
        // Three 1-bits → odd parity → 1
        assert_eq!(xor_n(&[1, 1, 1]), 1);
        // Eight bits of 0xFF (all ones) → even parity → 0
        assert_eq!(xor_n(&[1, 1, 1, 1, 1, 1, 1, 1]), 0);
        // Five 1-bits → odd parity → 1
        assert_eq!(xor_n(&[1, 0, 1, 1, 0, 1, 0, 1]), 1);
    }

    #[test]
    #[should_panic(expected = "xor_n requires at least 2 inputs")]
    fn test_xor_n_panics_on_too_few_inputs() {
        xor_n(&[1]);
    }
}
