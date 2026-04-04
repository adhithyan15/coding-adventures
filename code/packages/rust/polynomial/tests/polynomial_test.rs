//! Integration tests for the `polynomial` crate.
//!
//! Every public function is covered. We aim for well over 80% coverage by
//! testing both normal cases and edge cases (zero polynomial, single-term,
//! cancellation, near-zero coefficients, etc.).

use polynomial::*;

// =============================================================================
// Helpers
// =============================================================================

/// Compare two f64 slices element-wise within a tolerance.
fn approx_eq(a: &[f64], b: &[f64]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b.iter()).all(|(x, y)| (x - y).abs() < 1e-9)
}

// =============================================================================
// normalize
// =============================================================================

#[test]
fn test_normalize_already_normal() {
    let p = vec![1.0, 2.0, 3.0];
    assert_eq!(normalize(&p), vec![1.0, 2.0, 3.0]);
}

#[test]
fn test_normalize_trailing_zeros() {
    let p = vec![1.0, 0.0, 0.0];
    assert_eq!(normalize(&p), vec![1.0]);
}

#[test]
fn test_normalize_all_zeros() {
    let p = vec![0.0, 0.0, 0.0];
    assert!(normalize(&p).is_empty());
}

#[test]
fn test_normalize_single_zero() {
    assert!(normalize(&[0.0]).is_empty());
}

#[test]
fn test_normalize_empty() {
    assert!(normalize(&[]).is_empty());
}

#[test]
fn test_normalize_single_nonzero() {
    assert_eq!(normalize(&[5.0]), vec![5.0]);
}

#[test]
fn test_normalize_near_zero_threshold() {
    // Values below EPSILON * 1e6 should be stripped.
    let tiny = f64::EPSILON * 1e5; // below threshold
    let p = vec![1.0, tiny];
    assert_eq!(normalize(&p), vec![1.0]);
}

#[test]
fn test_normalize_preserves_non_tiny_trailing() {
    // Values above the threshold must NOT be stripped.
    let big_enough = f64::EPSILON * 1e7; // above threshold
    let p = vec![1.0, big_enough];
    assert_eq!(normalize(&p).len(), 2);
}

// =============================================================================
// degree
// =============================================================================

#[test]
fn test_degree_constant() {
    // Degree of constant polynomial [7] is 0.
    assert_eq!(degree(&[7.0]), 0);
}

#[test]
fn test_degree_linear() {
    assert_eq!(degree(&[1.0, 2.0]), 1);
}

#[test]
fn test_degree_quadratic() {
    assert_eq!(degree(&[3.0, 0.0, 2.0]), 2);
}

#[test]
fn test_degree_zero_polynomial_empty() {
    // Empty slice is the zero polynomial.
    assert_eq!(degree(&[]), 0);
}

#[test]
fn test_degree_zero_polynomial_zeros() {
    // [0.0] normalizes to [] → degree 0.
    assert_eq!(degree(&[0.0]), 0);
}

#[test]
fn test_degree_with_trailing_zeros() {
    // [1, 2, 0, 0] has true degree 1 after normalization.
    assert_eq!(degree(&[1.0, 2.0, 0.0, 0.0]), 1);
}

// =============================================================================
// zero / one
// =============================================================================

#[test]
fn test_zero_is_additive_identity() {
    let p = vec![1.0, 2.0, 3.0];
    // Adding zero on the left.
    let z = zero();
    assert!(approx_eq(&add(&z, &p), &p));
    // Adding zero on the right.
    assert!(approx_eq(&add(&p, &z), &p));
}

#[test]
fn test_one_is_multiplicative_identity() {
    let p = vec![1.0, 2.0, 3.0];
    let o = one();
    // Multiplying by one on either side should give p.
    assert!(approx_eq(&multiply(&o, &p), &p));
    assert!(approx_eq(&multiply(&p, &o), &p));
}

// =============================================================================
// add
// =============================================================================

#[test]
fn test_add_same_degree() {
    // (1 + 2x) + (3 + 4x) = 4 + 6x
    let a = vec![1.0, 2.0];
    let b = vec![3.0, 4.0];
    assert_eq!(add(&a, &b), vec![4.0, 6.0]);
}

#[test]
fn test_add_different_degrees() {
    // (1 + 2x + 3x²) + (4 + 5x) = 5 + 7x + 3x²
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0];
    assert_eq!(add(&a, &b), vec![5.0, 7.0, 3.0]);
}

#[test]
fn test_add_cancellation() {
    // (1 + 2x + 3x²) + (-1 - 2x - 3x²) = 0
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![-1.0, -2.0, -3.0];
    // Result should normalize to empty (zero polynomial).
    assert!(add(&a, &b).is_empty());
}

#[test]
fn test_add_zero_polynomial() {
    let p = vec![5.0, 6.0];
    let z: Vec<f64> = vec![];
    assert_eq!(add(&p, &z), p);
    assert_eq!(add(&z, &p), p);
}

#[test]
fn test_add_commutativity() {
    let a = vec![1.0, 3.0, 5.0];
    let b = vec![2.0, 4.0];
    assert!(approx_eq(&add(&a, &b), &add(&b, &a)));
}

// =============================================================================
// subtract
// =============================================================================

#[test]
fn test_subtract_same_degree() {
    // (5 + 7x) - (3 + 4x) = 2 + 3x
    let a = vec![5.0, 7.0];
    let b = vec![3.0, 4.0];
    assert_eq!(subtract(&a, &b), vec![2.0, 3.0]);
}

#[test]
fn test_subtract_cancels_high_term() {
    // (4 + 5x + 3x²) - (1 + 2x + 3x²) = 3 + 3x
    let a = vec![4.0, 5.0, 3.0];
    let b = vec![1.0, 2.0, 3.0];
    assert_eq!(subtract(&a, &b), vec![3.0, 3.0]);
}

#[test]
fn test_subtract_self_is_zero() {
    let p = vec![1.0, 2.0, 3.0];
    assert!(subtract(&p, &p).is_empty());
}

#[test]
fn test_subtract_from_zero() {
    // 0 - p = -p
    let p = vec![1.0, 2.0];
    let z: Vec<f64> = vec![];
    let neg_p: Vec<f64> = vec![-1.0, -2.0];
    assert_eq!(subtract(&z, &p), neg_p);
}

#[test]
fn test_subtract_zero_gives_same() {
    let p = vec![3.0, 4.0, 5.0];
    let z: Vec<f64> = vec![];
    assert_eq!(subtract(&p, &z), p);
}

// =============================================================================
// multiply
// =============================================================================

#[test]
fn test_multiply_simple() {
    // (1 + 2x)(3 + 4x) = 3 + 10x + 8x²
    let a = vec![1.0, 2.0];
    let b = vec![3.0, 4.0];
    assert_eq!(multiply(&a, &b), vec![3.0, 10.0, 8.0]);
}

#[test]
fn test_multiply_constant() {
    // 3 × (1 + 2x) = 3 + 6x
    let a = vec![3.0];
    let b = vec![1.0, 2.0];
    assert_eq!(multiply(&a, &b), vec![3.0, 6.0]);
}

#[test]
fn test_multiply_commutativity() {
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0];
    assert!(approx_eq(&multiply(&a, &b), &multiply(&b, &a)));
}

#[test]
fn test_multiply_by_zero() {
    let p = vec![1.0, 2.0, 3.0];
    let z: Vec<f64> = vec![];
    assert!(multiply(&p, &z).is_empty());
    assert!(multiply(&z, &p).is_empty());
}

#[test]
fn test_multiply_monic_linear_factors() {
    // (x - 1)(x + 1) = x² - 1 → [-1, 0, 1]
    let a = vec![-1.0, 1.0]; // x - 1
    let b = vec![1.0, 1.0];  // x + 1
    assert!(approx_eq(&multiply(&a, &b), &[-1.0, 0.0, 1.0]));
}

#[test]
fn test_multiply_degree_sum() {
    // Product of degree-m and degree-n has degree m+n.
    let a = vec![1.0, 0.0, 1.0]; // degree 2
    let b = vec![1.0, 1.0];       // degree 1
    let result = multiply(&a, &b);
    // degree of result should be 3
    assert_eq!(result.len() - 1, 3);
}

// =============================================================================
// divmod
// =============================================================================

#[test]
fn test_divmod_exact_division() {
    // x² - 1 = (x - 1)(x + 1), so (x² - 1) / (x - 1) = (x + 1), rem = 0
    let dividend = vec![-1.0, 0.0, 1.0]; // x² - 1
    let divisor  = vec![-1.0, 1.0];       // x - 1
    let (q, r) = divmod(&dividend, &divisor);
    // quotient should be x + 1 = [1, 1]
    assert!(approx_eq(&q, &[1.0, 1.0]), "quotient was {:?}", q);
    // remainder should be zero
    assert!(r.is_empty(), "remainder was {:?}", r);
}

#[test]
fn test_divmod_with_remainder() {
    // x² / (x + 1) = x - 1, remainder 1
    let dividend = vec![0.0, 0.0, 1.0]; // x²
    let divisor  = vec![1.0, 1.0];       // x + 1
    let (q, r) = divmod(&dividend, &divisor);
    // q = x - 1 = [-1, 1]
    assert!(approx_eq(&q, &[-1.0, 1.0]), "quotient was {:?}", q);
    // r = 1 = [1]
    assert!(approx_eq(&r, &[1.0]), "remainder was {:?}", r);
}

#[test]
fn test_divmod_lower_degree_dividend() {
    // If degree(dividend) < degree(divisor), quotient = 0, remainder = dividend.
    let dividend = vec![3.0, 4.0];    // 3 + 4x, degree 1
    let divisor  = vec![1.0, 0.0, 1.0]; // x² + 1, degree 2
    let (q, r) = divmod(&dividend, &divisor);
    assert!(q.is_empty(), "quotient was {:?}", q);
    assert!(approx_eq(&r, &[3.0, 4.0]), "remainder was {:?}", r);
}

#[test]
fn test_divmod_by_constant() {
    // (2 + 4x) / 2 = 1 + 2x, rem 0
    let dividend = vec![2.0, 4.0];
    let divisor  = vec![2.0];
    let (q, r) = divmod(&dividend, &divisor);
    assert!(approx_eq(&q, &[1.0, 2.0]), "quotient was {:?}", q);
    assert!(r.is_empty(), "remainder was {:?}", r);
}

#[test]
fn test_divmod_verify_identity() {
    // Verify: dividend = divisor * quotient + remainder for a general case.
    let dividend = vec![5.0, 1.0, 3.0, 2.0]; // 5 + x + 3x² + 2x³
    let divisor  = vec![2.0, 1.0];              // 2 + x
    let (q, r) = divmod(&dividend, &divisor);
    // Reconstruct: divisor * q + r should equal dividend.
    let reconstructed = add(&multiply(&divisor, &q), &r);
    assert!(approx_eq(&reconstructed, &normalize(&dividend)),
        "reconstruct failed: {:?}", reconstructed);
}

#[test]
#[should_panic(expected = "polynomial division by zero")]
fn test_divmod_by_zero_panics() {
    divmod(&[1.0, 2.0], &[0.0]);
}

#[test]
#[should_panic(expected = "polynomial division by zero")]
fn test_divmod_by_empty_panics() {
    divmod(&[1.0, 2.0], &[]);
}

// =============================================================================
// divide / modulo
// =============================================================================

#[test]
fn test_divide_returns_quotient() {
    let a = vec![-1.0, 0.0, 1.0]; // x² - 1
    let b = vec![-1.0, 1.0];       // x - 1
    assert!(approx_eq(&divide(&a, &b), &[1.0, 1.0]));
}

#[test]
fn test_modulo_returns_remainder() {
    // x² mod (x + 1) = 1
    let a = vec![0.0, 0.0, 1.0];
    let b = vec![1.0, 1.0];
    assert!(approx_eq(&modulo(&a, &b), &[1.0]));
}

#[test]
fn test_modulo_exact_division_is_zero() {
    let a = vec![-1.0, 0.0, 1.0]; // x² - 1
    let b = vec![-1.0, 1.0];       // x - 1
    assert!(modulo(&a, &b).is_empty());
}

// =============================================================================
// evaluate
// =============================================================================

#[test]
fn test_evaluate_constant() {
    // p(x) = 7, so p(0) = p(1) = p(42) = 7
    let p = vec![7.0];
    assert!((evaluate(&p, 0.0) - 7.0).abs() < 1e-12);
    assert!((evaluate(&p, 42.0) - 7.0).abs() < 1e-12);
}

#[test]
fn test_evaluate_linear() {
    // p(x) = 3 + 2x, so p(1) = 5, p(4) = 11
    let p = vec![3.0, 2.0];
    assert!((evaluate(&p, 1.0) - 5.0).abs() < 1e-12);
    assert!((evaluate(&p, 4.0) - 11.0).abs() < 1e-12);
}

#[test]
fn test_evaluate_quadratic_at_two() {
    // p(x) = 3 + 0x + 1x² = 3 + x², so p(2) = 3 + 4 = 7
    let p = vec![3.0, 0.0, 1.0];
    assert!((evaluate(&p, 2.0) - 7.0).abs() < 1e-12);
}

#[test]
fn test_evaluate_zero_polynomial() {
    // The zero polynomial evaluates to 0 everywhere.
    assert!((evaluate(&[], 99.0) - 0.0).abs() < 1e-12);
    assert!((evaluate(&[0.0], 5.0) - 0.0).abs() < 1e-12);
}

#[test]
fn test_evaluate_at_zero() {
    // p(0) equals the constant term a₀.
    let p = vec![42.0, 1.0, 1.0];
    assert!((evaluate(&p, 0.0) - 42.0).abs() < 1e-12);
}

#[test]
fn test_evaluate_horner_correctness() {
    // p(x) = 1 + 2x + 3x² + 4x³ evaluated at x = 2.
    // = 1 + 4 + 12 + 32 = 49
    let p = vec![1.0, 2.0, 3.0, 4.0];
    let result = evaluate(&p, 2.0);
    assert!((result - 49.0).abs() < 1e-9, "got {}", result);
}

#[test]
fn test_evaluate_root_check() {
    // (x - 1)(x - 2) = x² - 3x + 2 → [2, -3, 1]
    // roots at x=1 and x=2 → p(1)=0, p(2)=0
    let p = vec![2.0, -3.0, 1.0];
    assert!(evaluate(&p, 1.0).abs() < 1e-12);
    assert!(evaluate(&p, 2.0).abs() < 1e-12);
}

// =============================================================================
// gcd
// =============================================================================

#[test]
fn test_gcd_common_linear_factor() {
    // gcd(x² - 3x + 2, x - 1) = x - 1
    // x² - 3x + 2 = (x-1)(x-2)
    let a = vec![2.0, -3.0, 1.0]; // x² - 3x + 2
    let b = vec![-1.0, 1.0];       // x - 1
    let g = gcd(&a, &b);
    // Should be monic x - 1 (or a scalar multiple).
    // Normalize by leading coefficient.
    let lead = g[g.len() - 1];
    let monic: Vec<f64> = g.iter().map(|c| c / lead).collect();
    assert!(approx_eq(&monic, &[-1.0, 1.0]), "got {:?}", monic);
}

#[test]
fn test_gcd_coprime() {
    // gcd(x + 1, x + 2): no common factor, GCD is a constant.
    let a = vec![1.0, 1.0]; // x + 1
    let b = vec![2.0, 1.0]; // x + 2
    let g = gcd(&a, &b);
    // GCD of coprime polynomials is a constant (degree 0 or empty is zero poly).
    // Either degree(g) == 0 or g is empty (which represents zero, though that's
    // unlikely for distinct monic linears).
    assert!(g.len() <= 1, "GCD of coprime should be constant, got {:?}", g);
}

#[test]
fn test_gcd_with_zero_is_self() {
    // gcd(p, 0) = p for any p.
    let p = vec![1.0, 2.0, 3.0];
    let z: Vec<f64> = vec![];
    let g = gcd(&p, &z);
    let lead = g[g.len() - 1];
    let p_lead = p[p.len() - 1];
    let scale = p_lead / lead;
    let scaled: Vec<f64> = g.iter().map(|c| c * scale).collect();
    assert!(approx_eq(&scaled, &p), "got {:?}", scaled);
}

#[test]
fn test_gcd_identical_polynomials() {
    // gcd(p, p) = p (up to scaling).
    let p = vec![1.0, -3.0, 2.0];
    let g = gcd(&p, &p);
    // The GCD should have the same degree as p.
    assert_eq!(g.len(), p.len(), "got {:?}", g);
}

#[test]
fn test_gcd_quadratic_common_factor() {
    // gcd((x-1)(x-2)(x-3), (x-1)(x-4))
    // = (x-1)
    // Build: a = [1,-1]*[1,-2]*[1,-3] ... let's use the specific arrays.
    // (x-1)(x-2) = x²-3x+2 = [2,-3,1]
    // [2,-3,1] * (x-3) = 2x-6 - 3x²+9x + x³-3x² = x³ - 6x²+11x-6 = [-6,11,-6,1]
    let a = vec![-6.0, 11.0, -6.0, 1.0]; // (x-1)(x-2)(x-3)
    // (x-1)(x-4) = x²-5x+4 = [4,-5,1]
    let b = vec![4.0, -5.0, 1.0];         // (x-1)(x-4)
    let g = gcd(&a, &b);
    // GCD should be x-1 = [-1, 1] up to scaling.
    let lead = g[g.len() - 1];
    let monic: Vec<f64> = g.iter().map(|c| c / lead).collect();
    assert!(approx_eq(&monic, &[-1.0, 1.0]), "GCD was {:?}", monic);
}

// =============================================================================
// Roundtrip / invariant tests
// =============================================================================

#[test]
fn test_add_subtract_roundtrip() {
    // a + b - b == a
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![4.0, 5.0];
    let sum = add(&a, &b);
    let back = subtract(&sum, &b);
    assert!(approx_eq(&back, &normalize(&a)), "got {:?}", back);
}

#[test]
fn test_multiply_then_divide_roundtrip() {
    // (a * b) / b == a  when b is not zero.
    let a = vec![1.0, 2.0, 3.0];
    let b = vec![1.0, 1.0]; // x + 1
    let product = multiply(&a, &b);
    let back = divide(&product, &b);
    assert!(approx_eq(&back, &a), "got {:?}", back);
}

#[test]
fn test_divmod_reconstruction() {
    // a = q * b + r  for any a and b.
    let a = vec![1.0, -4.0, 6.0, -3.0]; // some cubic
    let b = vec![1.0, 2.0];              // x + 2
    let (q, r) = divmod(&a, &b);
    let reconstructed = add(&multiply(&b, &q), &r);
    assert!(approx_eq(&reconstructed, &normalize(&a)),
        "reconstruction failed: {:?}", reconstructed);
}
