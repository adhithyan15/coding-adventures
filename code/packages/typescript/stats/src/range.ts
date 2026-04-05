/**
 * # Range
 *
 * The range is the simplest measure of spread: max - min. It tells you
 * the total span of the data but is sensitive to outliers.
 *
 * ## Example
 *
 *     range([2, 4, 4, 4, 5, 5, 7, 9]) = 9 - 2 = 7.0
 *
 * @param values - Array of numbers
 * @returns The range (max - min)
 * @throws Error if the array is empty
 */
import { min } from "./min.js";
import { max } from "./max.js";

export function range(values: number[]): number {
  return max(values) - min(values);
}
