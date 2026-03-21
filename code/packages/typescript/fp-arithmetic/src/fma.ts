/**
 * Fused Multiply-Add and format conversion.
 *
 * === What is FMA (Fused Multiply-Add)? ===
 *
 * FMA computes a * b + c with only ONE rounding step at the end. Compare:
 *
 *     Without FMA (separate operations):
 *         temp = fpMul(a, b)      // round #1 (loses precision)
 *         result = fpAdd(temp, c) // round #2 (loses more precision)
 *
 *     With FMA:
 *         result = fpFma(a, b, c) // round only once!
 *
 * === Why FMA matters for ML ===
 *
 * In machine learning, the dominant computation is the dot product:
 *     result = sum(a[i] * w[i] for i in range(N))
 *
 * Each multiply-add in the sum is a potential FMA. By rounding only once per
 * operation instead of twice, FMA gives more accurate gradients during training.
 * This seemingly small improvement compounds over millions of operations.
 *
 * Every modern processor has FMA:
 * - Intel Haswell (2013): FMA3 instruction (AVX2)
 * - NVIDIA GPUs: native FMA in CUDA cores
 * - Google TPU: the MAC (Multiply-Accumulate) unit IS an FMA
 * - Apple M-series: FMA in both CPU and Neural Engine
 *
 * === Algorithm ===
 *
 *     Step 1: Multiply a * b with FULL precision (no rounding!)
 *             For FP32: 24-bit x 24-bit = 48-bit product (no information lost)
 *
 *     Step 2: Align c's mantissa to the product's exponent
 *             (same as the alignment step in fpAdd)
 *
 *     Step 3: Add the full-precision product and aligned c
 *
 *     Step 4: Normalize and round ONCE
 *
 * The key insight is Step 1: by keeping the full 48-bit product without rounding,
 * we preserve all the information for the final result. The separate mul+add
 * approach throws away bits in the intermediate rounding, which can never be
 * recovered.
 *
 * === Format conversion ===
 *
 * This module also provides fpConvert() for converting between FP32, FP16, and
 * BF16. Format conversion is essentially re-encoding: decode the value, then
 * encode in the target format (possibly losing precision).
 */

import { type FloatBits, type FloatFormat } from "./formats.js";
import {
  bitsMsbToInt,
  intToBitsMsb,
  bitsToFloat,
  floatToBits,
  isInf,
  isNaN,
  isZero,
  bitLength,
} from "./ieee754.js";

/**
 * Fused multiply-add: compute a * b + c with single rounding.
 *
 * === Worked example: FMA(1.5, 2.0, 0.25) in FP32 ===
 *
 *     a = 1.5 = 1.1 x 2^0    (exp=127, mant=1.100...0)
 *     b = 2.0 = 1.0 x 2^1    (exp=128, mant=1.000...0)
 *     c = 0.25 = 1.0 x 2^-2  (exp=125, mant=1.000...0)
 *
 *     Step 1: Full-precision multiply
 *             1.100...0 x 1.000...0 = 1.100...0 (48-bit, no rounding)
 *             Product exponent: 127 + 128 - 127 = 128 (true exp = 1)
 *             So product = 1.1 x 2^1 = 3.0
 *
 *     Step 2: Align c to product's exponent
 *             c = 1.0 x 2^-2, product exponent = 128
 *             Shift c right by 128 - 125 = 3 positions
 *             cAligned = 0.001 x 2^1
 *
 *     Step 3: Add
 *             1.100 x 2^1 + 0.001 x 2^1 = 1.101 x 2^1
 *
 *     Step 4: Normalize and round
 *             Already normalized, result = 1.101 x 2^1 = 3.25
 *             Check: 1.5 * 2.0 + 0.25 = 3.0 + 0.25 = 3.25 correct!
 *
 * @param a - First multiplicand.
 * @param b - Second multiplicand.
 * @param c - Addend.
 * @returns a * b + c as FloatBits, with only one rounding step.
 */
export function fpFma(a: FloatBits, b: FloatBits, c: FloatBits): FloatBits {
  const fmt = a.fmt;

  // ===================================================================
  // Step 0: Handle special cases
  // ===================================================================
  // NaN propagation
  if (isNaN(a) || isNaN(b) || isNaN(c)) {
    return {
      sign: 0,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
      fmt,
    };
  }

  const aInf = isInf(a);
  const bInf = isInf(b);
  const cInf = isInf(c);
  const aZero = isZero(a);
  const bZero = isZero(b);

  // Inf * 0 = NaN
  if ((aInf && bZero) || (bInf && aZero)) {
    return {
      sign: 0,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
      fmt,
    };
  }

  const productSign = a.sign ^ b.sign;

  // Inf * finite + c
  if (aInf || bInf) {
    if (cInf && productSign !== c.sign) {
      // Inf + (-Inf) = NaN
      return {
        sign: 0,
        exponent: new Array(fmt.exponentBits).fill(1),
        mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
        fmt,
      };
    }
    return {
      sign: productSign,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  // a * b = 0, result is just c (but we need to handle 0 + 0 sign)
  if (aZero || bZero) {
    if (isZero(c)) {
      // 0 + 0: sign depends on rounding mode, default to +0
      // unless both are negative
      const resultSign = productSign & c.sign; // AND gate
      return {
        sign: resultSign,
        exponent: new Array(fmt.exponentBits).fill(0),
        mantissa: new Array(fmt.mantissaBits).fill(0),
        fmt,
      };
    }
    return c;
  }

  // c is Inf
  if (cInf) return c;

  // ===================================================================
  // Step 1: Multiply a * b with full precision (no rounding!)
  // ===================================================================
  let expA = bitsMsbToInt(a.exponent);
  let expB = bitsMsbToInt(b.exponent);
  let mantA = BigInt(bitsMsbToInt(a.mantissa));
  let mantB = BigInt(bitsMsbToInt(b.mantissa));

  // Add implicit leading 1 for normal numbers
  if (expA !== 0) {
    mantA = (1n << BigInt(fmt.mantissaBits)) | mantA;
  } else {
    expA = 1;
  }

  if (expB !== 0) {
    mantB = (1n << BigInt(fmt.mantissaBits)) | mantB;
  } else {
    expB = 1;
  }

  // Full-precision product: no truncation, no rounding!
  // For FP32: 24-bit x 24-bit = up to 48-bit result
  let product = mantA * mantB;

  // Product exponent (before normalization)
  let productExp = expA + expB - fmt.bias;

  // Normalize the product
  let productLeading = bitLength(product) - 1;
  const normalProductPos = 2 * fmt.mantissaBits;

  if (productLeading > normalProductPos) {
    productExp += productLeading - normalProductPos;
  } else if (productLeading < normalProductPos) {
    productExp -= normalProductPos - productLeading;
  }

  // ===================================================================
  // Step 2: Align c's mantissa to the product's exponent
  // ===================================================================
  let expC = bitsMsbToInt(c.exponent);
  let mantC = BigInt(bitsMsbToInt(c.mantissa));

  if (expC !== 0) {
    mantC = (1n << BigInt(fmt.mantissaBits)) | mantC;
  } else {
    expC = 1;
  }

  // The product has (productLeading + 1) bits.
  // c has (mantissaBits + 1) bits.
  // We need to align them to the same exponent.

  // Scale c's mantissa to match product's bit width
  const expDiff = productExp - expC;

  // Use a wide enough workspace for the addition
  const cScaleShift = productLeading - fmt.mantissaBits;
  let cAligned: bigint;
  if (cScaleShift >= 0) {
    cAligned = mantC << BigInt(cScaleShift);
  } else {
    cAligned = mantC >> BigInt(-cScaleShift);
  }

  let resultExp: number;
  if (expDiff >= 0) {
    // Product exponent >= c exponent: shift c right
    cAligned >>= BigInt(expDiff);
    resultExp = productExp;
  } else {
    // c exponent > product exponent: shift product right
    product >>= BigInt(-expDiff);
    resultExp = expC;
  }

  // ===================================================================
  // Step 3: Add product and c
  // ===================================================================
  let resultMant: bigint;
  let resultSign: number;
  if (productSign === c.sign) {
    resultMant = product + cAligned;
    resultSign = productSign;
  } else {
    if (product >= cAligned) {
      resultMant = product - cAligned;
      resultSign = productSign;
    } else {
      resultMant = cAligned - product;
      resultSign = c.sign;
    }
  }

  // Handle zero result
  if (resultMant === 0n) {
    return {
      sign: 0,
      exponent: new Array(fmt.exponentBits).fill(0),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  // ===================================================================
  // Step 4: Normalize and round ONCE
  // ===================================================================
  // Find the leading 1 in the result
  let resultLeading = bitLength(resultMant) - 1;
  // The target position for the leading 1
  const targetPos =
    productLeading > fmt.mantissaBits ? productLeading : fmt.mantissaBits;

  if (resultLeading > targetPos) {
    const shift = resultLeading - targetPos;
    resultExp += shift;
  } else if (resultLeading < targetPos) {
    const shiftNeeded = targetPos - resultLeading;
    resultExp -= shiftNeeded;
  }

  // Now round to mantissaBits precision
  resultLeading = bitLength(resultMant) - 1;
  const roundPos = resultLeading - fmt.mantissaBits;

  if (roundPos > 0) {
    const guard = Number((resultMant >> BigInt(roundPos - 1)) & 1n);
    let roundBit = 0;
    let sticky = 0;
    if (roundPos >= 2) {
      roundBit = Number((resultMant >> BigInt(roundPos - 2)) & 1n);
      sticky =
        (resultMant & ((1n << BigInt(roundPos - 2)) - 1n)) !== 0n ? 1 : 0;
    }

    resultMant >>= BigInt(roundPos);

    // Round to nearest even
    if (guard === 1) {
      if (roundBit === 1 || sticky === 1) {
        resultMant += 1n;
      } else if ((resultMant & 1n) === 1n) {
        resultMant += 1n;
      }
    }

    // Check rounding overflow
    if (resultMant >= 1n << BigInt(fmt.mantissaBits + 1)) {
      resultMant >>= 1n;
      resultExp += 1;
    }
  } else if (roundPos < 0) {
    resultMant <<= BigInt(-roundPos);
  }

  // Handle exponent overflow/underflow
  const maxExp = (1 << fmt.exponentBits) - 1;

  if (resultExp >= maxExp) {
    return {
      sign: resultSign,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  if (resultExp <= 0) {
    if (resultExp < -fmt.mantissaBits) {
      return {
        sign: resultSign,
        exponent: new Array(fmt.exponentBits).fill(0),
        mantissa: new Array(fmt.mantissaBits).fill(0),
        fmt,
      };
    }
    const shift = 1 - resultExp;
    resultMant >>= BigInt(shift);
    resultExp = 0;
  }

  // Remove implicit leading 1
  if (resultExp > 0) {
    resultMant &= (1n << BigInt(fmt.mantissaBits)) - 1n;
  }

  return {
    sign: resultSign,
    exponent: intToBitsMsb(resultExp, fmt.exponentBits),
    mantissa: intToBitsMsb(Number(resultMant), fmt.mantissaBits),
    fmt,
  };
}

// ---------------------------------------------------------------------------
// Format conversion: FP32 <-> FP16 <-> BF16
// ---------------------------------------------------------------------------

/**
 * Convert a floating-point number from one format to another.
 *
 * === Why format conversion matters ===
 *
 * In ML pipelines, data frequently changes precision:
 * - Training starts in FP32 (full precision)
 * - Forward pass uses FP16 or BF16 (faster, less memory)
 * - Gradients accumulated in FP32 (need precision)
 * - Weights stored as BF16 on TPU
 *
 * Each conversion potentially loses precision (if going to a smaller format)
 * or is exact (if going to a larger format).
 *
 * === FP32 -> BF16 conversion (trivially simple!) ===
 *
 * BF16 was designed so that conversion from FP32 is dead simple:
 * just truncate the lower 16 bits! Both formats use the same 8-bit
 * exponent with bias 127, so no exponent adjustment is needed.
 *
 *     FP32: [sign(1)] [exponent(8)] [mantissa(23)]
 *     BF16: [sign(1)] [exponent(8)] [mantissa(7) ]
 *                                    ^^^^^^^^^^^ just take the top 7 of 23
 *
 * This is why Google chose this format for TPU: the conversion circuit
 * is essentially free (just wires, no logic gates needed).
 *
 * @param bits - The source FloatBits to convert.
 * @param targetFmt - The target FloatFormat.
 * @returns The value in the target format (possibly with precision loss).
 *
 * Example:
 *     const fp32Val = floatToBits(3.14, FP32);
 *     const bf16Val = fpConvert(fp32Val, BF16);
 *     bitsToFloat(bf16Val)  // Less precise: 3.140625
 */
export function fpConvert(
  bits: FloatBits,
  targetFmt: FloatFormat
): FloatBits {
  // Same format: no conversion needed
  if (bits.fmt === targetFmt) return bits;

  // Strategy: decode to JavaScript number, then re-encode in target format.
  // This handles all the edge cases (denormals, rounding, overflow)
  // correctly by leveraging our existing encode/decode functions.
  //
  // A hardware implementation would directly manipulate the bit fields
  // (adjust exponent bias, truncate/extend mantissa), but for educational
  // purposes, the decode-then-encode approach is clearer and provably correct.
  const value = bitsToFloat(bits);
  return floatToBits(value, targetFmt);
}
