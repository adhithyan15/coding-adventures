package fparithmetic

import (
	"math"
	"testing"
)

// TestIntToBitsMSB tests the integer-to-bits conversion.
func TestIntToBitsMSB(t *testing.T) {
	tests := []struct {
		value int
		width int
		want  []int
	}{
		{5, 8, []int{0, 0, 0, 0, 0, 1, 0, 1}},
		{0, 4, []int{0, 0, 0, 0}},
		{15, 4, []int{1, 1, 1, 1}},
		{1, 1, []int{1}},
		{127, 8, []int{0, 1, 1, 1, 1, 1, 1, 1}},
	}

	for _, tt := range tests {
		got := IntToBitsMSB(tt.value, tt.width)
		if len(got) != len(tt.want) {
			t.Errorf("IntToBitsMSB(%d, %d) length = %d, want %d", tt.value, tt.width, len(got), len(tt.want))
			continue
		}
		for i := range got {
			if got[i] != tt.want[i] {
				t.Errorf("IntToBitsMSB(%d, %d)[%d] = %d, want %d", tt.value, tt.width, i, got[i], tt.want[i])
			}
		}
	}
}

// TestBitsMSBToInt tests the bits-to-integer conversion.
func TestBitsMSBToInt(t *testing.T) {
	tests := []struct {
		bits []int
		want int
	}{
		{[]int{0, 0, 0, 0, 0, 1, 0, 1}, 5},
		{[]int{0, 0, 0, 0}, 0},
		{[]int{1, 1, 1, 1}, 15},
		{[]int{1}, 1},
		{[]int{0, 1, 1, 1, 1, 1, 1, 1}, 127},
	}

	for _, tt := range tests {
		got := BitsMSBToInt(tt.bits)
		if got != tt.want {
			t.Errorf("BitsMSBToInt(%v) = %d, want %d", tt.bits, got, tt.want)
		}
	}
}

// TestRoundTrip verifies that encoding and decoding produces the original value.
func TestRoundTrip(t *testing.T) {
	// Test a variety of values in FP32
	fp32Values := []float64{0.0, 1.0, -1.0, 3.14, -3.14, 0.5, 100.0, 0.001}
	for _, v := range fp32Values {
		bits := FloatToBits(v, FP32)
		got := BitsToFloat(bits)
		if math.IsNaN(v) {
			if !math.IsNaN(got) {
				t.Errorf("FP32 round-trip(%v): got %v", v, got)
			}
			continue
		}
		// FP32 conversion from float64 may lose some precision
		if float32(v) != float32(got) {
			t.Errorf("FP32 round-trip(%v): got %v", v, got)
		}
	}
}

// TestFloatToBitsSpecialValues tests encoding of special IEEE 754 values.
func TestFloatToBitsSpecialValues(t *testing.T) {
	// Positive zero
	t.Run("positive zero", func(t *testing.T) {
		bits := FloatToBits(0.0, FP32)
		if bits.Sign != 0 {
			t.Errorf("sign = %d, want 0", bits.Sign)
		}
		if !IsZero(bits) {
			t.Error("expected IsZero = true")
		}
	})

	// Negative zero
	t.Run("negative zero", func(t *testing.T) {
		bits := FloatToBits(math.Copysign(0, -1), FP32)
		if bits.Sign != 1 {
			t.Errorf("sign = %d, want 1", bits.Sign)
		}
		if !IsZero(bits) {
			t.Error("expected IsZero = true")
		}
	})

	// Positive infinity
	t.Run("positive infinity", func(t *testing.T) {
		bits := FloatToBits(math.Inf(1), FP32)
		if bits.Sign != 0 {
			t.Errorf("sign = %d, want 0", bits.Sign)
		}
		if !IsInf(bits) {
			t.Error("expected IsInf = true")
		}
	})

	// Negative infinity
	t.Run("negative infinity", func(t *testing.T) {
		bits := FloatToBits(math.Inf(-1), FP32)
		if bits.Sign != 1 {
			t.Errorf("sign = %d, want 1", bits.Sign)
		}
		if !IsInf(bits) {
			t.Error("expected IsInf = true")
		}
	})

	// NaN
	t.Run("NaN", func(t *testing.T) {
		bits := FloatToBits(math.NaN(), FP32)
		if !IsNaN(bits) {
			t.Error("expected IsNaN = true")
		}
	})
}

// TestFloatToBitsKnownValues tests encoding of specific well-known values.
func TestFloatToBitsKnownValues(t *testing.T) {
	// 1.0 in FP32: sign=0, exponent=01111111 (127), mantissa=all zeros
	t.Run("1.0 FP32", func(t *testing.T) {
		bits := FloatToBits(1.0, FP32)
		if bits.Sign != 0 {
			t.Errorf("sign = %d, want 0", bits.Sign)
		}
		expVal := BitsMSBToInt(bits.Exponent)
		if expVal != 127 {
			t.Errorf("exponent = %d, want 127", expVal)
		}
		mantVal := BitsMSBToInt(bits.Mantissa)
		if mantVal != 0 {
			t.Errorf("mantissa = %d, want 0", mantVal)
		}
	})

	// -2.0 in FP32: sign=1, exponent=128, mantissa=0
	t.Run("-2.0 FP32", func(t *testing.T) {
		bits := FloatToBits(-2.0, FP32)
		if bits.Sign != 1 {
			t.Errorf("sign = %d, want 1", bits.Sign)
		}
		expVal := BitsMSBToInt(bits.Exponent)
		if expVal != 128 {
			t.Errorf("exponent = %d, want 128", expVal)
		}
	})
}

// TestBitsToFloatSpecialValues tests decoding of special values.
func TestBitsToFloatSpecialValues(t *testing.T) {
	t.Run("NaN", func(t *testing.T) {
		nan := makeNaN(FP32)
		v := BitsToFloat(nan)
		if !math.IsNaN(v) {
			t.Errorf("expected NaN, got %v", v)
		}
	})

	t.Run("+Inf", func(t *testing.T) {
		inf := makeInf(0, FP32)
		v := BitsToFloat(inf)
		if !math.IsInf(v, 1) {
			t.Errorf("expected +Inf, got %v", v)
		}
	})

	t.Run("-Inf", func(t *testing.T) {
		inf := makeInf(1, FP32)
		v := BitsToFloat(inf)
		if !math.IsInf(v, -1) {
			t.Errorf("expected -Inf, got %v", v)
		}
	})

	t.Run("+0", func(t *testing.T) {
		z := makeZero(0, FP32)
		v := BitsToFloat(z)
		if v != 0.0 || math.Signbit(v) {
			t.Errorf("expected +0, got %v", v)
		}
	})

	t.Run("-0", func(t *testing.T) {
		z := makeZero(1, FP32)
		v := BitsToFloat(z)
		if v != 0.0 || !math.Signbit(v) {
			t.Errorf("expected -0, got %v", v)
		}
	})
}

// TestIsNaN tests the NaN detector.
func TestIsNaN(t *testing.T) {
	if !IsNaN(FloatToBits(math.NaN(), FP32)) {
		t.Error("IsNaN(NaN) should be true")
	}
	if IsNaN(FloatToBits(1.0, FP32)) {
		t.Error("IsNaN(1.0) should be false")
	}
	if IsNaN(FloatToBits(math.Inf(1), FP32)) {
		t.Error("IsNaN(Inf) should be false")
	}
}

// TestIsInf tests the infinity detector.
func TestIsInf(t *testing.T) {
	if !IsInf(FloatToBits(math.Inf(1), FP32)) {
		t.Error("IsInf(+Inf) should be true")
	}
	if !IsInf(FloatToBits(math.Inf(-1), FP32)) {
		t.Error("IsInf(-Inf) should be true")
	}
	if IsInf(FloatToBits(1.0, FP32)) {
		t.Error("IsInf(1.0) should be false")
	}
}

// TestIsZero tests the zero detector.
func TestIsZero(t *testing.T) {
	if !IsZero(FloatToBits(0.0, FP32)) {
		t.Error("IsZero(+0) should be true")
	}
	if !IsZero(FloatToBits(math.Copysign(0, -1), FP32)) {
		t.Error("IsZero(-0) should be true")
	}
	if IsZero(FloatToBits(1.0, FP32)) {
		t.Error("IsZero(1.0) should be false")
	}
}

// TestIsDenormalized tests the denormal detector.
func TestIsDenormalized(t *testing.T) {
	// Create the smallest positive denormal in FP32
	tiny := FloatBits{
		Sign:     0,
		Exponent: zerosSlice(8),
		Mantissa: append(zerosSlice(22), 1),
		Fmt:      FP32,
	}
	if !IsDenormalized(tiny) {
		t.Error("expected denormal to be detected")
	}
	if IsDenormalized(FloatToBits(1.0, FP32)) {
		t.Error("1.0 should not be denormalized")
	}
	if IsDenormalized(FloatToBits(0.0, FP32)) {
		t.Error("0.0 should not be denormalized")
	}
}

// TestFP16Encoding tests encoding/decoding in FP16 format.
func TestFP16Encoding(t *testing.T) {
	tests := []struct {
		name  string
		value float64
	}{
		{"1.0", 1.0},
		{"-1.0", -1.0},
		{"0.5", 0.5},
		{"0.0", 0.0},
		{"+Inf", math.Inf(1)},
		{"-Inf", math.Inf(-1)},
		{"NaN", math.NaN()},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			bits := FloatToBits(tt.value, FP16)
			got := BitsToFloat(bits)
			if math.IsNaN(tt.value) {
				if !math.IsNaN(got) {
					t.Errorf("FP16 NaN: got %v", got)
				}
				return
			}
			if math.IsInf(tt.value, 0) {
				if !math.IsInf(got, 0) {
					t.Errorf("FP16 Inf: got %v", got)
				}
				return
			}
			if got != tt.value {
				t.Errorf("FP16(%v): got %v", tt.value, got)
			}
		})
	}
}

// TestBF16Encoding tests encoding/decoding in BF16 format.
func TestBF16Encoding(t *testing.T) {
	tests := []struct {
		name  string
		value float64
	}{
		{"1.0", 1.0},
		{"-1.0", -1.0},
		{"0.0", 0.0},
		{"+Inf", math.Inf(1)},
		{"NaN", math.NaN()},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			bits := FloatToBits(tt.value, BF16)
			got := BitsToFloat(bits)
			if math.IsNaN(tt.value) {
				if !math.IsNaN(got) {
					t.Errorf("BF16 NaN: got %v", got)
				}
				return
			}
			if math.IsInf(tt.value, 0) {
				if !math.IsInf(got, 0) {
					t.Errorf("BF16 Inf: got %v", got)
				}
				return
			}
			if got != tt.value {
				t.Errorf("BF16(%v): got %v", tt.value, got)
			}
		})
	}
}

// TestFP16Overflow tests that large FP32 values overflow to Inf in FP16.
func TestFP16Overflow(t *testing.T) {
	// FP16 max is ~65504. A larger value should become Inf.
	bits := FloatToBits(100000.0, FP16)
	if !IsInf(bits) {
		t.Error("100000.0 in FP16 should overflow to Inf")
	}
}

// TestFP16Underflow tests that very small values underflow to zero in FP16.
func TestFP16Underflow(t *testing.T) {
	// Very tiny value below FP16 denormal range
	bits := FloatToBits(1e-20, FP16)
	if !IsZero(bits) {
		t.Error("1e-20 in FP16 should underflow to zero")
	}
}

// TestBitLength tests the bitLength utility function.
func TestBitLength(t *testing.T) {
	tests := []struct {
		value int
		want  int
	}{
		{0, 0},
		{1, 1},
		{2, 2},
		{3, 2},
		{4, 3},
		{5, 3},
		{255, 8},
		{256, 9},
	}

	for _, tt := range tests {
		got := bitLength(tt.value)
		if got != tt.want {
			t.Errorf("bitLength(%d) = %d, want %d", tt.value, got, tt.want)
		}
	}
}

// TestFP16SpecialValuesDetection tests special value detectors with FP16.
func TestFP16SpecialValuesDetection(t *testing.T) {
	nan := FloatToBits(math.NaN(), FP16)
	if !IsNaN(nan) {
		t.Error("FP16 NaN should be detected")
	}

	inf := FloatToBits(math.Inf(1), FP16)
	if !IsInf(inf) {
		t.Error("FP16 Inf should be detected")
	}

	zero := FloatToBits(0.0, FP16)
	if !IsZero(zero) {
		t.Error("FP16 zero should be detected")
	}
}

// TestBF16SpecialValuesDetection tests special value detectors with BF16.
func TestBF16SpecialValuesDetection(t *testing.T) {
	nan := FloatToBits(math.NaN(), BF16)
	if !IsNaN(nan) {
		t.Error("BF16 NaN should be detected")
	}

	inf := FloatToBits(math.Inf(-1), BF16)
	if !IsInf(inf) {
		t.Error("BF16 -Inf should be detected")
	}
}
