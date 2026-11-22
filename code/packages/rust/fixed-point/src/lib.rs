//! Deterministic signed Q16.16 arithmetic over the complete `i32` raw domain.
//!
//! A raw value denotes the exact rational `raw / 65_536`; no operation passes
//! through floating point. `i128` intermediates contain every `i32` product
//! and scaled division numerator before the rounded result is range checked.

const SCALE: i128 = 65_536;

pub mod capabilities;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Q16 {
    pub raw: i32,
}

impl Q16 {
    pub const fn from_raw(raw: i32) -> Self {
        Self { raw }
    }
    pub const fn raw(self) -> i32 {
        self.raw
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingMode {
    TowardZero,
    Floor,
    Ceil,
    NearestEven,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Q16Error {
    Overflow,
    DivisionByZero,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaturatingResult {
    pub value: Q16,
    pub overflowed: bool,
}

fn rounded_ratio(numerator: i128, denominator: i128, mode: RoundingMode) -> i128 {
    debug_assert!(denominator > 0);
    // Signed division truncates toward zero and leaves the remainder with the
    // numerator's sign. Floor and ceil adjust that quotient only when needed.
    let q = numerator / denominator;
    let r = numerator % denominator;
    match mode {
        RoundingMode::TowardZero => q,
        RoundingMode::Floor if r < 0 => q - 1,
        RoundingMode::Floor => q,
        RoundingMode::Ceil if r > 0 => q + 1,
        RoundingMode::Ceil => q,
        RoundingMode::NearestEven => match (2 * r.abs()).cmp(&denominator) {
            // Comparing distances via doubled remainder keeps exact ties
            // visible; odd quotients move one step to the adjacent even value.
            std::cmp::Ordering::Less => q,
            std::cmp::Ordering::Greater => q + numerator.signum(),
            std::cmp::Ordering::Equal if q % 2 != 0 => q + numerator.signum(),
            std::cmp::Ordering::Equal => q,
        },
    }
}

fn checked_raw(raw: i128) -> Result<Q16, Q16Error> {
    i32::try_from(raw)
        .map(Q16::from_raw)
        .map_err(|_| Q16Error::Overflow)
}

fn saturated_raw(raw: i128) -> SaturatingResult {
    // Set the flag from the rounded exact value, before clamping it. In-range
    // endpoint results therefore retain `overflowed = false`.
    let overflowed = raw < i128::from(i32::MIN) || raw > i128::from(i32::MAX);
    let bounded = raw.clamp(i128::from(i32::MIN), i128::from(i32::MAX)) as i32;
    SaturatingResult {
        value: Q16::from_raw(bounded),
        overflowed,
    }
}

pub fn from_integer(value: i32) -> Result<Q16, Q16Error> {
    checked_raw(i128::from(value) * SCALE)
}

pub fn checked_add(a: Q16, b: Q16) -> Result<Q16, Q16Error> {
    checked_raw(i128::from(a.raw) + i128::from(b.raw))
}

pub fn checked_sub(a: Q16, b: Q16) -> Result<Q16, Q16Error> {
    checked_raw(i128::from(a.raw) - i128::from(b.raw))
}

pub fn saturating_add(a: Q16, b: Q16) -> SaturatingResult {
    saturated_raw(i128::from(a.raw) + i128::from(b.raw))
}

pub fn saturating_sub(a: Q16, b: Q16) -> SaturatingResult {
    saturated_raw(i128::from(a.raw) - i128::from(b.raw))
}

pub fn checked_mul(a: Q16, b: Q16, mode: RoundingMode) -> Result<Q16, Q16Error> {
    checked_raw(rounded_ratio(
        i128::from(a.raw) * i128::from(b.raw),
        SCALE,
        mode,
    ))
}

pub fn saturating_mul(a: Q16, b: Q16, mode: RoundingMode) -> SaturatingResult {
    saturated_raw(rounded_ratio(
        i128::from(a.raw) * i128::from(b.raw),
        SCALE,
        mode,
    ))
}

pub fn checked_div(a: Q16, b: Q16, mode: RoundingMode) -> Result<Q16, Q16Error> {
    // Division by zero wins before rounding and range checks for every mode.
    if b.raw == 0 {
        return Err(Q16Error::DivisionByZero);
    }
    let mut numerator = i128::from(a.raw) * SCALE;
    let mut denominator = i128::from(b.raw);
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    checked_raw(rounded_ratio(numerator, denominator, mode))
}

pub fn saturating_div(a: Q16, b: Q16, mode: RoundingMode) -> Result<SaturatingResult, Q16Error> {
    // Saturation applies after rounding a nonzero quotient; it never converts
    // division by zero into a clamped value.
    if b.raw == 0 {
        return Err(Q16Error::DivisionByZero);
    }
    let mut numerator = i128::from(a.raw) * SCALE;
    let mut denominator = i128::from(b.raw);
    if denominator < 0 {
        numerator = -numerator;
        denominator = -denominator;
    }
    Ok(saturated_raw(rounded_ratio(numerator, denominator, mode)))
}

pub fn to_integer(value: Q16, mode: RoundingMode) -> i32 {
    // The rounded quotient is in [-32_768, 32_768], so this cast is safe.
    rounded_ratio(i128::from(value.raw), SCALE, mode) as i32
}
