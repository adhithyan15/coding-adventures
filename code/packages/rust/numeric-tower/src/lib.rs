//! Numeric Tower
//!
//! A small shared `Number` type for the VisiCalc/R statistics substrate.
//! It provides the five rungs from `code/specs/numeric-tower.md` plus the
//! coercion and arithmetic entry points that domain crates can share.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};
use std::fmt;

pub type Integer = BigInt;
pub type Rational = BigRational;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Complex {
    pub re: f64,
    pub im: f64,
}

impl Complex {
    pub const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    pub const fn from_real(re: f64) -> Self {
        Self { re, im: 0.0 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decimal {
    units: BigInt,
    scale: u32,
}

impl Decimal {
    pub fn new(units: BigInt, scale: u32) -> Self {
        normalize_decimal(units, scale)
    }

    pub fn from_i64(value: i64) -> Self {
        Self::new(BigInt::from(value), 0)
    }

    pub fn units(&self) -> &BigInt {
        &self.units
    }

    pub fn scale(&self) -> u32 {
        self.scale
    }

    pub fn to_f64(&self) -> f64 {
        let units = big_int_to_f64(&self.units);
        units / 10_f64.powi(self.scale as i32)
    }

    fn add_decimal(&self, other: &Self) -> Self {
        let (left, right, scale) = align_decimals(self, other);
        Self::new(left + right, scale)
    }

    fn sub_decimal(&self, other: &Self) -> Self {
        let (left, right, scale) = align_decimals(self, other);
        Self::new(left - right, scale)
    }

    fn mul_decimal(&self, other: &Self) -> Self {
        Self::new(&self.units * &other.units, self.scale + other.scale)
    }

    fn div_decimal(&self, other: &Self, precision: u32) -> Option<Self> {
        if other.units.is_zero() {
            return None;
        }

        let numerator = &self.units * pow10(other.scale + precision);
        let denominator = &other.units * pow10(self.scale);
        Some(Self::new(numerator / denominator, precision))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rung {
    Integer,
    Rational,
    Float,
    Complex,
    Decimal,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    Integer(Integer),
    Rational(Rational),
    Float(f64),
    Complex(Complex),
    Decimal(Decimal),
}

impl Number {
    pub fn rung(&self) -> Rung {
        match self {
            Number::Integer(_) => Rung::Integer,
            Number::Rational(_) => Rung::Rational,
            Number::Float(_) => Rung::Float,
            Number::Complex(_) => Rung::Complex,
            Number::Decimal(_) => Rung::Decimal,
        }
    }

    pub fn to_f64_lossy(&self) -> f64 {
        match self {
            Number::Integer(value) => big_int_to_f64(value),
            Number::Rational(value) => rational_to_f64(value),
            Number::Float(value) => *value,
            Number::Complex(value) => {
                if value.im == 0.0 {
                    value.re
                } else {
                    f64::NAN
                }
            }
            Number::Decimal(value) => value.to_f64(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoercionError {
    NonFiniteFloat { target: Rung },
    FractionalFloat { target: Rung },
    ComplexToReal { target: Rung },
    NonIntegralRational { target: Rung },
    NonIntegralDecimal { target: Rung },
}

pub fn join_rung(left: Rung, right: Rung) -> Rung {
    use Rung::*;

    match (left, right) {
        (Complex, _) | (_, Complex) => Complex,
        (Float, _) | (_, Float) => Float,
        (Decimal, _) | (_, Decimal) => Decimal,
        (Rational, _) | (_, Rational) => Rational,
        (Integer, Integer) => Integer,
    }
}

pub fn coerce_to_join(left: &Number, right: &Number) -> (Number, Number) {
    let rung = join_rung(left.rung(), right.rung());
    (coerce_lossy(left, rung), coerce_lossy(right, rung))
}

pub fn try_coerce(number: &Number, target: Rung) -> Result<Number, CoercionError> {
    match (number, target) {
        (Number::Integer(value), Rung::Integer) => Ok(Number::Integer(value.clone())),
        (Number::Integer(value), Rung::Rational) => {
            Ok(Number::Rational(BigRational::from_integer(value.clone())))
        }
        (Number::Integer(value), Rung::Decimal) => {
            Ok(Number::Decimal(Decimal::new(value.clone(), 0)))
        }
        (Number::Integer(value), Rung::Float) => Ok(Number::Float(big_int_to_f64(value))),
        (Number::Integer(value), Rung::Complex) => Ok(Number::Complex(crate::Complex::from_real(
            big_int_to_f64(value),
        ))),

        (Number::Rational(value), Rung::Rational) => Ok(Number::Rational(value.clone())),
        (Number::Rational(value), Rung::Float) => Ok(Number::Float(rational_to_f64(value))),
        (Number::Rational(value), Rung::Complex) => Ok(Number::Complex(crate::Complex::from_real(
            rational_to_f64(value),
        ))),
        (Number::Rational(value), Rung::Decimal) => {
            Ok(Number::Decimal(decimal_from_rational(value, 12)))
        }
        (Number::Rational(value), Rung::Integer) => {
            if value.denom().is_one() {
                Ok(Number::Integer(value.numer().clone()))
            } else {
                Err(CoercionError::NonIntegralRational { target })
            }
        }

        (Number::Decimal(value), Rung::Decimal) => Ok(Number::Decimal(value.clone())),
        (Number::Decimal(value), Rung::Float) => Ok(Number::Float(value.to_f64())),
        (Number::Decimal(value), Rung::Complex) => {
            Ok(Number::Complex(crate::Complex::from_real(value.to_f64())))
        }
        (Number::Decimal(value), Rung::Rational) => {
            Ok(Number::Rational(decimal_to_rational(value)))
        }
        (Number::Decimal(value), Rung::Integer) => {
            if value.scale == 0 {
                Ok(Number::Integer(value.units.clone()))
            } else {
                Err(CoercionError::NonIntegralDecimal { target })
            }
        }

        (Number::Float(value), Rung::Float) => Ok(Number::Float(*value)),
        (Number::Float(value), Rung::Complex) => {
            Ok(Number::Complex(crate::Complex::from_real(*value)))
        }
        (Number::Float(value), Rung::Integer | Rung::Rational | Rung::Decimal)
            if !value.is_finite() =>
        {
            Err(CoercionError::NonFiniteFloat { target })
        }
        (Number::Float(value), Rung::Integer) => {
            if value.fract() == 0.0 {
                Ok(Number::Integer(BigInt::from(*value as i128)))
            } else {
                Err(CoercionError::FractionalFloat { target })
            }
        }
        (Number::Float(value), Rung::Rational) => BigRational::from_float(*value)
            .map(Number::Rational)
            .ok_or(CoercionError::NonFiniteFloat { target }),
        (Number::Float(value), Rung::Decimal) => Ok(Number::Decimal(decimal_from_f64(*value, 12))),

        (Number::Complex(value), Rung::Complex) => Ok(Number::Complex(*value)),
        (Number::Complex(_), Rung::Integer | Rung::Rational | Rung::Float | Rung::Decimal) => {
            Err(CoercionError::ComplexToReal { target })
        }
    }
}

pub fn add(left: &Number, right: &Number) -> Number {
    match coerce_to_join(left, right) {
        (Number::Integer(a), Number::Integer(b)) => Number::Integer(a + b),
        (Number::Rational(a), Number::Rational(b)) => Number::Rational(a + b),
        (Number::Float(a), Number::Float(b)) => Number::Float(a + b),
        (Number::Complex(a), Number::Complex(b)) => {
            Number::Complex(Complex::new(a.re + b.re, a.im + b.im))
        }
        (Number::Decimal(a), Number::Decimal(b)) => Number::Decimal(a.add_decimal(&b)),
        _ => unreachable!("coerce_to_join must return same-rung pairs"),
    }
}

pub fn sub(left: &Number, right: &Number) -> Number {
    match coerce_to_join(left, right) {
        (Number::Integer(a), Number::Integer(b)) => Number::Integer(a - b),
        (Number::Rational(a), Number::Rational(b)) => Number::Rational(a - b),
        (Number::Float(a), Number::Float(b)) => Number::Float(a - b),
        (Number::Complex(a), Number::Complex(b)) => {
            Number::Complex(Complex::new(a.re - b.re, a.im - b.im))
        }
        (Number::Decimal(a), Number::Decimal(b)) => Number::Decimal(a.sub_decimal(&b)),
        _ => unreachable!("coerce_to_join must return same-rung pairs"),
    }
}

pub fn mul(left: &Number, right: &Number) -> Number {
    match coerce_to_join(left, right) {
        (Number::Integer(a), Number::Integer(b)) => Number::Integer(a * b),
        (Number::Rational(a), Number::Rational(b)) => Number::Rational(a * b),
        (Number::Float(a), Number::Float(b)) => Number::Float(a * b),
        (Number::Complex(a), Number::Complex(b)) => Number::Complex(Complex::new(
            a.re * b.re - a.im * b.im,
            a.re * b.im + a.im * b.re,
        )),
        (Number::Decimal(a), Number::Decimal(b)) => Number::Decimal(a.mul_decimal(&b)),
        _ => unreachable!("coerce_to_join must return same-rung pairs"),
    }
}

pub fn div(left: &Number, right: &Number) -> Number {
    if let (Number::Integer(a), Number::Integer(b)) = (left, right) {
        if b.is_zero() {
            return Number::Float(f64::NAN);
        }
        let rem = a % b;
        if rem.is_zero() {
            return Number::Integer(a / b);
        }
        return Number::Rational(BigRational::new(a.clone(), b.clone()));
    }

    match coerce_to_join(left, right) {
        (Number::Rational(a), Number::Rational(b)) => {
            if b.is_zero() {
                Number::Float(f64::NAN)
            } else {
                Number::Rational(a / b)
            }
        }
        (Number::Float(a), Number::Float(b)) => Number::Float(a / b),
        (Number::Complex(a), Number::Complex(b)) => {
            let denominator = b.re * b.re + b.im * b.im;
            if denominator == 0.0 {
                Number::Complex(Complex::new(f64::NAN, f64::NAN))
            } else {
                Number::Complex(Complex::new(
                    (a.re * b.re + a.im * b.im) / denominator,
                    (a.im * b.re - a.re * b.im) / denominator,
                ))
            }
        }
        (Number::Decimal(a), Number::Decimal(b)) => match a.div_decimal(&b, 12) {
            Some(value) => Number::Decimal(value),
            None => Number::Float(f64::NAN),
        },
        (Number::Integer(_), Number::Integer(_)) => unreachable!("handled above"),
        _ => unreachable!("coerce_to_join must return same-rung pairs"),
    }
}

pub fn neg(number: &Number) -> Number {
    match number {
        Number::Integer(value) => Number::Integer(-value),
        Number::Rational(value) => Number::Rational(-value),
        Number::Float(value) => Number::Float(-value),
        Number::Complex(value) => Number::Complex(Complex::new(-value.re, -value.im)),
        Number::Decimal(value) => Number::Decimal(Decimal::new(-value.units.clone(), value.scale)),
    }
}

fn coerce_lossy(number: &Number, target: Rung) -> Number {
    try_coerce(number, target).unwrap_or_else(|_| match target {
        Rung::Integer => Number::Integer(BigInt::zero()),
        Rung::Rational => Number::Rational(BigRational::from_integer(BigInt::zero())),
        Rung::Float => Number::Float(number.to_f64_lossy()),
        Rung::Complex => Number::Complex(Complex::from_real(number.to_f64_lossy())),
        Rung::Decimal => Number::Decimal(decimal_from_f64(number.to_f64_lossy(), 12)),
    })
}

fn big_int_to_f64(value: &BigInt) -> f64 {
    value.to_f64().unwrap_or_else(|| {
        if value.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

fn rational_to_f64(value: &BigRational) -> f64 {
    value.to_f64().unwrap_or_else(|| {
        if value.numer().is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::INFINITY
        }
    })
}

fn decimal_from_f64(value: f64, scale: u32) -> Decimal {
    if !value.is_finite() {
        return Decimal::new(BigInt::zero(), 0);
    }
    let units = (value * 10_f64.powi(scale as i32)).round() as i128;
    Decimal::new(BigInt::from(units), scale)
}

fn decimal_from_rational(value: &BigRational, scale: u32) -> Decimal {
    let numerator = value.numer() * pow10(scale);
    Decimal::new(numerator / value.denom(), scale)
}

fn decimal_to_rational(value: &Decimal) -> BigRational {
    BigRational::new(value.units.clone(), pow10(value.scale))
}

fn align_decimals(left: &Decimal, right: &Decimal) -> (BigInt, BigInt, u32) {
    let scale = left.scale.max(right.scale);
    let left_units = &left.units * pow10(scale - left.scale);
    let right_units = &right.units * pow10(scale - right.scale);
    (left_units, right_units, scale)
}

fn normalize_decimal(units: BigInt, scale: u32) -> Decimal {
    if units.is_zero() {
        return Decimal { units, scale: 0 };
    }

    let ten = BigInt::from(10u8);
    let mut units = units;
    let mut scale = scale;

    while scale > 0 && (&units % &ten).is_zero() {
        units /= &ten;
        scale -= 1;
    }

    Decimal { units, scale }
}

fn pow10(power: u32) -> BigInt {
    let mut value = BigInt::one();
    for _ in 0..power {
        value *= 10u8;
    }
    value
}

impl fmt::Display for Decimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.scale == 0 {
            return write!(f, "{}", self.units);
        }

        let sign = if self.units.is_negative() { "-" } else { "" };
        let digits = self.units.abs().to_string();
        let scale = self.scale as usize;

        if digits.len() <= scale {
            write!(f, "{sign}0.{:0>width$}", digits, width = scale)
        } else {
            let split = digits.len() - scale;
            write!(f, "{sign}{}.{}", &digits[..split], &digits[split..])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_division_lifts_to_rational_when_needed() {
        let actual = div(&Number::Integer(1.into()), &Number::Integer(2.into()));
        match actual {
            Number::Rational(value) => assert_eq!(value, BigRational::new(1.into(), 2.into())),
            other => panic!("expected rational, got {other:?}"),
        }
    }

    #[test]
    fn decimal_integer_join_preserves_decimal_rung() {
        let left = Number::Decimal(Decimal::new(123.into(), 2));
        let right = Number::Integer(2.into());
        let (left, right) = coerce_to_join(&left, &right);

        assert_eq!(left.rung(), Rung::Decimal);
        assert_eq!(right.rung(), Rung::Decimal);
        assert_eq!(
            add(&left, &right),
            Number::Decimal(Decimal::new(323.into(), 2))
        );
    }

    #[test]
    fn complex_arithmetic_uses_complex_join() {
        let left = Number::Complex(Complex::new(1.0, 2.0));
        let right = Number::Float(3.0);
        assert_eq!(add(&left, &right), Number::Complex(Complex::new(4.0, 2.0)));
    }
}
