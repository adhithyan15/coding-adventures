//! Exact rational arithmetic used by the equation solvers.
//!
//! [`Frac`] is a reduced fraction `numer / denom` with `denom > 0`, stored
//! as `i64` pair.  Arithmetic uses `i128` intermediaries to avoid overflow
//! when multiplying two denominators or numerators together.
//!
//! This is a thin internal helper; the public API of `cas-solve` uses `Frac`
//! only in its function signatures.  Callers typically construct fractions
//! from integers via [`Frac::from_int`] or [`Frac::new`].

use std::ops::{Add, Div, Mul, Neg, Sub};

/// An exact rational number in reduced form.
///
/// Invariants:
/// - `denom > 0`
/// - `gcd(|numer|, denom) == 1`
/// - `0 / 1` is the canonical zero
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Frac {
    /// Numerator (may be negative).
    pub numer: i64,
    /// Denominator (always positive).
    pub denom: i64,
}

impl Frac {
    /// Construct a fraction from `numer` and `denom` (auto-reduced).
    ///
    /// # Panics
    ///
    /// Panics if `denom == 0`.
    pub fn new(numer: i64, denom: i64) -> Self {
        assert!(denom != 0, "Frac: denominator must not be zero");
        if numer == 0 {
            return Self { numer: 0, denom: 1 };
        }
        // Make denom positive.
        let (n, d) = if denom < 0 { (-numer, -denom) } else { (numer, denom) };
        let g = gcd(n.unsigned_abs() as u128, d.unsigned_abs() as u128) as i64;
        Self { numer: n / g, denom: d / g }
    }

    /// Construct from an integer: `n / 1`.
    pub fn from_int(n: i64) -> Self {
        Self { numer: n, denom: 1 }
    }

    /// Zero (0/1).
    pub fn zero() -> Self {
        Self { numer: 0, denom: 1 }
    }

    /// One (1/1).
    pub fn one() -> Self {
        Self { numer: 1, denom: 1 }
    }

    /// True iff this fraction is zero.
    pub fn is_zero(&self) -> bool {
        self.numer == 0
    }

    /// Convert to an `IRNode` — `Integer` when the denominator is 1, else `Rational`.
    pub fn to_irnode(self) -> symbolic_ir::IRNode {
        if self.denom == 1 {
            symbolic_ir::IRNode::Integer(self.numer)
        } else {
            symbolic_ir::IRNode::rational(self.numer, self.denom)
        }
    }

    /// Return the numerator and denominator as `i128` for safe arithmetic.
    fn wide(self) -> (i128, i128) {
        (self.numer as i128, self.denom as i128)
    }
}

impl Neg for Frac {
    type Output = Self;
    fn neg(self) -> Self {
        Self { numer: -self.numer, denom: self.denom }
    }
}

impl Add for Frac {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        let (an, ad) = self.wide();
        let (bn, bd) = rhs.wide();
        Frac::new_i128(an * bd + bn * ad, ad * bd)
    }
}

impl Sub for Frac {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl Mul for Frac {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        let (an, ad) = self.wide();
        let (bn, bd) = rhs.wide();
        Frac::new_i128(an * bn, ad * bd)
    }
}

impl Div for Frac {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        // a/b ÷ c/d = a*d / b*c
        let (an, ad) = self.wide();
        let (bn, bd) = rhs.wide();
        // Division by zero: panic (well-defined error for equation solving)
        assert!(bn != 0, "Frac: division by zero");
        Frac::new_i128(an * bd, ad * bn)
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

impl Frac {
    fn new_i128(numer: i128, denom: i128) -> Self {
        assert!(denom != 0, "Frac: zero denominator");
        if numer == 0 {
            return Self { numer: 0, denom: 1 };
        }
        let (n, d) = if denom < 0 { (-numer, -denom) } else { (numer, denom) };
        let g = gcd(n.unsigned_abs(), d.unsigned_abs()) as i128;
        Self {
            numer: (n / g) as i64,
            denom: (d / g) as i64,
        }
    }
}

fn gcd(a: u128, b: u128) -> u128 {
    if b == 0 { a } else { gcd(b, a % b) }
}

// ---------------------------------------------------------------------------
// From i64
// ---------------------------------------------------------------------------

impl From<i64> for Frac {
    fn from(n: i64) -> Self {
        Self::from_int(n)
    }
}
