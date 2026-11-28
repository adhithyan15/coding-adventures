/** Test-only exact models, intentionally separate from the production helpers. */

const SIGN_BIT = 1n << 63n;
const FRACTION_BITS = (1n << 52n) - 1n;
const MAX_FINITE_BITS = 0x7fefffffffffffffn;

export function bitsOf(value: number): string {
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, value, false);
  return view.getBigUint64(0, false).toString(16).padStart(16, "0");
}

export function numberFromBits(hex: string): number {
  const view = new DataView(new ArrayBuffer(8));
  view.setBigUint64(0, BigInt(`0x${hex}`), false);
  return view.getFloat64(0, false);
}

interface Rational {
  readonly numerator: bigint;
  readonly denominator: bigint;
}

function exactRational(value: number): Rational {
  const view = new DataView(new ArrayBuffer(8));
  view.setFloat64(0, value, false);
  const bits = view.getBigUint64(0, false);
  const negative = (bits & SIGN_BIT) !== 0n;
  const exponentField = Number((bits >> 52n) & 0x7ffn);
  const fraction = bits & FRACTION_BITS;
  const significand = exponentField === 0 ? fraction : (1n << 52n) + fraction;
  const signed = negative ? -significand : significand;
  const binaryExponent = exponentField === 0 ? -1074 : exponentField - 1075;
  return binaryExponent >= 0
    ? { numerator: signed << BigInt(binaryExponent), denominator: 1n }
    : { numerator: signed, denominator: 1n << BigInt(-binaryExponent) };
}

function rationalForPositiveBits(bits: bigint): Rational {
  const exponentField = Number((bits >> 52n) & 0x7ffn);
  const fraction = bits & FRACTION_BITS;
  const significand = exponentField === 0 ? fraction : (1n << 52n) + fraction;
  const binaryExponent = exponentField === 0 ? -1074 : exponentField - 1075;
  return binaryExponent >= 0
    ? { numerator: significand << BigInt(binaryExponent), denominator: 1n }
    : { numerator: significand, denominator: 1n << BigInt(-binaryExponent) };
}

function compare(left: Rational, right: Rational): number {
  const difference = left.numerator * right.denominator - right.numerator * left.denominator;
  return difference < 0n ? -1 : difference > 0n ? 1 : 0;
}

function nearestPositiveBinary64(value: Rational): bigint {
  let low = 0n;
  let high = MAX_FINITE_BITS;
  while (low <= high) {
    const middle = (low + high) >> 1n;
    if (compare(rationalForPositiveBits(middle), value) <= 0) {
      low = middle + 1n;
    } else {
      high = middle - 1n;
    }
  }

  const lowerBits = high;
  if (lowerBits === MAX_FINITE_BITS && compare(rationalForPositiveBits(lowerBits), value) === 0) {
    return lowerBits;
  }
  const upperBits = low;
  const lower = rationalForPositiveBits(lowerBits);
  const upper = rationalForPositiveBits(upperBits);
  const lowerDistance = value.numerator * lower.denominator - lower.numerator * value.denominator;
  const upperDistance = upper.numerator * value.denominator - value.numerator * upper.denominator;
  const scaledLower = lowerDistance * upper.denominator;
  const scaledUpper = upperDistance * lower.denominator;
  if (scaledLower < scaledUpper) return lowerBits;
  if (scaledUpper < scaledLower) return upperBits;
  return (lowerBits & 1n) === 0n ? lowerBits : upperBits;
}

/**
 * Independent exact-rational midpoint oracle: binary-search the ordered
 * positive binary64 encoding space, then compare rational distances.
 */
export function exactMidpoint(left: number, right: number): number {
  if (Object.is(left, -0) && Object.is(right, -0)) return -0;
  const leftRational = exactRational(left);
  const rightRational = exactRational(right);
  const numerator = leftRational.numerator * rightRational.denominator +
    rightRational.numerator * leftRational.denominator;
  if (numerator === 0n) return 0;
  const denominator = 2n * leftRational.denominator * rightRational.denominator;
  const negative = numerator < 0n;
  const magnitude: Rational = {
    numerator: negative ? -numerator : numerator,
    denominator,
  };
  const magnitudeBits = nearestPositiveBinary64(magnitude);
  const finalBits = magnitudeBits | (negative ? SIGN_BIT : 0n);
  const view = new DataView(new ArrayBuffer(8));
  view.setBigUint64(0, finalBits, false);
  return view.getFloat64(0, false);
}

export function compareNumbers(left: number, right: number): number {
  if (left < right) return -1;
  if (left > right) return 1;
  return 0;
}

export function sortedCopy(values: readonly number[]): number[] {
  return [...values].sort(compareNumbers);
}

export function exactMedian(values: readonly number[]): number {
  const ordered = sortedCopy(values);
  const middle = Math.floor(ordered.length / 2);
  return ordered.length % 2 === 1
    ? ordered[middle]
    : exactMidpoint(ordered[middle - 1], ordered[middle]);
}

/** Return whether each one-based rank meets the cutoff by sign enumeration. */
export function signEnumerationRankProfile(sampleSize: number, confidence: number): boolean[] {
  const assignmentCount = 1 << sampleSize;
  const histogram = new Array<number>(sampleSize + 1).fill(0);
  for (let signs = 0; signs < assignmentCount; signs += 1) {
    let negativeCount = 0;
    for (let bits = signs; bits !== 0; bits >>>= 1) negativeCount += bits & 1;
    histogram[negativeCount] += 1;
  }

  const profile = new Array<boolean>(sampleSize).fill(false);
  let lowerTail = 0;
  const rightSide = (100 - confidence) * assignmentCount;
  for (let rank = 1; rank <= sampleSize; rank += 1) {
    lowerTail += histogram[rank - 1];
    profile[rank - 1] = 200 * lowerTail <= rightSide;
  }
  return profile;
}

/** Test differential using Pascal's additive recurrence, not production code. */
export function pascalRankProfile(sampleSize: number, confidence: number): boolean[] {
  let previous = [1n];
  for (let row = 1; row <= sampleSize; row += 1) {
    const next = new Array<bigint>(row + 1).fill(1n);
    for (let column = 1; column < row; column += 1) {
      next[column] = previous[column - 1] + previous[column];
    }
    previous = next;
  }

  const total = 1n << BigInt(sampleSize);
  const rightSide = BigInt(100 - confidence) * total;
  const profile: boolean[] = [];
  let lowerTail = 0n;
  for (let rank = 1; rank <= sampleSize; rank += 1) {
    lowerTail += previous[rank - 1];
    profile.push(200n * lowerTail <= rightSide);
  }
  return profile;
}

export function selectedRank(profile: readonly boolean[]): number {
  let rank = 0;
  for (let index = 0; index < profile.length; index += 1) {
    if (profile[index]) rank = index + 1;
  }
  return rank;
}

export function expectedInterval(values: readonly number[], rank: number): null | {
  low: number;
  high: number;
} {
  if (rank === 0) return null;
  const ordered = sortedCopy(values);
  return { low: ordered[rank - 1], high: ordered[ordered.length - rank] };
}
