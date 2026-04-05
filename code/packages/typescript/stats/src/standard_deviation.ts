/**
 * # Standard Deviation
 *
 * The standard deviation is the square root of the variance. While variance
 * is in "squared units" (e.g., dollars squared), standard deviation brings
 * us back to the original units (dollars), making it more interpretable.
 *
 * ## Formula
 *
 *     std_dev = sqrt(variance(values, population))
 *
 * ## The 68-95-99.7 Rule
 *
 * For normally distributed data:
 * - ~68% of values fall within 1 standard deviation of the mean
 * - ~95% fall within 2 standard deviations
 * - ~99.7% fall within 3 standard deviations
 *
 * @param values - Array of numbers
 * @param population - If true, use population variance. Default: false (sample).
 * @returns The standard deviation
 */
import { variance } from "./variance.js";

export function standardDeviation(
  values: number[],
  population: boolean = false
): number {
  return Math.sqrt(variance(values, population));
}
