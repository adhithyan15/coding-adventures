/**
 * # Mean (Arithmetic Average)
 *
 * The mean is the most common measure of central tendency. It answers the
 * question: "If we spread the total equally among all values, what would
 * each value be?"
 *
 * ## Formula
 *
 *     mean = sum(values) / n
 *
 * ## Example
 *
 *     mean([1, 2, 3, 4, 5])
 *       = (1 + 2 + 3 + 4 + 5) / 5
 *       = 15 / 5
 *       = 3.0
 *
 * ## Edge Cases
 *
 * - Empty array: throws an error (mean of nothing is undefined)
 * - Single value: returns that value
 *
 * @param values - Array of numbers to average
 * @returns The arithmetic mean
 * @throws Error if the array is empty
 */
export function mean(values: number[]): number {
  if (values.length === 0) {
    throw new Error("Cannot compute mean of an empty array");
  }

  // Sum all values, then divide by the count. We use reduce for clarity —
  // it walks through each element accumulating a running total.
  const sum = values.reduce((acc, val) => acc + val, 0);
  return sum / values.length;
}
