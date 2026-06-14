//! Kronecker factor splitting for primitive integer polynomials.
//!
//! This module searches for one non-trivial factor/cofactor pair over `Z[x]`.
//! Linear integer-root extraction is still handled by `rational_roots`; this
//! phase is for residual quadratics and higher-degree pieces with no linear
//! roots.

use crate::polynomial::{degree, divisors, evaluate, normalize, Poly};

const MAX_COMBOS: usize = 10_000;

/// Find one non-trivial factor/cofactor pair using Kronecker's method.
///
/// `p` is expected to be primitive. The returned factor and cofactor are
/// normalized to positive leading coefficient.
pub fn kronecker_factor(p: &[i64]) -> Option<(Poly, Poly)> {
    let p = normalize(p);
    let d = degree(&p);
    if d < 2 {
        return None;
    }

    for k in 1..=(d as usize / 2) {
        let points = eval_points(k + 1);
        let values: Vec<i64> = points.iter().map(|&point| evaluate(&p, point)).collect();
        if values.iter().any(|&value| value == 0) {
            continue;
        }

        let divisor_sets: Vec<Vec<i64>> =
            values.iter().map(|&value| signed_divisors(value)).collect();
        if divisor_sets.iter().any(Vec::is_empty) {
            continue;
        }

        let mut combos_tried = 0usize;
        let mut combo = Vec::with_capacity(divisor_sets.len());
        if let Some(split) =
            search_combos(&p, &points, &divisor_sets, 0, &mut combo, &mut combos_tried)
        {
            return Some(split);
        }
    }

    None
}

fn search_combos(
    p: &[i64],
    points: &[i64],
    divisor_sets: &[Vec<i64>],
    index: usize,
    combo: &mut Vec<i64>,
    combos_tried: &mut usize,
) -> Option<(Poly, Poly)> {
    if *combos_tried >= MAX_COMBOS {
        return None;
    }

    if index == divisor_sets.len() {
        *combos_tried += 1;
        let coeffs = lagrange_interpolate(points, combo)?;
        let mut candidate = Vec::with_capacity(coeffs.len());
        for coeff in coeffs {
            candidate.push(coeff.to_i64()?);
        }

        let candidate = normalize_positive_leading(&candidate);
        if candidate.len() <= 1 || candidate.len() >= p.len() {
            return None;
        }

        let cofactor = divides_exactly(p, &candidate)?;
        return Some((candidate, normalize_positive_leading(&cofactor)));
    }

    for &value in &divisor_sets[index] {
        combo.push(value);
        if let Some(split) = search_combos(p, points, divisor_sets, index + 1, combo, combos_tried)
        {
            return Some(split);
        }
        combo.pop();
        if *combos_tried >= MAX_COMBOS {
            break;
        }
    }

    None
}

fn eval_points(count: usize) -> Vec<i64> {
    let mut points = Vec::with_capacity(count);
    let mut i = 0i64;
    while points.len() < count {
        if i == 0 {
            points.push(0);
        } else {
            points.push(i);
            if points.len() < count {
                points.push(-i);
            }
        }
        i += 1;
    }
    points
}

fn signed_divisors(value: i64) -> Vec<i64> {
    if value == 0 {
        return vec![];
    }

    divisors(value)
        .into_iter()
        .flat_map(|divisor| [divisor, -divisor])
        .collect()
}

fn lagrange_interpolate(xs: &[i64], ys: &[i64]) -> Option<Vec<Rational>> {
    let n = xs.len();
    let mut result = vec![Rational::zero(); n];

    for i in 0..n {
        let mut denom = Rational::one();
        for j in 0..n {
            if i == j {
                continue;
            }
            let diff = xs[i] - xs[j];
            if diff == 0 {
                return None;
            }
            denom = denom.mul(Rational::from_i64(diff));
        }

        let weight = Rational::from_i64(ys[i]).div(denom)?;
        let mut basis = vec![Rational::one()];
        for j in 0..n {
            if i == j {
                continue;
            }

            let mut next = vec![Rational::zero(); basis.len() + 1];
            for (k, coeff) in basis.iter().enumerate() {
                next[k + 1] = next[k + 1].add(*coeff);
                next[k] = next[k].sub(coeff.mul(Rational::from_i64(xs[j])));
            }
            basis = next;
        }

        for (k, coeff) in basis.iter().enumerate() {
            result[k] = result[k].add(weight.mul(*coeff));
        }
    }

    Some(result)
}

fn divides_exactly(p: &[i64], candidate: &[i64]) -> Option<Poly> {
    let dividend: Vec<Rational> = p.iter().map(|&coeff| Rational::from_i64(coeff)).collect();
    let divisor: Vec<Rational> = candidate
        .iter()
        .map(|&coeff| Rational::from_i64(coeff))
        .collect();
    let (quotient, remainder) = poly_divmod_frac(dividend, divisor)?;
    if !remainder.is_empty() {
        return None;
    }

    let mut out = Vec::with_capacity(quotient.len());
    for coeff in quotient {
        out.push(coeff.to_i64()?);
    }
    Some(normalize(&out))
}

fn poly_divmod_frac(
    mut dividend: Vec<Rational>,
    mut divisor: Vec<Rational>,
) -> Option<(Vec<Rational>, Vec<Rational>)> {
    trim_rational(&mut dividend);
    trim_rational(&mut divisor);
    if divisor.is_empty() {
        return None;
    }

    let divisor_degree = divisor.len() - 1;
    let mut quotient = vec![Rational::zero(); dividend.len().saturating_sub(divisor.len()) + 1];

    while dividend.len() > divisor_degree {
        let lead = dividend[dividend.len() - 1].div(divisor[divisor.len() - 1])?;
        let shift = dividend.len() - divisor.len();
        quotient[shift] = lead;
        for k in 0..divisor.len() {
            dividend[shift + k] = dividend[shift + k].sub(lead.mul(divisor[k]));
        }
        trim_rational(&mut dividend);
    }

    trim_rational(&mut quotient);
    Some((quotient, dividend))
}

fn trim_rational(values: &mut Vec<Rational>) {
    while values.last().is_some_and(Rational::is_zero) {
        values.pop();
    }
}

fn normalize_positive_leading(poly: &[i64]) -> Poly {
    let normalized = normalize(poly);
    if normalized.last().is_some_and(|&lead| lead < 0) {
        normalized.into_iter().map(|coeff| -coeff).collect()
    } else {
        normalized
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Rational {
    numer: i128,
    denom: i128,
}

impl Rational {
    fn new(numer: i128, denom: i128) -> Option<Self> {
        if denom == 0 {
            return None;
        }
        if numer == 0 {
            return Some(Self::zero());
        }

        let sign = if denom < 0 { -1 } else { 1 };
        let numer = numer * sign;
        let denom = denom.abs();
        let g = gcd_i128(numer.abs(), denom);
        Some(Self {
            numer: numer / g,
            denom: denom / g,
        })
    }

    fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    fn one() -> Self {
        Self { numer: 1, denom: 1 }
    }

    fn from_i64(value: i64) -> Self {
        Self {
            numer: value as i128,
            denom: 1,
        }
    }

    fn add(self, rhs: Self) -> Self {
        Self::new(
            self.numer * rhs.denom + rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
        .expect("non-zero denominator")
    }

    fn sub(self, rhs: Self) -> Self {
        Self::new(
            self.numer * rhs.denom - rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
        .expect("non-zero denominator")
    }

    fn mul(self, rhs: Self) -> Self {
        Self::new(self.numer * rhs.numer, self.denom * rhs.denom).expect("non-zero denominator")
    }

    fn div(self, rhs: Self) -> Option<Self> {
        Self::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }

    fn is_zero(&self) -> bool {
        self.numer == 0
    }

    fn to_i64(self) -> Option<i64> {
        if self.denom != 1 {
            return None;
        }
        i64::try_from(self.numer).ok()
    }
}

fn gcd_i128(mut a: i128, mut b: i128) -> i128 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signed_divisors_include_both_signs() {
        assert_eq!(signed_divisors(4), vec![1, -1, 2, -2, 4, -4]);
    }

    #[test]
    fn rational_reduces_negative_denominator() {
        assert_eq!(
            Rational::new(2, -4),
            Some(Rational {
                numer: -1,
                denom: 2
            })
        );
    }
}
