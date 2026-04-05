/**
 * # Minimum
 *
 * Returns the smallest value in an array. Simple, but essential as a
 * building block for range and normalization.
 *
 * We use a manual loop instead of Math.min(...values) because
 * Math.min with spread syntax hits the JavaScript engine's argument
 * limit on very large arrays (typically ~65K elements).
 *
 * @param values - Array of numbers
 * @returns The minimum value
 * @throws Error if the array is empty
 */
export function min(values: number[]): number {
  if (values.length === 0) {
    throw new Error("Cannot compute min of an empty array");
  }

  let result = values[0];
  for (let i = 1; i < values.length; i++) {
    if (values[i] < result) {
      result = values[i];
    }
  }
  return result;
}
