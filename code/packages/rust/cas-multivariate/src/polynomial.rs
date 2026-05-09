use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Add, Mul, Neg, Sub};

use crate::monomial::{
    cmp_monomials, div_monomial, divides, total_degree, Monomial, MonomialOrder, MonomialOrderError,
};
use crate::Rational;

/// Sparse multivariate polynomial over Q.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MPoly {
    pub coeffs: BTreeMap<Monomial, Rational>,
    pub nvars: usize,
}

impl MPoly {
    pub fn new<I>(coeffs: I, nvars: usize) -> Self
    where
        I: IntoIterator<Item = (Monomial, Rational)>,
    {
        let mut out = BTreeMap::new();
        for (m, c) in coeffs {
            assert_eq!(m.len(), nvars, "monomial variable count mismatch");
            if !c.is_zero() {
                out.insert(m, c);
            }
        }
        Self { coeffs: out, nvars }
    }

    pub fn zero(nvars: usize) -> Self {
        Self {
            coeffs: BTreeMap::new(),
            nvars,
        }
    }

    pub fn constant<C: Into<Rational>>(c: C, nvars: usize) -> Self {
        let c = c.into();
        if c.is_zero() {
            return Self::zero(nvars);
        }
        Self::new([(vec![0; nvars], c)], nvars)
    }

    pub fn monomial_poly<C: Into<Rational>>(exp: Monomial, c: C, nvars: usize) -> Self {
        let c = c.into();
        if c.is_zero() {
            return Self::zero(nvars);
        }
        Self::new([(exp, c)], nvars)
    }

    pub fn is_zero(&self) -> bool {
        self.coeffs.is_empty()
    }

    pub fn lm(&self, order: &str) -> Result<Monomial, PolynomialError> {
        let order = MonomialOrder::parse(order)?;
        self.leading_monomial(order)
    }

    pub fn lc(&self, order: &str) -> Result<Rational, PolynomialError> {
        let lm = self.lm(order)?;
        Ok(self.coeffs[&lm])
    }

    pub fn lt(&self, order: &str) -> Result<Self, PolynomialError> {
        let lm = self.lm(order)?;
        Ok(Self::monomial_poly(
            lm.clone(),
            self.coeffs[&lm],
            self.nvars,
        ))
    }

    pub(crate) fn leading_monomial(
        &self,
        order: MonomialOrder,
    ) -> Result<Monomial, PolynomialError> {
        self.coeffs
            .keys()
            .max_by(|a, b| cmp_monomials(a, b, order))
            .cloned()
            .ok_or(PolynomialError::ZeroLeadingTerm)
    }

    pub(crate) fn leading_coefficient(
        &self,
        order: MonomialOrder,
    ) -> Result<Rational, PolynomialError> {
        let lm = self.leading_monomial(order)?;
        Ok(self.coeffs[&lm])
    }

    pub(crate) fn leading_term(&self, order: MonomialOrder) -> Result<Self, PolynomialError> {
        let lm = self.leading_monomial(order)?;
        Ok(Self::monomial_poly(
            lm.clone(),
            self.coeffs[&lm],
            self.nvars,
        ))
    }

    pub fn total_degree(&self) -> usize {
        self.coeffs
            .keys()
            .map(|m| total_degree(m))
            .max()
            .unwrap_or(0)
    }

    pub fn scale<C: Into<Rational>>(&self, c: C) -> Self {
        let c = c.into();
        if c.is_zero() {
            return Self::zero(self.nvars);
        }
        Self::new(
            self.coeffs.iter().map(|(m, coeff)| (m.clone(), *coeff * c)),
            self.nvars,
        )
    }

    pub fn mul_monomial<C: Into<Rational>>(&self, exp: &[usize], c: C) -> Self {
        assert_eq!(exp.len(), self.nvars, "monomial variable count mismatch");
        let c = c.into();
        if c.is_zero() {
            return Self::zero(self.nvars);
        }
        Self::new(
            self.coeffs.iter().map(|(m, coeff)| {
                (
                    exp.iter().zip(m).map(|(&ei, &mi)| ei + mi).collect(),
                    *coeff * c,
                )
            }),
            self.nvars,
        )
    }

    pub fn monomials_descending(&self, order: &str) -> Result<Vec<Monomial>, PolynomialError> {
        let order = MonomialOrder::parse(order)?;
        let mut monomials: Vec<_> = self.coeffs.keys().cloned().collect();
        monomials.sort_by(|a, b| cmp_monomials(b, a, order));
        Ok(monomials)
    }

    pub fn is_univariate(&self) -> Option<usize> {
        let mut active = BTreeSet::new();
        for m in self.coeffs.keys() {
            for (i, &e) in m.iter().enumerate() {
                if e != 0 {
                    active.insert(i);
                }
            }
        }
        match active.len().cmp(&1) {
            Ordering::Less => Some(0),
            Ordering::Equal => active.into_iter().next(),
            Ordering::Greater => None,
        }
    }

    pub fn to_univariate_coeffs(&self, var_idx: usize) -> Vec<Rational> {
        let max_deg = self
            .coeffs
            .keys()
            .map(|m| m[var_idx])
            .max()
            .unwrap_or_default();
        let mut out = vec![Rational::ZERO; max_deg + 1];
        for (m, c) in &self.coeffs {
            out[m[var_idx]] = *c;
        }
        out
    }

    pub fn leading_monomial_divides(
        &self,
        m: &[usize],
        order: &str,
    ) -> Result<bool, PolynomialError> {
        let lm = self.lm(order)?;
        Ok(divides(&lm, m))
    }

    pub fn diff(&self, var_idx: usize) -> Self {
        let mut out: BTreeMap<Monomial, Rational> = BTreeMap::new();
        for (m, c) in &self.coeffs {
            let exp = m[var_idx];
            if exp == 0 {
                continue;
            }
            let mut new_m = m.clone();
            new_m[var_idx] -= 1;
            let coeff = *c * Rational::from_int(exp as i64);
            let current = out.get(&new_m).copied().unwrap_or(Rational::ZERO);
            out.insert(new_m, current + coeff);
        }
        Self::new(out, self.nvars)
    }

    pub fn eval_at(&self, var_idx: usize, value: Rational) -> Self {
        let mut out: BTreeMap<Monomial, Rational> = BTreeMap::new();
        for (m, c) in &self.coeffs {
            let mut new_m = m.clone();
            new_m[var_idx] = 0;
            let coeff = *c * value.pow_usize(m[var_idx]);
            let current = out.get(&new_m).copied().unwrap_or(Rational::ZERO);
            out.insert(new_m, current + coeff);
        }
        Self::new(out, self.nvars)
    }
}

impl Add for MPoly {
    type Output = MPoly;

    fn add(self, rhs: Self) -> Self::Output {
        assert_eq!(self.nvars, rhs.nvars, "MPoly variable count mismatch");
        let mut out = self.coeffs;
        for (m, c) in rhs.coeffs {
            let next = out.get(&m).copied().unwrap_or(Rational::ZERO) + c;
            if next.is_zero() {
                out.remove(&m);
            } else {
                out.insert(m, next);
            }
        }
        Self::new(out, self.nvars)
    }
}

impl Sub for MPoly {
    type Output = MPoly;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Neg for MPoly {
    type Output = MPoly;

    fn neg(self) -> Self::Output {
        Self::new(self.coeffs.into_iter().map(|(m, c)| (m, -c)), self.nvars)
    }
}

impl Mul for MPoly {
    type Output = MPoly;

    fn mul(self, rhs: Self) -> Self::Output {
        assert_eq!(self.nvars, rhs.nvars, "MPoly variable count mismatch");
        let mut out: BTreeMap<Monomial, Rational> = BTreeMap::new();
        for (ma, ca) in &self.coeffs {
            for (mb, cb) in &rhs.coeffs {
                let m = ma.iter().zip(mb).map(|(&a, &b)| a + b).collect::<Vec<_>>();
                let next = out.get(&m).copied().unwrap_or(Rational::ZERO) + (*ca * *cb);
                if next.is_zero() {
                    out.remove(&m);
                } else {
                    out.insert(m, next);
                }
            }
        }
        Self::new(out, self.nvars)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolynomialError {
    BadOrder(MonomialOrderError),
    ZeroLeadingTerm,
}

impl std::fmt::Display for PolynomialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadOrder(err) => write!(f, "{err}"),
            Self::ZeroLeadingTerm => {
                write!(f, "leading monomial of the zero polynomial is undefined")
            }
        }
    }
}

impl std::error::Error for PolynomialError {}

impl From<MonomialOrderError> for PolynomialError {
    fn from(value: MonomialOrderError) -> Self {
        Self::BadOrder(value)
    }
}

pub fn make_var(var_idx: usize, nvars: usize) -> MPoly {
    let mut exp = vec![0; nvars];
    exp[var_idx] = 1;
    MPoly::monomial_poly(exp, Rational::ONE, nvars)
}

pub fn div_reduction_step(
    f: &MPoly,
    g: &MPoly,
    order: &str,
) -> Result<Option<(MPoly, MPoly)>, PolynomialError> {
    if f.is_zero() {
        return Ok(None);
    }
    let order = MonomialOrder::parse(order)?;
    let lm_f = f.leading_monomial(order)?;
    let lm_g = g.leading_monomial(order)?;
    if !divides(&lm_g, &lm_f) {
        return Ok(None);
    }
    let exp_diff = div_monomial(&lm_f, &lm_g);
    let coeff = f.leading_coefficient(order)? / g.leading_coefficient(order)?;
    let term = MPoly::monomial_poly(exp_diff.clone(), coeff, f.nvars);
    Ok(Some((term, f.clone() - g.mul_monomial(&exp_diff, coeff))))
}
