/**
 * # Mode
 *
 * The mode is the most frequently occurring value. In a dataset of
 * [4, 4, 4, 5, 5, 7], the mode is 4 because it appears 3 times.
 *
 * ## Tie-breaking
 *
 * When multiple values share the highest frequency, we return the one
 * that appeared first in the original array. This matches the spec's
 * "first occurrence wins ties" rule and ensures deterministic output.
 *
 * ## Algorithm
 *
 * 1. Count occurrences using a Map (preserves insertion order).
 * 2. Find the maximum count.
 * 3. Return the first value that achieves that count.
 *
 * @param values - Array of numbers
 * @returns The most frequent value (first occurrence wins ties)
 * @throws Error if the array is empty
 */
export function mode(values: number[]): number {
  if (values.length === 0) {
    throw new Error("Cannot compute mode of an empty array");
  }

  // Build a frequency map. JavaScript Maps preserve insertion order,
  // which is exactly what we need for tie-breaking.
  const counts = new Map<number, number>();
  for (const val of values) {
    counts.set(val, (counts.get(val) ?? 0) + 1);
  }

  // Walk the map to find the value with the highest count.
  // Because Map iterates in insertion order, the first value with the
  // max count is the first occurrence in the original array.
  let bestValue = values[0];
  let bestCount = 0;
  for (const [val, count] of counts) {
    if (count > bestCount) {
      bestCount = count;
      bestValue = val;
    }
  }

  return bestValue;
}
