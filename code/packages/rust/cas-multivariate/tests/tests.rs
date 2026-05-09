use std::cmp::Ordering;

use cas_multivariate::{
    buchberger, cmp_monomials, div_monomial, div_reduction_step, divides, ideal_solve,
    lcm_monomial, make_var, rational_roots, reduce_poly, s_poly, solve_univariate, total_degree,
    GrobnerError, MPoly, MonomialOrder, Rational,
};

fn f(n: i64) -> Rational {
    Rational::from_int(n)
}

fn frac(n: i64, d: i64) -> Rational {
    Rational::new(n, d)
}

fn p2(terms: &[([usize; 2], Rational)]) -> MPoly {
    MPoly::new(terms.iter().map(|(m, c)| (m.to_vec(), *c)), 2)
}

#[test]
fn monomial_orders_match_python_examples() {
    assert_eq!(
        cmp_monomials(&[2, 1], &[1, 2], MonomialOrder::GrLex),
        Ordering::Greater
    );
    assert_eq!(
        cmp_monomials(&[0, 3], &[1, 1], MonomialOrder::GrLex),
        Ordering::Greater
    );
    assert_eq!(
        cmp_monomials(&[2, 0], &[1, 5], MonomialOrder::Lex),
        Ordering::Greater
    );
    assert_eq!(
        cmp_monomials(&[3, 0, 0], &[1, 1, 1], MonomialOrder::GRevLex),
        Ordering::Greater
    );
}

#[test]
fn monomial_arithmetic_helpers() {
    assert_eq!(lcm_monomial(&[2, 1, 0], &[1, 2, 3]), vec![2, 2, 3]);
    assert!(divides(&[1, 1], &[2, 3]));
    assert!(!divides(&[2, 1], &[1, 2]));
    assert_eq!(div_monomial(&[3, 2], &[1, 1]), vec![2, 1]);
    assert_eq!(total_degree(&[3, 2, 1]), 6);
}

#[test]
fn mpoly_constructs_and_cleans_zero_terms() {
    let poly = p2(&[([2, 0], f(1)), ([0, 1], Rational::ZERO), ([0, 0], f(-1))]);
    assert_eq!(poly.coeffs.len(), 2);
    assert!(!poly.coeffs.contains_key(&vec![0, 1]));
    assert!(MPoly::zero(2).is_zero());
    assert_eq!(
        MPoly::constant(frac(3, 2), 2).coeffs[&vec![0, 0]],
        frac(3, 2)
    );
}

#[test]
fn mpoly_leading_terms_and_arithmetic() {
    let poly = p2(&[([2, 1], f(3)), ([0, 3], f(2))]);
    assert_eq!(poly.lm("grlex").unwrap(), vec![2, 1]);
    assert_eq!(poly.lc("grlex").unwrap(), f(3));
    assert_eq!(poly.lt("grlex").unwrap().coeffs[&vec![2, 1]], f(3));

    let lhs = p2(&[([1, 0], f(1)), ([0, 0], f(1))]);
    let rhs = p2(&[([1, 0], f(1)), ([0, 0], f(-1))]);
    let product = lhs.clone() * rhs;
    assert_eq!(product, p2(&[([2, 0], f(1)), ([0, 0], f(-1))]));

    assert_eq!(
        lhs.mul_monomial(&[1, 0], Rational::ONE),
        p2(&[([2, 0], f(1)), ([1, 0], f(1))])
    );
}

#[test]
fn mpoly_utilities_match_python_behavior() {
    let poly = p2(&[([2, 1], f(1)), ([1, 0], f(1))]);
    assert_eq!(poly.total_degree(), 3);
    assert_eq!(poly.diff(0), p2(&[([1, 1], f(2)), ([0, 0], f(1))]));

    let eval = p2(&[([2, 0], f(1)), ([0, 1], f(1))]).eval_at(0, f(2));
    assert_eq!(eval, p2(&[([0, 1], f(1)), ([0, 0], f(4))]));

    assert_eq!(make_var(0, 2), p2(&[([1, 0], f(1))]));
    assert_eq!(
        p2(&[([2, 0], f(1)), ([0, 0], f(-1))]).to_univariate_coeffs(0),
        vec![f(-1), Rational::ZERO, Rational::ONE]
    );
}

#[test]
fn div_reduction_step_applies_once() {
    let result = div_reduction_step(&p2(&[([2, 0], f(1))]), &p2(&[([1, 0], f(1))]), "lex")
        .unwrap()
        .unwrap();
    assert_eq!(result.0, p2(&[([1, 0], f(1))]));
    assert!(result.1.is_zero());

    let no_step =
        div_reduction_step(&p2(&[([0, 1], f(1))]), &p2(&[([1, 0], f(1))]), "lex").unwrap();
    assert!(no_step.is_none());
}

#[test]
fn s_polynomial_examples() {
    let sp = s_poly(&p2(&[([2, 0], f(1))]), &p2(&[([1, 1], f(1))]), "grlex").unwrap();
    assert!(sp.is_zero());

    let f_poly = p2(&[([2, 0], f(1)), ([0, 1], f(1))]);
    let g_poly = p2(&[([1, 1], f(1)), ([0, 0], f(1))]);
    assert_eq!(
        s_poly(&f_poly, &g_poly, "grlex").unwrap(),
        p2(&[([0, 2], f(1)), ([1, 0], f(-1))])
    );
}

#[test]
fn reduce_poly_examples() {
    let remainder = reduce_poly(
        &p2(&[([2, 0], f(1)), ([0, 0], f(-1))]),
        &[p2(&[([1, 0], f(1)), ([0, 0], f(-1))])],
        "lex",
    )
    .unwrap();
    assert!(remainder.is_zero());

    let remainder = reduce_poly(
        &p2(&[([2, 0], f(1)), ([0, 1], f(1))]),
        &[p2(&[([2, 0], f(1)), ([0, 0], f(-1))])],
        "grlex",
    )
    .unwrap();
    assert_eq!(remainder, p2(&[([0, 1], f(1)), ([0, 0], f(1))]));
}

#[test]
fn groebner_linear_system_reduces_generators() {
    let f1 = p2(&[([1, 0], f(1)), ([0, 1], f(1)), ([0, 0], f(-1))]);
    let f2 = p2(&[([1, 0], f(1)), ([0, 1], f(-1))]);
    let basis = buchberger(&[f1.clone(), f2.clone()], "lex").unwrap();
    assert_eq!(basis.len(), 2);
    assert!(reduce_poly(&f1, &basis, "lex").unwrap().is_zero());
    assert!(reduce_poly(&f2, &basis, "lex").unwrap().is_zero());
}

#[test]
fn groebner_quadratic_and_basis_membership() {
    let f1 = p2(&[([2, 0], f(1)), ([0, 0], f(-1))]);
    let f2 = p2(&[([0, 1], f(1)), ([1, 0], f(-1))]);
    let basis = buchberger(&[f1.clone(), f2.clone()], "lex").unwrap();
    assert!(reduce_poly(&f1, &basis, "lex").unwrap().is_zero());
    assert!(reduce_poly(&f2, &basis, "lex").unwrap().is_zero());

    let g1 = p2(&[([2, 1], f(1)), ([0, 0], f(-1))]);
    let g2 = p2(&[([1, 2], f(1)), ([0, 0], f(-1))]);
    let basis = buchberger(&[g1.clone(), g2.clone()], "grlex").unwrap();
    assert!(reduce_poly(&g1, &basis, "grlex").unwrap().is_zero());
    assert!(reduce_poly(&g2, &basis, "grlex").unwrap().is_zero());
}

#[test]
fn groebner_degree_limit() {
    let err = buchberger(&[p2(&[([9, 0], f(1))])], "grlex").unwrap_err();
    assert!(matches!(err, GrobnerError::DegreeTooLarge { .. }));
}

#[test]
fn rational_root_and_univariate_solving() {
    let mut roots = rational_roots(&[f(-1), Rational::ZERO, Rational::ONE]);
    roots.sort();
    assert_eq!(roots, vec![f(-1), f(1)]);
    assert_eq!(
        rational_roots(&[f(-2), Rational::ZERO, Rational::ONE]),
        vec![]
    );
    assert_eq!(rational_roots(&[f(-1), f(2)]), vec![frac(1, 2)]);

    assert_eq!(solve_univariate(&[f(-4), f(2)]).unwrap(), vec![f(2)]);
    let mut roots = solve_univariate(&[f(-4), Rational::ZERO, Rational::ONE]).unwrap();
    roots.sort();
    assert_eq!(roots, vec![f(-2), f(2)]);
    assert_eq!(
        solve_univariate(&[f(1), Rational::ZERO, Rational::ONE]).unwrap(),
        Vec::<Rational>::new()
    );
    assert_eq!(
        solve_univariate(&[f(-2), Rational::ZERO, Rational::ONE]).unwrap(),
        Vec::<Rational>::new()
    );
}

#[test]
fn ideal_solve_linear_and_quadratic_systems() {
    let f1 = p2(&[([1, 0], f(1)), ([0, 1], f(1)), ([0, 0], f(-1))]);
    let f2 = p2(&[([1, 0], f(1)), ([0, 1], f(-1))]);
    assert_eq!(
        ideal_solve(&[f1, f2]).unwrap(),
        vec![vec![frac(1, 2), frac(1, 2)]]
    );

    let f1 = p2(&[([2, 0], f(1)), ([0, 0], f(-1))]);
    let f2 = p2(&[([0, 1], f(1)), ([1, 0], f(-1))]);
    let mut solutions = ideal_solve(&[f1, f2]).unwrap();
    solutions.sort();
    assert_eq!(solutions, vec![vec![f(-1), f(-1)], vec![f(1), f(1)]]);
}

#[test]
fn ideal_solve_none_cases() {
    let f1 = p2(&[([2, 0], f(1)), ([0, 0], f(1))]);
    let f2 = p2(&[([0, 1], f(1)), ([1, 0], f(-1))]);
    assert!(ideal_solve(&[f1, f2]).is_none());
    assert!(ideal_solve(&[]).is_none());
}
