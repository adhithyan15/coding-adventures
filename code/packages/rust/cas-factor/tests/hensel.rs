// Acceptance tests for bivariate Hensel lifting.
//
// Mirrors the Python `test_hensel.py` suite — same 5 acceptance cases
// plus a univariate fall-through regression — to guarantee
// cross-language parity with the Python reference.

use std::collections::BTreeMap;

use cas_factor::{bi_mul, try_bivariate_hensel, BiPoly, Rat};

fn make(terms: &[(usize, usize, i128)]) -> BiPoly {
    let mut out: BiPoly = BTreeMap::new();
    for &(i, j, c) in terms {
        if c == 0 {
            continue;
        }
        let cur = out.get(&(i, j)).copied().unwrap_or(Rat::ZERO);
        out.insert((i, j), cur.add(&Rat::from_int(c)));
    }
    out.retain(|_, v| !v.is_zero());
    out
}

fn verify_product(factors: &[BiPoly], expected: &BiPoly) -> bool {
    let mut prod: BiPoly = BTreeMap::new();
    prod.insert((0, 0), Rat::ONE);
    for f in factors {
        prod = bi_mul(&prod, f);
    }
    prod.retain(|_, v| !v.is_zero());
    let mut exp = expected.clone();
    exp.retain(|_, v| !v.is_zero());
    prod == exp
}

#[test]
fn hensel_factors_x2_xy_minus_2y2() {
    // x² + xy − 2y² = (x + 2y)(x − y)
    let f = make(&[(2, 0, 1), (1, 1, 1), (0, 2, -2)]);
    let result = try_bivariate_hensel(&f).expect("expected factorisation");
    assert_eq!(result.len(), 2);
    assert!(verify_product(&result, &f));
}

#[test]
fn hensel_factors_non_unit_leading_2x2_3xy_minus_2y2() {
    // 2x² + 3xy − 2y² = (2x − y)(x + 2y)
    let f = make(&[(2, 0, 2), (1, 1, 3), (0, 2, -2)]);
    let result = try_bivariate_hensel(&f).expect("expected factorisation");
    assert!(result.len() >= 2);
    assert!(verify_product(&result, &f));
}

#[test]
fn hensel_factors_x3_minus_y3() {
    // x³ − y³ = (x − y)(x² + xy + y²)
    let f = make(&[(3, 0, 1), (0, 3, -1)]);
    let result = try_bivariate_hensel(&f).expect("expected factorisation");
    assert_eq!(result.len(), 2);
    assert!(verify_product(&result, &f));
}

#[test]
fn hensel_returns_none_for_irreducible_x2_y2_plus_1() {
    // x² + y² + 1 — irreducible over ℚ
    let f = make(&[(2, 0, 1), (0, 2, 1), (0, 0, 1)]);
    assert!(try_bivariate_hensel(&f).is_none());
}

#[test]
fn hensel_returns_none_for_univariate_x2_minus_1() {
    // Univariate falls through — caller's univariate path handles it.
    let f = make(&[(2, 0, 1), (0, 0, -1)]);
    assert!(try_bivariate_hensel(&f).is_none());
}

#[test]
fn hensel_returns_none_for_linear_x_plus_y() {
    // Bare x + y is irreducible.
    let f = make(&[(1, 0, 1), (0, 1, 1)]);
    assert!(try_bivariate_hensel(&f).is_none());
}
