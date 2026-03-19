/**
 * IEEE 754 encoding and decoding -- converting between JavaScript numbers and bits.
 *
 * === How does a computer store 3.14? ===
 *
 * When you write `const x = 3.14` in JavaScript, the computer stores it as 64 bits
 * following the IEEE 754 standard. This module converts between JavaScript's native
 * number representation and our explicit bit-level representation (FloatBits).
 *
 * === Encoding: number -> bits ===
 *
 * For FP32, we use JavaScript's DataView to get the exact same bit pattern
 * that the hardware uses. For FP16 and BF16, we manually extract the bits
 * because JavaScript doesn't natively support these formats.
 *
 * === Special values in IEEE 754 ===
 *
 * IEEE 754 reserves certain bit patterns for special values:
 *
 *     Exponent      Mantissa    Meaning
 *     ----------    --------    -------
 *     All 1s        All 0s      +/- Infinity
 *     All 1s        Non-zero    NaN (Not a Number)
 *     All 0s        All 0s      +/- Zero
 *     All 0s        Non-zero    Denormalized number (very small, near zero)
 *     Other         Any         Normal number
 *
 * These special values allow floating-point to handle edge cases gracefully:
 * - 1.0 / 0.0 = Inf (not a crash!)
 * - 0.0 / 0.0 = NaN (undefined, but doesn't crash)
 * - Denormals allow "gradual underflow" near zero
 *
 * === Why BigInt? ===
 *
 * JavaScript numbers are 64-bit doubles, which can only represent integers
 * exactly up to 2^53. Since FP32 mantissa products can reach 48 bits,
 * we use BigInt for all bit-level manipulation to avoid precision loss.
 * BigInt provides arbitrary-precision integer arithmetic, similar to
 * Python's native int.
 */

import {
  type FloatBits,
  type FloatFormat,
  FP32,
  FP16,
  BF16,
} from "./formats.js";

// ---------------------------------------------------------------------------
// Helper: integer <-> bit array conversions
// ---------------------------------------------------------------------------

/**
 * Convert a non-negative integer to an array of bits, MSB first.
 *
 * This is the fundamental conversion between JavaScript integers and our
 * bit-level representation.
 *
 * Example:
 *     intToBitsMsb(5, 8)
 *     // => [0, 0, 0, 0, 0, 1, 0, 1]
 *     //     128 64 32 16  8  4  2  1
 *     //                   4     1  = 5
 *
 * How it works:
 *     We check each bit position from the most significant (leftmost) to
 *     the least significant (rightmost). For each position i (counting from
 *     width-1 down to 0), we check if that bit is set using a right-shift
 *     and AND with 1.
 *
 * @param value - The integer to convert (must be >= 0).
 * @param width - The number of bits in the output array.
 * @returns An array of 0s and 1s, MSB first, with exactly `width` elements.
 */
export function intToBitsMsb(value: number, width: number): number[] {
  const bits: number[] = new Array(width);
  for (let i = 0; i < width; i++) {
    bits[i] = (value >>> (width - 1 - i)) & 1;
  }
  return bits;
}

/**
 * Convert an array of bits (MSB first) back to a non-negative integer.
 *
 * This is the inverse of intToBitsMsb.
 *
 * Example:
 *     bitsMsbToInt([0, 0, 0, 0, 0, 1, 0, 1])
 *     // => 5
 *     // Each bit contributes: bit_value * 2^position
 *     // 0*128 + 0*64 + 0*32 + 0*16 + 0*8 + 1*4 + 0*2 + 1*1 = 5
 *
 * How it works:
 *     We iterate through the bits from MSB to LSB. For each bit, we shift
 *     the accumulator left by 1 (multiply by 2) and OR in the new bit.
 *     This is equivalent to: sum(bit * 2^(width-1-i) for each bit)
 *
 * @param bits - Array of 0s and 1s, MSB first.
 * @returns The integer value represented by the bits.
 */
export function bitsMsbToInt(bits: readonly number[]): number {
  let result = 0;
  for (const bit of bits) {
    result = (result << 1) | bit;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Helper: fill arrays
// ---------------------------------------------------------------------------

/** Create an array of n zeros. */
function zeros(n: number): number[] {
  return new Array(n).fill(0);
}

/** Create an array of n ones. */
function ones(n: number): number[] {
  return new Array(n).fill(1);
}

// ---------------------------------------------------------------------------
// Encoding: JavaScript number -> FloatBits
// ---------------------------------------------------------------------------

/**
 * Convert a JavaScript number to its IEEE 754 bit representation.
 *
 * === How FP32 encoding works (using DataView) ===
 *
 * For FP32, we leverage JavaScript's DataView which gives us access to the
 * exact bit pattern that the hardware uses:
 *
 *     const buf = new ArrayBuffer(4);
 *     new DataView(buf).setFloat32(0, 3.14);
 *     // Now buf contains the raw IEEE 754 bytes
 *
 * We then read those 4 bytes as a 32-bit unsigned integer to get the raw bits.
 *
 * === How FP16/BF16 encoding works (manual) ===
 *
 * For FP16 and BF16, JavaScript doesn't have native support, so we:
 * 1. First encode as FP32 (which we know is exact for the hardware)
 * 2. Extract the sign, exponent, and mantissa from the FP32 encoding
 * 3. Re-encode into the target format, adjusting exponent bias and
 *    truncating the mantissa
 *
 * === Worked example: encoding 3.14 as FP32 ===
 *
 *     3.14 in binary: 11.00100011110101110000101...
 *     Normalized:     1.100100011110101110000101... x 2^1
 *
 *     Sign:     0 (positive)
 *     Exponent: 1 + 127 (bias) = 128 = 10000000 in binary
 *     Mantissa: 10010001111010111000010 (23 bits after the implicit 1)
 *               ^-- note: the leading 1 is NOT stored
 *
 *     Packed: 0 10000000 10010001111010111000011
 *             s exponent         mantissa
 *
 * @param value - The JavaScript number to encode.
 * @param fmt - The target format (FP32, FP16, or BF16). Default is FP32.
 * @returns FloatBits with the sign, exponent, and mantissa bit arrays.
 *
 * Example:
 *     const bits = floatToBits(1.0, FP32);
 *     bits.sign       // 0
 *     bits.exponent   // [0, 1, 1, 1, 1, 1, 1, 1]  (127 in binary)
 *     bits.mantissa   // [0, 0, 0, ..., 0]  (23 zeros)
 */
export function floatToBits(value: number, fmt: FloatFormat = FP32): FloatBits {
  // --- Handle NaN specially ---
  // JavaScript has NaN, and IEEE 754 defines NaN as exponent=all-1s,
  // mantissa=non-zero. We use a "quiet NaN" with the MSB of mantissa set to 1.
  if (Number.isNaN(value)) {
    return {
      sign: 0,
      exponent: ones(fmt.exponentBits),
      mantissa: [1, ...zeros(fmt.mantissaBits - 1)],
      fmt,
    };
  }

  // --- Handle Infinity ---
  // +Inf and -Inf: exponent=all-1s, mantissa=all-0s.
  if (!Number.isFinite(value)) {
    const sign = value > 0 ? 0 : 1;
    return {
      sign,
      exponent: ones(fmt.exponentBits),
      mantissa: zeros(fmt.mantissaBits),
      fmt,
    };
  }

  // --- FP32: use DataView for hardware-exact encoding ---
  if (fmt === FP32) {
    // DataView.setFloat32 gives us the raw IEEE 754 representation.
    // DataView.getUint32 reads those bytes as an unsigned 32-bit int.
    const buf = new ArrayBuffer(4);
    const view = new DataView(buf);
    view.setFloat32(0, value);
    const intBits = view.getUint32(0);

    // Extract the three fields using bit shifts and masks:
    //   Bit 31:     sign
    //   Bits 30-23: exponent (8 bits)
    //   Bits 22-0:  mantissa (23 bits)
    const sign = (intBits >>> 31) & 1;
    const expInt = (intBits >>> 23) & 0xff;
    const mantInt = intBits & 0x7fffff;

    return {
      sign,
      exponent: intToBitsMsb(expInt, 8),
      mantissa: intToBitsMsb(mantInt, 23),
      fmt: FP32,
    };
  }

  // --- FP16 and BF16: manual conversion from FP32 ---
  //
  // Strategy: encode as FP32 first, then convert.
  // This handles all the tricky cases (denormals, rounding) correctly
  // because the FP32 encoding uses hardware-exact DataView.

  const fp32Bits = floatToBits(value, FP32);
  const fp32Exp = bitsMsbToInt(fp32Bits.exponent);
  const fp32Mant = bitsMsbToInt(fp32Bits.mantissa);
  const sign = fp32Bits.sign;

  // --- Handle zero ---
  if (fp32Exp === 0 && fp32Mant === 0) {
    return {
      sign,
      exponent: zeros(fmt.exponentBits),
      mantissa: zeros(fmt.mantissaBits),
      fmt,
    };
  }

  // --- Compute the true (unbiased) exponent ---
  // For normal FP32 numbers: trueExp = storedExp - 127
  // For denormal FP32 numbers: trueExp = 1 - 127 = -126
  let trueExp: number;
  let fullMantissa: number;
  if (fp32Exp === 0) {
    // Denormal in FP32: true exponent is -126, implicit bit is 0
    trueExp = 1 - FP32.bias; // = -126
    // Denormal mantissa: no implicit 1, so full mantissa is 0.mantissa
    fullMantissa = fp32Mant;
  } else {
    trueExp = fp32Exp - FP32.bias;
    // Normal: full mantissa includes the implicit leading 1
    fullMantissa = (1 << FP32.mantissaBits) | fp32Mant;
  }

  // --- Map to target format ---
  let targetExp = trueExp + fmt.bias;
  const maxExp = (1 << fmt.exponentBits) - 1; // All 1s = special

  // --- Overflow: exponent too large for target format -> Infinity ---
  if (targetExp >= maxExp) {
    return {
      sign,
      exponent: ones(fmt.exponentBits),
      mantissa: zeros(fmt.mantissaBits),
      fmt,
    };
  }

  // --- Normal case: exponent fits in target format ---
  if (targetExp > 0) {
    let truncated: number;
    // Truncate mantissa from 23 bits to fmt.mantissaBits
    // We take the top bits and apply round-to-nearest-even
    if (fmt.mantissaBits < FP32.mantissaBits) {
      const shift = FP32.mantissaBits - fmt.mantissaBits;
      truncated = fp32Mant >>> shift;
      // Round-to-nearest-even: check the bit just below the truncation point
      const roundBit = (fp32Mant >>> (shift - 1)) & 1;
      const sticky = fp32Mant & ((1 << (shift - 1)) - 1);
      if (roundBit !== 0 && (sticky !== 0 || (truncated & 1) !== 0)) {
        truncated += 1;
        // Rounding overflow: mantissa exceeded max, carry into exponent
        if (truncated >= 1 << fmt.mantissaBits) {
          truncated = 0;
          targetExp += 1;
          if (targetExp >= maxExp) {
            return {
              sign,
              exponent: ones(fmt.exponentBits),
              mantissa: zeros(fmt.mantissaBits),
              fmt,
            };
          }
        }
      }
    } else {
      truncated = fp32Mant << (fmt.mantissaBits - FP32.mantissaBits);
    }

    return {
      sign,
      exponent: intToBitsMsb(targetExp, fmt.exponentBits),
      mantissa: intToBitsMsb(truncated, fmt.mantissaBits),
      fmt,
    };
  }

  // --- Underflow: number is too small for normal representation ---
  // It might still be representable as a denormal in the target format.
  // Denormal: exponent=0, mantissa encodes the value directly.
  //
  // The shift amount tells us how many bits we lose going denormal.
  const denormShift = 1 - targetExp; // how far below the minimum normal exponent

  if (denormShift > fmt.mantissaBits) {
    // Too small even for denormal -> flush to zero
    return {
      sign,
      exponent: zeros(fmt.exponentBits),
      mantissa: zeros(fmt.mantissaBits),
      fmt,
    };
  }

  // Shift the full mantissa right to create a denormal
  // fullMantissa has (mantissaBits + 1) bits (including implicit 1)
  // We need to fit it into fmt.mantissaBits after shifting
  const denormMant =
    fullMantissa >>> (denormShift + FP32.mantissaBits - fmt.mantissaBits);

  return {
    sign,
    exponent: zeros(fmt.exponentBits),
    mantissa: intToBitsMsb(
      denormMant & ((1 << fmt.mantissaBits) - 1),
      fmt.mantissaBits
    ),
    fmt,
  };
}

// ---------------------------------------------------------------------------
// Decoding: FloatBits -> JavaScript number
// ---------------------------------------------------------------------------

/**
 * Convert an IEEE 754 bit representation back to a JavaScript number.
 *
 * === How decoding works ===
 *
 * For FP32, we reconstruct the 32-bit integer and use DataView to get
 * the exact JavaScript number. For FP16/BF16, we manually compute the value
 * using the formula:
 *
 *     value = (-1)^sign x 2^(exponent - bias) x 1.mantissa
 *
 * === Worked example: decoding FP32 bits for 3.14 ===
 *
 *     Sign: 0 -> positive
 *     Exponent: 10000000 -> 128 -> true exponent = 128 - 127 = 1
 *     Mantissa: 10010001111010111000011
 *
 *     Value = +1.0 x 2^1 x (1 + 0.5 + 0.0625 + ...)
 *           = 2 x 1.5700000524520874
 *           = 3.140000104904175
 *
 * @param bits - The FloatBits to decode.
 * @returns The JavaScript number value.
 *
 * Example:
 *     const bits = floatToBits(3.14, FP32);
 *     bitsToFloat(bits)  // 3.140000104904175
 */
export function bitsToFloat(bits: FloatBits): number {
  const expInt = bitsMsbToInt(bits.exponent);
  const mantInt = bitsMsbToInt(bits.mantissa);
  const maxExp = (1 << bits.fmt.exponentBits) - 1;

  // --- Special values ---

  // NaN: exponent all 1s, mantissa non-zero
  if (expInt === maxExp && mantInt !== 0) {
    return NaN;
  }

  // Infinity: exponent all 1s, mantissa all zeros
  if (expInt === maxExp && mantInt === 0) {
    return bits.sign === 1 ? -Infinity : Infinity;
  }

  // Zero: exponent all 0s, mantissa all zeros
  if (expInt === 0 && mantInt === 0) {
    // IEEE 754 has both +0 and -0
    return bits.sign === 1 ? -0 : 0;
  }

  // --- For FP32, use DataView for exact conversion ---
  if (bits.fmt === FP32) {
    // Reconstruct the 32-bit integer
    const intBits = ((bits.sign << 31) | (expInt << 23) | mantInt) >>> 0;
    const buf = new ArrayBuffer(4);
    const view = new DataView(buf);
    view.setUint32(0, intBits);
    return view.getFloat32(0);
  }

  // --- For FP16/BF16, compute the float value manually ---

  let trueExp: number;
  let mantissaValue: number;

  // Denormalized: exponent=0, implicit bit is 0
  if (expInt === 0) {
    // value = (-1)^sign x 2^(1-bias) x 0.mantissa
    // The mantissa represents a fraction: mantInt / 2^mantissaBits
    trueExp = 1 - bits.fmt.bias;
    mantissaValue = mantInt / (1 << bits.fmt.mantissaBits);
  } else {
    // Normal: implicit leading 1
    // value = (-1)^sign x 2^(exponent-bias) x 1.mantissa
    trueExp = expInt - bits.fmt.bias;
    mantissaValue = 1.0 + mantInt / (1 << bits.fmt.mantissaBits);
  }

  let result = mantissaValue * 2.0 ** trueExp;
  if (bits.sign === 1) {
    result = -result;
  }

  return result;
}

// ---------------------------------------------------------------------------
// Special value detection -- using logic gate operations
// ---------------------------------------------------------------------------
// These functions detect special IEEE 754 values by examining the bit pattern.
// We use AND and OR operations to check bit fields, staying true to
// the "built from gates" philosophy.

/**
 * Check if all bits in an array are 1, using AND gate logic.
 *
 * In hardware, this would be a wide AND gate:
 *     allOnes = AND(bit[0], AND(bit[1], AND(bit[2], ...)))
 *
 * If ALL bits are 1, the final AND output is 1.
 * If ANY bit is 0, the chain collapses to 0.
 *
 * Example:
 *     allOnes([1, 1, 1, 1])  // true
 *     allOnes([1, 0, 1, 1])  // false
 */
export function allOnes(bits: readonly number[]): boolean {
  let result = bits[0];
  for (let i = 1; i < bits.length; i++) {
    result = result & bits[i]; // AND gate
  }
  return result === 1;
}

/**
 * Check if all bits in an array are 0, using OR gate logic then NOT.
 *
 * In hardware: NOR across all bits.
 *     anyOne = OR(bit[0], OR(bit[1], OR(bit[2], ...)))
 *     allZeros = NOT(anyOne)
 *
 * If ANY bit is 1, the OR chain produces 1, and we return false.
 * If ALL bits are 0, the OR chain produces 0, and we return true.
 *
 * Example:
 *     allZeros([0, 0, 0, 0])  // true
 *     allZeros([0, 0, 1, 0])  // false
 */
export function allZeros(bits: readonly number[]): boolean {
  let result = bits[0];
  for (let i = 1; i < bits.length; i++) {
    result = result | bits[i]; // OR gate
  }
  return result === 0;
}

/**
 * Check if a FloatBits represents NaN (Not a Number).
 *
 * NaN is defined as: exponent = all 1s AND mantissa != all 0s.
 *
 * In IEEE 754, NaN is the result of undefined operations like:
 *     0 / 0, Inf - Inf, sqrt(-1)
 *
 * There are two types of NaN:
 * - Quiet NaN (qNaN): mantissa MSB = 1, propagates silently
 * - Signaling NaN (sNaN): mantissa MSB = 0, raises exception
 *
 * We don't distinguish between them here.
 *
 * @param bits - The FloatBits to check.
 * @returns true if the value is NaN.
 *
 * Example:
 *     isNaN(floatToBits(NaN))   // true
 *     isNaN(floatToBits(1.0))   // false
 */
export function isNaN(bits: FloatBits): boolean {
  return allOnes(bits.exponent) && !allZeros(bits.mantissa);
}

/**
 * Check if a FloatBits represents Infinity (+Inf or -Inf).
 *
 * Infinity is defined as: exponent = all 1s AND mantissa = all 0s.
 *
 * IEEE 754 uses Infinity to represent overflow results:
 *     1e38 * 10 = +Inf (in FP32)
 *     -1.0 / 0.0 = -Inf
 *
 * @param bits - The FloatBits to check.
 * @returns true if the value is +Inf or -Inf.
 *
 * Example:
 *     isInf(floatToBits(Infinity))   // true
 *     isInf(floatToBits(-Infinity))  // true
 *     isInf(floatToBits(1.0))        // false
 */
export function isInf(bits: FloatBits): boolean {
  return allOnes(bits.exponent) && allZeros(bits.mantissa);
}

/**
 * Check if a FloatBits represents zero (+0 or -0).
 *
 * Zero is defined as: exponent = all 0s AND mantissa = all 0s.
 *
 * IEEE 754 has both +0 and -0. They compare equal (0.0 === -0.0 is true
 * in JavaScript), but they are different bit patterns. Having -0 is important
 * for preserving the sign through operations like 1.0 / -Inf = -0.
 *
 * @param bits - The FloatBits to check.
 * @returns true if the value is +0 or -0.
 *
 * Example:
 *     isZero(floatToBits(0.0))    // true
 *     isZero(floatToBits(-0.0))   // true
 *     isZero(floatToBits(1.0))    // false
 */
export function isZero(bits: FloatBits): boolean {
  return allZeros(bits.exponent) && allZeros(bits.mantissa);
}

/**
 * Check if a FloatBits represents a denormalized (subnormal) number.
 *
 * Denormalized is defined as: exponent = all 0s AND mantissa != all 0s.
 *
 * === What are denormalized numbers? ===
 *
 * Normal IEEE 754 numbers have an implicit leading 1: the value is 1.mantissa.
 * But what about very small numbers close to zero? The smallest normal FP32
 * number is about 1.18e-38. Without denormals, the next smaller value would
 * be 0 -- a sudden jump called "the underflow gap."
 *
 * Denormalized numbers fill this gap. When the exponent is all zeros, the
 * implicit bit becomes 0 instead of 1, and the true exponent is fixed at
 * (1 - bias). This allows gradual underflow: numbers smoothly approach zero
 * rather than jumping to it.
 *
 *     Normal:     1.mantissa x 2^(exp-bias)     (implicit 1)
 *     Denormal:   0.mantissa x 2^(1-bias)       (implicit 0)
 *
 * The smallest positive denormal in FP32 is:
 *     0.00000000000000000000001 x 2^(-126) = 2^(-149) ~ 1.4e-45
 *
 * @param bits - The FloatBits to check.
 * @returns true if the value is denormalized.
 *
 * Example:
 *     // The smallest positive FP32 denormal
 *     const tiny: FloatBits = {
 *       sign: 0,
 *       exponent: new Array(8).fill(0),
 *       mantissa: [...new Array(22).fill(0), 1],
 *       fmt: FP32,
 *     };
 *     isDenormalized(tiny)  // true
 */
export function isDenormalized(bits: FloatBits): boolean {
  return allZeros(bits.exponent) && !allZeros(bits.mantissa);
}

/**
 * Compute the number of bits needed to represent a non-negative integer.
 *
 * Equivalent to Python's int.bit_length(). Returns the position of the
 * highest set bit + 1. For zero, returns 0.
 *
 * Examples:
 *     bitLength(0n)  // 0
 *     bitLength(1n)  // 1
 *     bitLength(5n)  // 3
 *
 * This is essential for normalization: we need to know where the leading 1 is.
 */
export function bitLength(v: bigint): number {
  if (v === 0n) return 0;
  let n = 0;
  let x = v;
  while (x > 0n) {
    n++;
    x >>= 1n;
  }
  return n;
}
