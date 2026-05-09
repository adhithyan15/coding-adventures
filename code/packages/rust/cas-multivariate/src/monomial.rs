use std::cmp::Ordering;

/// Exponent vector for a monomial.
pub type Monomial = Vec<usize>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonomialOrder {
    Lex,
    GrLex,
    GRevLex,
}

impl MonomialOrder {
    pub fn parse(order: &str) -> Result<Self, MonomialOrderError> {
        match order {
            "lex" => Ok(Self::Lex),
            "grlex" => Ok(Self::GrLex),
            "grevlex" => Ok(Self::GRevLex),
            _ => Err(MonomialOrderError {
                order: order.to_string(),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonomialOrderError {
    pub order: String,
}

impl std::fmt::Display for MonomialOrderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown monomial order {:?}; use lex, grlex, or grevlex",
            self.order
        )
    }
}

impl std::error::Error for MonomialOrderError {}

pub fn cmp_monomials(a: &[usize], b: &[usize], order: MonomialOrder) -> Ordering {
    match order {
        MonomialOrder::Lex => a.cmp(b),
        MonomialOrder::GrLex => total_degree(a).cmp(&total_degree(b)).then_with(|| a.cmp(b)),
        MonomialOrder::GRevLex => total_degree(a)
            .cmp(&total_degree(b))
            .then_with(|| cmp_grevlex_tie(a, b)),
    }
}

fn cmp_grevlex_tie(a: &[usize], b: &[usize]) -> Ordering {
    for (&ai, &bi) in a.iter().rev().zip(b.iter().rev()) {
        match bi.cmp(&ai) {
            Ordering::Equal => {}
            ord => return ord,
        }
    }
    Ordering::Equal
}

pub fn lcm_monomial(a: &[usize], b: &[usize]) -> Monomial {
    assert_eq!(a.len(), b.len(), "monomial variable count mismatch");
    a.iter().zip(b).map(|(&ai, &bi)| ai.max(bi)).collect()
}

pub fn divides(a: &[usize], b: &[usize]) -> bool {
    assert_eq!(a.len(), b.len(), "monomial variable count mismatch");
    a.iter().zip(b).all(|(&ai, &bi)| ai <= bi)
}

pub fn div_monomial(b: &[usize], a: &[usize]) -> Monomial {
    assert_eq!(a.len(), b.len(), "monomial variable count mismatch");
    b.iter()
        .zip(a)
        .map(|(&bi, &ai)| {
            assert!(bi >= ai, "monomial divisor does not divide dividend");
            bi - ai
        })
        .collect()
}

pub fn total_degree(m: &[usize]) -> usize {
    m.iter().sum()
}
