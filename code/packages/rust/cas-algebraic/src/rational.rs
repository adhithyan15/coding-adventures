use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Rational {
    pub numer: i64,
    pub denom: i64,
}

impl Rational {
    pub const ZERO: Rational = Rational { numer: 0, denom: 1 };
    pub const ONE: Rational = Rational { numer: 1, denom: 1 };

    pub fn new(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "Rational denominator cannot be zero");
        if numer == 0 {
            return Self::ZERO;
        }

        let (numer, denom) = if denom < 0 {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let g = gcd(numer.unsigned_abs(), denom as u64) as i64;
        Self {
            numer: numer / g,
            denom: denom / g,
        }
    }

    pub fn from_int(value: i64) -> Self {
        Self::new(value, 1)
    }

    pub fn is_zero(self) -> bool {
        self.numer == 0
    }

    pub fn is_one(self) -> bool {
        self.numer == 1 && self.denom == 1
    }

    pub fn is_negative(self) -> bool {
        self.numer < 0
    }
}

impl Add for Rational {
    type Output = Rational;

    fn add(self, rhs: Self) -> Self::Output {
        Rational::new(
            self.numer * rhs.denom + rhs.numer * self.denom,
            self.denom * rhs.denom,
        )
    }
}

impl Sub for Rational {
    type Output = Rational;

    fn sub(self, rhs: Self) -> Self::Output {
        self + (-rhs)
    }
}

impl Mul for Rational {
    type Output = Rational;

    fn mul(self, rhs: Self) -> Self::Output {
        Rational::new(self.numer * rhs.numer, self.denom * rhs.denom)
    }
}

impl Div for Rational {
    type Output = Rational;

    fn div(self, rhs: Self) -> Self::Output {
        assert!(!rhs.is_zero(), "Rational division by zero");
        Rational::new(self.numer * rhs.denom, self.denom * rhs.numer)
    }
}

impl Neg for Rational {
    type Output = Rational;

    fn neg(self) -> Self::Output {
        Rational {
            numer: -self.numer,
            denom: self.denom,
        }
    }
}

pub fn rational_square_root(value: Rational) -> Option<Rational> {
    if value.is_negative() {
        return None;
    }
    if value.is_zero() {
        return Some(Rational::ZERO);
    }

    let numer_root = integer_square_root(value.numer)?;
    let denom_root = integer_square_root(value.denom)?;
    Some(Rational::new(numer_root, denom_root))
}

fn integer_square_root(value: i64) -> Option<i64> {
    if value < 0 {
        return None;
    }
    let root = (value as f64).sqrt() as i64;
    for candidate in root.saturating_sub(1)..=root + 1 {
        if candidate >= 0 && candidate.saturating_mul(candidate) == value {
            return Some(candidate);
        }
    }
    None
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
