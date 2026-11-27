/**
 * Reusable sample median and exact median-confidence-interval functions.
 *
 * This module works only with numbers supplied by its caller. It does not
 * read the benchmark fixture, touch the filesystem, or decide whether a
 * benchmark is faster. Validation first copies a bounded JavaScript array
 * into a plain number array; every later operation uses that private snapshot.
 */

/** A closed interval whose endpoints are selected observations. */
export interface MedianInterval {
  readonly low: number;
  readonly high: number;
}

const MAX_ARRAY_LENGTH = 0xffff_ffff;
const FRACTION_MASK = (1n << 52n) - 1n;
const SIGN_MASK = 1n << 63n;
const HIDDEN_BIT = 1n << 52n;
const TWO_TO_53 = 1n << 53n;

/**
 * A private, ascending numeric comparison.
 *
 * Do not omit this callback: JavaScript's default Array.sort order is
 * lexicographic, so it would put 10 before 2. The relational comparisons also
 * leave the two zero signs tied, as the statistical rank treats them equal.
 */
function compareAscending(left: number, right: number): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

/**
 * Capture one safe value per own array index before doing any computation.
 *
 * Reading length once matters for proxies: a changing length must not alter
 * loop bounds after validation. Checking ownership distinguishes a true dense
 * sample from an inherited value that happens to fill a sparse slot. Slot
 * reads and proxy traps are caller code, so their exceptions intentionally
 * propagate unchanged. The library itself never writes the caller's array.
 */
function finiteSnapshot(values: unknown): number[] {
  if (!Array.isArray(values)) {
    throw new TypeError("sample must be an array");
  }

  const length: unknown = (values as unknown as { readonly length: unknown }).length;
  if (typeof length !== "number") {
    throw new TypeError("sample length must be a number");
  }
  if (!Number.isInteger(length) || length < 0 || length > MAX_ARRAY_LENGTH) {
    throw new RangeError("sample length must be an integer in the JavaScript array range");
  }
  if (length === 0) {
    throw new RangeError("sample must not be empty");
  }

  const snapshot: number[] = [];
  for (let index = 0; index < length; index += 1) {
    if (!Object.prototype.hasOwnProperty.call(values, index)) {
      throw new TypeError(`sample index ${index} must be an own property`);
    }
    const value: unknown = values[index];
    if (typeof value !== "number") {
      throw new TypeError(`sample index ${index} must be a number`);
    }
    if (!Number.isFinite(value)) {
      throw new RangeError(`sample index ${index} must be finite`);
    }
    snapshot.push(value);
  }
  return snapshot;
}

function bitsOf(value: number): bigint {
  const bytes = new ArrayBuffer(8);
  const view = new DataView(bytes);
  view.setFloat64(0, value, false);
  return view.getBigUint64(0, false);
}

function numberFromBits(bits: bigint): number {
  const bytes = new ArrayBuffer(8);
  const view = new DataView(bytes);
  view.setBigUint64(0, bits, false);
  return view.getFloat64(0, false);
}

interface Dyadic {
  readonly significand: bigint;
  readonly exponent: number;
}

/** Express a finite binary64 number exactly as an integer times a power of 2. */
function decodeFinite(value: number): Dyadic {
  const bits = bitsOf(value);
  const negative = (bits & SIGN_MASK) !== 0n;
  const exponentField = Number((bits >> 52n) & 0x7ffn);
  const fraction = bits & FRACTION_MASK;
  const significand = exponentField === 0 ? fraction : HIDDEN_BIT | fraction;
  const signedSignificand = negative ? -significand : significand;
  const exponent = exponentField === 0 ? -1074 : exponentField - 1023 - 52;
  return { significand: signedSignificand, exponent };
}

/** Round a positive integer shifted right, with binary ties going to even. */
function roundRightToEven(value: bigint, shift: number): bigint {
  if (shift <= 0) return value << BigInt(-shift);
  const divisor = 1n << BigInt(shift);
  const quotient = value >> BigInt(shift);
  const remainder = value - quotient * divisor;
  const halfway = divisor >> 1n;
  if (remainder > halfway || (remainder === halfway && (quotient & 1n) !== 0n)) {
    return quotient + 1n;
  }
  return quotient;
}

/**
 * Correctly round `significand * 2^exponent` to binary64.
 *
 * Normal numbers retain 53 significand bits; subnormal values instead round
 * on the fixed 2^-1074 grid. The final bit construction avoids a second
 * floating-point operation that could introduce another rounding step.
 */
function roundDyadic(significand: bigint, exponent: number): number {
  if (significand === 0n) return 0;
  const negative = significand < 0n;
  const magnitude = negative ? -significand : significand;
  const signBit = negative ? SIGN_MASK : 0n;
  const bitLength = magnitude.toString(2).length;
  const leadingExponent = bitLength - 1 + exponent;

  if (leadingExponent < -1022) {
    const rounded = roundRightToEven(magnitude, -1074 - exponent);
    // Rounding the largest subnormal midpoint may produce the smallest normal.
    return numberFromBits(signBit | rounded);
  }

  const shift = bitLength - 53;
  let rounded = roundRightToEven(magnitude, shift);
  let normalizedExponent = exponent + shift;
  if (rounded === TWO_TO_53) {
    rounded >>= 1n;
    normalizedExponent += 1;
  }
  const unbiasedExponent = normalizedExponent + 52;
  const exponentField = BigInt(unbiasedExponent + 1023);
  const fraction = rounded - HIDDEN_BIT;
  return numberFromBits(signBit | (exponentField << 52n) | fraction);
}

/**
 * Return the exact, correctly rounded midpoint of two finite binary64 values.
 *
 * Aligning the integer significands forms the exact sum before dividing by
 * two. In particular this avoids overflow for two large values of one sign.
 */
function midpoint(left: number, right: number): number {
  if (Object.is(left, -0) && Object.is(right, -0)) return -0;
  const first = decodeFinite(left);
  const second = decodeFinite(right);
  const commonExponent = Math.min(first.exponent, second.exponent);
  const exactSum =
    (first.significand << BigInt(first.exponent - commonExponent)) +
    (second.significand << BigInt(second.exponent - commonExponent));
  if (exactSum === 0n) return 0;
  return roundDyadic(exactSum, commonExponent - 1);
}

function sortedSnapshot(values: unknown): number[] {
  const snapshot = finiteSnapshot(values);
  snapshot.sort(compareAscending);
  return snapshot;
}

/** Return the sample median without changing the supplied array. */
export function median(values: readonly number[]): number {
  const ordered = sortedSnapshot(values);
  const middle = Math.floor(ordered.length / 2);
  if (ordered.length % 2 === 1) return ordered[middle];
  return midpoint(ordered[middle - 1], ordered[middle]);
}

/**
 * Find the largest one-based rank satisfying the exact two-sided cutoff.
 *
 * The binomial coefficient recurrence and tail sum are integers. No
 * floating-point probability is rounded to decide whether a rank qualifies.
 */
function exactMedianRank(sampleSize: number, confidencePercent: number): number {
  const threshold = BigInt(100 - confidencePercent) * (1n << BigInt(sampleSize));
  const coefficientLimit = BigInt(sampleSize);
  let coefficient = 1n;
  let lowerTail = 0n;
  let rank = 0;

  for (let j = 0n; j < coefficientLimit; j += 1n) {
    lowerTail += coefficient;
    if (200n * lowerTail > threshold) break;
    rank = Number(j + 1n);
    coefficient = (coefficient * (coefficientLimit - j)) / (j + 1n);
  }
  return rank;
}

/**
 * Return the exact distribution-free confidence interval for the median.
 * Invalid confidence is rejected before inspecting the supplied sample.
 */
export function medianInterval(
  values: readonly number[],
  confidencePercent: number,
): MedianInterval | null {
  if (typeof confidencePercent !== "number") {
    throw new TypeError("confidence percentage must be a number");
  }
  if (!Number.isInteger(confidencePercent) || confidencePercent < 1 || confidencePercent > 99) {
    throw new RangeError("confidence percentage must be an integer from 1 to 99");
  }

  const ordered = sortedSnapshot(values);
  const rank = exactMedianRank(ordered.length, confidencePercent);
  if (rank === 0) return null;
  return {
    low: ordered[rank - 1],
    high: ordered[ordered.length - rank],
  };
}
