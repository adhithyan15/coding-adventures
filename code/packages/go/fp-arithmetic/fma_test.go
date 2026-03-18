package fparithmetic

import (
	"math"
	"testing"
)

// TestFMABasic tests basic fused multiply-add operations.
func TestFMABasic(t *testing.T) {
	tests := []struct {
		name    string
		a, b, c float64
		want    float64
	}{
		{"1.5 * 2.0 + 0.25 = 3.25", 1.5, 2.0, 0.25, 3.25},
		{"2.0 * 3.0 + 1.0 = 7.0", 2.0, 3.0, 1.0, 7.0},
		{"1.0 * 1.0 + 0.0 = 1.0", 1.0, 1.0, 0.0, 1.0},
		{"0.5 * 0.5 + 0.5 = 0.75", 0.5, 0.5, 0.5, 0.75},
		{"10.0 * 10.0 + 0.0 = 100.0", 10.0, 10.0, 0.0, 100.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			c := FloatToBits(tt.c, FP32)
			result := FMA(a, b, c)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FMA(%v, %v, %v) = %v, want %v", tt.a, tt.b, tt.c, got, tt.want)
			}
		})
	}
}

// TestFMAWithNegatives tests FMA with negative operands.
func TestFMAWithNegatives(t *testing.T) {
	tests := []struct {
		name    string
		a, b, c float64
		want    float64
	}{
		{"-1.0 * 2.0 + 3.0 = 1.0", -1.0, 2.0, 3.0, 1.0},
		{"2.0 * -3.0 + 10.0 = 4.0", 2.0, -3.0, 10.0, 4.0},
		{"-2.0 * -3.0 + 1.0 = 7.0", -2.0, -3.0, 1.0, 7.0},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			a := FloatToBits(tt.a, FP32)
			b := FloatToBits(tt.b, FP32)
			c := FloatToBits(tt.c, FP32)
			result := FMA(a, b, c)
			got := BitsToFloat(result)
			if float32(got) != float32(tt.want) {
				t.Errorf("FMA(%v, %v, %v) = %v, want %v", tt.a, tt.b, tt.c, got, tt.want)
			}
		})
	}
}

// TestFMASpecialValues tests IEEE 754 special value rules for FMA.
func TestFMASpecialValues(t *testing.T) {
	nan := FloatToBits(math.NaN(), FP32)
	inf := FloatToBits(math.Inf(1), FP32)
	negInf := FloatToBits(math.Inf(-1), FP32)
	zero := FloatToBits(0.0, FP32)
	one := FloatToBits(1.0, FP32)
	two := FloatToBits(2.0, FP32)

	// NaN in any position -> NaN
	t.Run("NaN * 1.0 + 1.0", func(t *testing.T) {
		result := FMA(nan, one, one)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	t.Run("1.0 * NaN + 1.0", func(t *testing.T) {
		result := FMA(one, nan, one)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	t.Run("1.0 * 1.0 + NaN", func(t *testing.T) {
		result := FMA(one, one, nan)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// Inf * 0 = NaN
	t.Run("Inf * 0 + 1.0", func(t *testing.T) {
		result := FMA(inf, zero, one)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// 0 * Inf = NaN
	t.Run("0 * Inf + 1.0", func(t *testing.T) {
		result := FMA(zero, inf, one)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// Inf * finite + c = Inf
	t.Run("Inf * 2.0 + 1.0", func(t *testing.T) {
		result := FMA(inf, two, one)
		if !IsInf(result) {
			t.Error("expected Inf")
		}
	})

	// Inf * finite + (-Inf) = NaN
	t.Run("Inf * 1.0 + (-Inf)", func(t *testing.T) {
		result := FMA(inf, one, negInf)
		if !IsNaN(result) {
			t.Error("expected NaN")
		}
	})

	// 0 * 0 + 0 = 0
	t.Run("0 * 0 + 0", func(t *testing.T) {
		result := FMA(zero, zero, zero)
		if !IsZero(result) {
			t.Error("expected zero")
		}
	})

	// 0 * finite + c = c
	t.Run("0 * 5.0 + 3.0", func(t *testing.T) {
		five := FloatToBits(5.0, FP32)
		three := FloatToBits(3.0, FP32)
		result := FMA(zero, five, three)
		got := BitsToFloat(result)
		if float32(got) != 3.0 {
			t.Errorf("FMA(0, 5, 3) = %v, want 3.0", got)
		}
	})

	// finite * finite + Inf = Inf
	t.Run("2.0 * 3.0 + Inf", func(t *testing.T) {
		result := FMA(two, FloatToBits(3.0, FP32), inf)
		if !IsInf(result) {
			t.Error("expected Inf")
		}
	})
}

// TestFPConvert tests format conversion between FP32, FP16, and BF16.
func TestFPConvert(t *testing.T) {
	// Same format: no-op
	t.Run("FP32 to FP32", func(t *testing.T) {
		bits := FloatToBits(3.14, FP32)
		converted := FPConvert(bits, FP32)
		if BitsToFloat(converted) != BitsToFloat(bits) {
			t.Error("same-format conversion should be identity")
		}
	})

	// FP32 -> FP16 -> FP32 round-trip
	t.Run("FP32 to FP16 to FP32", func(t *testing.T) {
		bits32 := FloatToBits(1.5, FP32)
		bits16 := FPConvert(bits32, FP16)
		bitsBack := FPConvert(bits16, FP32)
		got := BitsToFloat(bitsBack)
		if float32(got) != 1.5 {
			t.Errorf("round-trip 1.5 through FP16: got %v", got)
		}
	})

	// FP32 -> BF16 -> FP32 round-trip
	t.Run("FP32 to BF16 to FP32", func(t *testing.T) {
		bits32 := FloatToBits(1.0, FP32)
		bitsBF := FPConvert(bits32, BF16)
		bitsBack := FPConvert(bitsBF, FP32)
		got := BitsToFloat(bitsBack)
		if float32(got) != 1.0 {
			t.Errorf("round-trip 1.0 through BF16: got %v", got)
		}
	})

	// Special values survive conversion
	t.Run("NaN conversion", func(t *testing.T) {
		nan32 := FloatToBits(math.NaN(), FP32)
		nan16 := FPConvert(nan32, FP16)
		if !IsNaN(nan16) {
			t.Error("NaN should survive FP32->FP16 conversion")
		}
	})

	t.Run("Inf conversion", func(t *testing.T) {
		inf32 := FloatToBits(math.Inf(1), FP32)
		inf16 := FPConvert(inf32, FP16)
		if !IsInf(inf16) {
			t.Error("Inf should survive FP32->FP16 conversion")
		}
	})

	t.Run("Zero conversion", func(t *testing.T) {
		zero32 := FloatToBits(0.0, FP32)
		zero16 := FPConvert(zero32, FP16)
		if !IsZero(zero16) {
			t.Error("Zero should survive FP32->FP16 conversion")
		}
	})
}

// TestFMADotProduct simulates a small dot product using FMA.
//
// dot(a, w) = a[0]*w[0] + a[1]*w[1] + a[2]*w[2]
// = FMA(a[2], w[2], FMA(a[1], w[1], a[0]*w[0]))
func TestFMADotProduct(t *testing.T) {
	// Simple dot product: [1, 2, 3] . [4, 5, 6] = 4 + 10 + 18 = 32
	a := []float64{1.0, 2.0, 3.0}
	w := []float64{4.0, 5.0, 6.0}

	// Start with first multiply
	acc := FPMul(FloatToBits(a[0], FP32), FloatToBits(w[0], FP32))

	// Accumulate using FMA
	for i := 1; i < len(a); i++ {
		acc = FMA(FloatToBits(a[i], FP32), FloatToBits(w[i], FP32), acc)
	}

	got := BitsToFloat(acc)
	if float32(got) != 32.0 {
		t.Errorf("dot product = %v, want 32.0", got)
	}
}
