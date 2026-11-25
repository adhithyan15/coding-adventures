/**
 * Rank statistics for finite binary64 samples.
 *
 * The median sorts a copy so callers retain their original order. For an even
 * sample, sum before halving to preserve subnormal rounding; halve the operands
 * first only when their sum overflows. The interval endpoints are observed
 * order statistics, not interpolated quantiles.
 */

export interface MedianInterval {
  readonly low: number;
  readonly high: number;
}

function sortedFinite(values: readonly number[]): number[] {
  if (!Array.isArray(values) || values.length === 0) {
    throw new RangeError("values must be a non-empty array");
  }
  const copy = [...values];
  for (const value of copy) {
    if (typeof value !== "number" || !Number.isFinite(value)) {
      throw new TypeError("values must contain only finite numbers");
    }
  }
  return copy.sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
}

/** Return the sample median without mutating `values`. */
export function median(values: readonly number[]): number {
  const sorted = sortedFinite(values);
  const middle = Math.floor(sorted.length / 2);
  if (sorted.length % 2 === 1) return sorted[middle];
  const low = sorted[middle - 1];
  const high = sorted[middle];
  const sum = low + high;
  return Number.isFinite(sum) ? sum / 2 : low / 2 + high / 2;
}

/**
 * Return the exact two-sided, distribution-free confidence interval for the
 * population median. The cutoff is compared with BigInt arithmetic so a
 * rounded probability cannot change the chosen rank.
 */
export function medianInterval(
  values: readonly number[],
  confidencePercent: number,
): MedianInterval | null {
  const sorted = sortedFinite(values);
  if (!Number.isInteger(confidencePercent) || confidencePercent < 1 || confidencePercent > 99) {
    throw new RangeError("confidencePercent must be an integer from 1 through 99");
  }
  const n = sorted.length;
  let choose = 1n;
  let cumulative = 0n;
  let bestRank = 0;
  const denominator = 1n << BigInt(n);
  for (let j = 0; j < n; j += 1) {
    if (j > 0) choose = choose * BigInt(n - j + 1) / BigInt(j);
    cumulative += choose;
    const rank = j + 1;
    // 2 * P(Binomial(n, 1/2) <= rank - 1) <= 1 - confidence.
    if (2n * cumulative * 100n > BigInt(100 - confidencePercent) * denominator) break;
    bestRank = rank;
  }
  if (bestRank === 0) return null;
  return { low: sorted[bestRank - 1], high: sorted[n - bestRank] };
}
