/**
 * Floating-point addition and subtraction -- built from logic gates.
 *
 * === How FP addition works at the hardware level ===
 *
 * Adding two floating-point numbers is surprisingly complex compared to integer
 * addition. The core difficulty is that the two numbers might have very different
 * exponents, so their mantissas are "misaligned" and must be shifted before they
 * can be added.
 *
 * Consider adding 1.5 + 0.125 in decimal scientific notation:
 *     1.5 x 10^0  +  1.25 x 10^-1
 *
 * You can't just add 1.5 + 1.25 because they have different exponents. First,
 * you align them to the same exponent:
 *     1.5   x 10^0
 *     0.125 x 10^0   (shifted 1.25 right by 1 decimal place)
 *     -------------
 *     1.625 x 10^0
 *
 * Binary FP addition follows the exact same principle, but with binary mantissas
 * and power-of-2 exponents.
 *
 * === The five steps of FP addition ===
 *
 *     Step 1: Compare exponents
 *             Subtract exponents to find the difference.
 *             The number with the smaller exponent gets shifted.
 *
 *     Step 2: Align mantissas
 *             Shift the smaller number's mantissa right by the exponent
 *             difference. This is like converting 0.125 to line up with 1.5.
 *
 *     Step 3: Add or subtract mantissas
 *             If signs are the same: add mantissas
 *             If signs differ: subtract the smaller from the larger
 *
 *     Step 4: Normalize
 *             The result might not be in 1.xxx form. Adjust:
 *             - If overflow (10.xxx): shift right, increment exponent
 *             - If underflow (0.0xxx): shift left, decrement exponent
 *
 *     Step 5: Round
 *             The result might have more bits than the format allows.
 *             Round to fit, using "round to nearest even" (banker's rounding).
 *
 * === Why this is slow but clear ===
 *
 * A real hardware FPU does all of this in 1-3 clock cycles using parallel
 * circuits (barrel shifters, leading-zero anticipators, etc.). Our implementation
 * is sequential and uses simple loops, which is much slower but much easier to
 * understand. Every step maps directly to the algorithm described above.
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

// ---------------------------------------------------------------------------
// Helper: shift mantissa right by N positions
// ---------------------------------------------------------------------------

/**
 * Shift a bit array right by `amount` positions, filling with zeros.
 *
 * In hardware, this would be a barrel shifter built from layers of MUX gates.
 * Each layer shifts by a power of 2 (1, 2, 4, 8, ...) controlled by one bit
 * of the shift amount. Our implementation is simpler: just a loop.
 *
 * Example:
 *     shiftRight([1, 0, 1, 1], 2)
 *     // => [0, 0, 1, 0]
 *     //  The bits shifted in from the left are 0s.
 *     //  The bits shifted out on the right are lost.
 *
 * The bits that fall off the right are lost (truncated). In a full hardware
 * implementation, we'd keep track of the "sticky bit" (OR of all lost bits)
 * for rounding. We handle rounding separately.
 *
 * @param bits - Bit array, MSB first.
 * @param amount - Number of positions to shift right.
 * @returns New bit array with zeros shifted in from the left.
 */
export function shiftRight(bits: readonly number[], amount: number): number[] {
  if (amount <= 0) return [...bits];
  if (amount >= bits.length) return new Array(bits.length).fill(0);
  return [
    ...new Array(amount).fill(0),
    ...bits.slice(0, bits.length - amount),
  ];
}

/**
 * Shift a bit array left by `amount` positions, filling with zeros.
 *
 * Example:
 *     shiftLeft([1, 0, 1, 1], 2)
 *     // => [1, 1, 0, 0]
 *
 * @param bits - Bit array, MSB first.
 * @param amount - Number of positions to shift left.
 * @returns New bit array with zeros shifted in from the right.
 */
export function shiftLeft(bits: readonly number[], amount: number): number[] {
  if (amount <= 0) return [...bits];
  if (amount >= bits.length) return new Array(bits.length).fill(0);
  return [...bits.slice(amount), ...new Array(amount).fill(0)];
}

// ---------------------------------------------------------------------------
// Helper: find the position of the leading 1 (most significant set bit)
// ---------------------------------------------------------------------------

/**
 * Find the index of the first 1 bit in an array (MSB first).
 *
 * In hardware, this is called a "leading-one detector" or "priority encoder."
 * It's built from a tree of OR gates. Our implementation is a simple scan.
 *
 * Returns -1 if all bits are 0.
 *
 * Example:
 *     findLeadingOne([0, 0, 1, 0, 1])  // 2
 *     findLeadingOne([0, 0, 0, 0, 0])  // -1
 */
export function findLeadingOne(bits: readonly number[]): number {
  for (let i = 0; i < bits.length; i++) {
    if (bits[i] === 1) return i;
  }
  return -1;
}

// ---------------------------------------------------------------------------
// Helper: two's complement subtraction
// ---------------------------------------------------------------------------

/**
 * Subtract two unsigned numbers: a - b using two's complement.
 *
 * To compute a - b, we use the identity:
 *     a - b = a + NOT(b) + 1
 *
 * This is how ALL subtraction works in binary hardware. There is no
 * dedicated subtraction circuit -- it's always addition with negation.
 *
 * @param aBits - First number as bits, MSB first.
 * @param bBits - Second number as bits, MSB first.
 * @returns [result_bits_msb, borrow] where borrow=1 if b > a.
 */
export function subtractUnsigned(
  aBits: readonly number[],
  bBits: readonly number[]
): [number[], number] {
  const width = aBits.length;
  // Convert MSB-first to LSB-first for ripple carry add
  const aLsb = [...aBits].reverse();
  const bLsb = [...bBits].reverse();

  // NOT(b) = one's complement
  const bInvLsb = bLsb.map((bit) => bit ^ 1);

  // a + NOT(b) + 1 = a - b in two's complement
  const [resultLsb, carry] = rippleCarryAdd(aLsb, bInvLsb, 1);

  // carry=1 means no borrow (result is non-negative)
  // carry=0 means borrow (result is negative, i.e., b > a)
  const borrow = carry ^ 1;

  return [resultLsb.reverse(), borrow];
}

/**
 * Add two bit arrays (LSB first) using ripple carry addition.
 *
 * @param a - First operand, LSB first.
 * @param b - Second operand, LSB first. Must be same length as a.
 * @param carryIn - Initial carry bit (0 or 1).
 * @returns [result_lsb, carry_out] where result is LSB first.
 */
function rippleCarryAdd(
  a: number[],
  b: number[],
  carryIn: number
): [number[], number] {
  const result: number[] = new Array(a.length);
  let carry = carryIn;
  for (let i = 0; i < a.length; i++) {
    const sum = a[i] + b[i] + carry;
    result[i] = sum & 1;
    carry = sum >> 1;
  }
  return [result, carry];
}

// ---------------------------------------------------------------------------
// Helper: add two bit arrays (MSB first) using ripple carry
// ---------------------------------------------------------------------------

/**
 * Add two bit arrays (MSB first) using ripple carry addition.
 *
 * rippleCarryAdd expects LSB-first, so we reverse, add, and reverse back.
 *
 * @param a - First operand, MSB first.
 * @param b - Second operand, MSB first. Must be same length as a.
 * @param carryIn - Initial carry bit (0 or 1).
 * @returns [result_msb, carry_out] where result is MSB first.
 */
export function addBitsMsb(
  a: readonly number[],
  b: readonly number[],
  carryIn: number = 0
): [number[], number] {
  const aLsb = [...a].reverse();
  const bLsb = [...b].reverse();
  const [resultLsb, carry] = rippleCarryAdd(aLsb, bLsb, carryIn);
  return [resultLsb.reverse(), carry];
}

// ---------------------------------------------------------------------------
// Core: fpAdd -- floating-point addition from logic gates
// ---------------------------------------------------------------------------

/**
 * Add two floating-point numbers using logic gates.
 *
 * This implements the full IEEE 754 addition algorithm:
 * 1. Handle special cases (NaN, Inf, Zero)
 * 2. Compare exponents
 * 3. Align mantissas
 * 4. Add/subtract mantissas
 * 5. Normalize result
 * 6. Round to nearest even
 *
 * === Worked example: 1.5 + 0.25 in FP32 ===
 *
 *     1.5 = 1.1 x 2^0    -> exp=127, mant=10000...0
 *     0.25 = 1.0 x 2^-2   -> exp=125, mant=00000...0
 *
 *     Step 1: expDiff = 127 - 125 = 2 (b has smaller exponent)
 *     Step 2: Shift b's mantissa right by 2:
 *             1.10000...0  (a, with implicit 1)
 *             0.01000...0  (b, shifted right by 2)
 *     Step 3: Add:  1.10000...0 + 0.01000...0 = 1.11000...0
 *     Step 4: Already normalized (starts with 1.)
 *     Step 5: No rounding needed (exact)
 *     Result: 1.11 x 2^0 = 1.75 (correct!)
 *
 * We use BigInt for mantissa manipulation because mantissas with guard bits
 * can exceed 32 bits, and JavaScript's bitwise operators only work on 32-bit
 * integers.
 *
 * @param a - First operand as FloatBits.
 * @param b - Second operand as FloatBits. Must use the same FloatFormat as a.
 * @returns The sum as FloatBits in the same format.
 */
export function fpAdd(a: FloatBits, b: FloatBits): FloatBits {
  const fmt = a.fmt;

  // ===================================================================
  // Step 0: Handle special cases
  // ===================================================================
  // IEEE 754 defines strict rules for special values:
  //   NaN + anything = NaN
  //   Inf + (-Inf) = NaN
  //   Inf + x = Inf (for finite x)
  //   0 + x = x

  // NaN propagation: any NaN input produces NaN output
  if (isNaN(a) || isNaN(b)) {
    return {
      sign: 0,
      exponent: new Array(fmt.exponentBits).fill(1),
      mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
      fmt,
    };
  }

  // Infinity handling
  const aInf = isInf(a);
  const bInf = isInf(b);
  if (aInf && bInf) {
    // Inf + Inf = Inf (same sign) or NaN (different signs)
    if (a.sign === b.sign) {
      return {
        sign: a.sign,
        exponent: new Array(fmt.exponentBits).fill(1),
        mantissa: new Array(fmt.mantissaBits).fill(0),
        fmt,
      };
    } else {
      // Inf + (-Inf) = NaN
      return {
        sign: 0,
        exponent: new Array(fmt.exponentBits).fill(1),
        mantissa: [1, ...new Array(fmt.mantissaBits - 1).fill(0)],
        fmt,
      };
    }
  }
  if (aInf) return a;
  if (bInf) return b;

  // Zero handling
  const aZero = isZero(a);
  const bZero = isZero(b);
  if (aZero && bZero) {
    // +0 + +0 = +0, -0 + -0 = -0, +0 + -0 = +0
    const resultSign = a.sign & b.sign; // AND gate
    return {
      sign: resultSign,
      exponent: new Array(fmt.exponentBits).fill(0),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }
  if (aZero) return b;
  if (bZero) return a;

  // ===================================================================
  // Step 1: Extract exponents and mantissas as BigInts
  // ===================================================================
  //
  // We work with extended mantissas that include the implicit leading bit.
  // For normal numbers, this is 1; for denormals, it's 0.
  //
  // We also add extra guard bits for rounding precision. The guard bits
  // are: Guard (G), Round (R), and Sticky (S) -- 3 extra bits that capture
  // information about the bits that would otherwise be lost during shifting.
  //
  //   [implicit_1] [mantissa bits] [G] [R] [S]
  //    1 bit        N bits          1   1   1

  let expA = bitsMsbToInt(a.exponent);
  let expB = bitsMsbToInt(b.exponent);
  let mantA = BigInt(bitsMsbToInt(a.mantissa));
  let mantB = BigInt(bitsMsbToInt(b.mantissa));

  // Add implicit leading 1 for normal numbers (exponent != 0)
  // For denormals (exponent == 0), the implicit bit is 0
  if (expA !== 0) {
    mantA = (1n << BigInt(fmt.mantissaBits)) | mantA;
  } else {
    expA = 1; // Denormal true exponent = 1 - bias, stored as 1 for alignment
  }
  if (expB !== 0) {
    mantB = (1n << BigInt(fmt.mantissaBits)) | mantB;
  } else {
    expB = 1;
  }

  // Add 3 guard bits (shift left by 3) for rounding precision
  const guardBits = 3;
  mantA <<= BigInt(guardBits);
  mantB <<= BigInt(guardBits);

  // ===================================================================
  // Step 2: Align mantissas by shifting the smaller one right
  // ===================================================================
  //
  // If expA > expB, then b has a smaller magnitude per mantissa bit,
  // so we shift b's mantissa right by (expA - expB) positions.
  //
  // Example:
  //   1.5 = 1.1 x 2^0   (exp=127)
  //   0.25 = 1.0 x 2^-2  (exp=125)
  //   Shift 0.25's mantissa right by 2: 001.0 -> 0.01

  let resultExp: number;
  if (expA >= expB) {
    const expDiff = expA - expB;
    // Before shifting, save the sticky bits (all bits that will be shifted out)
    if (expDiff > 0 && expDiff < fmt.mantissaBits + 1 + guardBits) {
      const shiftedOut = mantB & ((1n << BigInt(expDiff)) - 1n);
      const sticky = shiftedOut !== 0n ? 1n : 0n;
      mantB >>= BigInt(expDiff);
      if (sticky !== 0n && expDiff > 0) {
        mantB |= 1n; // Set the sticky bit (LSB)
      }
    } else if (expDiff > 0) {
      const sticky = mantB !== 0n ? 1n : 0n;
      mantB >>= BigInt(expDiff);
      if (sticky !== 0n) {
        mantB |= 1n;
      }
    }
    resultExp = expA;
  } else {
    const expDiff = expB - expA;
    if (expDiff > 0 && expDiff < fmt.mantissaBits + 1 + guardBits) {
      const shiftedOut = mantA & ((1n << BigInt(expDiff)) - 1n);
      const sticky = shiftedOut !== 0n ? 1n : 0n;
      mantA >>= BigInt(expDiff);
      if (sticky !== 0n && expDiff > 0) {
        mantA |= 1n;
      }
    } else if (expDiff > 0) {
      const sticky = mantA !== 0n ? 1n : 0n;
      mantA >>= BigInt(expDiff);
      if (sticky !== 0n) {
        mantA |= 1n;
      }
    }
    resultExp = expB;
  }

  // ===================================================================
  // Step 3: Add or subtract mantissas based on signs
  // ===================================================================
  //
  // If signs are the same: add mantissas, keep the sign
  // If signs differ: subtract the smaller from the larger
  //
  // In hardware, subtraction is done by adding the two's complement.

  let resultMant: bigint;
  let resultSign: number;
  if (a.sign === b.sign) {
    // Same sign: simple addition
    resultMant = mantA + mantB;
    resultSign = a.sign;
  } else {
    // Different signs: subtract smaller from larger
    if (mantA >= mantB) {
      resultMant = mantA - mantB;
      resultSign = a.sign;
    } else {
      resultMant = mantB - mantA;
      resultSign = b.sign;
    }
  }

  // ===================================================================
  // Step 4: Handle zero result
  // ===================================================================
  if (resultMant === 0n) {
    return {
      sign: 0, // +0 by convention
      exponent: new Array(fmt.exponentBits).fill(0),
      mantissa: new Array(fmt.mantissaBits).fill(0),
      fmt,
    };
  }

  // ===================================================================
  // Step 5: Normalize the result
  // ===================================================================
  //
  // The result mantissa should be in the form 1.xxxx (the leading 1 in
  // position mantissaBits + guardBits).
  //
  // If the result is too large (e.g., 10.xxx from overflow), shift right
  // and increment the exponent.
  //
  // If the result is too small (e.g., 0.001xxx from cancellation), shift
  // left and decrement the exponent.

  // The "normal" position for the leading 1 is at bit (mantissaBits + guardBits)
  const normalPos = fmt.mantissaBits + guardBits;

  // Find where the leading 1 actually is
  const leadingPos = bitLength(resultMant) - 1;

  if (leadingPos > normalPos) {
    // Overflow: shift right to normalize
    const shiftAmount = leadingPos - normalPos;
    // Save bits being shifted out for rounding
    const lostBits = resultMant & ((1n << BigInt(shiftAmount)) - 1n);
    resultMant >>= BigInt(shiftAmount);
    if (lostBits !== 0n) {
      resultMant |= 1n; // sticky
    }
    resultExp += shiftAmount;
  } else if (leadingPos < normalPos) {
    // Underflow: shift left to normalize
    const shiftAmount = normalPos - leadingPos;
    if (resultExp - shiftAmount >= 1) {
      resultMant <<= BigInt(shiftAmount);
      resultExp -= shiftAmount;
    } else {
      // Can't shift all the way -- result becomes denormal
      const actualShift = resultExp - 1;
      if (actualShift > 0) {
        resultMant <<= BigInt(actualShift);
      }
      resultExp = 0;
    }
  }

  // ===================================================================
  // Step 6: Round to nearest even
  // ===================================================================
  //
  // We have 3 extra guard bits beyond the mantissa. The rounding decision
  // depends on these bits:
  //
  //   [mantissa bits] [G] [R] [S]
  //                    ^   ^   ^
  //                    |   |   |
  //                    |   |   +-- sticky: OR of all bits below R
  //                    |   +------ round: the bit just below the last mantissa bit
  //                    +---------- guard: the first extra bit
  //
  // Round to nearest even rules:
  //   - If GRS = 0xx: round down (truncate)
  //   - If GRS = 100: round to even (round up if mantissa LSB is 1)
  //   - If GRS = 101, 110, 111: round up

  const guard = Number((resultMant >> BigInt(guardBits - 1)) & 1n);
  const roundBit = Number((resultMant >> BigInt(guardBits - 2)) & 1n);
  let stickyBit = resultMant & ((1n << BigInt(guardBits - 2)) - 1n);
  const stickyVal = stickyBit !== 0n ? 1 : 0;

  // Remove guard bits
  resultMant >>= BigInt(guardBits);

  // Apply rounding
  if (guard === 1) {
    if (roundBit === 1 || stickyVal === 1) {
      // Round up
      resultMant += 1n;
    } else if ((resultMant & 1n) === 1n) {
      // Tie-breaking: round to even (round up if LSB is 1)
      resultMant += 1n;
    }
  }

  // Check if rounding caused overflow
  if (resultMant >= 1n << BigInt(fmt.mantissaBits + 1)) {
    resultMant >>= 1n;
    resultExp += 1;
  }

  // ===================================================================
  // Step 7: Handle exponent overflow/underflow
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
    // Denormal or zero
    if (resultExp < -fmt.mantissaBits) {
      // Too small, flush to zero
      return {
        sign: resultSign,
        exponent: new Array(fmt.exponentBits).fill(0),
        mantissa: new Array(fmt.mantissaBits).fill(0),
        fmt,
      };
    }
    // Denormal: shift mantissa right, exponent stays at 0
    const shift = 1 - resultExp;
    resultMant >>= BigInt(shift);
    resultExp = 0;
  }

  // ===================================================================
  // Step 8: Pack the result
  // ===================================================================
  // Remove the implicit leading 1 (if normal)
  if (resultExp > 0) {
    resultMant &= (1n << BigInt(fmt.mantissaBits)) - 1n; // Remove implicit 1
  }

  return {
    sign: resultSign,
    exponent: intToBitsMsb(resultExp, fmt.exponentBits),
    mantissa: intToBitsMsb(Number(resultMant), fmt.mantissaBits),
    fmt,
  };
}

// ---------------------------------------------------------------------------
// fpSub -- subtraction is just addition with a flipped sign
// ---------------------------------------------------------------------------

/**
 * Subtract two floating-point numbers: a - b.
 *
 * === Why subtraction is trivial once you have addition ===
 *
 * In IEEE 754, a - b = a + (-b). To negate b, we just flip its sign bit.
 * This is a single NOT gate in hardware -- the cheapest possible operation.
 *
 * Then we feed the result into fpAdd, which handles all the complexity
 * of alignment, normalization, and rounding.
 *
 * @param a - The minuend (what we subtract from).
 * @param b - The subtrahend (what we subtract).
 * @returns a - b as FloatBits.
 *
 * Example:
 *     // 3.0 - 1.0 = 2.0
 *     const a = floatToBits(3.0);
 *     const b = floatToBits(1.0);
 *     const result = fpSub(a, b);
 *     bitsToFloat(result)  // 2.0
 */
export function fpSub(a: FloatBits, b: FloatBits): FloatBits {
  // Flip b's sign bit using XOR (XOR with 1 flips a bit)
  const negB: FloatBits = {
    sign: b.sign ^ 1,
    exponent: b.exponent,
    mantissa: b.mantissa,
    fmt: b.fmt,
  };
  return fpAdd(a, negB);
}

// ---------------------------------------------------------------------------
// fpNeg -- negate a floating-point number
// ---------------------------------------------------------------------------

/**
 * Negate a floating-point number: return -a.
 *
 * This is the simplest floating-point operation: just flip the sign bit.
 * In hardware, it's literally one NOT gate (or XOR with 1).
 *
 * Note: neg(+0) = -0 and neg(-0) = +0. Both are valid IEEE 754 zeros.
 *
 * @param a - The number to negate.
 * @returns -a as FloatBits.
 */
export function fpNeg(a: FloatBits): FloatBits {
  return {
    sign: a.sign ^ 1,
    exponent: a.exponent,
    mantissa: a.mantissa,
    fmt: a.fmt,
  };
}

// ---------------------------------------------------------------------------
// fpAbs -- absolute value
// ---------------------------------------------------------------------------

/**
 * Return the absolute value of a floating-point number.
 *
 * Even simpler than negation: just force the sign bit to 0.
 * In hardware, this is done by AND-ing the sign bit with 0 (or simply
 * not connecting the sign wire).
 *
 * Note: abs(NaN) is still NaN (with sign=0). This is the IEEE 754 behavior.
 *
 * @param a - The input number.
 * @returns |a| as FloatBits.
 */
export function fpAbs(a: FloatBits): FloatBits {
  return {
    sign: 0,
    exponent: a.exponent,
    mantissa: a.mantissa,
    fmt: a.fmt,
  };
}

// ---------------------------------------------------------------------------
// fpCompare -- compare two floating-point numbers
// ---------------------------------------------------------------------------

/**
 * Compare two floating-point numbers.
 *
 * @returns -1 if a < b, 0 if a == b, 1 if a > b
 *
 * NaN comparisons always return 0 (unordered). This is a simplification;
 * real IEEE 754 has a separate "unordered" result, but for our purposes
 * returning 0 is sufficient.
 *
 * === How FP comparison works in hardware ===
 *
 * Floating-point comparison is more complex than integer comparison because:
 * 1. The sign bit inverts the ordering (negative numbers are "backwards")
 * 2. The exponent is more significant than the mantissa
 * 3. Special values (NaN, Inf, zero) need special handling
 *
 * For two positive normal numbers:
 * - Compare exponents first (larger exponent = larger number)
 * - If exponents equal, compare mantissas
 *
 * For mixed signs: positive > negative (always).
 * For two negative numbers: comparison is reversed.
 */
export function fpCompare(a: FloatBits, b: FloatBits): number {
  // NaN is unordered -- any comparison involving NaN returns 0
  if (isNaN(a) || isNaN(b)) return 0;

  // Handle zeros: +0 == -0
  if (isZero(a) && isZero(b)) return 0;

  // Different signs: positive > negative
  if (a.sign !== b.sign) {
    if (isZero(a)) return b.sign === 1 ? 1 : -1;
    if (isZero(b)) return a.sign === 1 ? -1 : 1;
    return a.sign === 1 ? -1 : 1;
  }

  // Same sign: compare exponent, then mantissa
  const expA = bitsMsbToInt(a.exponent);
  const expB = bitsMsbToInt(b.exponent);
  const mantA = bitsMsbToInt(a.mantissa);
  const mantB = bitsMsbToInt(b.mantissa);

  if (expA !== expB) {
    if (a.sign === 0) return expA > expB ? 1 : -1;
    else return expA > expB ? -1 : 1;
  }

  if (mantA !== mantB) {
    if (a.sign === 0) return mantA > mantB ? 1 : -1;
    else return mantA > mantB ? -1 : 1;
  }

  return 0;
}
