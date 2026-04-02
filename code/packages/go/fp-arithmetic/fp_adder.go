package fparithmetic

// Floating-point addition and subtraction -- built from logic gates.
//
// === How FP addition works at the hardware level ===
//
// Adding two floating-point numbers is surprisingly complex compared to integer
// addition. The core difficulty is that the two numbers might have very different
// exponents, so their mantissas are "misaligned" and must be shifted before they
// can be added.
//
// Consider adding 1.5 + 0.125 in decimal scientific notation:
//
//	1.5 x 10^0  +  1.25 x 10^-1
//
// You can't just add 1.5 + 1.25 because they have different exponents. First,
// you align them to the same exponent:
//
//	1.5   x 10^0
//	0.125 x 10^0   (shifted 1.25 right by 1 decimal place)
//	-------------
//	1.625 x 10^0
//
// Binary FP addition follows the exact same principle, but with binary mantissas
// and power-of-2 exponents.
//
// === The five steps of FP addition ===
//
//	Step 1: Compare exponents
//	        Subtract exponents to find the difference.
//	        The number with the smaller exponent gets shifted.
//
//	Step 2: Align mantissas
//	        Shift the smaller number's mantissa right by the exponent
//	        difference. This is like converting 0.125 to line up with 1.5.
//
//	Step 3: Add or subtract mantissas
//	        If signs are the same: add mantissas
//	        If signs differ: subtract the smaller from the larger
//
//	Step 4: Normalize
//	        The result might not be in 1.xxx form. Adjust:
//	        - If overflow (10.xxx): shift right, increment exponent
//	        - If underflow (0.0xxx): shift left, decrement exponent
//
//	Step 5: Round
//	        The result might have more bits than the format allows.
//	        Round to fit, using "round to nearest even" (banker's rounding).

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// FPAdd adds two floating-point numbers using the IEEE 754 algorithm.
//
// This implements the full addition algorithm:
//  1. Handle special cases (NaN, Inf, Zero)
//  2. Compare exponents
//  3. Align mantissas
//  4. Add/subtract mantissas
//  5. Normalize result
//  6. Round to nearest even
//
// === Worked example: 1.5 + 0.25 in FP32 ===
//
//	1.5 = 1.1 x 2^0    -> exp=127, mant=10000...0
//	0.25 = 1.0 x 2^-2   -> exp=125, mant=00000...0
//
//	Step 1: exp_diff = 127 - 125 = 2 (b has smaller exponent)
//	Step 2: Shift b's mantissa right by 2:
//	        1.10000...0  (a, with implicit 1)
//	        0.01000...0  (b, shifted right by 2)
//	Step 3: Add:  1.10000...0 + 0.01000...0 = 1.11000...0
//	Step 4: Already normalized (starts with 1.)
//	Step 5: No rounding needed (exact)
//	Result: 1.11 x 2^0 = 1.75 (correct!)
func FPAdd(a, b FloatBits) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FPAdd", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("format", a.Fmt.Name)
			return rf.Generate(true, false, fpAddImpl(a, b))
		}).GetResult()
	return result
}

func fpAddImpl(a, b FloatBits) FloatBits {
	fmt := a.Fmt

	// ===================================================================
	// Step 0: Handle special cases
	// ===================================================================
	// IEEE 754 defines strict rules for special values:
	//   NaN + anything = NaN
	//   Inf + (-Inf) = NaN
	//   Inf + x = Inf (for finite x)
	//   0 + x = x

	// NaN propagation: any NaN input produces NaN output
	if IsNaN(a) || IsNaN(b) {
		return makeNaN(fmt)
	}

	// Infinity handling
	aInf := IsInf(a)
	bInf := IsInf(b)
	if aInf && bInf {
		if a.Sign == b.Sign {
			return makeInf(a.Sign, fmt)
		}
		// Inf + (-Inf) = NaN
		return makeNaN(fmt)
	}
	if aInf {
		return a
	}
	if bInf {
		return b
	}

	// Zero handling
	aZero := IsZero(a)
	bZero := IsZero(b)
	if aZero && bZero {
		// +0 + +0 = +0, -0 + -0 = -0, +0 + -0 = +0
		resultSign := logicgates.AND(a.Sign, b.Sign)
		return makeZero(resultSign, fmt)
	}
	if aZero {
		return b
	}
	if bZero {
		return a
	}

	// ===================================================================
	// Step 1: Extract exponents and mantissas as integers
	// ===================================================================
	//
	// We work with extended mantissas that include the implicit leading bit.
	// For normal numbers, this is 1; for denormals, it's 0.
	//
	// We also add extra guard bits for rounding precision. The guard bits
	// are: Guard (G), Round (R), and Sticky (S) -- 3 extra bits that capture
	// information about bits that would otherwise be lost during shifting.

	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)

	// Add implicit leading 1 for normal numbers (exponent != 0)
	if expA != 0 {
		mantA = (1 << fmt.MantissaBits) | mantA
	} else {
		expA = 1 // Denormal true exponent = 1 - bias, stored as 1 for alignment
	}
	if expB != 0 {
		mantB = (1 << fmt.MantissaBits) | mantB
	} else {
		expB = 1
	}

	// Add 3 guard bits (shift left by 3) for rounding precision
	guardBits := 3
	mantA <<= guardBits
	mantB <<= guardBits

	// ===================================================================
	// Step 2: Align mantissas by shifting the smaller one right
	// ===================================================================

	var resultExp int
	if expA >= expB {
		expDiff := expA - expB
		if expDiff > 0 && expDiff < (fmt.MantissaBits+1+guardBits) {
			shiftedOut := mantB & ((1 << expDiff) - 1)
			sticky := 0
			if shiftedOut != 0 {
				sticky = 1
			}
			mantB >>= expDiff
			if sticky != 0 && expDiff > 0 {
				mantB |= 1
			}
		} else if expDiff > 0 {
			sticky := 0
			if mantB != 0 {
				sticky = 1
			}
			mantB >>= expDiff
			if sticky != 0 {
				mantB |= 1
			}
		}
		resultExp = expA
	} else {
		expDiff := expB - expA
		if expDiff > 0 && expDiff < (fmt.MantissaBits+1+guardBits) {
			shiftedOut := mantA & ((1 << expDiff) - 1)
			sticky := 0
			if shiftedOut != 0 {
				sticky = 1
			}
			mantA >>= expDiff
			if sticky != 0 && expDiff > 0 {
				mantA |= 1
			}
		} else if expDiff > 0 {
			sticky := 0
			if mantA != 0 {
				sticky = 1
			}
			mantA >>= expDiff
			if sticky != 0 {
				mantA |= 1
			}
		}
		resultExp = expB
	}

	// ===================================================================
	// Step 3: Add or subtract mantissas based on signs
	// ===================================================================

	var resultMant int
	var resultSign int
	if a.Sign == b.Sign {
		resultMant = mantA + mantB
		resultSign = a.Sign
	} else {
		if mantA >= mantB {
			resultMant = mantA - mantB
			resultSign = a.Sign
		} else {
			resultMant = mantB - mantA
			resultSign = b.Sign
		}
	}

	// ===================================================================
	// Step 4: Handle zero result
	// ===================================================================
	if resultMant == 0 {
		return makeZero(0, fmt) // +0 by convention
	}

	// ===================================================================
	// Step 5: Normalize the result
	// ===================================================================
	//
	// The result mantissa should be in the form 1.xxxx (the leading 1 in
	// position mantissaBits + guardBits).

	normalPos := fmt.MantissaBits + guardBits
	leadingPos := bitLength(resultMant) - 1

	if leadingPos > normalPos {
		// Overflow: shift right to normalize
		shiftAmount := leadingPos - normalPos
		lostBits := resultMant & ((1 << shiftAmount) - 1)
		resultMant >>= shiftAmount
		if lostBits != 0 {
			resultMant |= 1 // sticky
		}
		resultExp += shiftAmount
	} else if leadingPos < normalPos {
		// Underflow: shift left to normalize
		shiftAmount := normalPos - leadingPos
		if resultExp-shiftAmount >= 1 {
			resultMant <<= shiftAmount
			resultExp -= shiftAmount
		} else {
			// Can't shift all the way -- result becomes denormal
			actualShift := resultExp - 1
			if actualShift > 0 {
				resultMant <<= actualShift
			}
			resultExp = 0
		}
	}

	// ===================================================================
	// Step 6: Round to nearest even
	// ===================================================================
	//
	// Round to nearest even rules:
	//   - If GRS = 0xx: round down (truncate)
	//   - If GRS = 100: round to even (round up if mantissa LSB is 1)
	//   - If GRS = 101, 110, 111: round up

	guard := (resultMant >> (guardBits - 1)) & 1
	roundBit := (resultMant >> (guardBits - 2)) & 1
	stickyBit := resultMant & ((1 << (guardBits - 2)) - 1)
	if stickyBit != 0 {
		stickyBit = 1
	}

	// Remove guard bits
	resultMant >>= guardBits

	// Apply rounding
	if guard == 1 {
		if roundBit == 1 || stickyBit == 1 {
			resultMant++ // Round up
		} else if (resultMant & 1) == 1 {
			resultMant++ // Tie-breaking: round to even
		}
	}

	// Check if rounding caused overflow
	if resultMant >= (1 << (fmt.MantissaBits + 1)) {
		resultMant >>= 1
		resultExp++
	}

	// ===================================================================
	// Step 7: Handle exponent overflow/underflow
	// ===================================================================
	maxExp := (1 << fmt.ExponentBits) - 1

	if resultExp >= maxExp {
		return makeInf(resultSign, fmt)
	}

	if resultExp <= 0 {
		if resultExp < -(fmt.MantissaBits) {
			return makeZero(resultSign, fmt)
		}
		shift := 1 - resultExp
		resultMant >>= shift
		resultExp = 0
	}

	// ===================================================================
	// Step 8: Pack the result
	// ===================================================================
	// Remove the implicit leading 1 (if normal)
	if resultExp > 0 {
		resultMant &= (1 << fmt.MantissaBits) - 1
	}

	return FloatBits{
		Sign:     resultSign,
		Exponent: IntToBitsMSB(resultExp, fmt.ExponentBits),
		Mantissa: IntToBitsMSB(resultMant, fmt.MantissaBits),
		Fmt:      fmt,
	}
}

// FPSub subtracts two floating-point numbers: a - b.
//
// === Why subtraction is trivial once you have addition ===
//
// In IEEE 754, a - b = a + (-b). To negate b, we just flip its sign bit.
// This is a single XOR gate in hardware -- the cheapest possible operation.
func FPSub(a, b FloatBits) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FPSub", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("format", a.Fmt.Name)
			negB := FloatBits{
				Sign:     logicgates.XOR(b.Sign, 1),
				Exponent: b.Exponent,
				Mantissa: b.Mantissa,
				Fmt:      b.Fmt,
			}
			return rf.Generate(true, false, FPAdd(a, negB))
		}).GetResult()
	return result
}

// FPNeg negates a floating-point number: return -a.
//
// This is the simplest floating-point operation: just flip the sign bit.
// In hardware, it's literally one NOT gate (or XOR with 1).
//
// Note: neg(+0) = -0 and neg(-0) = +0. Both are valid IEEE 754 zeros.
func FPNeg(a FloatBits) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FPNeg", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("format", a.Fmt.Name)
			return rf.Generate(true, false, FloatBits{
				Sign:     logicgates.XOR(a.Sign, 1),
				Exponent: a.Exponent,
				Mantissa: a.Mantissa,
				Fmt:      a.Fmt,
			})
		}).GetResult()
	return result
}

// FPAbs returns the absolute value of a floating-point number.
//
// Even simpler than negation: just force the sign bit to 0.
// In hardware, this is done by AND-ing the sign bit with 0.
//
// Note: abs(NaN) is still NaN (with sign=0). This is the IEEE 754 behavior.
func FPAbs(a FloatBits) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FPAbs", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("format", a.Fmt.Name)
			return rf.Generate(true, false, FloatBits{
				Sign:     0,
				Exponent: a.Exponent,
				Mantissa: a.Mantissa,
				Fmt:      a.Fmt,
			})
		}).GetResult()
	return result
}

// FPCompare compares two floating-point numbers.
//
// Returns:
//
//	-1 if a < b
//	 0 if a == b
//	 1 if a > b
//
// NaN comparisons always return 0 (unordered).
//
// === How FP comparison works in hardware ===
//
// For two positive normal numbers:
//   - Compare exponents first (larger exponent = larger number)
//   - If exponents equal, compare mantissas
//
// For mixed signs: positive > negative (always).
// For two negative numbers: comparison is reversed.
func FPCompare(a, b FloatBits) int {
	result, _ := StartNew[int]("fp-arithmetic.FPCompare", 0,
		func(op *Operation[int], rf *ResultFactory[int]) *OperationResult[int] {
			op.AddProperty("format", a.Fmt.Name)
			return rf.Generate(true, false, fpCompareImpl(a, b))
		}).GetResult()
	return result
}

func fpCompareImpl(a, b FloatBits) int {
	// NaN is unordered
	if IsNaN(a) || IsNaN(b) {
		return 0
	}

	// Handle zeros: +0 == -0
	if IsZero(a) && IsZero(b) {
		return 0
	}

	// Different signs: positive > negative
	if a.Sign != b.Sign {
		if IsZero(a) {
			if b.Sign == 1 {
				return 1
			}
			return -1
		}
		if IsZero(b) {
			if a.Sign == 1 {
				return -1
			}
			return 1
		}
		if a.Sign == 1 {
			return -1
		}
		return 1
	}

	// Same sign: compare exponent, then mantissa
	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)

	if expA != expB {
		if a.Sign == 0 {
			if expA > expB {
				return 1
			}
			return -1
		}
		if expA > expB {
			return -1
		}
		return 1
	}

	if mantA != mantB {
		if a.Sign == 0 {
			if mantA > mantB {
				return 1
			}
			return -1
		}
		if mantA > mantB {
			return -1
		}
		return 1
	}

	return 0
}
