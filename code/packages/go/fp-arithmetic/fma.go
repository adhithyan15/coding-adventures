package fparithmetic

// Fused Multiply-Add and format conversion.
//
// === What is FMA (Fused Multiply-Add)? ===
//
// FMA computes a * b + c with only ONE rounding step at the end. Compare:
//
//	Without FMA (separate operations):
//	    temp = FPMul(a, b)   // round #1 (loses precision)
//	    result = FPAdd(temp, c)  // round #2 (loses more precision)
//
//	With FMA:
//	    result = FMA(a, b, c)  // round only once!
//
// === Why FMA matters for ML ===
//
// In machine learning, the dominant computation is the dot product:
//
//	result = sum(a[i] * w[i] for i in range(N))
//
// Each multiply-add in the sum is a potential FMA. By rounding only once per
// operation instead of twice, FMA gives more accurate gradients during training.
//
// Every modern processor has FMA:
//   - Intel Haswell (2013): FMA3 instruction (AVX2)
//   - NVIDIA GPUs: native FMA in CUDA cores
//   - Google TPU: the MAC (Multiply-Accumulate) unit IS an FMA
//   - Apple M-series: FMA in both CPU and Neural Engine
//
// === Algorithm ===
//
//	Step 1: Multiply a * b with FULL precision (no rounding!)
//	Step 2: Align c's mantissa to the product's exponent
//	Step 3: Add the full-precision product and aligned c
//	Step 4: Normalize and round ONCE

import (
	logicgates "github.com/adhithyan15/coding-adventures/code/packages/go/logic-gates"
)

// FMA computes a * b + c with a single rounding step (fused multiply-add).
//
// === Worked example: FMA(1.5, 2.0, 0.25) in FP32 ===
//
//	a = 1.5 = 1.1 x 2^0    (exp=127, mant=1.100...0)
//	b = 2.0 = 1.0 x 2^1    (exp=128, mant=1.000...0)
//	c = 0.25 = 1.0 x 2^-2  (exp=125, mant=1.000...0)
//
//	Step 1: Full-precision multiply
//	        1.100...0 x 1.000...0 = 1.100...0 (48-bit, no rounding)
//	        Product exponent: 127 + 128 - 127 = 128 (true exp = 1)
//	        So product = 1.1 x 2^1 = 3.0
//
//	Step 2: Align c to product's exponent
//	        c = 1.0 x 2^-2, product exponent = 128
//	        Shift c right by 128 - 125 = 3 positions
//
//	Step 3: Add
//	        1.100 x 2^1 + 0.001 x 2^1 = 1.101 x 2^1
//
//	Step 4: Normalize and round
//	        Already normalized, result = 1.101 x 2^1 = 3.25
//	        Check: 1.5 * 2.0 + 0.25 = 3.0 + 0.25 = 3.25 correct!
func FMA(a, b, c FloatBits) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FMA", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("format", a.Fmt.Name)
			return rf.Generate(true, false, fmaImpl(a, b, c))
		}).GetResult()
	return result
}

func fmaImpl(a, b, c FloatBits) FloatBits {
	fmt := a.Fmt

	// ===================================================================
	// Step 0: Handle special cases
	// ===================================================================
	if IsNaN(a) || IsNaN(b) || IsNaN(c) {
		return makeNaN(fmt)
	}

	aInf := IsInf(a)
	bInf := IsInf(b)
	cInf := IsInf(c)
	aZero := IsZero(a)
	bZero := IsZero(b)

	// Inf * 0 = NaN
	if (aInf && bZero) || (bInf && aZero) {
		return makeNaN(fmt)
	}

	productSign := logicgates.XOR(a.Sign, b.Sign)

	// Inf * finite + c
	if aInf || bInf {
		if cInf && productSign != c.Sign {
			return makeNaN(fmt) // Inf + (-Inf) = NaN
		}
		return makeInf(productSign, fmt)
	}

	// a * b = 0, result is just c
	if aZero || bZero {
		if IsZero(c) {
			resultSign := logicgates.AND(productSign, c.Sign)
			return makeZero(resultSign, fmt)
		}
		return c
	}

	// c is Inf
	if cInf {
		return c
	}

	// ===================================================================
	// Step 1: Multiply a * b with full precision (no rounding!)
	// ===================================================================
	expA := BitsMSBToInt(a.Exponent)
	expB := BitsMSBToInt(b.Exponent)
	mantA := BitsMSBToInt(a.Mantissa)
	mantB := BitsMSBToInt(b.Mantissa)

	if expA != 0 {
		mantA = (1 << fmt.MantissaBits) | mantA
	} else {
		expA = 1
	}
	if expB != 0 {
		mantB = (1 << fmt.MantissaBits) | mantB
	} else {
		expB = 1
	}

	// Full-precision product: no truncation, no rounding!
	product := mantA * mantB
	productExp := expA + expB - fmt.Bias

	// Normalize the product
	productLeading := bitLength(product) - 1
	normalProductPos := 2 * fmt.MantissaBits

	if productLeading > normalProductPos {
		productExp += productLeading - normalProductPos
	} else if productLeading < normalProductPos {
		productExp -= normalProductPos - productLeading
	}

	// ===================================================================
	// Step 2: Align c's mantissa to the product's exponent
	// ===================================================================
	expC := BitsMSBToInt(c.Exponent)
	mantC := BitsMSBToInt(c.Mantissa)

	if expC != 0 {
		mantC = (1 << fmt.MantissaBits) | mantC
	} else {
		expC = 1
	}

	expDiff := productExp - expC

	cScaleShift := productLeading - fmt.MantissaBits
	var cAligned int
	if cScaleShift >= 0 {
		cAligned = mantC << cScaleShift
	} else {
		cAligned = mantC >> (-cScaleShift)
	}

	var resultExp int
	if expDiff >= 0 {
		cAligned >>= expDiff
		resultExp = productExp
	} else {
		product >>= (-expDiff)
		resultExp = expC
	}

	// ===================================================================
	// Step 3: Add product and c
	// ===================================================================
	var resultMant int
	var resultSign int
	if productSign == c.Sign {
		resultMant = product + cAligned
		resultSign = productSign
	} else {
		if product >= cAligned {
			resultMant = product - cAligned
			resultSign = productSign
		} else {
			resultMant = cAligned - product
			resultSign = c.Sign
		}
	}

	if resultMant == 0 {
		return makeZero(0, fmt)
	}

	// ===================================================================
	// Step 4: Normalize and round ONCE
	// ===================================================================
	resultLeading := bitLength(resultMant) - 1
	targetPos := productLeading
	if targetPos < fmt.MantissaBits {
		targetPos = fmt.MantissaBits
	}

	if resultLeading > targetPos {
		shift := resultLeading - targetPos
		resultExp += shift
	} else if resultLeading < targetPos {
		shiftNeeded := targetPos - resultLeading
		resultExp -= shiftNeeded
	}

	// Round to mantissaBits precision
	resultLeading = bitLength(resultMant) - 1
	roundPos := resultLeading - fmt.MantissaBits

	if roundPos > 0 {
		guard := (resultMant >> (roundPos - 1)) & 1
		var roundBit, sticky int
		if roundPos >= 2 {
			roundBit = (resultMant >> (roundPos - 2)) & 1
			mask := (1 << (roundPos - 2)) - 1
			if resultMant&mask != 0 {
				sticky = 1
			}
		}

		resultMant >>= roundPos

		// Round to nearest even
		if guard == 1 {
			if roundBit == 1 || sticky == 1 {
				resultMant++
			} else if (resultMant & 1) == 1 {
				resultMant++
			}
		}

		if resultMant >= (1 << (fmt.MantissaBits + 1)) {
			resultMant >>= 1
			resultExp++
		}
	} else if roundPos < 0 {
		resultMant <<= (-roundPos)
	}

	// Handle exponent overflow/underflow
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

	// Remove implicit leading 1
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

// FPConvert converts a floating-point number from one format to another.
//
// === Why format conversion matters ===
//
// In ML pipelines, data frequently changes precision:
//   - Training starts in FP32 (full precision)
//   - Forward pass uses FP16 or BF16 (faster, less memory)
//   - Gradients accumulated in FP32 (need precision)
//   - Weights stored as BF16 on TPU
//
// === FP32 -> BF16 conversion (trivially simple!) ===
//
// BF16 was designed so that conversion from FP32 is dead simple:
// just truncate the lower 16 bits! Both formats use the same 8-bit
// exponent with bias 127, so no exponent adjustment is needed.
//
//	FP32: [sign(1)] [exponent(8)] [mantissa(23)]
//	BF16: [sign(1)] [exponent(8)] [mantissa(7) ]
//	                               ^^^^^^^^^^^ just take the top 7 of 23
func FPConvert(bits FloatBits, targetFmt FloatFormat) FloatBits {
	result, _ := StartNew[FloatBits]("fp-arithmetic.FPConvert", FloatBits{},
		func(op *Operation[FloatBits], rf *ResultFactory[FloatBits]) *OperationResult[FloatBits] {
			op.AddProperty("sourceFormat", bits.Fmt.Name)
			op.AddProperty("targetFormat", targetFmt.Name)
			// Same format: no conversion needed
			if bits.Fmt == targetFmt {
				return rf.Generate(true, false, bits)
			}

			// Strategy: decode to Go float64, then re-encode in target format.
			// This handles all edge cases correctly.
			value := BitsToFloat(bits)
			return rf.Generate(true, false, FloatToBits(value, targetFmt))
		}).GetResult()
	return result
}
