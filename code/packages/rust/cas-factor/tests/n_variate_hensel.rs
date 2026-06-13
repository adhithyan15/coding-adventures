// Tests for n-variate (n ≥ 3) Hensel lifting — Track K2 (Rust port of
// Python Track K1, PR #5590).
//
// Mirrors `code/packages/python/cas-factor/tests/test_n_variate_hensel.py`.
// Each test builds the input from known factors, runs `try_n_variate_hensel`,
// and verifies the product of returned factors equals the input.  Factor
// ordering is not pinned — different specialisation tuples may yield
// permutations.

use std::collections::BTreeMap;

use cas_factor::{n_mul, try_n_variate_hensel, NPoly, Rat};

fn make(num_vars: usize, terms: &[(Vec<usize>, i128)]) -> NPoly {
    let mut out: NPoly = BTreeMap::new();
    for (k, c) in terms {
        assert_eq!(k.len(), num_vars, "tuple length must match num_vars");
        if *c == 0 {
            continue;
        }
        let cur = out.get(k).copied().unwrap_or(Rat::ZERO);
        out.insert(k.clone(), cur.add(&Rat::from_int(*c)));
    }
    out.retain(|_, v| !v.is_zero());
    out
}

fn n_one_poly(num_vars: usize) -> NPoly {
    let mut m = BTreeMap::new();
    m.insert(vec![0usize; num_vars], Rat::ONE);
    m
}

fn verify_product(num_vars: usize, factors: &[NPoly], expected: &NPoly) -> bool {
    let mut prod = n_one_poly(num_vars);
    for f in factors {
        prod = n_mul(&prod, f, num_vars);
    }
    prod.retain(|_, v| !v.is_zero());
    let mut exp = expected.clone();
    exp.retain(|_, v| !v.is_zero());
    prod == exp
}

#[test]
fn trivariate_quadratic_three_factors_known() {
    // x² − y² − z² − 2yz = (x + y + z)(x − y − z)
    let poly = make(
        3,
        &[
            (vec![2, 0, 0], 1),
            (vec![0, 2, 0], -1),
            (vec![0, 0, 2], -1),
            (vec![0, 1, 1], -2),
        ],
    );
    let result = try_n_variate_hensel(&poly, 3).expect("expected trivariate factorisation");
    assert!(result.len() >= 2);
    assert!(verify_product(3, &result, &poly));
}

#[test]
fn trivariate_product_of_two_linear() {
    // (x + y + z)(x + 2y + 3z) = x² + 3xy + 4xz + 2y² + 5yz + 3z²
    let poly = make(
        3,
        &[
            (vec![2, 0, 0], 1),
            (vec![1, 1, 0], 3),
            (vec![1, 0, 1], 4),
            (vec![0, 2, 0], 2),
            (vec![0, 1, 1], 5),
            (vec![0, 0, 2], 3),
        ],
    );
    let result = try_n_variate_hensel(&poly, 3).expect("expected factorisation");
    assert_eq!(result.len(), 2);
    assert!(verify_product(3, &result, &poly));
}

#[test]
fn trivariate_sum_of_cubes_companion() {
    // (x+y+z)(x²+y²+z²−xy−yz−xz)
    let factor_a = make(
        3,
        &[
            (vec![1, 0, 0], 1),
            (vec![0, 1, 0], 1),
            (vec![0, 0, 1], 1),
        ],
    );
    let factor_b = make(
        3,
        &[
            (vec![2, 0, 0], 1),
            (vec![0, 2, 0], 1),
            (vec![0, 0, 2], 1),
            (vec![1, 1, 0], -1),
            (vec![0, 1, 1], -1),
            (vec![1, 0, 1], -1),
        ],
    );
    let poly = n_mul(&factor_a, &factor_b, 3);
    let result = try_n_variate_hensel(&poly, 3).expect("expected factorisation");
    assert!(result.len() >= 2);
    assert!(verify_product(3, &result, &poly));
}

#[test]
fn quadrivariate_linear_times_linear() {
    // (x+y)(x+z+w) — iterated lift across two aux vars.
    let factor_a = make(
        4,
        &[(vec![1, 0, 0, 0], 1), (vec![0, 1, 0, 0], 1)],
    );
    let factor_b = make(
        4,
        &[
            (vec![1, 0, 0, 0], 1),
            (vec![0, 0, 1, 0], 1),
            (vec![0, 0, 0, 1], 1),
        ],
    );
    let poly = n_mul(&factor_a, &factor_b, 4);
    let result = try_n_variate_hensel(&poly, 4).expect("expected factorisation");
    assert_eq!(result.len(), 2);
    assert!(verify_product(4, &result, &poly));
}

// ---------------------------------------------------------------------------
// Fall-through cases.
// ---------------------------------------------------------------------------

#[test]
fn irreducible_trivariate_returns_none() {
    // x² + y² + z² + 1
    let poly = make(
        3,
        &[
            (vec![2, 0, 0], 1),
            (vec![0, 2, 0], 1),
            (vec![0, 0, 2], 1),
            (vec![0, 0, 0], 1),
        ],
    );
    assert!(try_n_variate_hensel(&poly, 3).is_none());
}

#[test]
fn single_variable_returns_none() {
    let poly = make(3, &[(vec![2, 0, 0], 1), (vec![0, 0, 0], -1)]);
    assert!(try_n_variate_hensel(&poly, 3).is_none());
}

#[test]
fn num_vars_less_than_two_returns_none() {
    let poly = make(1, &[(vec![2], 1), (vec![0], -1)]);
    assert!(try_n_variate_hensel(&poly, 1).is_none());
}

#[test]
fn constant_returns_none() {
    let poly = make(3, &[(vec![0, 0, 0], 7)]);
    assert!(try_n_variate_hensel(&poly, 3).is_none());
}

#[test]
fn empty_polynomial_returns_none() {
    let poly: NPoly = BTreeMap::new();
    assert!(try_n_variate_hensel(&poly, 3).is_none());
}

#[test]
fn linear_polynomial_returns_none() {
    let poly = make(
        3,
        &[(vec![1, 0, 0], 1), (vec![0, 1, 0], 1), (vec![0, 0, 1], 1)],
    );
    assert!(try_n_variate_hensel(&poly, 3).is_none());
}

// ---------------------------------------------------------------------------
// Regression: bivariate via n-variate front door.
// ---------------------------------------------------------------------------

#[test]
fn bivariate_via_n_variate_x_squared_plus_xy_minus_2y_squared() {
    let poly = make(2, &[(vec![2, 0], 1), (vec![1, 1], 1), (vec![0, 2], -2)]);
    let result = try_n_variate_hensel(&poly, 2).expect("expected factorisation");
    assert_eq!(result.len(), 2);
    assert!(verify_product(2, &result, &poly));
}

#[test]
fn bivariate_via_n_variate_x_cubed_minus_y_cubed() {
    let poly = make(2, &[(vec![3, 0], 1), (vec![0, 3], -1)]);
    let result = try_n_variate_hensel(&poly, 2).expect("expected factorisation");
    assert_eq!(result.len(), 2);
    assert!(verify_product(2, &result, &poly));
}

// ---------------------------------------------------------------------------
// Robustness.
// ---------------------------------------------------------------------------

#[test]
fn high_degree_irreducible_does_not_loop() {
    // x^4 + y^2 + z^2 + 1 — irreducible over Q.
    let poly = make(
        3,
        &[
            (vec![4, 0, 0], 1),
            (vec![0, 0, 0], 1),
            (vec![0, 0, 2], 1),
            (vec![0, 2, 0], 1),
        ],
    );
    let result = try_n_variate_hensel(&poly, 3);
    assert!(result.is_none());
}
