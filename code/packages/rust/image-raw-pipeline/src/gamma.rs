// # gamma.rs — sRGB Transfer Functions
//
// The IEC 61966-2-1 standard (sRGB) defines a piecewise transfer function
// that maps linear-light values to display-encoded values and vice-versa.
//
// ## Why piecewise?
//
// A pure power law (L^(1/2.2)) would require infinite derivative at L=0,
// which amplifies noise in very dark values. The linear segment near black
// has finite slope (12.92) and avoids this problem. The crossover point
// is chosen so the two pieces join smoothly (same value and same
// first derivative at the transition).
//
// ## The constants (do not change these)
//
// - 0.0031308  — linear/power crossover in linear light
// - 0.04045    — the same point in display encoding (12.92 × 0.0031308)
// - 12.92      — slope of the linear segment
// - 1.055      — gain of the power segment
// - 0.055      — offset of the power segment  (continuity constraint)
// - 2.4        — inverse gamma (reciprocal of ~0.4167)
//
// The exponent 1/2.4 ≈ 0.4167 is slightly higher than the nominal sRGB
// "gamma 2.2" because the linear segment effectively lowers the overall
// perceptual exponent. In practice: γ_eff ≈ 2.2 when the linear segment
// is included.

/// Apply the sRGB EOTF: convert linear light `linear ∈ [0, 1]` to
/// display-encoded value `∈ [0, 1]`.
///
/// Fixed points: `srgb_gamma(0.0) == 0.0` and `srgb_gamma(1.0) == 1.0`.
/// Values outside [0, 1] are handled gracefully (not clamped here — the
/// caller decides whether to clamp before or after the gamma step).
///
/// # Formula (IEC 61966-2-1)
///
/// ```text
/// V = 12.92 × L                  if L ≤ 0.0031308
/// V = 1.055 × L^(1/2.4) − 0.055 if L > 0.0031308
/// ```
#[inline]
pub fn srgb_gamma(linear: f64) -> f64 {
    if linear <= 0.0031308 {
        12.92 * linear
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Invert the sRGB EOTF: convert display-encoded value `encoded ∈ [0, 1]`
/// back to linear light.
///
/// Round-trip: `srgb_decode(srgb_gamma(x)) ≈ x` for `x ∈ [0, 1]`
/// (within f64 floating-point rounding).
///
/// # Formula (IEC 61966-2-1)
///
/// ```text
/// L = V / 12.92                     if V ≤ 0.04045
/// L = ((V + 0.055) / 1.055)^2.4    if V > 0.04045
/// ```
#[inline]
pub fn srgb_decode(encoded: f64) -> f64 {
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── srgb_gamma ────────────────────────────────────────────────────────

    #[test]
    fn gamma_zero_is_zero() {
        assert!((srgb_gamma(0.0)).abs() < 1e-15);
    }

    #[test]
    fn gamma_one_is_one() {
        // Fixed point: both endpoints map to themselves.
        assert!((srgb_gamma(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn gamma_linear_segment_exact() {
        // Values at and below 0.0031308 use V = 12.92 × L.
        let x = 0.001;
        let expected = 12.92 * x;
        assert!((srgb_gamma(x) - expected).abs() < 1e-15);
    }

    #[test]
    fn gamma_at_crossover_point() {
        // At exactly 0.0031308 the linear formula applies.
        let x = 0.0031308;
        let expected = 12.92 * x;
        assert!((srgb_gamma(x) - expected).abs() < 1e-12);
    }

    #[test]
    fn gamma_above_crossover_uses_power_law() {
        // Just above 0.0031308 the power formula applies.
        let x = 0.004_f64;
        let expected = 1.055 * x.powf(1.0 / 2.4) - 0.055;
        assert!((srgb_gamma(x) - expected).abs() < 1e-12);
    }

    #[test]
    fn gamma_midpoint_is_approximately_0735() {
        // From reference sRGB tables: gamma(0.5) ≈ 0.7354.
        let v = srgb_gamma(0.5);
        assert!(v > 0.73 && v < 0.74, "gamma(0.5) expected ~0.735, got {}", v);
    }

    #[test]
    fn gamma_is_monotone_increasing() {
        // Sample 100 points and verify strict ordering.
        let mut prev = srgb_gamma(0.0);
        for i in 1..=100 {
            let x = i as f64 / 100.0;
            let v = srgb_gamma(x);
            assert!(v > prev, "gamma not monotone at x={}", x);
            prev = v;
        }
    }

    #[test]
    fn gamma_negative_input() {
        // Negative linear light: linear segment gives negative output (not clamped).
        let v = srgb_gamma(-0.01);
        assert!(v < 0.0);
    }

    // ── srgb_decode ───────────────────────────────────────────────────────

    #[test]
    fn decode_zero_is_zero() {
        assert!(srgb_decode(0.0).abs() < 1e-15);
    }

    #[test]
    fn decode_one_is_one() {
        assert!((srgb_decode(1.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn decode_linear_segment_exact() {
        // V ≤ 0.04045 → L = V / 12.92.
        let v = 0.02;
        let expected = v / 12.92;
        assert!((srgb_decode(v) - expected).abs() < 1e-15);
    }

    #[test]
    fn decode_at_crossover_point() {
        // At exactly 0.04045 the linear formula applies.
        let v = 0.04045;
        let expected = v / 12.92;
        assert!((srgb_decode(v) - expected).abs() < 1e-12);
    }

    #[test]
    fn decode_above_crossover_uses_power_law() {
        let v = 0.05;
        let expected = ((v + 0.055) / 1.055_f64).powf(2.4);
        assert!((srgb_decode(v) - expected).abs() < 1e-12);
    }

    // ── Round-trip ────────────────────────────────────────────────────────

    #[test]
    fn round_trip_gamma_then_decode() {
        // For all x in [0, 1]: decode(gamma(x)) ≈ x.
        for i in 0..=50 {
            let x = i as f64 / 50.0;
            let roundtrip = srgb_decode(srgb_gamma(x));
            assert!(
                (roundtrip - x).abs() < 1e-10,
                "round-trip failed at x={}: got {}",
                x, roundtrip
            );
        }
    }

    #[test]
    fn round_trip_decode_then_gamma() {
        // For all v in [0, 1]: gamma(decode(v)) ≈ v.
        for i in 0..=50 {
            let v = i as f64 / 50.0;
            let roundtrip = srgb_gamma(srgb_decode(v));
            assert!(
                (roundtrip - v).abs() < 1e-10,
                "round-trip failed at v={}: got {}",
                v, roundtrip
            );
        }
    }
}
