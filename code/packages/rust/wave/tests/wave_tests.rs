// ============================================================================
// wave — Integration tests
// ============================================================================
//
// These tests verify the Wave struct against known physical and mathematical
// properties of sinusoidal waves.  We use an approximate equality helper
// because floating-point arithmetic introduces tiny rounding errors.
//
// The tolerance of 1e-10 is generous — our trig library (20-term Maclaurin
// series) is accurate to about 15 significant digits for inputs in [-pi, pi].
// ============================================================================

use wave::Wave;
use trig::PI;

// ============================================================================
// Helper: approximate floating-point equality
// ============================================================================
//
// Floating-point numbers cannot represent most decimal fractions exactly.
// For example, 0.1 in f64 is actually 0.1000000000000000055511151231257827...
//
// When we chain arithmetic operations (multiply, add, sine), these tiny
// errors accumulate.  So instead of asking "are these equal?" we ask
// "are these close enough?"
//
// A tolerance of 1e-10 means we accept answers that differ by less than
// 0.0000000001 — far more precise than any real-world measurement.

/// Check that two f64 values are within `tolerance` of each other.
fn approx_equal(a: f64, b: f64, tolerance: f64) -> bool {
    (a - b).abs() < tolerance
}

/// Tolerance for all comparisons in this test suite.
const TOL: f64 = 1e-10;

// ============================================================================
// Basic evaluation tests
// ============================================================================

#[test]
fn wave_at_t_zero_with_zero_phase_gives_zero() {
    // y(0) = A * sin(2*pi*f*0 + 0) = A * sin(0) = 0
    //
    // Regardless of amplitude or frequency, a wave with zero phase
    // always starts at zero.  This is because sin(0) = 0.
    let w = Wave::new(5.0, 100.0, 0.0).unwrap();
    assert!(
        approx_equal(w.evaluate(0.0), 0.0, TOL),
        "Wave with zero phase should be 0 at t=0, got {}",
        w.evaluate(0.0)
    );
}

#[test]
fn one_hz_wave_reaches_amplitude_at_quarter_period() {
    // A 1 Hz wave has period T = 1 second.
    // At t = T/4 = 0.25s, the argument to sine is:
    //   2*pi*1*0.25 + 0 = pi/2
    // sin(pi/2) = 1, so y(0.25) = amplitude * 1 = amplitude.
    //
    // This is the wave's peak — the highest point in its cycle.
    let w = Wave::new(1.0, 1.0, 0.0).unwrap();
    assert!(
        approx_equal(w.evaluate(0.25), 1.0, TOL),
        "1 Hz unit wave should reach 1.0 at t=0.25, got {}",
        w.evaluate(0.25)
    );
}

#[test]
fn wave_is_periodic() {
    // A fundamental property of waves: the value repeats after each period.
    //
    //   y(t) = y(t + T) = y(t + 2T) = ...
    //
    // We test at an arbitrary time t=0.13 (not a special value) and verify
    // that adding one period (T = 1/f = 0.5s) gives the same result.
    let w = Wave::new(3.0, 2.0, 0.7).unwrap();
    let t = 0.13;
    let period = w.period(); // 0.5 seconds

    let y_at_t = w.evaluate(t);
    let y_at_t_plus_period = w.evaluate(t + period);
    let y_at_t_plus_two_periods = w.evaluate(t + 2.0 * period);

    assert!(
        approx_equal(y_at_t, y_at_t_plus_period, TOL),
        "y({}) = {} but y({}) = {} — should be equal (periodicity)",
        t,
        y_at_t,
        t + period,
        y_at_t_plus_period
    );
    assert!(
        approx_equal(y_at_t, y_at_t_plus_two_periods, TOL),
        "y({}) should equal y({}) (two periods later)",
        t,
        t + 2.0 * period
    );
}

#[test]
fn phase_pi_over_2_starts_at_peak() {
    // If phase = PI/2, then at t=0:
    //   y(0) = A * sin(0 + PI/2) = A * sin(PI/2) = A * 1 = A
    //
    // A phase shift of PI/2 turns a sine wave into a cosine wave:
    //   sin(x + PI/2) = cos(x)
    //
    // This is why cosine is sometimes called "sine with a head start."
    let w = Wave::new(2.5, 10.0, PI / 2.0).unwrap();
    assert!(
        approx_equal(w.evaluate(0.0), 2.5, TOL),
        "Wave with phase PI/2 should start at amplitude, got {}",
        w.evaluate(0.0)
    );
}

// ============================================================================
// Derived quantity tests
// ============================================================================

#[test]
fn period_is_inverse_of_frequency() {
    // T = 1 / f
    //
    // A 4 Hz wave completes 4 cycles per second, so each cycle takes 0.25s.
    let w = Wave::new(1.0, 4.0, 0.0).unwrap();
    assert!(
        approx_equal(w.period(), 0.25, TOL),
        "Period of 4 Hz wave should be 0.25, got {}",
        w.period()
    );
}

#[test]
fn angular_frequency_is_two_pi_times_frequency() {
    // omega = 2 * pi * f
    //
    // For f = 3 Hz: omega = 6*pi ≈ 18.8496...
    let w = Wave::new(1.0, 3.0, 0.0).unwrap();
    let expected = 2.0 * PI * 3.0;
    assert!(
        approx_equal(w.angular_frequency(), expected, TOL),
        "Angular frequency of 3 Hz wave should be {}, got {}",
        expected,
        w.angular_frequency()
    );
}

// ============================================================================
// Validation tests
// ============================================================================

#[test]
fn negative_amplitude_returns_err() {
    // Amplitude represents a magnitude — it cannot be negative.
    // A wave with amplitude -1 has no physical meaning.
    let result = Wave::new(-1.0, 1.0, 0.0);
    assert!(result.is_err(), "Negative amplitude should return Err");
    assert_eq!(result.unwrap_err(), "amplitude must be non-negative");
}

#[test]
fn zero_frequency_returns_err() {
    // A wave with zero frequency never oscillates — it's a constant.
    // That's not a wave, so we reject it.
    let result = Wave::new(1.0, 0.0, 0.0);
    assert!(result.is_err(), "Zero frequency should return Err");
    assert_eq!(result.unwrap_err(), "frequency must be positive");
}

#[test]
fn negative_frequency_returns_err() {
    // Negative frequency has no physical meaning for a simple wave model.
    let result = Wave::new(1.0, -5.0, 0.0);
    assert!(result.is_err(), "Negative frequency should return Err");
    assert_eq!(result.unwrap_err(), "frequency must be positive");
}

// ============================================================================
// Edge cases
// ============================================================================

#[test]
fn zero_amplitude_produces_flat_line() {
    // A wave with amplitude 0 is just silence — y(t) = 0 for all t.
    let w = Wave::new(0.0, 440.0, 0.0).unwrap();
    assert!(
        approx_equal(w.evaluate(0.0), 0.0, TOL),
        "Zero-amplitude wave should be 0 everywhere"
    );
    assert!(
        approx_equal(w.evaluate(0.25), 0.0, TOL),
        "Zero-amplitude wave should be 0 everywhere"
    );
    assert!(
        approx_equal(w.evaluate(1.0), 0.0, TOL),
        "Zero-amplitude wave should be 0 everywhere"
    );
}

#[test]
fn wave_at_half_period_returns_to_zero() {
    // At t = T/2, the argument to sine is:
    //   2*pi*f*(1/(2f)) = pi
    // sin(pi) = 0, so the wave crosses zero at the halfway point.
    let w = Wave::new(7.0, 5.0, 0.0).unwrap();
    let half_period = w.period() / 2.0;
    assert!(
        approx_equal(w.evaluate(half_period), 0.0, TOL),
        "Wave should be 0 at half period, got {}",
        w.evaluate(half_period)
    );
}

#[test]
fn wave_at_three_quarter_period_reaches_negative_amplitude() {
    // At t = 3T/4, the argument to sine is:
    //   2*pi*f*(3/(4f)) = 3*pi/2
    // sin(3*pi/2) = -1, so y = -amplitude (the trough).
    let w = Wave::new(4.0, 1.0, 0.0).unwrap();
    assert!(
        approx_equal(w.evaluate(0.75), -4.0, TOL),
        "Wave should reach -amplitude at 3/4 period, got {}",
        w.evaluate(0.75)
    );
}

#[test]
fn phase_pi_inverts_the_wave() {
    // sin(x + PI) = -sin(x)
    //
    // A phase of PI flips the wave upside down.  At t=0.25 (where a
    // normal 1 Hz wave peaks at +1), this wave should be at -1.
    let w = Wave::new(1.0, 1.0, PI).unwrap();
    assert!(
        approx_equal(w.evaluate(0.25), -1.0, TOL),
        "Phase PI should invert the wave, got {}",
        w.evaluate(0.25)
    );
}

#[test]
fn high_frequency_wave_evaluates_correctly() {
    // Test with a high frequency to ensure no numerical instability.
    // 1000 Hz wave at t = 0.00025 (quarter period):
    //   2*pi*1000*0.00025 = pi/2
    //   sin(pi/2) = 1
    let w = Wave::new(1.0, 1000.0, 0.0).unwrap();
    assert!(
        approx_equal(w.evaluate(0.00025), 1.0, TOL),
        "High-frequency wave should reach peak at quarter period, got {}",
        w.evaluate(0.00025)
    );
}
