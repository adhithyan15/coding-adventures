use std::ops::{Add, Div, Mul, Neg, Sub};

/// Exact rational number in reduced form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Rational {
    pub numer: i64,
    pub denom: i64,
}

impl Rational {
    pub const ZERO: Rational = Rational { numer: 0, denom: 1 };
    pub const ONE: Rational = Rational { numer: 1, denom: 1 };

    /// Construct a rational number and reduce it to canonical form.
    ///
    /// # Panics
    ///
    /// Panics when `denom == 0`.
    pub fn new(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "Rational denominator cannot be zero");
        if numer == 0 {
            return Self::ZERO;
        }
        let (n, d) = if denom < 0 {
            (-(numer as i128), -(denom as i128))
        } else {
            (numer as i128, denom as i128)
        };
        let g = gcd_u128(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Self {
            numer: (n / g) as i64,
            denom: (d / g) as i64,
        }
    }

    pub fn from_int(value: i64) -> Self {
        Self::new(value, 1)
    }

    pub fn zero() -> Self {
        Self::ZERO
    }

    pub fn one() -> Self {
        Self::ONE
    }

    pub fn is_zero(self) -> bool {
        self.numer == 0
    }

    pub fn is_negative(self) -> bool {
        self.numer < 0
    }

    pub fn pow_usize(self, exp: usize) -> Self {
        let mut out = Self::ONE;
        for _ in 0..exp {
            out = out * self;
        }
        out
    }

    fn new_i128(numer: i128, denom: i128) -> Self {
        assert!(denom != 0, "Rational denominator cannot be zero");
        if numer == 0 {
            return Self::ZERO;
        }
        let (n, d) = if denom < 0 {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let g = gcd_u128(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Self {
            numer: (n / g) as i64,
            denom: (d / g) as i64,
        }
    }
}

impl From<i64> for Rational {
    fn from(value: i64) -> Self {
        Self::from_int(value)
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

impl Add for Rational {
    type Output = Rational;

    fn add(self, rhs: Self) -> Self::Output {
        Rational::new_i128(
            self.numer as i128 * rhs.denom as i128 + rhs.numer as i128 * self.denom as i128,
            self.denom as i128 * rhs.denom as i128,
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
        Rational::new_i128(
            self.numer as i128 * rhs.numer as i128,
            self.denom as i128 * rhs.denom as i128,
        )
    }
}

impl Div for Rational {
    type Output = Rational;

    fn div(self, rhs: Self) -> Self::Output {
        assert!(!rhs.is_zero(), "Rational division by zero");
        Rational::new_i128(
            self.numer as i128 * rhs.denom as i128,
            self.denom as i128 * rhs.numer as i128,
        )
    }
}

pub(crate) fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

pub(crate) fn lcm_i64(a: i64, b: i64) -> i64 {
    if a == 0 || b == 0 {
        0
    } else {
        (a / gcd_i64(a, b)) * b
    }
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
