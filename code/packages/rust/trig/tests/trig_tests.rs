// ============================================================================
// Tests for the `trig` crate
// ============================================================================
//
// Every function is exercised against known mathematical identities.  We use
// a small helper, `approx_equal`, because floating-point arithmetic cannot
// represent most real numbers exactly — we need a tolerance.
//

use trig::*;

// ---------------------------------------------------------------------------
// Helper: approximate equality
// ---------------------------------------------------------------------------

/// Returns `true` if `a` and `b` differ by less than `1e-10`.
///
/// Why 1e-10?  Our 20-term Maclaurin series is accurate to well beyond
/// double-precision for inputs in [-pi, pi].  A tolerance of 1e-10 is
/// generous — the actual error is typically on the order of 1e-15.
fn approx_equal(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-10
}

// ---------------------------------------------------------------------------
// sin tests
// ---------------------------------------------------------------------------

#[test]
fn sin_of_zero_is_zero() {
    assert!(approx_equal(sin(0.0), 0.0));
}

#[test]
fn sin_of_pi_over_2_is_one() {
    // sin(pi/2) = 1  — the peak of the sine wave.
    assert!(approx_equal(sin(PI / 2.0), 1.0));
}

#[test]
fn sin_of_pi_is_zero() {
    // sin(pi) = 0  — the sine wave crosses zero here.
    assert!(approx_equal(sin(PI), 0.0));
}

#[test]
fn sin_of_3pi_over_2() {
    // sin(3*pi/2) = -1  — the trough of the sine wave.
    assert!(approx_equal(sin(3.0 * PI / 2.0), -1.0));
}

// ---------------------------------------------------------------------------
// cos tests
// ---------------------------------------------------------------------------

#[test]
fn cos_of_zero_is_one() {
    assert!(approx_equal(cos(0.0), 1.0));
}

#[test]
fn cos_of_pi_over_2_is_zero() {
    // cos(pi/2) = 0  — cosine crosses zero at a quarter-turn.
    assert!(approx_equal(cos(PI / 2.0), 0.0));
}

#[test]
fn cos_of_pi_is_negative_one() {
    // cos(pi) = -1  — halfway around the circle, cosine is at its minimum.
    assert!(approx_equal(cos(PI), -1.0));
}

// ---------------------------------------------------------------------------
// Odd / Even symmetry
// ---------------------------------------------------------------------------
//
// Sine is an *odd* function:   sin(-x) = -sin(x)
// Cosine is an *even* function: cos(-x) =  cos(x)
//

#[test]
fn sin_is_odd() {
    let values = [0.5, 1.0, 2.0, PI / 4.0, PI / 3.0];
    for &x in &values {
        assert!(
            approx_equal(sin(-x), -sin(x)),
            "sin(-{}) should equal -sin({})",
            x,
            x
        );
    }
}

#[test]
fn cos_is_even() {
    let values = [0.5, 1.0, 2.0, PI / 4.0, PI / 3.0];
    for &x in &values {
        assert!(
            approx_equal(cos(-x), cos(x)),
            "cos(-{}) should equal cos({})",
            x,
            x
        );
    }
}

// ---------------------------------------------------------------------------
// Pythagorean identity: sin²(x) + cos²(x) = 1
// ---------------------------------------------------------------------------
//
// This is arguably the most important identity in trigonometry.  It comes
// from the unit circle: if (cos θ, sin θ) is a point on the circle x²+y²=1,
// then cos²θ + sin²θ must equal 1.
//

#[test]
fn pythagorean_identity() {
    let values = [0.0, 0.5, 1.0, PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0, PI, 2.5, 5.0];
    for &x in &values {
        let s = sin(x);
        let c = cos(x);
        assert!(
            approx_equal(s * s + c * c, 1.0),
            "sin²({}) + cos²({}) should equal 1, got {}",
            x,
            x,
            s * s + c * c
        );
    }
}

// ---------------------------------------------------------------------------
// Large inputs — stress-test range reduction
// ---------------------------------------------------------------------------

#[test]
fn sin_of_large_multiple_of_pi() {
    // sin(1000*pi) = sin(0) = 0  (since 1000 is even, 1000*pi is an
    // integer multiple of the period).
    assert!(approx_equal(sin(1000.0 * PI), 0.0));
}

#[test]
fn cos_of_large_multiple_of_2pi() {
    // cos(500 * 2*pi) = cos(0) = 1
    assert!(approx_equal(cos(500.0 * 2.0 * PI), 1.0));
}

// ---------------------------------------------------------------------------
// Angle conversion
// ---------------------------------------------------------------------------

#[test]
fn radians_180_is_pi() {
    assert!(approx_equal(radians(180.0), PI));
}

#[test]
fn radians_90_is_pi_over_2() {
    assert!(approx_equal(radians(90.0), PI / 2.0));
}

#[test]
fn radians_360_is_two_pi() {
    assert!(approx_equal(radians(360.0), 2.0 * PI));
}

#[test]
fn degrees_pi_is_180() {
    assert!(approx_equal(degrees(PI), 180.0));
}

#[test]
fn degrees_pi_over_2_is_90() {
    assert!(approx_equal(degrees(PI / 2.0), 90.0));
}

#[test]
fn degrees_two_pi_is_360() {
    assert!(approx_equal(degrees(2.0 * PI), 360.0));
}

// ---------------------------------------------------------------------------
// Round-trip conversion: degrees -> radians -> degrees
// ---------------------------------------------------------------------------

#[test]
fn round_trip_angle_conversion() {
    let angles = [0.0, 45.0, 90.0, 180.0, 270.0, 360.0];
    for &deg in &angles {
        assert!(
            approx_equal(degrees(radians(deg)), deg),
            "round-trip failed for {} degrees",
            deg
        );
    }
}
