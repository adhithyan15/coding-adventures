package fparithmetic

// Floating-point multiplication -- built from logic gates.
//
// === How FP multiplication works ===
//
// Floating-point multiplication is actually simpler than addition! That's because
// you don't need to align mantissas -- the exponents just add together.
//
// In scientific notation:
//
//	(1.5 x 10^3) x (2.0 x 10^2) = (1.5 x 2.0) x 10^(3+2) = 3.0 x 10^5
//
// The same principle applies in binary:
//
//	(-1)^s1 x 1.m1 x 2^e1  *  (-1)^s2 x 1.m2 x 2^e2
//	= (-1)^(s1 XOR s2) x (1.m1 x 1.m2) x 2^(e1 + e2)
//
// === The four steps of FP multiplication ===
//
//	Step 1: Result sign = XOR of input signs
//	        Positive x Positive = Positive (0 XOR 0 = 0)
//	        Positive x Negative = Negative (0 XOR 1 = 1)
//	        Negative x Negative = Positive (1 XOR 1 = 0)
//
//	Step 2: Result exponent = exp_a + exp_b - bias
//	        We subtract the bias once because both exponents include it.
//
//	Step 3: Multiply mantissas using shift-and-add
//	        The result is double-width (e.g., 48 bits for FP32's 24-bit mantissas).
//
//	Step 4: Normalize and round (same as addition)
//
// === Shift-and-add multiplication ===
//
// Binary multiplication works like long multiplication but simpler because
// each digit is only 0 or 1:
//
//	  1.101  (multiplicand = 1.625 in decimal)
//	x 1.011  (multiplier   = 1.375 in decimal)
//	-------
//	  1101   (1.101 x 1)     -- multiplier bit 0 is 1, add
//	 1101    (1.101 x 1)     -- multiplier bit 1 is 1, add (shifted left 1)
//	0000     (1.101 x 0)     -- multiplier bit 2 is 0, skip
//	1101      (1.101 x 1)   -- multiplier bit 3 is 1, add (shifted left 3)
//	---------
//	10.001111  = 2.234375 in decimal
//
// Check: 1.625 x 1.375 = 2.234375 correct!

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// FPMul multiplies two floating-point numbers using the IEEE 754 algorithm.
//
// === Worked example: 1.5 x 2.0 in FP32 ===
//
//	1.5 = 1.1 x 2^0    -> sign=0, exp=127, mant=100...0
//	2.0 = 1.0 x 2^1    -> sign=0, exp=128, mant=000...0
//
//	Step 1: result_sign = 0 XOR 0 = 0 (positive)
//	Step 2: result_exp = 127 + 128 - 127 = 128 (true exp = 1)
//	Step 3: mantissa product:
//	        1.100...0 x 1.000...0 = 1.100...0 (trivial case)
//	Step 4: Already normalized
//	Result: 1.1 x 2^1 = 3.0 (correct!)
func FPMul(a, b FloatBits) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FPMul", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("format", a.Fmt.Name)
			return rf.Generate(true, false, fpMulImpl(a, b))
		}).GetResult()
	return result
}

func fpMulImpl(a, b FloatBits) FloatBits {
	fmt := a.Fmt

	// ===================================================================
	// Step 0: Handle special cases
	// ===================================================================
	// IEEE 754 rules for multiplication:
	//   NaN x anything = NaN
	//   Inf x 0 = NaN
	//   Inf x finite = Inf (with appropriate sign)
	//   0 x finite = 0

	// Result sign: always XOR of input signs (even for special cases)
	resultSign := logicgates.XOR(a.Sign, b.Sign)

	// NaN propagation
	if IsNaN(a) || IsNaN(b) {
		return makeNaN(fmt)
	}

	aInf := IsInf(a)
	bInf := IsInf(b)
	aZero := IsZero(a)
	bZero := IsZero(b)

	// Inf x 0 = NaN (undefined)
	if (aInf && bZero) || (bInf && aZero) {
		return makeNaN(fmt)
	}

	// Inf x anything = Inf
	if aInf || bInf {
		return makeInf(resultSign, fmt)
	}

	// Zero x anything = Zero
	if aZero || bZero {
		return makeZero(resultSign, fmt)
	}

	// ===================================================================
	// Step 1: Extract exponents and mantissas
	// ===================================================================
	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)

	// Add implicit leading 1 for normal numbers
	if expA != 0 {
		mantA = (1 << fmt.MantissaBits) | mantA
	} else {
		expA = 1 // Denormal: true exponent = 1 - bias
	}

	if expB != 0 {
		mantB = (1 << fmt.MantissaBits) | mantB
	} else {
		expB = 1
	}

	// ===================================================================
	// Step 2: Add exponents, subtract bias
	// ===================================================================
	resultExp := expA + expB - fmt.Bias

	// ===================================================================
	// Step 3: Multiply mantissas (shift-and-add)
	// ===================================================================
	//
	// The mantissa product of two (mantissaBits+1)-bit numbers produces
	// a (2*(mantissaBits+1))-bit result. We use Go integer multiplication.
	product := mantA * mantB

	// ===================================================================
	// Step 4: Normalize
	// ===================================================================
	leadingPos := bitLength(product) - 1
	normalPos := 2 * fmt.MantissaBits

	if leadingPos > normalPos {
		extra := leadingPos - normalPos
		resultExp += extra
	} else if leadingPos < normalPos {
		deficit := normalPos - leadingPos
		resultExp -= deficit
	}

	// ===================================================================
	// Step 5: Round to nearest even
	// ===================================================================
	roundPos := leadingPos - fmt.MantissaBits

	var resultMant int
	if roundPos > 0 {
		guard := (product >> (roundPos - 1)) & 1
		var roundBit, sticky int
		if roundPos >= 2 {
			roundBit = (product >> (roundPos - 2)) & 1
			mask := (1 << (roundPos - 2)) - 1
			if product&mask != 0 {
				sticky = 1
			}
		}

		resultMant = product >> roundPos

		// Apply rounding
		if guard == 1 {
			if roundBit == 1 || sticky == 1 {
				resultMant++
			} else if (resultMant & 1) == 1 {
				resultMant++
			}
		}

		// Check if rounding caused mantissa overflow
		if resultMant >= (1 << (fmt.MantissaBits + 1)) {
			resultMant >>= 1
			resultExp++
		}
	} else if roundPos == 0 {
		resultMant = product
	} else {
		resultMant = product << (-roundPos)
	}

	// ===================================================================
	// Step 6: Handle exponent overflow/underflow
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
	// Step 7: Pack the result
	// ===================================================================
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
