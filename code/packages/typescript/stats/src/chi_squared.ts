/**
 * # Chi-Squared Statistic
 *
 * The chi-squared statistic measures how well observed data matches
 * expected data. A value of 0 means perfect agreement; larger values
 * indicate greater divergence.
 *
 * ## Formula
 *
 *     chi2 = sum( (observed_i - expected_i)^2 / expected_i )
 *
 * ## Intuition
 *
 * For each category, we ask: "How far off is the observed count from
 * what we expected?" We square the difference (so negatives don't
 * cancel positives) and divide by the expected count (so deviations
 * in small categories are weighted more heavily).
 *
 * ## Example
 *
 *     observed = [10, 20, 30]
 *     expected = [20, 20, 20]
 *
 *     chi2 = (10-20)^2/20 + (20-20)^2/20 + (30-20)^2/20
 *          = 100/20 + 0/20 + 100/20
 *          = 5.0 + 0.0 + 5.0
 *          = 10.0
 *
 * @param observed - Array of observed counts
 * @param expected - Array of expected counts (must be same length)
 * @returns The chi-squared statistic
 * @throws Error if arrays differ in length or expected contains zeros
 */
export function chiSquared(observed: number[], expected: number[]): number {
  if (observed.length !== expected.length) {
    throw new Error("Observed and expected arrays must have the same length");
  }
  if (observed.length === 0) {
    throw new Error("Arrays must not be empty");
  }

  let chi2 = 0;
  for (let i = 0; i < observed.length; i++) {
    if (expected[i] === 0) {
      throw new Error(`Expected value at index ${i} must not be zero`);
    }
    const diff = observed[i] - expected[i];
    chi2 += (diff * diff) / expected[i];
  }

  return chi2;
}
