package fparithmetic

import (
	"math"
	"testing"

	"github.com/adhithyan15/coding-adventures/code/packages/go/clock"
)

// These tests target specific untested branches to push coverage above 85%.

// TestFPAddLargeExponentDifference covers the alignment path where the exponent
// difference exceeds the mantissa width, triggering the large-shift branch.
func TestFPAddLargeExponentDifference(t *testing.T) {
	// Very different magnitudes: 1e30 + 1e-10. The smaller number is shifted
	// so far right that only sticky bits remain.
	a := FloatToBits(1e30, FP32)
	b := FloatToBits(1e-10, FP32)
	result := FPAdd(a, b)
	got := BitsToFloat(result)
	// Result should be approximately 1e30 (b is too tiny to change a)
	if float32(got) != float32(1e30) {
		t.Errorf("1e30 + 1e-10 = %v, want ~1e30", got)
	}

	// Reverse operand order (tests the other branch of exp_a < exp_b)
	result2 := FPAdd(b, a)
	got2 := BitsToFloat(result2)
	if float32(got2) != float32(1e30) {
		t.Errorf("1e-10 + 1e30 = %v, want ~1e30", got2)
	}
}

// TestFPAddOverflowToInf covers the exponent overflow path in addition.
func TestFPAddOverflowToInf(t *testing.T) {
	huge := FloatToBits(3e38, FP32)
	result := FPAdd(huge, huge)
	if !IsInf(result) {
		t.Error("3e38 + 3e38 should overflow to Inf")
	}
}

// TestFPAddNegativeOverflow covers negative overflow to -Inf.
func TestFPAddNegativeOverflow(t *testing.T) {
	huge := FloatToBits(-3e38, FP32)
	result := FPAdd(huge, huge)
	if !IsInf(result) || result.Sign != 1 {
		t.Error("-3e38 + -3e38 should overflow to -Inf")
	}
}

// TestFPMulDenormalResult tests multiplication producing underflow results.
func TestFPMulDenormalResult(t *testing.T) {
	// Multiply two very small numbers
	tiny := FloatToBits(1e-20, FP32)
	result := FPMul(tiny, tiny)
	// Result should be very close to zero or zero
	got := BitsToFloat(result)
	if got < 0 || got > 1e-35 {
		t.Errorf("1e-20 * 1e-20 = %v, expected ~0", got)
	}
}

// TestFPMulDenormalInputs tests multiplication with denormal inputs.
func TestFPMulDenormalInputs(t *testing.T) {
	// Create a denormal number
	denorm := FloatBits{
		Sign:     0,
		Exponent: zerosSlice(8),
		Mantissa: append(zerosSlice(22), 1),
		Fmt:      FP32,
	}
	one := FloatToBits(1.0, FP32)
	result := FPMul(denorm, one)
	got := BitsToFloat(result)
	expected := BitsToFloat(denorm)
	if float32(got) != float32(expected) {
		t.Errorf("denorm * 1.0 = %v, want %v", got, expected)
	}
}

// TestFPAddDenormalInputs tests addition with denormal inputs.
func TestFPAddDenormalInputs(t *testing.T) {
	denorm := FloatBits{
		Sign:     0,
		Exponent: zerosSlice(8),
		Mantissa: append(zerosSlice(22), 1),
		Fmt:      FP32,
	}
	zero := FloatToBits(0.0, FP32)
	result := FPAdd(denorm, zero)
	if BitsToFloat(result) != BitsToFloat(denorm) {
		t.Error("denorm + 0 should equal denorm")
	}

	// Two denormals -- the result should be non-zero and positive
	result2 := FPAdd(denorm, denorm)
	got := BitsToFloat(result2)
	if got <= 0 {
		t.Errorf("denorm + denorm = %v, expected positive", got)
	}
}

// TestFPCompareInfinity tests comparison with infinity values.
func TestFPCompareInfinity(t *testing.T) {
	inf := FloatToBits(math.Inf(1), FP32)
	negInf := FloatToBits(math.Inf(-1), FP32)
	one := FloatToBits(1.0, FP32)

	if FPCompare(inf, one) != 1 {
		t.Error("Inf > 1.0")
	}
	if FPCompare(one, inf) != -1 {
		t.Error("1.0 < Inf")
	}
	if FPCompare(negInf, one) != -1 {
		t.Error("-Inf < 1.0")
	}
	if FPCompare(inf, inf) != 0 {
		t.Error("Inf == Inf")
	}
	if FPCompare(negInf, negInf) != 0 {
		t.Error("-Inf == -Inf")
	}
	if FPCompare(inf, negInf) != 1 {
		t.Error("Inf > -Inf")
	}
}

// TestFPCompareNegativeMantissa tests negative number mantissa comparison.
func TestFPCompareNegativeMantissa(t *testing.T) {
	// -1.5 vs -1.25: -1.5 < -1.25
	a := FloatToBits(-1.5, FP32)
	b := FloatToBits(-1.25, FP32)
	if FPCompare(a, b) != -1 {
		t.Error("-1.5 < -1.25 should return -1")
	}
	if FPCompare(b, a) != 1 {
		t.Error("-1.25 > -1.5 should return 1")
	}
}

// TestFPAddSubtractiveCancel tests near-cancellation in subtraction.
func TestFPAddSubtractiveCancel(t *testing.T) {
	// 1.00001 - 1.0 = small positive number
	a := FloatToBits(1.00001, FP32)
	b := FloatToBits(-1.0, FP32)
	result := FPAdd(a, b)
	got := BitsToFloat(result)
	if got < 0 || got > 0.001 {
		t.Errorf("1.00001 - 1.0 = %v, expected small positive", got)
	}
}

// TestFPAddNegInfPlusNegInf tests -Inf + -Inf = -Inf.
func TestFPAddNegInfPlusNegInf(t *testing.T) {
	negInf := FloatToBits(math.Inf(-1), FP32)
	result := FPAdd(negInf, negInf)
	if !IsInf(result) || result.Sign != 1 {
		t.Error("-Inf + -Inf should be -Inf")
	}
}

// TestFPAddMinusInfPlusOne tests -Inf + 1.0 = -Inf.
func TestFPAddMinusInfPlusOne(t *testing.T) {
	negInf := FloatToBits(math.Inf(-1), FP32)
	one := FloatToBits(1.0, FP32)
	result := FPAdd(negInf, one)
	if !IsInf(result) || result.Sign != 1 {
		t.Error("-Inf + 1.0 should be -Inf")
	}
}

// TestFPAddOnePlusMinusInf tests 1.0 + (-Inf) = -Inf.
func TestFPAddOnePlusMinusInf(t *testing.T) {
	one := FloatToBits(1.0, FP32)
	negInf := FloatToBits(math.Inf(-1), FP32)
	result := FPAdd(one, negInf)
	if !IsInf(result) || result.Sign != 1 {
		t.Error("1.0 + (-Inf) should be -Inf")
	}
}

// TestFPAddNegZeros tests -0 + -0 = -0.
func TestFPAddNegZeros(t *testing.T) {
	negZero := FloatToBits(math.Copysign(0, -1), FP32)
	result := FPAdd(negZero, negZero)
	if !IsZero(result) {
		t.Error("-0 + -0 should be zero")
	}
	if result.Sign != 1 {
		t.Error("-0 + -0 should be -0")
	}
}

// TestFPMulNegativeExponent tests multiplication with result exponent near 0.
func TestFPMulNegativeExponent(t *testing.T) {
	small := FloatToBits(1e-19, FP32)
	result := FPMul(small, small)
	got := BitsToFloat(result)
	if got != 0 && got > 1e-35 {
		t.Errorf("1e-19 * 1e-19 = %v, expected very small or 0", got)
	}
}

// TestFPAddFlushToZero tests extreme underflow.
func TestFPAddFlushToZero(t *testing.T) {
	tiny := FloatBits{
		Sign:     0,
		Exponent: zerosSlice(8),
		Mantissa: append(zerosSlice(22), 1),
		Fmt:      FP32,
	}
	negTiny := FPNeg(tiny)
	result := FPAdd(tiny, negTiny)
	if !IsZero(result) {
		t.Error("tiny - tiny should be zero")
	}
}

// TestFloatToBitsDenormalFP16 tests encoding of denormal values in FP16.
func TestFloatToBitsDenormalFP16(t *testing.T) {
	// Very small value that becomes denormal in FP16
	// The smallest normal FP16 is 2^-14 = ~6.1e-5
	bits := FloatToBits(3e-5, FP16)
	if IsZero(bits) {
		t.Error("3e-5 should be representable as denormal in FP16")
	}
	got := BitsToFloat(bits)
	if got == 0 || got > 6e-5 {
		t.Errorf("FP16(3e-5) = %v, expected small positive", got)
	}
}

// TestPipelinedAdderCancellation tests pipeline with cancellation yielding zero.
func TestPipelinedAdderCancellation(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	adder.Submit(FloatToBits(5.0, FP32), FloatToBits(-5.0, FP32))

	for i := 0; i < 5; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(adder.Results))
	}
	if !IsZero(*adder.Results[0]) {
		t.Error("5.0 + (-5.0) should be zero")
	}
}

// TestPipelinedMultiplierNegative tests pipeline with negative multiplication.
func TestPipelinedMultiplierNegative(t *testing.T) {
	clk := clock.New(1000000)
	mul := NewPipelinedFPMultiplier(clk, FP32)

	mul.Submit(FloatToBits(-3.0, FP32), FloatToBits(4.0, FP32))

	for i := 0; i < 4; i++ {
		clk.FullCycle()
	}

	if len(mul.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(mul.Results))
	}
	got := BitsToFloat(*mul.Results[0])
	if float32(got) != -12.0 {
		t.Errorf("-3 * 4 = %v, want -12.0", got)
	}
}

// TestPipelinedFMANegative tests the FMA pipeline with subtraction.
func TestPipelinedFMANegative(t *testing.T) {
	clk := clock.New(1000000)
	fma := NewPipelinedFMA(clk, FP32)

	// -2.0 * 3.0 + 10.0 = -6.0 + 10.0 = 4.0
	fma.Submit(FloatToBits(-2.0, FP32), FloatToBits(3.0, FP32), FloatToBits(10.0, FP32))

	for i := 0; i < 6; i++ {
		clk.FullCycle()
	}

	if len(fma.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(fma.Results))
	}
	got := BitsToFloat(*fma.Results[0])
	if float32(got) != 4.0 {
		t.Errorf("FMA(-2, 3, 10) = %v, want 4.0", got)
	}
}

// TestPipelinedFMASubtractive tests FMA where product > c with different signs.
func TestPipelinedFMASubtractive(t *testing.T) {
	clk := clock.New(1000000)
	fma := NewPipelinedFMA(clk, FP32)

	// 3.0 * 2.0 + (-1.0) = 6.0 - 1.0 = 5.0
	fma.Submit(FloatToBits(3.0, FP32), FloatToBits(2.0, FP32), FloatToBits(-1.0, FP32))

	for i := 0; i < 6; i++ {
		clk.FullCycle()
	}

	if len(fma.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(fma.Results))
	}
	got := BitsToFloat(*fma.Results[0])
	if float32(got) != 5.0 {
		t.Errorf("FMA(3, 2, -1) = %v, want 5.0", got)
	}
}

// TestPipelinedFMACLargerThanProduct tests FMA where |c| > |product|.
func TestPipelinedFMACLargerThanProduct(t *testing.T) {
	clk := clock.New(1000000)
	fma := NewPipelinedFMA(clk, FP32)

	// 1.0 * 1.0 + (-5.0) = 1.0 - 5.0 = -4.0
	fma.Submit(FloatToBits(1.0, FP32), FloatToBits(1.0, FP32), FloatToBits(-5.0, FP32))

	for i := 0; i < 6; i++ {
		clk.FullCycle()
	}

	if len(fma.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(fma.Results))
	}
	got := BitsToFloat(*fma.Results[0])
	if float32(got) != -4.0 {
		t.Errorf("FMA(1, 1, -5) = %v, want -4.0", got)
	}
}

// TestPipelinedAdderLargeExpDiff tests pipeline with large exponent difference.
func TestPipelinedAdderLargeExpDiff(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	adder.Submit(FloatToBits(1e20, FP32), FloatToBits(1e-20, FP32))

	for i := 0; i < 5; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(adder.Results))
	}
	got := BitsToFloat(*adder.Results[0])
	if float32(got) != float32(1e20) {
		t.Errorf("1e20 + 1e-20 = %v, want ~1e20", got)
	}
}

// TestPipelinedAdderBothInfSameSign tests +Inf + +Inf in pipeline.
func TestPipelinedAdderBothInfSameSign(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	adder.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(math.Inf(1), FP32))

	for i := 0; i < 5; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(adder.Results))
	}
	if !IsInf(*adder.Results[0]) {
		t.Error("Inf + Inf should be Inf")
	}
}

// TestPipelinedAdderBothInfDiffSign tests +Inf + (-Inf) = NaN in pipeline.
func TestPipelinedAdderBothInfDiffSign(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	adder.Submit(FloatToBits(math.Inf(1), FP32), FloatToBits(math.Inf(-1), FP32))

	for i := 0; i < 5; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(adder.Results))
	}
	if !IsNaN(*adder.Results[0]) {
		t.Error("Inf + (-Inf) should be NaN")
	}
}

// TestPipelinedAdderZeroPlusZero tests 0 + 0 in pipeline.
func TestPipelinedAdderZeroPlusZero(t *testing.T) {
	clk := clock.New(1000000)
	adder := NewPipelinedFPAdder(clk, FP32)

	adder.Submit(FloatToBits(0.0, FP32), FloatToBits(0.0, FP32))

	for i := 0; i < 5; i++ {
		clk.FullCycle()
	}

	if len(adder.Results) != 1 {
		t.Fatalf("expected 1 result, got %d", len(adder.Results))
	}
	if !IsZero(*adder.Results[0]) {
		t.Error("0 + 0 should be 0")
	}
}

// TestFPAddExponentUnderflow tests addition resulting in denormal numbers.
func TestFPAddExponentUnderflow(t *testing.T) {
	// Add two very small numbers that produce a denormal result
	a := FloatToBits(1e-38, FP32)
	b := FloatToBits(-1e-38, FP32)
	result := FPAdd(a, b)
	got := BitsToFloat(result)
	if got != 0 {
		// Small subtraction should yield zero
		t.Errorf("1e-38 - 1e-38 = %v, want 0", got)
	}
}

// TestFPMulRoundPosZero tests multiplication round_pos == 0 edge case.
func TestFPMulRoundPosZero(t *testing.T) {
	// Smallest normal * smallest normal should exercise edge cases
	smallest := FloatToBits(1.1754944e-38, FP32) // ~2^-126
	big := FloatToBits(1.0, FP32)
	result := FPMul(smallest, big)
	got := BitsToFloat(result)
	expected := float64(float32(1.1754944e-38) * float32(1.0))
	if float32(got) != float32(expected) {
		t.Errorf("smallest * 1.0 = %v, want %v", got, expected)
	}
}

// TestFPCompareSameNegativeExponent tests negative numbers with same exponent
// but different mantissa.
func TestFPCompareSameNegativeExponent(t *testing.T) {
	a := FloatToBits(-1.5, FP32) // -1.5 = -1.1 x 2^0
	b := FloatToBits(-1.0, FP32) // -1.0 = -1.0 x 2^0
	// -1.5 < -1.0, so compare should return -1
	if FPCompare(a, b) != -1 {
		t.Error("-1.5 < -1.0 should return -1")
	}
}
