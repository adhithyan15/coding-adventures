use crate::monomial::{div_monomial, divides, lcm_monomial, MonomialOrder};
use crate::{MPoly, PolynomialError, Rational};

pub fn reduce_poly(f: &MPoly, basis: &[MPoly], order: &str) -> Result<MPoly, PolynomialError> {
    let order = MonomialOrder::parse(order)?;
    let mut p = f.clone();
    let mut r = MPoly::zero(f.nvars);

    while !p.is_zero() {
        let lm_p = p.leading_monomial(order)?;
        let mut reduced = false;

        for g in basis {
            if g.is_zero() {
                continue;
            }
            let lm_g = g.leading_monomial(order)?;
            if divides(&lm_g, &lm_p) {
                let exp_diff = div_monomial(&lm_p, &lm_g);
                let coeff = p.leading_coefficient(order)? / g.leading_coefficient(order)?;
                p = p - g.mul_monomial(&exp_diff, coeff);
                reduced = true;
                break;
            }
        }

        if !reduced {
            let lt = p.leading_term(order)?;
            r = r + lt.clone();
            p = p - lt;
        }
    }

    Ok(r)
}

pub fn s_poly(f: &MPoly, g: &MPoly, order: &str) -> Result<MPoly, PolynomialError> {
    assert!(
        !f.is_zero() && !g.is_zero(),
        "S-polynomial undefined for zero polynomials"
    );
    let order = MonomialOrder::parse(order)?;
    let lm_f = f.leading_monomial(order)?;
    let lm_g = g.leading_monomial(order)?;
    let lcm = lcm_monomial(&lm_f, &lm_g);
    let exp_f = div_monomial(&lcm, &lm_f);
    let exp_g = div_monomial(&lcm, &lm_g);
    let coeff_f = Rational::ONE / f.leading_coefficient(order)?;
    let coeff_g = Rational::ONE / g.leading_coefficient(order)?;
    Ok(f.mul_monomial(&exp_f, coeff_f) - g.mul_monomial(&exp_g, coeff_g))
}
