package fparithmetic

import (
	"math"
	"testing"
)

// TestFPAddBasic tests basic floating-point additions.
func TestFPAddBasic(t *testing.T) {
	tests := []struct {
		name string
		a, b float64
		want float64
	}{
		{"1.5 + 2.5 = 4.0", 1.5, 2.5, 4.0},
		{"1.0 + 1.0 = 2.0", 1.0, 1.0, 2.0},
		{"0.5 + 0.25 = 0.75", 0.5, 0.25, 0.75},
		{"100.0 + 0.01", 100.0, 0.01, float64(float32(100.0) + float32(0.01))},
		{"1.5 + 0.0 = 1.5", 1.5, 0.0, 1.5},
		{"0.0 + 2.5 = 2.5", 0.0, 2.5, 2.5},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			result := FPAdd(a, b)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FPAdd(%v, %v) = %v, want %v", tt.a, tt.b, got, tt.want)
			}
		})
	}
}

// TestFPAddNegative tests addition with negative numbers.
func TestFPAddNegative(t *testing.T) {
	tests := []struct {
		name string
		a, b float64
		want float64
	}{
		{"-1.0 + -2.0 = -3.0", -1.0, -2.0, -3.0},
		{"1.0 + -0.5 = 0.5", 1.0, -0.5, 0.5},
		{"-1.0 + 2.0 = 1.0", -1.0, 2.0, 1.0},
		{"1.0 + -1.0 = 0.0", 1.0, -1.0, 0.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			result := FPAdd(a, b)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FPAdd(%v, %v) = %v, want %v", tt.a, tt.b, got, tt.want)
			}
		})
	}
}

// TestFPAddSpecialValues tests IEEE 754 special value rules for addition.
func TestFPAddSpecialValues(t *testing.T) {
	nan := FloatToBits(math.NaN(), FP32)
	inf := FloatToBits(math.Inf(1), FP32)
	negInf := FloatToBits(math.Inf(-1), FP32)
	one := FloatToBits(1.0, FP32)
	zero := FloatToBits(0.0, FP32)

	// NaN + anything = NaN
	t.Run("NaN + 1.0", func(t *testing.T) {
		result := FPAdd(nan, one)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	t.Run("1.0 + NaN", func(t *testing.T) {
		result := FPAdd(one, nan)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// Inf + (-Inf) = NaN
	t.Run("Inf + (-Inf)", func(t *testing.T) {
		result := FPAdd(inf, negInf)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// Inf + Inf = Inf
	t.Run("Inf + Inf", func(t *testing.T) {
		result := FPAdd(inf, inf)
		if !IsInf(result) || result.Sign != 0 {
			t.Error("expected +Inf")
		}
	})

	// Inf + finite = Inf
	t.Run("Inf + 1.0", func(t *testing.T) {
		result := FPAdd(inf, one)
		if !IsInf(result) || result.Sign != 0 {
			t.Error("expected +Inf")
		}
	})

	// 0 + 0 = 0
	t.Run("0 + 0", func(t *testing.T) {
		result := FPAdd(zero, zero)
		if !IsZero(result) {
			t.Error("expected zero")
		}
	})
}

// TestFPSub tests floating-point subtraction.
func TestFPSub(t *testing.T) {
	tests := []struct {
		name string
		a, b float64
		want float64
	}{
		{"3.0 - 1.0 = 2.0", 3.0, 1.0, 2.0},
		{"1.0 - 3.0 = -2.0", 1.0, 3.0, -2.0},
		{"5.0 - 5.0 = 0.0", 5.0, 5.0, 0.0},
		{"-1.0 - -1.0 = 0.0", -1.0, -1.0, 0.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			result := FPSub(a, b)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FPSub(%v, %v) = %v, want %v", tt.a, tt.b, got, tt.want)
			}
		})
	}
}

// TestFPNeg tests floating-point negation.
func TestFPNeg(t *testing.T) {
	// Negate positive
	pos := FloatToBits(3.14, FP32)
	neg := FPNeg(pos)
	if neg.Sign != 1 {
		t.Errorf("neg(+3.14) sign = %d, want 1", neg.Sign)
	}
	if float32(BitsToFloat(neg)) != float32(-3.14) {
		t.Errorf("neg(+3.14) = %v, want %v", BitsToFloat(neg), -3.14)
	}

	// Double negation returns original
	doubleNeg := FPNeg(neg)
	if doubleNeg.Sign != 0 {
		t.Errorf("neg(neg(3.14)) sign = %d, want 0", doubleNeg.Sign)
	}

	// Negate zero
	posZero := FloatToBits(0.0, FP32)
	negZero := FPNeg(posZero)
	if negZero.Sign != 1 {
		t.Error("neg(+0) should produce -0")
	}
}

// TestFPAbs tests floating-point absolute value.
func TestFPAbs(t *testing.T) {
	neg := FloatToBits(-5.0, FP32)
	abs := FPAbs(neg)
	if abs.Sign != 0 {
		t.Errorf("abs(-5.0) sign = %d, want 0", abs.Sign)
	}
	if float32(BitsToFloat(abs)) != 5.0 {
		t.Errorf("abs(-5.0) = %v, want 5.0", BitsToFloat(abs))
	}

	// abs of positive stays positive
	pos := FloatToBits(5.0, FP32)
	absPos := FPAbs(pos)
	if absPos.Sign != 0 {
		t.Error("abs(5.0) should be positive")
	}
}

// TestFPCompare tests floating-point comparison.
func TestFPCompare(t *testing.T) {
	tests := []struct {
		name string
		a, b float64
		want int
	}{
		{"1.0 < 2.0", 1.0, 2.0, -1},
		{"2.0 > 1.0", 2.0, 1.0, 1},
		{"1.0 == 1.0", 1.0, 1.0, 0},
		{"-1.0 < 1.0", -1.0, 1.0, -1},
		{"1.0 > -1.0", 1.0, -1.0, 1},
		{"-2.0 < -1.0", -2.0, -1.0, -1},
		{"-1.0 > -2.0", -1.0, -2.0, 1},
		{"+0 == -0", 0.0, math.Copysign(0, -1), 0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			got := FPCompare(a, b)
			if got != tt.want {
				t.Errorf("FPCompare(%v, %v) = %d, want %d", tt.a, tt.b, got, tt.want)
			}
		})
	}

	// NaN comparisons return 0 (unordered)
	t.Run("NaN comparisons", func(t *testing.T) {
		nan := FloatToBits(math.NaN(), FP32)
		one := FloatToBits(1.0, FP32)
		if FPCompare(nan, one) != 0 {
			t.Error("NaN comparison should return 0")
		}
		if FPCompare(one, nan) != 0 {
			t.Error("NaN comparison should return 0")
		}
	})
}

// TestFPAddCommutative tests that addition is commutative: a + b == b + a.
func TestFPAddCommutative(t *testing.T) {
	pairs := [][2]float64{
		{1.5, 2.5},
		{-3.0, 7.0},
		{0.001, 1000.0},
	}

	for _, pair := range pairs {
		a := FloatToBits(pair[0], FP32)
		b := FloatToBits(pair[1], FP32)
		ab := BitsToFloat(FPAdd(a, b))
		ba := BitsToFloat(FPAdd(b, a))
		if float32(ab) != float32(ba) {
			t.Errorf("FPAdd not commutative: %v + %v = %v, but %v + %v = %v",
				pair[0], pair[1], ab, pair[1], pair[0], ba)
		}
	}
}

// TestFPCompareZeroIdentity tests comparison edge cases with zeros.
func TestFPCompareZeroIdentity(t *testing.T) {
	posZero := FloatToBits(0.0, FP32)
	one := FloatToBits(1.0, FP32)
	negOne := FloatToBits(-1.0, FP32)

	if FPCompare(posZero, one) != -1 {
		t.Error("0 < 1 should return -1")
	}
	if FPCompare(posZero, negOne) != 1 {
		t.Error("0 > -1 should return 1")
	}
}
