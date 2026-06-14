use std::collections::BTreeSet;

use crate::monomial::{divides, MonomialOrder};
use crate::{reduce_poly, s_poly, MPoly, PolynomialError, Rational};

const MAX_BASIS_SIZE: usize = 50;
const MAX_DEGREE: usize = 8;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrobnerError {
    Polynomial(PolynomialError),
    BasisTooLarge { max: usize },
    DegreeTooLarge { degree: usize, max: usize },
}

impl std::fmt::Display for GrobnerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Polynomial(err) => write!(f, "{err}"),
            Self::BasisTooLarge { max } => {
                write!(f, "Groebner basis grew beyond {max} elements")
            }
            Self::DegreeTooLarge { degree, max } => {
                write!(
                    f,
                    "polynomial total degree {degree} exceeds safety cap {max}"
                )
            }
        }
    }
}

impl std::error::Error for GrobnerError {}

impl From<PolynomialError> for GrobnerError {
    fn from(value: PolynomialError) -> Self {
        Self::Polynomial(value)
    }
}

pub fn buchberger(polys: &[MPoly], order: &str) -> Result<Vec<MPoly>, GrobnerError> {
    let order = MonomialOrder::parse(order).map_err(PolynomialError::from)?;
    let mut basis: Vec<MPoly> = polys.iter().filter(|p| !p.is_zero()).cloned().collect();
    if basis.is_empty() {
        return Ok(vec![]);
    }

    for p in &basis {
        if p.total_degree() > MAX_DEGREE {
            return Err(GrobnerError::DegreeTooLarge {
                degree: p.total_degree(),
                max: MAX_DEGREE,
            });
        }
    }

    let mut pairs: BTreeSet<(usize, usize)> = (0..basis.len())
        .flat_map(|i| (i + 1..basis.len()).map(move |j| (i, j)))
        .collect();

    while let Some(pair) = pairs.pop_first() {
        let sp = s_poly(&basis[pair.0], &basis[pair.1], order_name(order))?;
        let r = reduce_poly(&sp, &basis, order_name(order))?;
        if r.is_zero() {
            continue;
        }

        if basis.len() >= MAX_BASIS_SIZE {
            return Err(GrobnerError::BasisTooLarge {
                max: MAX_BASIS_SIZE,
            });
        }
        if r.total_degree() > MAX_DEGREE {
            return Err(GrobnerError::DegreeTooLarge {
                degree: r.total_degree(),
                max: MAX_DEGREE,
            });
        }

        let new_idx = basis.len();
        pairs.extend((0..new_idx).map(|i| (i, new_idx)));
        basis.push(r);
    }

    inter_reduce(&basis, order)
}

fn make_monic(p: &MPoly, order: MonomialOrder) -> Result<MPoly, PolynomialError> {
    if p.is_zero() {
        return Ok(p.clone());
    }
    Ok(p.scale(Rational::ONE / p.leading_coefficient(order)?))
}

fn inter_reduce(basis: &[MPoly], order: MonomialOrder) -> Result<Vec<MPoly>, GrobnerError> {
    if basis.is_empty() {
        return Ok(vec![]);
    }

    let mut minimal = Vec::new();
    for (i, g) in basis.iter().enumerate() {
        if g.is_zero() {
            continue;
        }
        let lm_g = g.leading_monomial(order)?;
        let mut dominated = false;
        for (j, h) in basis.iter().enumerate() {
            if i == j || h.is_zero() {
                continue;
            }
            let lm_h = h.leading_monomial(order)?;
            if divides(&lm_h, &lm_g) && (lm_h != lm_g || j < i) {
                dominated = true;
                break;
            }
        }
        if !dominated {
            minimal.push(make_monic(g, order)?);
        }
    }

    let mut reduced = Vec::new();
    for (i, g) in minimal.iter().enumerate() {
        let others: Vec<MPoly> = minimal
            .iter()
            .enumerate()
            .filter_map(|(j, h)| if i == j { None } else { Some(h.clone()) })
            .collect();
        let r = reduce_poly(g, &others, order_name(order))?;
        if !r.is_zero() {
            reduced.push(make_monic(&r, order)?);
        }
    }

    Ok(reduced)
}

fn order_name(order: MonomialOrder) -> &'static str {
    match order {
        MonomialOrder::Lex => "lex",
        MonomialOrder::GrLex => "grlex",
        MonomialOrder::GRevLex => "grevlex",
    }
}
