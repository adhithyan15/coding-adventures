// Package fparithmetic implements IEEE 754 floating-point formats and
// arithmetic operations built from logic gates.
//
// # What is a floating-point format?
//
// Floating-point is how computers represent real numbers (like 3.14 or -0.001).
// It works like scientific notation, but in binary:
//
//	Scientific notation:   -6.022 x 10^23
//	IEEE 754 (binary):     (-1)^sign x 1.mantissa x 2^(exponent - bias)
//
// A floating-point number is stored as three bit fields packed into a fixed-width
// binary word:
//
//	FP32 (32 bits):  [sign(1)] [exponent(8)] [mantissa(23)]
//	                  ^         ^              ^
//	                  |         |              |
//	                  |         |              +-- fractional part (after the "1.")
//	                  |         +-- power of 2 (biased: stored value - 127)
//	                  +-- 0 = positive, 1 = negative
//
// # The three formats we support
//
//	Format  Total  Exp  Mantissa  Bias   Used by
//	------  -----  ---  --------  ----   -------
//	FP32     32     8     23      127    CPU, GPU (default precision)
//	FP16     16     5     10       15    GPU training (mixed precision)
//	BF16     16     8      7      127    TPU (native), ML training
//
// # Why BF16 exists
//
// BF16 (Brain Float 16) was invented by Google for TPU hardware. It keeps the
// same exponent range as FP32 (8-bit exponent, bias 127) but truncates the
// mantissa from 23 bits to just 7. This means:
//
//   - Same range as FP32 (can represent very large and very small numbers)
//   - Much less precision (~2-3 decimal digits vs ~7 for FP32)
//   - Perfect for ML: gradients can be huge or tiny (need range), but don't
//     need to be super precise (need less precision)
//   - Trivial conversion from FP32: just truncate the lower 16 bits!
//
// # The implicit leading 1
//
// For normal (non-zero, non-denormal) numbers, the mantissa has an implicit
// leading 1 that is not stored. So a stored mantissa of [1, 0, 1, ...] actually
// represents 1.101... in binary. This trick gives us one extra bit of precision
// for free.
//
//	Stored bits:   [1, 0, 1, 0, 0, ...]
//	Actual value:  1.10100...  (the "1." is implicit)
//
// The only exception is denormalized numbers (exponent = all zeros), where the
// implicit bit is 0 instead of 1, allowing representation of very small numbers
// near zero.
package fparithmetic

// =========================================================================
// FloatFormat -- describes the shape of a floating-point format
// =========================================================================

// FloatFormat describes the bit layout of an IEEE 754 floating-point format.
//
// This is an immutable struct by convention (all fields are exported but should
// not be mutated after creation). Go does not have frozen dataclasses like
// Python, so discipline replaces language enforcement.
//
// Fields:
//   - Name: Human-readable name ("fp32", "fp16", "bf16").
//   - TotalBits: Total width of the format in bits.
//   - ExponentBits: Number of bits in the exponent field.
//   - MantissaBits: Number of explicit mantissa bits (without the implicit
//     leading 1). The actual precision is MantissaBits + 1.
//   - Bias: The exponent bias. The true exponent is (stored_exponent - Bias).
//     For FP32: bias=127, so stored exponent 127 means true exponent 0,
//     stored exponent 128 means true exponent 1, etc.
type FloatFormat struct {
	Name         string
	TotalBits    int
	ExponentBits int
	MantissaBits int
	Bias         int
}

// =========================================================================
// Standard format constants
// =========================================================================
//
// These are package-level singletons. All code that works with floating-point
// should reference these constants rather than constructing FloatFormat manually.

// FP32 (single precision) -- the workhorse of computing.
//
//	[sign(1)] [exponent(8)] [mantissa(23)]
//	 bit 31    bits 30-23    bits 22-0
//
// Used by CPU FPUs, GPU CUDA cores, and as the default for most computation.
// Range: ~1.18e-38 to ~3.40e38, precision: ~7 decimal digits.
var FP32 = FloatFormat{Name: "fp32", TotalBits: 32, ExponentBits: 8, MantissaBits: 23, Bias: 127}

// FP16 (half precision) -- GPU mixed-precision training.
//
//	[sign(1)] [exponent(5)] [mantissa(10)]
//	 bit 15    bits 14-10    bits 9-0
//
// Used for GPU training in mixed precision and inference. Saves memory and
// bandwidth at the cost of range and precision.
// Range: ~5.96e-8 to ~65504, precision: ~3-4 decimal digits.
var FP16 = FloatFormat{Name: "fp16", TotalBits: 16, ExponentBits: 5, MantissaBits: 10, Bias: 15}

// BF16 (brain float) -- Google's TPU native format.
//
//	[sign(1)] [exponent(8)] [mantissa(7)]
//	 bit 15    bits 14-7     bits 6-0
//
// Same exponent range as FP32, but with only 7 mantissa bits (vs 23).
// Converting FP32 -> BF16 is trivial: just drop the lower 16 bits.
// Range: same as FP32, precision: ~2-3 decimal digits.
var BF16 = FloatFormat{Name: "bf16", TotalBits: 16, ExponentBits: 8, MantissaBits: 7, Bias: 127}

// =========================================================================
// FloatBits -- the actual bit pattern of a floating-point number
// =========================================================================

// FloatBits is the bit-level representation of an IEEE 754 floating-point number.
//
// This stores the actual 0s and 1s that make up the number, decomposed into
// the three fields (sign, exponent, mantissa). All bit slices are stored
// MSB-first (index 0 = most significant bit).
//
// === Bit layout (FP32 example) ===
//
// Consider the number 3.14:
//
//	Binary: 1.10010001111010111000011 x 2^1
//	Sign: 0 (positive)
//	Exponent: 128 (= 1 + 127 bias) = [1,0,0,0,0,0,0,0]
//	Mantissa: [1,0,0,1,0,0,0,1,1,1,1,0,1,0,1,1,1,0,0,0,0,1,1]
//
// Packed as 32 bits:
//
//	[0] [10000000] [10010001111010111000011]
//	sign  exponent        mantissa
//
// Fields:
//   - Sign: 0 for positive, 1 for negative.
//   - Exponent: Slice of exponent bits, MSB first. Length = Fmt.ExponentBits.
//   - Mantissa: Slice of mantissa bits, MSB first. Length = Fmt.MantissaBits.
//     These are the explicit bits only (no implicit leading 1).
//   - Fmt: The FloatFormat this number is encoded in.
type FloatBits struct {
	Sign     int
	Exponent []int
	Mantissa []int
	Fmt      FloatFormat
}

// =========================================================================
// Helper constructors for common special values
// =========================================================================

// makeNaN creates a quiet NaN in the given format.
//
// NaN (Not a Number) is represented by exponent = all 1s and mantissa != 0.
// The MSB of the mantissa being 1 makes it a "quiet" NaN (as opposed to a
// "signaling" NaN with MSB 0).
func makeNaN(fmt FloatFormat) FloatBits {
	return FloatBits{
		Sign:     0,
		Exponent: onesSlice(fmt.ExponentBits),
		Mantissa: append([]int{1}, zerosSlice(fmt.MantissaBits-1)...),
		Fmt:      fmt,
	}
}

// makeInf creates positive or negative infinity in the given format.
//
// Infinity is represented by exponent = all 1s and mantissa = all 0s.
func makeInf(sign int, fmt FloatFormat) FloatBits {
	return FloatBits{
		Sign:     sign,
		Exponent: onesSlice(fmt.ExponentBits),
		Mantissa: zerosSlice(fmt.MantissaBits),
		Fmt:      fmt,
	}
}

// makeZero creates positive or negative zero in the given format.
//
// Zero is represented by exponent = all 0s and mantissa = all 0s.
// IEEE 754 has both +0 and -0 -- they compare equal but have different bits.
func makeZero(sign int, fmt FloatFormat) FloatBits {
	return FloatBits{
		Sign:     sign,
		Exponent: zerosSlice(fmt.ExponentBits),
		Mantissa: zerosSlice(fmt.MantissaBits),
		Fmt:      fmt,
	}
}

// =========================================================================
// Utility functions for bit slices
// =========================================================================

// zerosSlice creates a slice of n zeros.
func zerosSlice(n int) []int {
	return make([]int, n)
}

// onesSlice creates a slice of n ones.
func onesSlice(n int) []int {
	s := make([]int, n)
	for i := range s {
		s[i] = 1
	}
	return s
}
