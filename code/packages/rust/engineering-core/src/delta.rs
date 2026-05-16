//! # DELTA / GESTEP — Kronecker delta and step function.

/// Excel `DELTA(number1, [number2])`. Returns 1 if the two numbers
/// are equal (under f64 equality, which treats `NaN != NaN`),
/// otherwise 0.
pub fn delta(a: f64, b: f64) -> u8 {
    if a == b {
        1
    } else {
        0
    }
}

/// Excel `GESTEP(number, [step])`. Returns 1 if `number >= step`,
/// otherwise 0. NaN comparisons return 0 (NaN >= anything is false).
pub fn gestep(value: f64, step: f64) -> u8 {
    if value >= step {
        1
    } else {
        0
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_equal_inputs() {
        assert_eq!(delta(5.0, 5.0), 1);
        assert_eq!(delta(0.0, 0.0), 1);
        assert_eq!(delta(-3.5, -3.5), 1);
    }

    #[test]
    fn delta_unequal_inputs() {
        assert_eq!(delta(5.0, 5.000001), 0);
        assert_eq!(delta(1.0, 2.0), 0);
    }

    #[test]
    fn delta_with_nan() {
        // NaN != NaN under IEEE 754; delta returns 0.
        assert_eq!(delta(f64::NAN, f64::NAN), 0);
    }

    #[test]
    fn gestep_inclusive_threshold() {
        assert_eq!(gestep(5.0, 5.0), 1);
        assert_eq!(gestep(5.5, 5.0), 1);
        assert_eq!(gestep(4.9, 5.0), 0);
        assert_eq!(gestep(-1.0, 0.0), 0);
    }

    #[test]
    fn gestep_default_step_is_zero() {
        assert_eq!(gestep(0.0, 0.0), 1);
        assert_eq!(gestep(-0.0001, 0.0), 0);
    }

    #[test]
    fn gestep_with_nan() {
        assert_eq!(gestep(f64::NAN, 0.0), 0);
    }
}
