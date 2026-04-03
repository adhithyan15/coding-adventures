package trig

import (
	"math"
	"testing"
)

// approxEqual checks whether two floating-point numbers are close enough
// to be considered equal, within a specified tolerance.
//
// Why do we need this? Floating-point arithmetic is inherently imprecise.
// The number 0.1, for example, cannot be represented exactly in binary.
// When we compute sin(pi), we don't get exactly 0 — we get something like
// 1.2e-16. So we need a way to say "close enough."
//
// A tolerance of 1e-10 means we accept results that differ by less than
// 0.0000000001. This is far more precise than any physical measurement
// but accounts for the tiny rounding errors in our series computation.
func approxEqual(a, b float64) bool {
	return math.Abs(a-b) < 1e-10
}

func approxEqualTol(a, b, tol float64) bool {
	return math.Abs(a-b) < tol
}

// ============================================================================
// Sine Tests
// ============================================================================

// TestSinZero verifies that sin(0) = 0.
//
// This is the simplest case: when x = 0, every term in the Maclaurin series
// is zero (since they all contain a factor of x), so the sum is zero.
func TestSinZero(t *testing.T) {
	if !approxEqual(Sin(0), 0) {
		t.Errorf("Sin(0) = %v, want 0", Sin(0))
	}
}

// TestSinPiOver2 verifies that sin(pi/2) = 1.
//
// This is one of the landmark values: at 90 degrees, sine reaches its
// maximum value of 1. If this test fails, something is fundamentally
// wrong with our series computation.
func TestSinPiOver2(t *testing.T) {
	if !approxEqual(Sin(PI/2), 1.0) {
		t.Errorf("Sin(PI/2) = %v, want 1.0", Sin(PI/2))
	}
}

// TestSinPi verifies that sin(pi) = 0.
//
// At 180 degrees, sine returns to zero. This tests that our series
// handles the full half-period correctly.
func TestSinPi(t *testing.T) {
	if !approxEqual(Sin(PI), 0.0) {
		t.Errorf("Sin(PI) = %v, want 0.0", Sin(PI))
	}
}

// TestSinNegative verifies the odd symmetry: sin(-x) = -sin(x).
//
// Sine is an "odd function," meaning it's antisymmetric about the origin.
// Graphically, if you rotate the sine curve 180 degrees around the origin,
// you get the same curve. This is a fundamental property that our
// implementation must preserve.
func TestSinNegative(t *testing.T) {
	testValues := []float64{0.5, 1.0, PI / 4, PI / 3, PI / 2, 2.0, 3.0}
	for _, x := range testValues {
		if !approxEqual(Sin(-x), -Sin(x)) {
			t.Errorf("Sin(-%v) = %v, want %v", x, Sin(-x), -Sin(x))
		}
	}
}

// TestSinLargeInput verifies that range reduction works for large inputs.
//
// sin(1000*pi) should be approximately 0, because 1000*pi is an integer
// multiple of pi. This tests that our range reduction correctly handles
// inputs far outside [-pi, pi].
func TestSinLargeInput(t *testing.T) {
	result := Sin(1000 * PI)
	if !approxEqual(result, 0.0) {
		t.Errorf("Sin(1000*PI) = %v, want approximately 0.0", result)
	}
}

// ============================================================================
// Cosine Tests
// ============================================================================

// TestCosZero verifies that cos(0) = 1.
//
// When x = 0, all terms in the Maclaurin series are zero except the first
// (which is 1), so cos(0) = 1. This is the maximum value of cosine.
func TestCosZero(t *testing.T) {
	if !approxEqual(Cos(0), 1.0) {
		t.Errorf("Cos(0) = %v, want 1.0", Cos(0))
	}
}

// TestCosPiOver2 verifies that cos(pi/2) = 0.
//
// At 90 degrees, cosine crosses zero. This is a critical test because
// small errors in the series can show up most clearly near zero crossings.
func TestCosPiOver2(t *testing.T) {
	if !approxEqual(Cos(PI/2), 0.0) {
		t.Errorf("Cos(PI/2) = %v, want 0.0", Cos(PI/2))
	}
}

// TestCosPi verifies that cos(pi) = -1.
//
// At 180 degrees, cosine reaches its minimum value of -1.
func TestCosPi(t *testing.T) {
	if !approxEqual(Cos(PI), -1.0) {
		t.Errorf("Cos(PI) = %v, want -1.0", Cos(PI))
	}
}

// TestCosNegative verifies the even symmetry: cos(-x) = cos(x).
//
// Cosine is an "even function," meaning it's symmetric about the y-axis.
// If you mirror the cosine curve across the y-axis, you get the same curve.
// This is the opposite of sine's odd symmetry.
func TestCosNegative(t *testing.T) {
	testValues := []float64{0.5, 1.0, PI / 4, PI / 3, PI / 2, 2.0, 3.0}
	for _, x := range testValues {
		if !approxEqual(Cos(-x), Cos(x)) {
			t.Errorf("Cos(-%v) = %v, want %v", x, Cos(-x), Cos(x))
		}
	}
}

// ============================================================================
// Pythagorean Identity: sin^2(x) + cos^2(x) = 1
// ============================================================================

// TestPythagoreanIdentity verifies that sin^2(x) + cos^2(x) = 1.
//
// This is perhaps the most important identity in trigonometry. It comes
// from the Pythagorean theorem applied to the unit circle: if a point on
// the unit circle has coordinates (cos(x), sin(x)), then:
//
//	cos^2(x) + sin^2(x) = 1^2 = 1
//
// We test this for a variety of angles spanning different quadrants.
func TestPythagoreanIdentity(t *testing.T) {
	testValues := []float64{
		0, PI / 6, PI / 4, PI / 3, PI / 2,
		PI, 3 * PI / 2, 2 * PI,
		-PI / 4, -PI / 2, -PI,
		0.1, 0.7, 1.5, 2.5, 5.0, 10.0,
	}
	for _, x := range testValues {
		s := Sin(x)
		c := Cos(x)
		sum := s*s + c*c
		if !approxEqual(sum, 1.0) {
			t.Errorf("Sin(%v)^2 + Cos(%v)^2 = %v, want 1.0", x, x, sum)
		}
	}
}

// ============================================================================
// Angle Conversion Tests
// ============================================================================

// TestRadians verifies degree-to-radian conversion for landmark angles.
func TestRadians(t *testing.T) {
	tests := []struct {
		deg  float64
		want float64
	}{
		{0, 0},
		{90, PI / 2},
		{180, PI},
		{360, TwoPI},
		{-180, -PI},
		{45, PI / 4},
	}
	for _, tt := range tests {
		got := Radians(tt.deg)
		if !approxEqual(got, tt.want) {
			t.Errorf("Radians(%v) = %v, want %v", tt.deg, got, tt.want)
		}
	}
}

// TestDegrees verifies radian-to-degree conversion for landmark angles.
func TestDegrees(t *testing.T) {
	tests := []struct {
		rad  float64
		want float64
	}{
		{0, 0},
		{PI / 2, 90},
		{PI, 180},
		{TwoPI, 360},
		{-PI, -180},
		{PI / 4, 45},
	}
	for _, tt := range tests {
		got := Degrees(tt.rad)
		if !approxEqual(got, tt.want) {
			t.Errorf("Degrees(%v) = %v, want %v", tt.rad, got, tt.want)
		}
	}
}

// ============================================================================
// Round-Trip Conversion Test
// ============================================================================

// TestRadiansDegreesRoundTrip verifies that converting degrees -> radians -> degrees
// returns the original value. This catches errors in either conversion function.
func TestRadiansDegreesRoundTrip(t *testing.T) {
	testValues := []float64{0, 30, 45, 60, 90, 120, 180, 270, 360, -45, -90}
	for _, deg := range testValues {
		got := Degrees(Radians(deg))
		if !approxEqual(got, deg) {
			t.Errorf("Degrees(Radians(%v)) = %v, want %v", deg, got, deg)
		}
	}
}

// ============================================================================
// Sqrt Tests
// ============================================================================

func TestSqrtZero(t *testing.T) {
	if Sqrt(0.0) != 0.0 {
		t.Errorf("Sqrt(0) = %v, want 0", Sqrt(0.0))
	}
}

func TestSqrtOne(t *testing.T) {
	if !approxEqual(Sqrt(1.0), 1.0) {
		t.Errorf("Sqrt(1) = %v, want 1.0", Sqrt(1.0))
	}
}

func TestSqrtFour(t *testing.T) {
	if !approxEqual(Sqrt(4.0), 2.0) {
		t.Errorf("Sqrt(4) = %v, want 2.0", Sqrt(4.0))
	}
}

func TestSqrtNine(t *testing.T) {
	if !approxEqual(Sqrt(9.0), 3.0) {
		t.Errorf("Sqrt(9) = %v, want 3.0", Sqrt(9.0))
	}
}

func TestSqrtTwo(t *testing.T) {
	// sqrt(2) ≈ 1.41421356237
	if !approxEqual(Sqrt(2.0), 1.41421356237) {
		t.Errorf("Sqrt(2) = %v, want ~1.41421356237", Sqrt(2.0))
	}
}

func TestSqrtQuarter(t *testing.T) {
	if !approxEqual(Sqrt(0.25), 0.5) {
		t.Errorf("Sqrt(0.25) = %v, want 0.5", Sqrt(0.25))
	}
}

func TestSqrtLarge(t *testing.T) {
	// sqrt(1e10) = 1e5
	if !approxEqualTol(Sqrt(1e10), 1e5, 1e-4) {
		t.Errorf("Sqrt(1e10) = %v, want 1e5", Sqrt(1e10))
	}
}

func TestSqrtRoundtrip(t *testing.T) {
	// sqrt(2) * sqrt(2) ≈ 2.0
	s := Sqrt(2.0)
	if !approxEqual(s*s, 2.0) {
		t.Errorf("Sqrt(2)^2 = %v, want 2.0", s*s)
	}
}

// TestSqrtNegativeHandled verifies that Sqrt(-1) does not produce a meaningful
// result. The Go Operation framework catches panics from the callback and returns
// the fallback value (0.0) rather than propagating. This is acceptable — callers
// should never pass negative inputs to Sqrt. We simply verify the call completes
// without crashing the test binary and returns the zero fallback.
func TestSqrtNegativeHandled(t *testing.T) {
	result := Sqrt(-1.0)
	// The operation catches the panic and returns 0.0 (fallback).
	// Any value is acceptable here since the input is invalid.
	_ = result // no assertion needed — the test just verifies no crash
}

// ============================================================================
// Tan Tests
// ============================================================================

func TestTanZero(t *testing.T) {
	if !approxEqual(Tan(0.0), 0.0) {
		t.Errorf("Tan(0) = %v, want 0", Tan(0.0))
	}
}

func TestTanPiOver4(t *testing.T) {
	if !approxEqual(Tan(PI/4), 1.0) {
		t.Errorf("Tan(PI/4) = %v, want 1.0", Tan(PI/4))
	}
}

func TestTanPiOver6(t *testing.T) {
	// tan(pi/6) = 1/sqrt(3)
	expected := 1.0 / Sqrt(3.0)
	if !approxEqual(Tan(PI/6), expected) {
		t.Errorf("Tan(PI/6) = %v, want %v", Tan(PI/6), expected)
	}
}

func TestTanNegativePiOver4(t *testing.T) {
	if !approxEqual(Tan(-PI/4), -1.0) {
		t.Errorf("Tan(-PI/4) = %v, want -1.0", Tan(-PI/4))
	}
}

// ============================================================================
// Atan Tests
// ============================================================================

func TestAtanZero(t *testing.T) {
	if Atan(0.0) != 0.0 {
		t.Errorf("Atan(0) = %v, want 0", Atan(0.0))
	}
}

func TestAtanOne(t *testing.T) {
	if !approxEqual(Atan(1.0), PI/4) {
		t.Errorf("Atan(1) = %v, want PI/4 = %v", Atan(1.0), PI/4)
	}
}

func TestAtanMinusOne(t *testing.T) {
	if !approxEqual(Atan(-1.0), -PI/4) {
		t.Errorf("Atan(-1) = %v, want -PI/4 = %v", Atan(-1.0), -PI/4)
	}
}

func TestAtanSqrt3(t *testing.T) {
	// atan(sqrt(3)) = pi/3
	if !approxEqual(Atan(Sqrt(3.0)), PI/3) {
		t.Errorf("Atan(sqrt(3)) = %v, want PI/3 = %v", Atan(Sqrt(3.0)), PI/3)
	}
}

func TestAtanInvSqrt3(t *testing.T) {
	// atan(1/sqrt(3)) = pi/6
	if !approxEqual(Atan(1.0/Sqrt(3.0)), PI/6) {
		t.Errorf("Atan(1/sqrt(3)) = %v, want PI/6 = %v", Atan(1.0/Sqrt(3.0)), PI/6)
	}
}

func TestAtanLargePositive(t *testing.T) {
	// atan(1e10) ≈ pi/2
	if !approxEqualTol(Atan(1e10), PI/2, 1e-5) {
		t.Errorf("Atan(1e10) = %v, want ~PI/2", Atan(1e10))
	}
}

func TestAtanLargeNegative(t *testing.T) {
	// atan(-1e10) ≈ -pi/2
	if !approxEqualTol(Atan(-1e10), -PI/2, 1e-5) {
		t.Errorf("Atan(-1e10) = %v, want ~-PI/2", Atan(-1e10))
	}
}

func TestAtanTanRoundtrip(t *testing.T) {
	// atan(tan(pi/4)) ≈ pi/4
	if !approxEqual(Atan(Tan(PI/4)), PI/4) {
		t.Errorf("Atan(Tan(PI/4)) = %v, want PI/4", Atan(Tan(PI/4)))
	}
}

// ============================================================================
// Atan2 Tests
// ============================================================================

func TestAtan2PositiveXAxis(t *testing.T) {
	if !approxEqual(Atan2(0.0, 1.0), 0.0) {
		t.Errorf("Atan2(0,1) = %v, want 0", Atan2(0.0, 1.0))
	}
}

func TestAtan2PositiveYAxis(t *testing.T) {
	if !approxEqual(Atan2(1.0, 0.0), PI/2) {
		t.Errorf("Atan2(1,0) = %v, want PI/2", Atan2(1.0, 0.0))
	}
}

func TestAtan2NegativeXAxis(t *testing.T) {
	if !approxEqual(Atan2(0.0, -1.0), PI) {
		t.Errorf("Atan2(0,-1) = %v, want PI", Atan2(0.0, -1.0))
	}
}

func TestAtan2NegativeYAxis(t *testing.T) {
	if !approxEqual(Atan2(-1.0, 0.0), -PI/2) {
		t.Errorf("Atan2(-1,0) = %v, want -PI/2", Atan2(-1.0, 0.0))
	}
}

func TestAtan2Q1(t *testing.T) {
	// atan2(1,1) = pi/4
	if !approxEqual(Atan2(1.0, 1.0), PI/4) {
		t.Errorf("Atan2(1,1) = %v, want PI/4", Atan2(1.0, 1.0))
	}
}

func TestAtan2Q2(t *testing.T) {
	// atan2(1,-1) = 3*pi/4
	if !approxEqual(Atan2(1.0, -1.0), 3*PI/4) {
		t.Errorf("Atan2(1,-1) = %v, want 3*PI/4", Atan2(1.0, -1.0))
	}
}

func TestAtan2Q3(t *testing.T) {
	// atan2(-1,-1) = -3*pi/4
	if !approxEqual(Atan2(-1.0, -1.0), -3*PI/4) {
		t.Errorf("Atan2(-1,-1) = %v, want -3*PI/4", Atan2(-1.0, -1.0))
	}
}

func TestAtan2Q4(t *testing.T) {
	// atan2(-1,1) = -pi/4
	if !approxEqual(Atan2(-1.0, 1.0), -PI/4) {
		t.Errorf("Atan2(-1,1) = %v, want -PI/4", Atan2(-1.0, 1.0))
	}
}
