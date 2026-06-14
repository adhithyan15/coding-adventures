//! Angle conversion helpers.
//!
//! Excel `DEGREES(radians)` and `RADIANS(degrees)`. Trivial linear maps,
//! exposed as named functions to match the Excel symbol table.

use crate::constants::PI;
use r_vector::{is_na_real, na_real};

/// Excel `DEGREES(x)`. Convert radians to degrees: `x * 180 / pi`.
pub fn degrees(radians: f64) -> f64 {
    if is_na_real(radians) {
        return na_real();
    }
    radians * 180.0 / PI
}

/// Excel `RADIANS(x)`. Convert degrees to radians: `x * pi / 180`.
pub fn radians(degrees: f64) -> f64 {
    if is_na_real(degrees) {
        return na_real();
    }
    degrees * PI / 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-10, "expected {b}, got {a}");
    }

    #[test]
    fn degrees_radians_roundtrip() {
        for &d in &[0.0, 30.0, 45.0, 90.0, 180.0, 270.0, 360.0] {
            approx(degrees(radians(d)), d);
        }
    }

    #[test]
    fn known_conversions() {
        approx(radians(180.0), PI);
        approx(radians(360.0), 2.0 * PI);
        approx(degrees(PI), 180.0);
        approx(degrees(PI / 2.0), 90.0);
    }

    #[test]
    fn na_propagates() {
        assert!(is_na_real(degrees(na_real())));
        assert!(is_na_real(radians(na_real())));
    }
}
