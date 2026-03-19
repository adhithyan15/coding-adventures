//! Integration tests for the gates module.
//!
//! These tests verify every gate against its complete truth table, test
//! NAND-derived gates against their originals, and exercise multi-input
//! variants with various input sizes.

use logic_gates::gates::*;

// ===========================================================================
// Exhaustive truth table tests for all 7 fundamental gates
// ===========================================================================

/// Helper: test a 2-input gate against its full truth table.
/// The expected values are in order: (0,0), (0,1), (1,0), (1,1).
fn assert_truth_table_2(gate: fn(u8, u8) -> u8, expected: [u8; 4], name: &str) {
    let inputs = [(0, 0), (0, 1), (1, 0), (1, 1)];
    for (i, &(a, b)) in inputs.iter().enumerate() {
        assert_eq!(
            gate(a, b),
            expected[i],
            "{name}({a}, {b}) should be {}, got {}",
            expected[i],
            gate(a, b)
        );
    }
}

#[test]
fn test_and_gate_exhaustive() {
    assert_truth_table_2(and_gate, [0, 0, 0, 1], "AND");
}

#[test]
fn test_or_gate_exhaustive() {
    assert_truth_table_2(or_gate, [0, 1, 1, 1], "OR");
}

#[test]
fn test_xor_gate_exhaustive() {
    assert_truth_table_2(xor_gate, [0, 1, 1, 0], "XOR");
}

#[test]
fn test_nand_gate_exhaustive() {
    assert_truth_table_2(nand_gate, [1, 1, 1, 0], "NAND");
}

#[test]
fn test_nor_gate_exhaustive() {
    assert_truth_table_2(nor_gate, [1, 0, 0, 0], "NOR");
}

#[test]
fn test_xnor_gate_exhaustive() {
    assert_truth_table_2(xnor_gate, [1, 0, 0, 1], "XNOR");
}

#[test]
fn test_not_gate_exhaustive() {
    assert_eq!(not_gate(0), 1);
    assert_eq!(not_gate(1), 0);
}

// ===========================================================================
// NAND-derived gates must match their originals for ALL inputs
// ===========================================================================

#[test]
fn test_nand_not_matches_not_exhaustive() {
    for a in 0..=1u8 {
        assert_eq!(
            nand_not(a),
            not_gate(a),
            "nand_not({a}) should equal not_gate({a})"
        );
    }
}

#[test]
fn test_nand_and_matches_and_exhaustive() {
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            assert_eq!(nand_and(a, b), and_gate(a, b));
        }
    }
}

#[test]
fn test_nand_or_matches_or_exhaustive() {
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            assert_eq!(nand_or(a, b), or_gate(a, b));
        }
    }
}

#[test]
fn test_nand_xor_matches_xor_exhaustive() {
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            assert_eq!(nand_xor(a, b), xor_gate(a, b));
        }
    }
}

// ===========================================================================
// Multi-input gate tests
// ===========================================================================

#[test]
fn test_and_n_all_ones() {
    assert_eq!(and_n(&[1, 1, 1, 1, 1]), 1);
}

#[test]
fn test_and_n_one_zero() {
    assert_eq!(and_n(&[1, 1, 0, 1, 1]), 0);
}

#[test]
fn test_and_n_two_inputs() {
    assert_eq!(and_n(&[1, 1]), 1);
    assert_eq!(and_n(&[1, 0]), 0);
    assert_eq!(and_n(&[0, 0]), 0);
}

#[test]
fn test_or_n_all_zeros() {
    assert_eq!(or_n(&[0, 0, 0, 0, 0]), 0);
}

#[test]
fn test_or_n_one_one() {
    assert_eq!(or_n(&[0, 0, 1, 0, 0]), 1);
}

#[test]
fn test_or_n_two_inputs() {
    assert_eq!(or_n(&[0, 0]), 0);
    assert_eq!(or_n(&[0, 1]), 1);
    assert_eq!(or_n(&[1, 1]), 1);
}

#[test]
fn test_and_n_large_input() {
    let all_ones: Vec<u8> = vec![1; 16];
    assert_eq!(and_n(&all_ones), 1);
    let mut with_zero = all_ones.clone();
    with_zero[8] = 0;
    assert_eq!(and_n(&with_zero), 0);
}

#[test]
fn test_or_n_large_input() {
    let all_zeros: Vec<u8> = vec![0; 16];
    assert_eq!(or_n(&all_zeros), 0);
    let mut with_one = all_zeros.clone();
    with_one[15] = 1;
    assert_eq!(or_n(&with_one), 1);
}

// ===========================================================================
// Edge cases and panics
// ===========================================================================

#[test]
#[should_panic]
fn test_and_n_panics_single_input() {
    and_n(&[1]);
}

#[test]
#[should_panic]
fn test_and_n_panics_empty_input() {
    and_n(&[]);
}

#[test]
#[should_panic]
fn test_or_n_panics_single_input() {
    or_n(&[0]);
}

#[test]
#[should_panic]
fn test_or_n_panics_empty_input() {
    or_n(&[]);
}

// ===========================================================================
// Algebraic properties — sanity checks that boolean algebra holds
// ===========================================================================

#[test]
fn test_de_morgans_law() {
    // De Morgan's: NOT(A AND B) = (NOT A) OR (NOT B)
    // De Morgan's: NOT(A OR B) = (NOT A) AND (NOT B)
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            assert_eq!(
                not_gate(and_gate(a, b)),
                or_gate(not_gate(a), not_gate(b)),
                "De Morgan's (AND): failed for a={a}, b={b}"
            );
            assert_eq!(
                not_gate(or_gate(a, b)),
                and_gate(not_gate(a), not_gate(b)),
                "De Morgan's (OR): failed for a={a}, b={b}"
            );
        }
    }
}

#[test]
fn test_double_negation() {
    // NOT(NOT(a)) = a
    for a in 0..=1u8 {
        assert_eq!(not_gate(not_gate(a)), a);
    }
}

#[test]
fn test_xor_is_inequality() {
    // XOR(a, b) = 1 iff a != b
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            let expected = if a != b { 1 } else { 0 };
            assert_eq!(xor_gate(a, b), expected);
        }
    }
}

#[test]
fn test_xnor_is_equality() {
    // XNOR(a, b) = 1 iff a == b
    for a in 0..=1u8 {
        for b in 0..=1u8 {
            let expected = if a == b { 1 } else { 0 };
            assert_eq!(xnor_gate(a, b), expected);
        }
    }
}

#[test]
fn test_nand_is_functionally_complete() {
    // We can build all 4 fundamental gates from NAND alone.
    // This test verifies the complete truth tables match.
    for a in 0..=1u8 {
        assert_eq!(nand_not(a), not_gate(a), "NAND-NOT failed for {a}");
        for b in 0..=1u8 {
            assert_eq!(nand_and(a, b), and_gate(a, b), "NAND-AND failed for {a},{b}");
            assert_eq!(nand_or(a, b), or_gate(a, b), "NAND-OR failed for {a},{b}");
            assert_eq!(nand_xor(a, b), xor_gate(a, b), "NAND-XOR failed for {a},{b}");
        }
    }
}
