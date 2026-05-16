//! Named mathematical constants.
//!
//! Excel spells these as zero-arity functions: `PI()`, `EXP(1)` for e, etc.
//! R exposes them as bare names: `pi`. We provide both: the constants here as
//! `pub const`, and matching zero-arg functions in the Excel style.
//!
//! Values come from `std::f64::consts` so the bit-patterns match every other
//! Rust crate in the workspace.

/// Ratio of a circle's circumference to its diameter. Excel: `PI()`.
pub const PI: f64 = std::f64::consts::PI;

/// Euler's number, base of the natural logarithm. Excel: `EXP(1)`.
pub const E: f64 = std::f64::consts::E;

/// Square root of 2.
pub const SQRT2: f64 = std::f64::consts::SQRT_2;

/// Natural log of 2.
pub const LN2: f64 = std::f64::consts::LN_2;

/// Natural log of 10.
pub const LN10: f64 = std::f64::consts::LN_10;

/// Base-10 logarithm of e (i.e. `1 / ln(10)`).
pub const LOG10E: f64 = std::f64::consts::LOG10_E;

/// Excel `PI()`. Returns the constant `PI` as `f64`.
pub fn pi() -> f64 {
    PI
}

/// R `exp(1)` and Excel `EXP(1)`. Returns Euler's number.
pub fn e() -> f64 {
    E
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_match_std_consts() {
        assert_eq!(PI, std::f64::consts::PI);
        assert_eq!(E, std::f64::consts::E);
        assert_eq!(SQRT2, std::f64::consts::SQRT_2);
        assert_eq!(LN2, std::f64::consts::LN_2);
        assert_eq!(LN10, std::f64::consts::LN_10);
        assert_eq!(LOG10E, std::f64::consts::LOG10_E);
    }

    #[test]
    fn pi_function_returns_pi() {
        assert_eq!(pi(), PI);
        assert_eq!(e(), E);
    }
}
