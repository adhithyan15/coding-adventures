//! Integration tests for the `gf256` crate.
//!
//! Tests cover all public functions (add, subtract, multiply, divide, power,
//! inverse), plus standard test vectors and mathematical invariants.
//!
//! Mathematical invariants checked:
//! - add/subtract are both XOR
//! - multiply(a, 1) == a (identity)
//! - multiply(a, 0) == 0 (zero absorber)
//! - a * inverse(a) == 1 for all non-zero a
//! - divide(a, b) * b == a
//! - power(2, 8) == 0x1D (known value for generator)
//! - add(x, x) == 0 (characteristic 2)
//! - g^255 == 1 (group order)

use gf256::*;
use gf256::Field;

// =============================================================================
// Constants
// =============================================================================

#[test]
fn test_zero_constant() {
    assert_eq!(ZERO, 0u8);
}

#[test]
fn test_one_constant() {
    assert_eq!(ONE, 1u8);
}

#[test]
fn test_primitive_polynomial() {
    assert_eq!(PRIMITIVE_POLYNOMIAL, 0x11d);
}

// =============================================================================
// add / subtract
// =============================================================================

#[test]
fn test_add_known_vector() {
    // 0x53 XOR 0xCA = 0x99 — well-known test vector.
    assert_eq!(add(0x53, 0xca), 0x99);
}

#[test]
fn test_add_zero_identity() {
    // add(a, 0) == a for any a (0 is additive identity).
    for a in 0u8..=255 {
        assert_eq!(add(a, 0), a, "add({}, 0) failed", a);
    }
}

#[test]
fn test_add_commutative() {
    // add(a, b) == add(b, a) for representative pairs.
    let pairs = [(0x53u8, 0xcau8), (1, 2), (255, 128), (0, 0), (7, 7)];
    for (a, b) in pairs {
        assert_eq!(add(a, b), add(b, a), "commutativity failed for ({}, {})", a, b);
    }
}

#[test]
fn test_add_self_is_zero() {
    // In characteristic 2: a + a = 0 for all a.
    for a in 0u8..=255 {
        assert_eq!(add(a, a), 0, "add({0}, {0}) != 0", a);
    }
}

#[test]
fn test_subtract_equals_add() {
    // subtract(a, b) == add(a, b) because -1 = 1 in char-2 fields.
    for a in [0u8, 1, 2, 0x53, 0xca, 127, 255] {
        for b in [0u8, 1, 2, 0x53, 0xca, 127, 255] {
            assert_eq!(subtract(a, b), add(a, b));
        }
    }
}

#[test]
fn test_subtract_self_is_zero() {
    for a in 0u8..=255 {
        assert_eq!(subtract(a, a), 0);
    }
}

#[test]
fn test_subtract_known_vector() {
    // Same as add: 0x53 XOR 0xCA = 0x99.
    assert_eq!(subtract(0x53, 0xca), 0x99);
}

// =============================================================================
// multiply
// =============================================================================

#[test]
fn test_multiply_standard_test_vector() {
    // 0x53 × 0x8C = 1 — the pair of multiplicative inverses under the 0x11D polynomial.
    //
    // Note: the well-known pair 0x53 × 0xCA = 1 is specific to the AES primitive
    // polynomial 0x11B. For the Reed-Solomon polynomial 0x11D used here, the
    // inverse of 0x53 is 0x8C, not 0xCA.
    assert_eq!(multiply(0x53, 0x8c), 1);
}

#[test]
fn test_multiply_identity() {
    // multiply(a, 1) == a for all a.
    for a in 0u8..=255 {
        assert_eq!(multiply(a, 1), a, "multiply({}, 1) failed", a);
    }
}

#[test]
fn test_multiply_by_zero() {
    // multiply(a, 0) == 0 for all a.
    for a in 0u8..=255 {
        assert_eq!(multiply(a, 0), 0, "multiply({}, 0) failed", a);
        assert_eq!(multiply(0, a), 0, "multiply(0, {}) failed", a);
    }
}

#[test]
fn test_multiply_commutative() {
    let pairs = [(2u8, 3u8), (0x53, 0xca), (127, 255), (1, 200), (0, 99)];
    for (a, b) in pairs {
        assert_eq!(multiply(a, b), multiply(b, a), "commutativity failed for ({}, {})", a, b);
    }
}

#[test]
fn test_multiply_associative() {
    // (a * b) * c == a * (b * c)
    let triples = [(2u8, 3u8, 5u8), (0x53, 0xca, 7), (10, 20, 30)];
    for (a, b, c) in triples {
        let lhs = multiply(multiply(a, b), c);
        let rhs = multiply(a, multiply(b, c));
        assert_eq!(lhs, rhs, "associativity failed for ({}, {}, {})", a, b, c);
    }
}

#[test]
fn test_multiply_distributive_over_add() {
    // a * (b + c) == a*b + a*c  (distributive law).
    let triples = [(2u8, 3u8, 5u8), (0x53, 0xca, 7), (10, 20, 30)];
    for (a, b, c) in triples {
        let lhs = multiply(a, add(b, c));
        let rhs = add(multiply(a, b), multiply(a, c));
        assert_eq!(lhs, rhs,
            "distributive law failed for ({}, {}, {}): lhs={}, rhs={}", a, b, c, lhs, rhs);
    }
}

#[test]
fn test_multiply_generator_sequence() {
    // The generator g=2 generates all non-zero elements.
    // So the sequence 2^0, 2^1, ..., 2^254 must all be distinct and non-zero.
    let mut seen = [false; 256];
    let mut val: u8 = 1;
    for i in 0..255u32 {
        assert!(!seen[val as usize], "g^{} = {} already seen!", i, val);
        assert_ne!(val, 0, "g^{} = 0 — generator must not reach zero", i);
        seen[val as usize] = true;
        val = multiply(val, 2);
    }
    // After 255 steps, we should be back to 1 (group order 255).
    assert_eq!(val, 1, "g^255 != 1; got {}", val);
}

// =============================================================================
// divide
// =============================================================================

#[test]
fn test_divide_by_one() {
    // a / 1 == a for all a.
    for a in 0u8..=255 {
        assert_eq!(divide(a, 1), a, "divide({}, 1) failed", a);
    }
}

#[test]
fn test_divide_zero_by_anything() {
    // 0 / b == 0 for all non-zero b.
    for b in 1u8..=255 {
        assert_eq!(divide(0, b), 0, "divide(0, {}) failed", b);
    }
}

#[test]
fn test_divide_inverse_of_multiply() {
    // divide(multiply(a, b), b) == a for non-zero a, b.
    let pairs = [(1u8, 2u8), (3, 5), (0x53, 0xca), (100, 200), (127, 255)];
    for (a, b) in pairs {
        let product = multiply(a, b);
        let back = divide(product, b);
        assert_eq!(back, a, "divide(multiply({0},{1}), {1}) = {2} != {0}", a, b, back);
    }
}

#[test]
fn test_divide_self_is_one() {
    // a / a == 1 for all non-zero a.
    for a in 1u8..=255 {
        assert_eq!(divide(a, a), 1, "divide({0}, {0}) != 1", a);
    }
}

#[test]
#[should_panic(expected = "GF256: division by zero")]
fn test_divide_by_zero_panics() {
    divide(5, 0);
}

#[test]
#[should_panic(expected = "GF256: division by zero")]
fn test_divide_zero_by_zero_panics() {
    divide(0, 0);
}

// =============================================================================
// power
// =============================================================================

#[test]
fn test_power_generator_exp8() {
    // 2^8 mod 0x11D: 256 XOR 285 = 29 = 0x1D
    assert_eq!(power(2, 8), 0x1d);
}

#[test]
fn test_power_generator_order() {
    // g^255 = 1 (the multiplicative group has order 255).
    assert_eq!(power(2, 255), 1);
}

#[test]
fn test_power_zero_exp() {
    // a^0 = 1 for any non-zero a.
    for a in 1u8..=255 {
        assert_eq!(power(a, 0), 1, "power({}, 0) failed", a);
    }
}

#[test]
fn test_power_zero_base_positive_exp() {
    // 0^n = 0 for n > 0.
    for exp in 1u32..=10 {
        assert_eq!(power(0, exp), 0, "power(0, {}) failed", exp);
    }
}

#[test]
fn test_power_zero_zero() {
    // 0^0 = 1 by convention.
    assert_eq!(power(0, 0), 1);
}

#[test]
fn test_power_one() {
    // 1^n = 1 for any n.
    for exp in 0u32..=255 {
        assert_eq!(power(1, exp), 1, "power(1, {}) failed", exp);
    }
}

#[test]
fn test_power_exp1() {
    // a^1 = a for any a.
    for a in 0u8..=255 {
        assert_eq!(power(a, 1), a, "power({}, 1) failed", a);
    }
}

#[test]
fn test_power_matches_repeated_multiply() {
    // power(a, n) must equal multiplying a by itself n times.
    for a in [2u8, 3, 5, 7, 0x53, 127] {
        let mut expected: u8 = 1;
        for n in 0u32..=10 {
            assert_eq!(power(a, n), expected,
                "power({}, {}) = {} but repeated multiply gives {}", a, n, power(a, n), expected);
            expected = multiply(expected, a);
        }
    }
}

#[test]
fn test_power_exp_addition_law() {
    // power(a, m) * power(a, n) == power(a, m + n)  (when m+n < 255)
    let triples = [(2u8, 3u32, 5u32), (3, 10, 20), (7, 50, 100)];
    for (a, m, n) in triples {
        let lhs = multiply(power(a, m), power(a, n));
        let rhs = power(a, m + n);
        assert_eq!(lhs, rhs, "power law failed for ({}, {}, {})", a, m, n);
    }
}

// =============================================================================
// inverse
// =============================================================================

#[test]
fn test_inverse_known_value() {
    // inverse(2) = 142 (0x8E) for the 0x11D polynomial.
    // Verify: multiply(2, 142) == 1.
    //
    // Note: the commonly cited value inverse(2) = 141 is for the AES polynomial
    // 0x11B. For the Reed-Solomon polynomial 0x11D used here, the inverse is 142.
    let inv2 = inverse(2);
    assert_eq!(inv2, 142, "inverse(2) should be 142 under 0x11D, got {}", inv2);
    assert_eq!(multiply(2, inv2), 1, "2 * inverse(2) != 1; got {}", inv2);
}

#[test]
fn test_inverse_specific() {
    // Under the 0x11D polynomial: inverse(0x53) = 0x8C and inverse(0x8C) = 0x53.
    //
    // Note: under the AES polynomial 0x11B the pair is (0x53, 0xCA). This crate
    // uses 0x11D (Reed-Solomon), so the inverse pair is (0x53, 0x8C).
    assert_eq!(inverse(0x53), 0x8c);
    assert_eq!(inverse(0x8c), 0x53);
}

#[test]
fn test_inverse_times_self_is_one() {
    // a * inverse(a) == 1 for all non-zero a.
    for a in 1u8..=255 {
        let inv = inverse(a);
        assert_eq!(multiply(a, inv), 1,
            "{} * inverse({}) = {} * {} = {} != 1", a, a, a, inv, multiply(a, inv));
    }
}

#[test]
fn test_inverse_of_one() {
    // inverse(1) = 1 (1 is its own inverse: 1 * 1 = 1).
    assert_eq!(inverse(1), 1);
}

#[test]
fn test_inverse_of_inverse() {
    // inverse(inverse(a)) == a for all non-zero a.
    for a in 1u8..=255 {
        assert_eq!(inverse(inverse(a)), a, "inverse(inverse({})) != {}", a, a);
    }
}

#[test]
#[should_panic(expected = "GF256: zero has no multiplicative inverse")]
fn test_inverse_of_zero_panics() {
    inverse(0);
}

// =============================================================================
// Cross-operation invariants
// =============================================================================

#[test]
fn test_add_associative() {
    // (a + b) + c == a + (b + c)
    let triples = [(1u8, 2u8, 3u8), (0x53, 0xca, 7), (127, 128, 255)];
    for (a, b, c) in triples {
        assert_eq!(add(add(a, b), c), add(a, add(b, c)));
    }
}

#[test]
fn test_field_axiom_no_zero_divisors() {
    // In a field, a * b = 0 implies a = 0 or b = 0.
    for a in 1u8..=255 {
        for b in 1u8..=255 {
            assert_ne!(multiply(a, b), 0,
                "{} * {} = 0, which violates the field axiom", a, b);
        }
    }
}

#[test]
fn test_all_nonzero_elements_invertible() {
    // Every non-zero element has an inverse (field axiom).
    for a in 1u8..=255 {
        let inv = inverse(a);
        assert_ne!(inv, 0, "inverse({}) = 0", a);
        assert_eq!(multiply(a, inv), 1, "{} * {} = {} != 1", a, inv, multiply(a, inv));
    }
}

// =============================================================================
// Field — parameterizable factory
// =============================================================================

#[test]
fn test_field_aes_multiply_inverses() {
    // In the AES field (0x11B): 0x53 × 0x8C = 0x01.
    let f = Field::new(0x11B);
    assert_eq!(f.multiply(0x53, 0x8C), 0x01, "AES field 0x53 × 0x8C should be 1");
}

#[test]
fn test_field_aes_fips197_appendix_b() {
    // FIPS 197 Appendix B: 0x57 × 0x83 = 0xC1 in GF(2^8, 0x11B).
    let f = Field::new(0x11B);
    assert_eq!(f.multiply(0x57, 0x83), 0xC1);
}

#[test]
fn test_field_aes_inverse() {
    let f = Field::new(0x11B);
    assert_eq!(f.inverse(0x53), 0x8C);
    assert_eq!(f.multiply(0x53, f.inverse(0x53)), 1);
}

#[test]
fn test_field_rs_matches_module_level() {
    // Field(0x11D) must match the module-level functions.
    let f = Field::new(0x11D);
    for a in 0u8..32 {
        for b in 0u8..32 {
            assert_eq!(f.multiply(a, b), multiply(a, b),
                "Field(0x11D).multiply({},{}) mismatch", a, b);
        }
    }
}

#[test]
fn test_field_commutativity() {
    let f = Field::new(0x11B);
    for a in [0u8, 1, 0x53, 0x8C, 0xFF] {
        for b in [0u8, 1, 0x57, 0x83, 0xFF] {
            assert_eq!(f.multiply(a, b), f.multiply(b, a));
        }
    }
}

#[test]
fn test_field_inverse_times_self() {
    let f = Field::new(0x11B);
    for a in 1u8..=20 {
        assert_eq!(f.multiply(a, f.inverse(a)), 1,
            "{} * inverse({}) != 1 in AES field", a, a);
    }
}

#[test]
#[should_panic(expected = "GF256::Field: division by zero")]
fn test_field_divide_by_zero_panics() {
    let f = Field::new(0x11B);
    f.divide(5, 0);
}

#[test]
#[should_panic(expected = "GF256::Field: zero has no multiplicative inverse")]
fn test_field_inverse_zero_panics() {
    let f = Field::new(0x11B);
    f.inverse(0);
}

#[test]
fn test_field_primitive_poly_stored() {
    let f = Field::new(0x11B);
    assert_eq!(f.primitive_polynomial, 0x11B);
}
