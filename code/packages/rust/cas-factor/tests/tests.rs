// Integration tests for cas-factor.
//
// Mirrors the Python reference tests in
// code/packages/python/cas-factor/tests/.

use cas_factor::{
    content, degree, divide_linear, divisors, evaluate, extract_linear_factors,
    factor_integer_polynomial, find_integer_roots, normalize, primitive_part,
};

// ---------------------------------------------------------------------------
// polynomial helpers
// ---------------------------------------------------------------------------

#[test]
fn normalize_strips_trailing_zeros() {
    assert_eq!(normalize(&[1, 2, 0, 0]), vec![1, 2]);
}

#[test]
fn normalize_zero_polynomial() {
    assert_eq!(normalize(&[0, 0]), vec![]);
    assert_eq!(normalize(&[]), vec![]);
}

#[test]
fn degree_quadratic() {
    assert_eq!(degree(&[1, 2, 3]), 2);
}

#[test]
fn degree_constant() {
    assert_eq!(degree(&[5]), 0);
}

#[test]
fn degree_zero_polynomial() {
    assert_eq!(degree(&[]), -1);
}

#[test]
fn evaluate_quadratic() {
    // p(x) = 1 + 2x + 3x^2, p(2) = 1 + 4 + 12 = 17
    assert_eq!(evaluate(&[1, 2, 3], 2), 17);
}

#[test]
fn evaluate_at_zero() {
    assert_eq!(evaluate(&[5, 7, 9], 0), 5);
}

#[test]
fn content_simple() {
    assert_eq!(content(&[2, 4, 6]), 2);
}

#[test]
fn content_with_negatives() {
    assert_eq!(content(&[-6, 4, 2]), 2);
}

#[test]
fn content_of_zero_polynomial() {
    assert_eq!(content(&[]), 0);
}

#[test]
fn primitive_part_divides_by_gcd() {
    assert_eq!(primitive_part(&[2, 4, 6]), vec![1, 2, 3]);
}

#[test]
fn divide_linear_simple() {
    // (x^2 - 1) / (x - 1) = x + 1  [root=1, factor=(x-1)]
    assert_eq!(divide_linear(&[-1, 0, 1], 1), vec![1, 1]);
}

#[test]
fn divide_linear_nontrivial() {
    // (x^3 - 6x^2 + 11x - 6) / (x - 1) = x^2 - 5x + 6
    assert_eq!(divide_linear(&[-6, 11, -6, 1], 1), vec![6, -5, 1]);
}

#[test]
fn divisors_twelve() {
    assert_eq!(divisors(12), vec![1, 2, 3, 4, 6, 12]);
}

#[test]
fn divisors_negative() {
    assert_eq!(divisors(-12), vec![1, 2, 3, 4, 6, 12]);
}

#[test]
fn divisors_zero() {
    assert_eq!(divisors(0), vec![]);
}

// ---------------------------------------------------------------------------
// rational roots
// ---------------------------------------------------------------------------

#[test]
fn find_roots_simple_quadratic() {
    // x^2 - 1 → roots {-1, 1}
    let mut roots = find_integer_roots(&[-1, 0, 1]);
    roots.sort_unstable();
    assert_eq!(roots, vec![-1, 1]);
}

#[test]
fn find_roots_cubic() {
    // x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3)
    let mut roots = find_integer_roots(&[-6, 11, -6, 1]);
    roots.sort_unstable();
    assert_eq!(roots, vec![1, 2, 3]);
}

#[test]
fn find_roots_irreducible_quadratic() {
    // x^2 + 1 has no integer roots
    assert_eq!(find_integer_roots(&[1, 0, 1]), vec![]);
}

#[test]
fn find_roots_zero_constant() {
    // x(x-1) = x^2 - x → roots {0, 1}
    let mut roots = find_integer_roots(&[0, -1, 1]);
    roots.sort_unstable();
    assert_eq!(roots, vec![0, 1]);
}

#[test]
fn extract_simple_quadratic() {
    // x^2 - 1 → factors=[(-1,1),(1,1)], residual=[1]
    let (factors, residual) = extract_linear_factors(&[-1, 0, 1]);
    assert_eq!(factors, vec![(-1, 1), (1, 1)]);
    assert_eq!(residual, vec![1]);
}

#[test]
fn extract_with_multiplicity() {
    // (x+1)^2 = x^2 + 2x + 1 → factors=[(-1, 2)], residual=[1]
    let (factors, residual) = extract_linear_factors(&[1, 2, 1]);
    assert_eq!(factors, vec![(-1, 2)]);
    assert_eq!(residual, vec![1]);
}

#[test]
fn extract_leaves_irreducible() {
    // x^2 + 1 → factors=[], residual=[1, 0, 1]
    let (factors, residual) = extract_linear_factors(&[1, 0, 1]);
    assert_eq!(factors, vec![]);
    assert_eq!(residual, vec![1, 0, 1]);
}

#[test]
fn extract_mixed_linear_and_irreducible() {
    // (x-1)(x^2+1) = x^3 - x^2 + x - 1
    let p = vec![-1i64, 1, -1, 1];
    let (factors, residual) = extract_linear_factors(&p);
    assert_eq!(factors, vec![(1, 1)]);
    assert_eq!(residual, vec![1, 0, 1]);
}

// ---------------------------------------------------------------------------
// factor_integer_polynomial
// ---------------------------------------------------------------------------

#[test]
fn factor_x_squared_minus_one() {
    // x^2 - 1 = (x-1)(x+1)
    let (c, mut factors) = factor_integer_polynomial(&[-1, 0, 1]);
    assert_eq!(c, 1);
    factors.sort_by_key(|(f, _)| f.clone());
    let expected: Vec<(Vec<i64>, usize)> = vec![(vec![-1, 1], 1), (vec![1, 1], 1)];
    assert_eq!(factors, expected);
}

#[test]
fn factor_with_content() {
    // 2x^2 + 4x + 2 = 2*(x+1)^2
    let (c, factors) = factor_integer_polynomial(&[2, 4, 2]);
    assert_eq!(c, 2);
    assert_eq!(factors, vec![(vec![1, 1], 2)]);
}

#[test]
fn factor_irreducible_quadratic() {
    // x^2 + 1 — Phase 1 leaves it as a residual
    let (c, factors) = factor_integer_polynomial(&[1, 0, 1]);
    assert_eq!(c, 1);
    assert_eq!(factors, vec![(vec![1, 0, 1], 1)]);
}

#[test]
fn factor_cubic() {
    // x^3 - 6x^2 + 11x - 6 = (x-1)(x-2)(x-3)
    let (c, mut factors) = factor_integer_polynomial(&[-6, 11, -6, 1]);
    assert_eq!(c, 1);
    factors.sort_by_key(|(f, _)| f.clone());
    // Sorted lexicographically by coefficient vector: [-3,1] < [-2,1] < [-1,1]
    let expected: Vec<(Vec<i64>, usize)> =
        vec![(vec![-3, 1], 1), (vec![-2, 1], 1), (vec![-1, 1], 1)];
    assert_eq!(factors, expected);
}

#[test]
fn factor_with_zero_root() {
    // x^3 - x = x(x-1)(x+1)
    let (c, mut factors) = factor_integer_polynomial(&[0, -1, 0, 1]);
    assert_eq!(c, 1);
    factors.sort_by_key(|(f, _)| f.clone());
    let expected: Vec<(Vec<i64>, usize)> =
        vec![(vec![-1, 1], 1), (vec![0, 1], 1), (vec![1, 1], 1)];
    assert_eq!(factors, expected);
}

#[test]
fn factor_empty() {
    // Zero polynomial → (0, [])
    assert_eq!(factor_integer_polynomial(&[]), (0, vec![]));
}

#[test]
fn factor_constant() {
    // 6 → (6, [])
    let (c, factors) = factor_integer_polynomial(&[6]);
    assert_eq!(c, 6);
    assert_eq!(factors, vec![]);
}

#[test]
fn factor_linear() {
    // 2x + 4 = 2*(x + 2).  Root is -2, so factor (x - (-2)) = (x + 2) = [2, 1].
    let (c, factors) = factor_integer_polynomial(&[4, 2]);
    assert_eq!(c, 2);
    assert_eq!(factors, vec![(vec![2, 1], 1)]);
}

#[test]
fn factor_negative_leading() {
    // -(x^2-1) = -x^2+1 = [-1, 0, 1] negated = [1, 0, -1]
    // content = 1, pp = [1, 0, -1], roots of 1 - x^2 = x={-1,1}
    let (c, mut factors) = factor_integer_polynomial(&[1, 0, -1]);
    // 1 - x^2 = -(x^2 - 1) = -(x-1)(x+1).
    // Roots: -1 and 1. Residual after dividing both out is [-1].
    // The [-1] residual is absorbed into content: c = -content = -1.
    // factors: root=-1 → [1,1], root=1 → [-1,1].
    assert_eq!(c, -1);
    factors.sort_by_key(|(f, _)| f.clone());
    // Sorted: [-1,1] < [1,1] (first element -1 < 1)
    assert_eq!(factors, vec![(vec![-1, 1], 1), (vec![1, 1], 1)]);
}
