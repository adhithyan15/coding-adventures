package fparithmetic

// IEEE 754 encoding and decoding -- converting between Go float64 values and
// our explicit bit-level representation (FloatBits).
//
// === How does a computer store 3.14? ===
//
// When you write `x := 3.14` in Go, the computer stores it as 64 bits
// following the IEEE 754 standard. This module converts between Go's native
// float representation and our explicit bit-level representation (FloatBits).
//
// === Encoding: float -> bits ===
//
// For FP32, we use Go's math package and bit manipulation to get the exact
// same bit pattern that the hardware uses. For FP16 and BF16, we manually
// extract the bits because Go doesn't natively support these formats.
//
// === Special values in IEEE 754 ===
//
// IEEE 754 reserves certain bit patterns for special values:
//
//	Exponent      Mantissa    Meaning
//	----------    --------    -------
//	All 1s        All 0s      +/- Infinity
//	All 1s        Non-zero    NaN (Not a Number)
//	All 0s        All 0s      +/- Zero
//	All 0s        Non-zero    Denormalized number (very small, near zero)
//	Other         Any         Normal number

import (
	"math"

	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// =========================================================================
// Helper: integer <-> bit list conversions
// =========================================================================

// IntToBitsMSB converts a non-negative integer to a slice of bits, MSB first.
//
// This is the fundamental conversion between Go integers and our bit-level
// representation.
//
// Example:
//
//	IntToBitsMSB(5, 8) => [0, 0, 0, 0, 0, 1, 0, 1]
//	//                      128 64 32 16  8  4  2  1
//	//                                    4     1  = 5
//
// How it works: we check each bit position from MSB to LSB. For each
// position i (counting from width-1 down to 0), we check if that bit is
// set using a right-shift and AND with 1.
func IntToBitsMSB(value int, width int) []int {
	bits := make([]int, width)
	for i := 0; i < width; i++ {
		bits[i] = (value >> (width - 1 - i)) & 1
	}
	return bits
}

// BitsMSBToInt converts a slice of bits (MSB first) back to a non-negative integer.
//
// This is the inverse of IntToBitsMSB.
//
// Example:
//
//	BitsMSBToInt([]int{0, 0, 0, 0, 0, 1, 0, 1}) => 5
//	// Each bit contributes: bit_value * 2^position
//	// 0*128 + 0*64 + 0*32 + 0*16 + 0*8 + 1*4 + 0*2 + 1*1 = 5
//
// How it works: iterate from MSB to LSB. For each bit, shift the accumulator
// left by 1 (multiply by 2) and OR in the new bit.
func BitsMSBToInt(bits []int) int {
	result := 0
	for _, bit := range bits {
		result = (result << 1) | bit
	}
	return result
}

// =========================================================================
// Encoding: Go float64 -> FloatBits
// =========================================================================

// FloatToBits converts a Go float64 to its IEEE 754 bit representation.
//
// === How FP32 encoding works ===
//
// For FP32, we use math.Float32bits which gives us the exact bit pattern
// that the hardware uses. We then extract the sign, exponent, and mantissa
// fields using bit shifts and masks.
//
// === How FP16/BF16 encoding works (manual) ===
//
// For FP16 and BF16, Go doesn't have native support, so we:
//  1. First encode as FP32 (which we know is exact for the hardware)
//  2. Extract the sign, exponent, and mantissa from the FP32 encoding
//  3. Re-encode into the target format, adjusting exponent bias and
//     truncating the mantissa
//
// === Worked example: encoding 3.14 as FP32 ===
//
//	3.14 in binary: 11.00100011110101110000101...
//	Normalized:     1.100100011110101110000101... x 2^1
//
//	Sign:     0 (positive)
//	Exponent: 1 + 127 (bias) = 128 = 10000000 in binary
//	Mantissa: 10010001111010111000010 (23 bits after the implicit 1)
//	                                   ^-- note: the leading 1 is NOT stored
func FloatToBits(value float64, fmt FloatFormat) FloatBits {
	// --- Handle NaN specially ---
	// Go has math.NaN(), and IEEE 754 defines NaN as exponent=all-1s,
	// mantissa=non-zero. We use a "quiet NaN" with the MSB of mantissa set.
	if math.IsNaN(value) {
		return makeNaN(fmt)
	}

	// --- Handle Infinity ---
	// +Inf and -Inf: exponent=all-1s, mantissa=all-0s.
	if math.IsInf(value, 0) {
		sign := 0
		if value < 0 {
			sign = 1
		}
		return makeInf(sign, fmt)
	}

	// --- FP32: use math.Float32bits for hardware-exact encoding ---
	if fmt == FP32 {
		// math.Float32bits gives us the raw 32-bit IEEE 754 representation.
		intBits := math.Float32bits(float32(value))

		// Extract the three fields using bit shifts and masks:
		//   Bit 31:     sign
		//   Bits 30-23: exponent (8 bits)
		//   Bits 22-0:  mantissa (23 bits)
		sign := int((intBits >> 31) & 1)
		expInt := int((intBits >> 23) & 0xFF)
		mantInt := int(intBits & 0x7FFFFF)

		return FloatBits{
			Sign:     sign,
			Exponent: IntToBitsMSB(expInt, 8),
			Mantissa: IntToBitsMSB(mantInt, 23),
			Fmt:      FP32,
		}
	}

	// --- FP16 and BF16: manual conversion from FP32 ---
	//
	// Strategy: encode as FP32 first, then convert.
	// This handles all the tricky cases (denormals, rounding) correctly.
	fp32Bits := FloatToBits(value, FP32)
	fp32Exp := BitsMSBToInt(fp32Bits.Exponent)
	fp32Mant := BitsMSBToInt(fp32Bits.Mantissa)
	sign := fp32Bits.Sign

	// --- Handle zero ---
	if fp32Exp == 0 && fp32Mant == 0 {
		return makeZero(sign, fmt)
	}

	// --- Compute the true (unbiased) exponent ---
	var trueExp int
	var fullMantissa int
	if fp32Exp == 0 {
		// Denormal in FP32: true exponent is -126, implicit bit is 0
		trueExp = 1 - FP32.Bias // = -126
		fullMantissa = fp32Mant
	} else {
		trueExp = fp32Exp - FP32.Bias
		// Normal: full mantissa includes the implicit leading 1
		fullMantissa = (1 << FP32.MantissaBits) | fp32Mant
	}

	// --- Map to target format ---
	targetExp := trueExp + fmt.Bias
	maxExp := (1 << fmt.ExponentBits) - 1

	// --- Overflow: exponent too large for target format -> Infinity ---
	if targetExp >= maxExp {
		return makeInf(sign, fmt)
	}

	// --- Normal case: exponent fits in target format ---
	if targetExp > 0 {
		var truncated int
		if fmt.MantissaBits < FP32.MantissaBits {
			shift := FP32.MantissaBits - fmt.MantissaBits
			truncated = fp32Mant >> shift
			// Round-to-nearest-even
			roundBit := (fp32Mant >> (shift - 1)) & 1
			sticky := fp32Mant & ((1 << (shift - 1)) - 1)
			if roundBit != 0 && (sticky != 0 || (truncated&1) != 0) {
				truncated++
				// Rounding overflow
				if truncated >= (1 << fmt.MantissaBits) {
					truncated = 0
					targetExp++
					if targetExp >= maxExp {
						return makeInf(sign, fmt)
					}
				}
			}
		} else {
			truncated = fp32Mant << (fmt.MantissaBits - FP32.MantissaBits)
		}

		return FloatBits{
			Sign:     sign,
			Exponent: IntToBitsMSB(targetExp, fmt.ExponentBits),
			Mantissa: IntToBitsMSB(truncated, fmt.MantissaBits),
			Fmt:      fmt,
		}
	}

	// --- Underflow: number is too small for normal representation ---
	// It might still be representable as a denormal in the target format.
	denormShift := 1 - targetExp

	if denormShift > fmt.MantissaBits {
		// Too small even for denormal -> flush to zero
		return makeZero(sign, fmt)
	}

	// Shift the full mantissa right to create a denormal
	denormMant := fullMantissa >> (denormShift + FP32.MantissaBits - fmt.MantissaBits)

	return FloatBits{
		Sign:     sign,
		Exponent: zerosSlice(fmt.ExponentBits),
		Mantissa: IntToBitsMSB(denormMant&((1<<fmt.MantissaBits)-1), fmt.MantissaBits),
		Fmt:      fmt,
	}
}

// =========================================================================
// Decoding: FloatBits -> Go float64
// =========================================================================

// BitsToFloat converts an IEEE 754 bit representation back to a Go float64.
//
// === How decoding works ===
//
// For FP32, we reconstruct the 32-bit integer and use math.Float32frombits to
// get the exact Go float. For FP16/BF16, we manually compute the value using:
//
//	value = (-1)^sign x 2^(exponent - bias) x 1.mantissa
func BitsToFloat(bits FloatBits) float64 {
	expInt := BitsMSBToInt(bits.Exponent)
	mantInt := BitsMSBToInt(bits.Mantissa)
	maxExp := (1 << bits.Fmt.ExponentBits) - 1

	// --- Special values ---

	// NaN: exponent all 1s, mantissa non-zero
	if expInt == maxExp && mantInt != 0 {
		return math.NaN()
	}

	// Infinity: exponent all 1s, mantissa all zeros
	if expInt == maxExp && mantInt == 0 {
		if bits.Sign == 1 {
			return math.Inf(-1)
		}
		return math.Inf(1)
	}

	// Zero: exponent all 0s, mantissa all zeros
	if expInt == 0 && mantInt == 0 {
		if bits.Sign == 1 {
			return math.Copysign(0, -1)
		}
		return 0.0
	}

	// --- For FP32, use math.Float32frombits for exact conversion ---
	if bits.Fmt == FP32 {
		intBits := uint32(bits.Sign<<31) | uint32(expInt<<23) | uint32(mantInt)
		return float64(math.Float32frombits(intBits))
	}

	// --- For FP16/BF16, compute the float value manually ---

	var trueExp int
	var mantissaValue float64

	if expInt == 0 {
		// Denormalized: value = (-1)^sign x 2^(1-bias) x 0.mantissa
		trueExp = 1 - bits.Fmt.Bias
		mantissaValue = float64(mantInt) / float64(int(1)<<bits.Fmt.MantissaBits)
	} else {
		// Normal: implicit leading 1
		trueExp = expInt - bits.Fmt.Bias
		mantissaValue = 1.0 + float64(mantInt)/float64(int(1)<<bits.Fmt.MantissaBits)
	}

	result := mantissaValue * math.Pow(2.0, float64(trueExp))
	if bits.Sign == 1 {
		result = -result
	}

	return result
}

// =========================================================================
// Special value detection -- using logic gates
// =========================================================================
//
// These functions detect special IEEE 754 values by examining the bit pattern.
// We use AND and OR from logicgates to check bit fields, staying true to the
// "built from gates" philosophy.

// allOnes checks if all bits in a slice are 1, using AND gates.
//
// In hardware, this would be a wide AND gate:
//
//	all_ones = AND(bit[0], AND(bit[1], AND(bit[2], ...)))
//
// If ALL bits are 1, the final AND output is 1. If ANY bit is 0, it collapses to 0.
func allOnes(bits []int) bool {
	result := bits[0]
	for i := 1; i < len(bits); i++ {
		result = logicgates.AND(result, bits[i])
	}
	return result == 1
}

// allZeros checks if all bits in a slice are 0, using OR gates then NOT.
//
// In hardware: NOR across all bits.
//
//	any_one = OR(bit[0], OR(bit[1], OR(bit[2], ...)))
//	all_zeros = NOT(any_one)
//
// If ANY bit is 1, the OR chain produces 1, and we return false.
// If ALL bits are 0, the OR chain produces 0, and we return true.
func allZeros(bits []int) bool {
	result := bits[0]
	for i := 1; i < len(bits); i++ {
		result = logicgates.OR(result, bits[i])
	}
	return result == 0
}

// IsNaN checks if a FloatBits represents NaN (Not a Number).
//
// NaN is defined as: exponent = all 1s AND mantissa != all 0s.
//
// In IEEE 754, NaN is the result of undefined operations like:
//
//	0 / 0, Inf - Inf, sqrt(-1)
//
// There are two types of NaN:
//   - Quiet NaN (qNaN): mantissa MSB = 1, propagates silently
//   - Signaling NaN (sNaN): mantissa MSB = 0, raises exception
//
// We don't distinguish between them here.
func IsNaN(bits FloatBits) bool {
	return allOnes(bits.Exponent) && !allZeros(bits.Mantissa)
}

// IsInf checks if a FloatBits represents Infinity (+Inf or -Inf).
//
// Infinity is defined as: exponent = all 1s AND mantissa = all 0s.
//
// IEEE 754 uses Infinity to represent overflow results:
//
//	1e38 * 10 = +Inf (in FP32)
//	-1.0 / 0.0 = -Inf
func IsInf(bits FloatBits) bool {
	return allOnes(bits.Exponent) && allZeros(bits.Mantissa)
}

// IsZero checks if a FloatBits represents zero (+0 or -0).
//
// Zero is defined as: exponent = all 0s AND mantissa = all 0s.
//
// IEEE 754 has both +0 and -0. They compare equal (0.0 == -0.0 in Go),
// but they are different bit patterns. Having -0 is important for preserving
// the sign through operations like 1.0 / -Inf = -0.
func IsZero(bits FloatBits) bool {
	return allZeros(bits.Exponent) && allZeros(bits.Mantissa)
}

// IsDenormalized checks if a FloatBits represents a denormalized (subnormal) number.
//
// Denormalized is defined as: exponent = all 0s AND mantissa != all 0s.
//
// === What are denormalized numbers? ===
//
// Normal IEEE 754 numbers have an implicit leading 1: the value is 1.mantissa.
// But what about very small numbers close to zero? The smallest normal FP32
// number is about 1.18e-38. Without denormals, the next smaller value would
// be 0 -- a sudden jump called "the underflow gap."
//
// Denormalized numbers fill this gap. When the exponent is all zeros, the
// implicit bit becomes 0 instead of 1, and the true exponent is fixed at
// (1 - bias). This allows gradual underflow: numbers smoothly approach zero.
//
//	Normal:     1.mantissa x 2^(exp-bias)     (implicit 1)
//	Denormal:   0.mantissa x 2^(1-bias)       (implicit 0)
func IsDenormalized(bits FloatBits) bool {
	return allZeros(bits.Exponent) && !allZeros(bits.Mantissa)
}

// bitLength returns the position of the highest set bit + 1, like Python's int.bit_length().
//
// For example: bitLength(5) = 3, bitLength(1) = 1, bitLength(0) = 0.
// This is essential for normalization: we need to know where the leading 1 is.
func bitLength(v int) int {
	if v == 0 {
		return 0
	}
	n := 0
	for v > 0 {
		n++
		v >>= 1
	}
	return n
}
