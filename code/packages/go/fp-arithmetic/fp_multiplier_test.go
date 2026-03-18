package fparithmetic

import (
	"math"
	"testing"
)

// TestFPMulBasic tests basic floating-point multiplication.
func TestFPMulBasic(t *testing.T) {
	tests := []struct {
		name string
		a, b float64
		want float64
	}{
		{"1.5 * 2.0 = 3.0", 1.5, 2.0, 3.0},
		{"3.0 * 4.0 = 12.0", 3.0, 4.0, 12.0},
		{"0.5 * 0.5 = 0.25", 0.5, 0.5, 0.25},
		{"10.0 * 0.1", 10.0, 0.1, float64(float32(10.0) * float32(0.1))},
		{"1.0 * 1.0 = 1.0", 1.0, 1.0, 1.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			result := FPMul(a, b)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FPMul(%v, %v) = %v, want %v", tt.a, tt.b, got, tt.want)
			}
		})
	}
}

// TestFPMulSigns tests sign handling in multiplication.
func TestFPMulSigns(t *testing.T) {
	tests := []struct {
		name string
		a, b float64
		want float64
	}{
		{"pos * pos = pos", 2.0, 3.0, 6.0},
		{"pos * neg = neg", 2.0, -3.0, -6.0},
		{"neg * pos = neg", -2.0, 3.0, -6.0},
		{"neg * neg = pos", -2.0, -3.0, 6.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			result := FPMul(a, b)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FPMul(%v, %v) = %v, want %v", tt.a, tt.b, got, tt.want)
			}
		})
	}
}

// TestFPMulSpecialValues tests IEEE 754 special value rules for multiplication.
func TestFPMulSpecialValues(t *testing.T) {
	nan := FloatToBits(math.NaN(), FP32)
	inf := FloatToBits(math.Inf(1), FP32)
	negInf := FloatToBits(math.Inf(-1), FP32)
	zero := FloatToBits(0.0, FP32)
	one := FloatToBits(1.0, FP32)
	two := FloatToBits(2.0, FP32)

	// NaN * anything = NaN
	t.Run("NaN * 1.0", func(t *testing.T) {
		result := FPMul(nan, one)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// Inf * 0 = NaN
	t.Run("Inf * 0", func(t *testing.T) {
		result := FPMul(inf, zero)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// 0 * Inf = NaN
	t.Run("0 * Inf", func(t *testing.T) {
		result := FPMul(zero, inf)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// Inf * finite = Inf
	t.Run("Inf * 2.0", func(t *testing.T) {
		result := FPMul(inf, two)
		if !IsInf(result) || result.Sign != 0 {
			t.Error("expected +Inf")
		}
	})

	// Inf * -finite = -Inf
	t.Run("Inf * -2.0", func(t *testing.T) {
		negTwo := FloatToBits(-2.0, FP32)
		result := FPMul(inf, negTwo)
		if !IsInf(result) || result.Sign != 1 {
			t.Error("expected -Inf")
		}
	})

	// -Inf * -Inf = +Inf
	t.Run("-Inf * -Inf", func(t *testing.T) {
		result := FPMul(negInf, negInf)
		if !IsInf(result) || result.Sign != 0 {
			t.Error("expected +Inf")
		}
	})

	// 0 * finite = 0
	t.Run("0 * 5.0", func(t *testing.T) {
		five := FloatToBits(5.0, FP32)
		result := FPMul(zero, five)
		if !IsZero(result) {
			t.Error("expected zero")
		}
	})
}

// TestFPMulOverflow tests that overflow produces infinity.
func TestFPMulOverflow(t *testing.T) {
	huge := FloatToBits(1e38, FP32)
	result := FPMul(huge, huge)
	if !IsInf(result) {
		t.Error("1e38 * 1e38 should overflow to Inf")
	}
}

// TestFPMulCommutative tests that multiplication is commutative.
func TestFPMulCommutative(t *testing.T) {
	pairs := [][2]float64{
		{1.5, 2.5},
		{-3.0, 7.0},
		{0.001, 1000.0},
	}

	for _, pair := range pairs {
		a := FloatToBits(pair[0], FP32)
		b := FloatToBits(pair[1], FP32)
		ab := BitsToFloat(FPMul(a, b))
		ba := BitsToFloat(FPMul(b, a))
		if float32(ab) != float32(ba) {
			t.Errorf("FPMul not commutative: %v * %v = %v, but %v * %v = %v",
				pair[0], pair[1], ab, pair[1], pair[0], ba)
		}
	}
}

// TestFPMulByOne tests the identity property: x * 1.0 = x.
func TestFPMulByOne(t *testing.T) {
	values := []float64{1.0, -1.0, 3.14, -0.001, 1e10, 1e-10}
	one := FloatToBits(1.0, FP32)

	for _, v := range values {
		a := FloatToBits(v, FP32)
		result := FPMul(a, one)
		got := BitsToFloat(result)
		if float32(got) != float32(v) {
			t.Errorf("FPMul(%v, 1.0) = %v, want %v", v, got, v)
		}
	}
}

// TestFPMulByZero tests that x * 0 = 0.
func TestFPMulByZero(t *testing.T) {
	values := []float64{1.0, -5.0, 1e30}
	zero := FloatToBits(0.0, FP32)

	for _, v := range values {
		a := FloatToBits(v, FP32)
		result := FPMul(a, zero)
		if !IsZero(result) {
			t.Errorf("FPMul(%v, 0) should be zero, got %v", v, BitsToFloat(result))
		}
	}
}
