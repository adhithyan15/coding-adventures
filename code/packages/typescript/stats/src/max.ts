/**
 * # Maximum
 *
 * Returns the largest value in an array. Like min, we use a manual loop
 * to avoid the argument-count limit of Math.max(...values).
 *
 * @param values - Array of numbers
 * @returns The maximum value
 * @throws Error if the array is empty
 */
export function max(values: number[]): number {
  if (values.length === 0) {
    throw new Error("Cannot compute max of an empty array");
  }

  let result = values[0];
  for (let i = 1; i < values.length; i++) {
    if (values[i] > result) {
      result = values[i];
    }
  }
  return result;
}
