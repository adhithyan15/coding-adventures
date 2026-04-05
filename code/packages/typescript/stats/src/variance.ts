/**
 * # Variance
 *
 * Variance measures how spread out the data is from the mean. A low
 * variance means values cluster tightly around the mean; a high variance
 * means they are spread far apart.
 *
 * ## Formula
 *
 *     variance = sum((x_i - mean)^2) / d
 *
 * Where `d` is the divisor:
 * - **Population variance** (population=true): d = n
 *   Use when you have ALL the data (the entire population).
 * - **Sample variance** (population=false, the default): d = n - 1
 *   Use when you have a SAMPLE of a larger population. Dividing by n-1
 *   instead of n corrects for the bias in estimating the population
 *   variance from a sample. This is called Bessel's correction.
 *
 * ## Why n-1? (Bessel's Correction)
 *
 * When computing variance from a sample, the sample mean is "pulled toward"
 * the data points, systematically underestimating the true spread. Dividing
 * by n-1 compensates for this, giving an unbiased estimate of population
 * variance.
 *
 * ## Examples
 *
 *     values = [2, 4, 4, 4, 5, 5, 7, 9]
 *     mean = 5.0
 *
 *     Sample variance (default):
 *       sum of squared deviations = 32.0
 *       variance = 32.0 / 7 = 4.571428571428571
 *
 *     Population variance:
 *       variance = 32.0 / 8 = 4.0
 *
 * @param values - Array of numbers
 * @param population - If true, divide by n (population). Default: false (sample, divide by n-1).
 * @returns The variance
 * @throws Error if the array is empty, or if sample variance with < 2 values
 */
export function variance(values: number[], population: boolean = false): number {
  if (values.length === 0) {
    throw new Error("Cannot compute variance of an empty array");
  }
  if (!population && values.length < 2) {
    throw new Error("Sample variance requires at least 2 values");
  }

  // Step 1: Compute the mean.
  const n = values.length;
  const avg = values.reduce((acc, val) => acc + val, 0) / n;

  // Step 2: Sum the squared deviations from the mean.
  const sumSquaredDev = values.reduce((acc, val) => acc + (val - avg) ** 2, 0);

  // Step 3: Divide by n (population) or n-1 (sample).
  const divisor = population ? n : n - 1;
  return sumSquaredDev / divisor;
}
