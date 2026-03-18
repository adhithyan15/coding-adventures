package fparithmetic

import (
	"testing"
)

// TestFloatFormatConstants verifies that the three standard IEEE 754 format
// constants (FP32, FP16, BF16) have the correct parameters. These constants
// define the bit layout of every floating-point number in this package, so
// getting them wrong would break everything.
func TestFloatFormatConstants(t *testing.T) {
	// FP32: The standard single-precision format. 32 total bits:
	//   1 sign + 8 exponent + 23 mantissa = 32
	t.Run("FP32 parameters", func(t *testing.T) {
		if FP32.TotalBits != 32 {
			t.Errorf("FP32.TotalBits = %d, want 32", FP32.TotalBits)
		}
		if FP32.ExponentBits != 8 {
			t.Errorf("FP32.ExponentBits = %d, want 8", FP32.ExponentBits)
		}
		if FP32.MantissaBits != 23 {
			t.Errorf("FP32.MantissaBits = %d, want 23", FP32.MantissaBits)
		}
		if FP32.Bias != 127 {
			t.Errorf("FP32.Bias = %d, want 127", FP32.Bias)
		}
		if FP32.Name != "fp32" {
			t.Errorf("FP32.Name = %q, want %q", FP32.Name, "fp32")
		}
	})

	// FP16: Half-precision. 16 total bits:
	//   1 sign + 5 exponent + 10 mantissa = 16
	t.Run("FP16 parameters", func(t *testing.T) {
		if FP16.TotalBits != 16 {
			t.Errorf("FP16.TotalBits = %d, want 16", FP16.TotalBits)
		}
		if FP16.ExponentBits != 5 {
			t.Errorf("FP16.ExponentBits = %d, want 5", FP16.ExponentBits)
		}
		if FP16.MantissaBits != 10 {
			t.Errorf("FP16.MantissaBits = %d, want 10", FP16.MantissaBits)
		}
		if FP16.Bias != 15 {
			t.Errorf("FP16.Bias = %d, want 15", FP16.Bias)
		}
	})

	// BF16: Brain Float. 16 total bits:
	//   1 sign + 8 exponent + 7 mantissa = 16
	//   Same exponent as FP32, much less mantissa.
	t.Run("BF16 parameters", func(t *testing.T) {
		if BF16.TotalBits != 16 {
			t.Errorf("BF16.TotalBits = %d, want 16", BF16.TotalBits)
		}
		if BF16.ExponentBits != 8 {
			t.Errorf("BF16.ExponentBits = %d, want 8", BF16.ExponentBits)
		}
		if BF16.MantissaBits != 7 {
			t.Errorf("BF16.MantissaBits = %d, want 7", BF16.MantissaBits)
		}
		if BF16.Bias != 127 {
			t.Errorf("BF16.Bias = %d, want 127", BF16.Bias)
		}
	})

	// Verify bit counts add up: sign(1) + exponent + mantissa = total
	t.Run("bit counts add up", func(t *testing.T) {
		for _, fmt := range []FloatFormat{FP32, FP16, BF16} {
			total := 1 + fmt.ExponentBits + fmt.MantissaBits
			if total != fmt.TotalBits {
				t.Errorf("%s: 1 + %d + %d = %d, want %d",
					fmt.Name, fmt.ExponentBits, fmt.MantissaBits, total, fmt.TotalBits)
			}
		}
	})
}

// TestMakeNaN tests the NaN constructor.
func TestMakeNaN(t *testing.T) {
	nan := makeNaN(FP32)
	if nan.Sign != 0 {
		t.Errorf("NaN sign = %d, want 0", nan.Sign)
	}
	if len(nan.Exponent) != 8 {
		t.Errorf("NaN exponent length = %d, want 8", len(nan.Exponent))
	}
	// All exponent bits should be 1
	for i, bit := range nan.Exponent {
		if bit != 1 {
			t.Errorf("NaN exponent[%d] = %d, want 1", i, bit)
		}
	}
	// First mantissa bit should be 1 (quiet NaN)
	if nan.Mantissa[0] != 1 {
		t.Errorf("NaN mantissa[0] = %d, want 1", nan.Mantissa[0])
	}
}

// TestMakeInf tests the infinity constructor.
func TestMakeInf(t *testing.T) {
	posInf := makeInf(0, FP32)
	negInf := makeInf(1, FP32)

	if posInf.Sign != 0 {
		t.Errorf("+Inf sign = %d, want 0", posInf.Sign)
	}
	if negInf.Sign != 1 {
		t.Errorf("-Inf sign = %d, want 1", negInf.Sign)
	}
	// Mantissa should be all zeros
	for i, bit := range posInf.Mantissa {
		if bit != 0 {
			t.Errorf("+Inf mantissa[%d] = %d, want 0", i, bit)
		}
	}
}

// TestMakeZero tests the zero constructor.
func TestMakeZero(t *testing.T) {
	posZero := makeZero(0, FP32)
	negZero := makeZero(1, FP32)

	if posZero.Sign != 0 {
		t.Errorf("+0 sign = %d, want 0", posZero.Sign)
	}
	if negZero.Sign != 1 {
		t.Errorf("-0 sign = %d, want 1", negZero.Sign)
	}
	for i, bit := range posZero.Exponent {
		if bit != 0 {
			t.Errorf("+0 exponent[%d] = %d, want 0", i, bit)
		}
	}
}

// TestZerosSlice and TestOnesSlice verify the utility functions.
func TestZerosSlice(t *testing.T) {
	z := zerosSlice(5)
	if len(z) != 5 {
		t.Fatalf("zerosSlice(5) length = %d, want 5", len(z))
	}
	for i, v := range z {
		if v != 0 {
			t.Errorf("zerosSlice(5)[%d] = %d, want 0", i, v)
		}
	}
}

func TestOnesSlice(t *testing.T) {
	o := onesSlice(5)
	if len(o) != 5 {
		t.Fatalf("onesSlice(5) length = %d, want 5", len(o))
	}
	for i, v := range o {
		if v != 1 {
			t.Errorf("onesSlice(5)[%d] = %d, want 1", i, v)
		}
	}
}
