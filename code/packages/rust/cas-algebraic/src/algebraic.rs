use cas_factor::factor_integer_polynomial;

use crate::rational::{rational_square_root, Rational};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AlgCoeff {
    pub rational: Rational,
    pub radical: Rational,
}

pub type AlgPoly = Vec<AlgCoeff>;

impl AlgCoeff {
    pub fn new(rational: Rational, radical: Rational) -> Self {
        Self { rational, radical }
    }

    pub fn rational(value: i64) -> Self {
        Self::new(Rational::from_int(value), Rational::ZERO)
    }
}

pub fn factor_over_extension(coeffs: &[i64], d: i64) -> Option<Vec<AlgPoly>> {
    if coeffs.len() <= 2 || d <= 0 {
        return None;
    }

    let (_content, factors_z) = factor_integer_polynomial(coeffs);
    let mut result = Vec::new();
    let mut any_split = false;

    for (factor, multiplicity) in factors_z {
        if let Some(split) = try_split_single(&factor, d) {
            for _ in 0..multiplicity {
                result.extend(split.clone());
            }
            any_split = true;
        } else {
            let alg_factor = integer_poly_to_alg_poly(&factor);
            for _ in 0..multiplicity {
                result.push(alg_factor.clone());
            }
        }
    }

    any_split.then_some(result)
}

pub fn try_split_quadratic(coeffs: &[i64], d: i64) -> Option<Vec<AlgPoly>> {
    if coeffs.len() != 3 || coeffs[2] != 1 || d <= 0 {
        return None;
    }

    let c = Rational::from_int(coeffs[0]);
    let b = Rational::from_int(coeffs[1]);
    let disc = b * b - Rational::from_int(4) * c;
    if disc.is_zero() {
        return None;
    }

    let two_beta = rational_square_root(disc / Rational::from_int(d))?;
    if two_beta.is_zero() {
        return None;
    }
    let beta = two_beta / Rational::from_int(2);
    let root_rational = -b / Rational::from_int(2);

    let linear = |radical: Rational| {
        vec![
            AlgCoeff::new(-root_rational, radical),
            AlgCoeff::rational(1),
        ]
    };

    Some(vec![linear(-beta), linear(beta)])
}

pub fn try_split_depressed_quartic(coeffs: &[i64], d: i64) -> Option<Vec<AlgPoly>> {
    if coeffs.len() != 5 || coeffs[4] != 1 || coeffs[3] != 0 || coeffs[1] != 0 || d <= 0 {
        return None;
    }

    let q = Rational::from_int(coeffs[0]);
    let p = Rational::from_int(coeffs[2]);
    let q_root = rational_square_root(q)?;

    for s in [q_root, -q_root] {
        let numerator = Rational::from_int(2) * s - p;
        if numerator.is_negative() {
            continue;
        }
        let Some(r) = rational_square_root(numerator / Rational::from_int(d)) else {
            continue;
        };
        if r.is_zero() {
            continue;
        }

        let h1 = vec![
            AlgCoeff::new(s, Rational::ZERO),
            AlgCoeff::new(Rational::ZERO, r),
            AlgCoeff::rational(1),
        ];
        let h2 = vec![
            AlgCoeff::new(s, Rational::ZERO),
            AlgCoeff::new(Rational::ZERO, -r),
            AlgCoeff::rational(1),
        ];
        return Some(vec![h1, h2]);
    }

    None
}

pub fn try_split_single(coeffs: &[i64], d: i64) -> Option<Vec<AlgPoly>> {
    if coeffs.len() < 3 {
        return None;
    }

    let normalized = if coeffs.last() == Some(&-1) {
        coeffs.iter().map(|value| -*value).collect::<Vec<_>>()
    } else {
        coeffs.to_vec()
    };

    match normalized.len() - 1 {
        2 => try_split_quadratic(&normalized, d),
        4 => try_split_depressed_quartic(&normalized, d),
        _ => None,
    }
}

fn integer_poly_to_alg_poly(coeffs: &[i64]) -> AlgPoly {
    coeffs.iter().copied().map(AlgCoeff::rational).collect()
}
