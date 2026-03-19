/**
 * Floating-point multiplication -- built from logic gates.
 *
 * === How FP multiplication works ===
 *
 * Floating-point multiplication is actually simpler than addition! That's because
 * you don't need to align mantissas -- the exponents just add together.
 *
 * In scientific notation:
 *     (1.5 x 10^3) x (2.0 x 10^2) = (1.5 x 2.0) x 10^(3+2) = 3.0 x 10^5
 *
 * The same principle applies in binary:
 *     (-1)^s1 x 1.m1 x 2^e1  *  (-1)^s2 x 1.m2 x 2^e2
 *     = (-1)^(s1 XOR s2) x (1.m1 x 1.m2) x 2^(e1 + e2)
 *
 * === The four steps of FP multiplication ===
 *
 *     Step 1: Result sign = XOR of input signs
 *             Positive x Positive = Positive (0 XOR 0 = 0)
 *             Positive x Negative = Negative (0 XOR 1 = 1)
 *             Negative x Negative = Positive (1 XOR 1 = 0)
 *
 *     Step 2: Result exponent = expA + expB - bias
 *             We subtract the bias once because both exponents include it:
 *             trueExpA = storedA - bias
 *             trueExpB = storedB - bias
 *             trueResult = trueA + trueB
 *             storedResult = trueResult + bias = storedA + storedB - bias
 *
 *     Step 3: Multiply mantissas using shift-and-add
 *             This is the core of the operation. For each bit of one mantissa,
 *             if that bit is 1, we add the other mantissa shifted by that position.
 *             The result is double-width (e.g., 48 bits for FP32's 24-bit mantissas).
 *
 *     Step 4: Normalize and round (same as addition)
 *
 * === Shift-and-add multiplication ===
 *
 * Binary multiplication works exactly like long multiplication you learned in
 * school, but simpler because each digit is only 0 or 1:
 *
 *     1.101  (multiplicand = 1.625 in decimal)
 *   x 1.011  (multiplier   = 1.375 in decimal)
 *   -------
 *     1101   (1.101 x 1)     -- multiplier bit 0 is 1, so add
 *    1101    (1.101 x 1)     -- multiplier bit 1 is 1, so add (shifted left 1)
 *   0000     (1.101 x 0)     -- multiplier bit 2 is 0, skip
 *  1101      (1.101 x 1)     -- multiplier bit 3 is 1, so add (shifted left 3)
 *  ---------
 *  10.001111  = 2.234375 in decimal
 *
 * Check: 1.625 x 1.375 = 2.234375 correct!
 *
 * In hardware, each "if bit is 1, add shifted value" is an AND gate (to gate
 * the addition) followed by a ripple_carry_adder. Our implementation uses
 * BigInt for clarity, but the algorithm is the same.
 */

import { type FloatBits, type FloatFormat } from "./formats.js";
import {
  bitsMsbToInt,
  intToBitsMsb,
  isInf,
  isNaN,
  isZero,
  bitLength,
} from "./ieee754.js";

/**
 * Multiply two floating-point numbers using logic gates.
 *
 * Implements the IEEE 754 multiplication algorithm:
 * 1. Handle special cases (NaN, Inf, Zero)
 * 2. XOR signs
 * 3. Add exponents, subtract bias
 * 4. Multiply mantissas (shift-and-add)
 * 5. Normalize and round
 *
 * === Worked example: 1.5 x 2.0 in FP32 ===
 *
 *     1.5 = 1.1 x 2^0    -> sign=0, exp=127, mant=100...0
 *     2.0 = 1.0 x 2^1    -> sign=0, exp=128, mant=000...0
 *
 *     Step 1: resultSign = 0 XOR 0 = 0 (positive)
 *     Step 2: resultExp = 127 + 128 - 127 = 128 (true exp = 1)
 *     Step 3: mantissa product:
 *             1.100...0 x 1.000...0 = 1.100...0 (trivial case)
 *     Step 4: Already normalized
 *     Result: 1.1 x 2^1 = 3.0 (correct!)
 *
 * @param a - First operand as FloatBits.
 * @param b - Second operand as FloatBits. Must use the same FloatFormat as a.
 * @returns The product as FloatBits in the same format.
 */
export function fpMul(a: FloatBits, b: FloatBits): FloatBits {
  const fmt = a.fmt;

  // ===================================================================
  // Step 0: Handle special cases
  // ===================================================================
  // IEEE 754 rules for multiplication:
  //   NaN x anything = NaN
  //   Inf x 0 = NaN
  //   Inf x finite = Inf (with appropriate sign)
  //   0 x finite = 0

  // Result sign: always XOR of input signs (even for special cases)
  const resultSign = a.sign ^ b.sign;

  // NaN propagation
  if (isNaN(a) || isNaN(b)) {
    return {
      sign: 0,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
      fmt,
    };
  }

  const aInf = isInf(a);
  const bInf = isInf(b);
  const aZero = isZero(a);
  const bZero = isZero(b);

  // Inf x 0 = NaN (undefined)
  if ((aInf && bZero) || (bInf && aZero)) {
    return {
      sign: 0,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
      fmt,
    };
  }

  // Inf x anything = Inf
  if (aInf || bInf) {
    return {
      sign: resultSign,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  // Zero x anything = Zero
  if (aZero || bZero) {
    return {
      sign: resultSign,
      exponent: new Array(fmt.exponentBits).fill(0),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  // ===================================================================
  // Step 1: Extract exponents and mantissas
  // ===================================================================
  let expA = bitsMsbToInt(a.exponent);
  let expB = bitsMsbToInt(b.exponent);
  let mantA = BigInt(bitsMsbToInt(a.mantissa));
  let mantB = BigInt(bitsMsbToInt(b.mantissa));

  // Add implicit leading 1 for normal numbers
  if (expA !== 0) {
    mantA = (1n << BigInt(fmt.mantissaBits)) | mantA;
  } else {
    expA = 1; // Denormal: true exponent = 1 - bias
  }

  if (expB !== 0) {
    mantB = (1n << BigInt(fmt.mantissaBits)) | mantB;
  } else {
    expB = 1;
  }

  // ===================================================================
  // Step 2: Add exponents, subtract bias
  // ===================================================================
  //
  // resultExp = expA + expB - bias
  //
  // Why subtract bias? Both expA and expB include the bias:
  //   trueA = expA - bias
  //   trueB = expB - bias
  //   trueResult = trueA + trueB = (expA - bias) + (expB - bias)
  //   storedResult = trueResult + bias = expA + expB - bias

  let resultExp = expA + expB - fmt.bias;

  // ===================================================================
  // Step 3: Multiply mantissas (shift-and-add)
  // ===================================================================
  //
  // The mantissa product of two (mantissaBits+1)-bit numbers produces
  // a (2*(mantissaBits+1))-bit result.
  //
  // For FP32: 24-bit x 24-bit = 48-bit product.
  //
  // We use BigInt multiplication here because the shift-and-add
  // at the JS level would be identical in result but much slower.
  // The important thing is understanding that hardware does it as shift-and-add.

  let product = mantA * mantB;
  // The product has up to 2*(mantissaBits+1) bits

  // ===================================================================
  // Step 4: Normalize
  // ===================================================================
  //
  // The product of two 1.xxx numbers is between 1.0 and 3.999..., so
  // the product is either 1x.xxx or 01.xxx in binary.

  const leadingPos = bitLength(product) - 1;
  const normalPos = 2 * fmt.mantissaBits; // Where leading 1 should be

  if (leadingPos > normalPos) {
    // Product overflowed: 1x.xxx form, shift right
    const extra = leadingPos - normalPos;
    resultExp += extra;
  } else if (leadingPos < normalPos) {
    // Product is smaller than expected (happens with denormals)
    const deficit = normalPos - leadingPos;
    resultExp -= deficit;
  }

  // ===================================================================
  // Step 5: Round to nearest even
  // ===================================================================
  //
  // We need to reduce the product from ~48 bits to 24 bits (for FP32).
  // The bits below the mantissa field determine rounding.

  // How many bits are below the mantissa in the product?
  const roundPos = leadingPos - fmt.mantissaBits;
  let resultMant: bigint;

  if (roundPos > 0) {
    // Extract guard, round, sticky bits for rounding
    const guard = Number((product >> BigInt(roundPos - 1)) & 1n);
    let roundBit = 0;
    let sticky = 0;
    if (roundPos >= 2) {
      roundBit = Number((product >> BigInt(roundPos - 2)) & 1n);
      sticky =
        (product & ((1n << BigInt(roundPos - 2)) - 1n)) !== 0n ? 1 : 0;
    }

    // Truncate to mantissa width + 1 (including implicit 1)
    resultMant = product >> BigInt(roundPos);

    // Apply rounding
    if (guard === 1) {
      if (roundBit === 1 || sticky === 1) {
        resultMant += 1n;
      } else if ((resultMant & 1n) === 1n) {
        resultMant += 1n;
      }
    }

    // Check if rounding caused mantissa overflow
    if (resultMant >= 1n << BigInt(fmt.mantissaBits + 1)) {
      resultMant >>= 1n;
      resultExp += 1;
    }
  } else if (roundPos === 0) {
    resultMant = product;
  } else {
    // Product is very small, shift left
    resultMant = product << BigInt(-roundPos);
  }

  // ===================================================================
  // Step 6: Handle exponent overflow/underflow
  // ===================================================================
  const maxExp = (1 << fmt.exponentBits) - 1;

  if (resultExp >= maxExp) {
    // Overflow to infinity
    return {
      sign: resultSign,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  if (resultExp <= 0) {
    // Denormal or underflow
    if (resultExp < -fmt.mantissaBits) {
      return {
        sign: resultSign,
        exponent: new Array(fmt.exponentBits).fill(0),
        mantissa: new Array(fmt.mantissaBits).fill(0),
        fmt,
      };
    }
    // Shift mantissa right to make it denormal
    const shift = 1 - resultExp;
    resultMant >>= BigInt(shift);
    resultExp = 0;
  }

  // ===================================================================
  // Step 7: Pack the result
  // ===================================================================
  // Remove the implicit leading 1 (if normal)
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
