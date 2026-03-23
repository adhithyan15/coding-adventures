package wave

import (
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/trig"
)

// ============================================================================
// Test Helpers
// ============================================================================

// approxEqual checks whether two floating-point values are within a given
// tolerance of each other. Direct equality (==) is unreliable for floats
// because arithmetic operations introduce tiny rounding errors. For example,
// sin(pi) is not exactly 0 in floating-point — it's something like 1.2e-16.
//
// A tolerance of 1e-10 is generous enough to absorb accumulated rounding
// from our Maclaurin series, while tight enough to catch real bugs.
func approxEqual(a, b, tolerance float64) bool {
	diff := a - b
	if diff < 0 {
		diff = -diff
	}
	return diff < tolerance
}

const tol = 1e-10

// ============================================================================
// Constructor Tests
// ============================================================================

// TestNewValidWave verifies that valid parameters produce a Wave
// without error, and that all fields are set correctly.
func TestNewValidWave(t *testing.T) {
	w, err := New(5.0, 440.0, 0.5)
	if err != nil {
		t.Fatalf("expected no error, got %v", err)
	}
	if w.Amplitude != 5.0 {
		t.Errorf("expected amplitude 5.0, got %f", w.Amplitude)
	}
	if w.Frequency != 440.0 {
		t.Errorf("expected frequency 440.0, got %f", w.Frequency)
	}
	if w.Phase != 0.5 {
		t.Errorf("expected phase 0.5, got %f", w.Phase)
	}
}

// TestNewZeroAmplitude verifies that zero amplitude is allowed.
// A zero-amplitude wave is just a flat line — boring but valid.
func TestNewZeroAmplitude(t *testing.T) {
	w, err := New(0.0, 1.0, 0.0)
	if err != nil {
		t.Fatalf("expected no error for zero amplitude, got %v", err)
	}
	if w.Amplitude != 0.0 {
		t.Errorf("expected amplitude 0.0, got %f", w.Amplitude)
	}
}

// TestNewNegativeAmplitude verifies that negative amplitude is rejected.
// Amplitude represents magnitude; use phase=pi to invert instead.
func TestNewNegativeAmplitude(t *testing.T) {
	_, err := New(-1.0, 1.0, 0.0)
	if err == nil {
		t.Fatal("expected error for negative amplitude, got nil")
	}
	if err != ErrNegativeAmplitude {
		t.Errorf("expected ErrNegativeAmplitude, got %v", err)
	}
}

// TestNewZeroFrequency verifies that zero frequency is rejected.
// A zero-frequency "wave" never oscillates — it's a constant, not a wave.
func TestNewZeroFrequency(t *testing.T) {
	_, err := New(1.0, 0.0, 0.0)
	if err == nil {
		t.Fatal("expected error for zero frequency, got nil")
	}
	if err != ErrZeroFrequency {
		t.Errorf("expected ErrZeroFrequency, got %v", err)
	}
}

// TestNewNegativeFrequency verifies that negative frequency is rejected.
// Negative frequency is mathematically valid but ambiguous; we enforce positive.
func TestNewNegativeFrequency(t *testing.T) {
	_, err := New(1.0, -5.0, 0.0)
	if err == nil {
		t.Fatal("expected error for negative frequency, got nil")
	}
	if err != ErrZeroFrequency {
		t.Errorf("expected ErrZeroFrequency, got %v", err)
	}
}

// ============================================================================
// Evaluate Tests
// ============================================================================

// TestEvaluateAtZero verifies that a wave with zero phase evaluates to 0
// at t=0. This follows directly from the formula:
//
//	y(0) = A * sin(2*pi*f*0 + 0) = A * sin(0) = 0
func TestEvaluateAtZero(t *testing.T) {
	w, _ := New(1.0, 1.0, 0.0)
	result := w.Evaluate(0.0)
	if !approxEqual(result, 0.0, tol) {
		t.Errorf("expected 0.0 at t=0 with phase=0, got %f", result)
	}
}

// TestEvaluateAtQuarterPeriod verifies that a 1 Hz wave reaches its peak
// amplitude at t=0.25 (one quarter of the period).
//
// At t=0.25 for a 1 Hz wave:
//
//	y(0.25) = A * sin(2*pi*1*0.25) = A * sin(pi/2) = A * 1 = A
//
// This is a fundamental property: sine reaches its maximum at pi/2 radians,
// which corresponds to one quarter of a full cycle.
func TestEvaluateAtQuarterPeriod(t *testing.T) {
	w, _ := New(3.0, 1.0, 0.0)
	result := w.Evaluate(0.25)
	if !approxEqual(result, 3.0, tol) {
		t.Errorf("expected amplitude 3.0 at quarter period, got %f", result)
	}
}

// TestEvaluateAtHalfPeriod verifies the wave crosses zero at the half period.
//
//	y(0.5) = A * sin(2*pi*1*0.5) = A * sin(pi) = 0
func TestEvaluateAtHalfPeriod(t *testing.T) {
	w, _ := New(2.0, 1.0, 0.0)
	result := w.Evaluate(0.5)
	if !approxEqual(result, 0.0, tol) {
		t.Errorf("expected 0.0 at half period, got %f", result)
	}
}

// TestEvaluateAtThreeQuarterPeriod verifies the wave reaches its negative
// peak (trough) at three quarters of the period.
//
//	y(0.75) = A * sin(2*pi*1*0.75) = A * sin(3*pi/2) = -A
func TestEvaluateAtThreeQuarterPeriod(t *testing.T) {
	w, _ := New(4.0, 1.0, 0.0)
	result := w.Evaluate(0.75)
	if !approxEqual(result, -4.0, tol) {
		t.Errorf("expected -4.0 at three-quarter period, got %f", result)
	}
}

// TestPeriodicity verifies that the wave repeats after exactly one period.
// This is the defining property of a periodic function:
//
//	y(t) = y(t + T)  for all t
//
// where T = 1/f is the period.
func TestPeriodicity(t *testing.T) {
	w, _ := New(2.5, 3.0, 0.7)
	period := w.Period()

	// Test at several different time points
	testTimes := []float64{0.0, 0.1, 0.25, 0.37, 0.5}
	for _, tm := range testTimes {
		v1 := w.Evaluate(tm)
		v2 := w.Evaluate(tm + period)
		if !approxEqual(v1, v2, tol) {
			t.Errorf("periodicity failed at t=%f: y(t)=%f, y(t+T)=%f", tm, v1, v2)
		}
	}
}

// TestPhaseShiftPiOver2 verifies that a phase of pi/2 makes the wave start
// at its peak. This is because:
//
//	y(0) = A * sin(0 + pi/2) = A * cos(0) = A
//
// The identity sin(x + pi/2) = cos(x) means a pi/2 phase shift converts
// sine into cosine, which starts at 1 instead of 0.
func TestPhaseShiftPiOver2(t *testing.T) {
	w, _ := New(1.0, 1.0, trig.PI/2)
	result := w.Evaluate(0.0)
	if !approxEqual(result, 1.0, tol) {
		t.Errorf("expected 1.0 at t=0 with phase=pi/2, got %f", result)
	}
}

// TestPhaseShiftPi verifies that a phase of pi inverts the wave.
//
//	y(0.25) = A * sin(2*pi*1*0.25 + pi) = A * sin(pi/2 + pi) = -A
//
// A phase shift of pi flips the wave upside down.
func TestPhaseShiftPi(t *testing.T) {
	w, _ := New(1.0, 1.0, trig.PI)
	result := w.Evaluate(0.25)
	if !approxEqual(result, -1.0, tol) {
		t.Errorf("expected -1.0 with phase=pi at quarter period, got %f", result)
	}
}

// TestZeroAmplitudeAlwaysZero verifies that a zero-amplitude wave always
// evaluates to zero, regardless of time or phase.
func TestZeroAmplitudeAlwaysZero(t *testing.T) {
	w, _ := New(0.0, 5.0, 1.23)
	testTimes := []float64{0.0, 0.1, 0.5, 1.0, 100.0}
	for _, tm := range testTimes {
		result := w.Evaluate(tm)
		if !approxEqual(result, 0.0, tol) {
			t.Errorf("expected 0.0 for zero-amplitude wave at t=%f, got %f", tm, result)
		}
	}
}

// ============================================================================
// Derived Property Tests
// ============================================================================

// TestPeriod verifies that Period() = 1/Frequency for various frequencies.
//
// Truth table:
//
//	Frequency (Hz)  |  Period (s)
//	----------------|------------
//	1.0             |  1.0
//	2.0             |  0.5
//	440.0           |  ~0.00227
//	0.5             |  2.0
func TestPeriod(t *testing.T) {
	cases := []struct {
		freq   float64
		period float64
	}{
		{1.0, 1.0},
		{2.0, 0.5},
		{440.0, 1.0 / 440.0},
		{0.5, 2.0},
	}
	for _, c := range cases {
		w, _ := New(1.0, c.freq, 0.0)
		if !approxEqual(w.Period(), c.period, tol) {
			t.Errorf("Period() for freq=%f: expected %f, got %f", c.freq, c.period, w.Period())
		}
	}
}

// TestAngularFrequency verifies that AngularFrequency() = 2*pi*f.
//
// Truth table:
//
//	Frequency (Hz)  |  Angular Frequency (rad/s)
//	----------------|---------------------------
//	1.0             |  2*pi ≈ 6.2832
//	0.5             |  pi   ≈ 3.1416
//	440.0           |  880*pi ≈ 2764.6
func TestAngularFrequency(t *testing.T) {
	cases := []struct {
		freq  float64
		omega float64
	}{
		{1.0, 2.0 * trig.PI},
		{0.5, trig.PI},
		{440.0, 880.0 * trig.PI},
	}
	for _, c := range cases {
		w, _ := New(1.0, c.freq, 0.0)
		if !approxEqual(w.AngularFrequency(), c.omega, tol) {
			t.Errorf("AngularFrequency() for freq=%f: expected %f, got %f",
				c.freq, c.omega, w.AngularFrequency())
		}
	}
}

// ============================================================================
// Higher Frequency Tests
// ============================================================================

// TestHigherFrequencyWave verifies that a wave with a higher frequency
// reaches its peak sooner. A 2 Hz wave reaches its peak at t=0.125
// (quarter of its 0.5-second period).
func TestHigherFrequencyWave(t *testing.T) {
	w, _ := New(1.0, 2.0, 0.0)
	result := w.Evaluate(0.125) // quarter period of 2 Hz wave
	if !approxEqual(result, 1.0, tol) {
		t.Errorf("expected 1.0 at quarter period of 2 Hz wave, got %f", result)
	}
}

// TestMultiplePeriods verifies the wave still works correctly after many
// cycles. This tests that the underlying trig range reduction handles
// large arguments properly.
func TestMultiplePeriods(t *testing.T) {
	w, _ := New(1.0, 1.0, 0.0)
	// After 100 complete cycles, the wave should be back at zero
	result := w.Evaluate(100.0)
	if !approxEqual(result, 0.0, 1e-6) {
		t.Errorf("expected ~0.0 at t=100 (100 full periods), got %f", result)
	}
}
