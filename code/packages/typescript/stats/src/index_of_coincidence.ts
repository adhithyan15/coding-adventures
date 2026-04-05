/**
 * # Index of Coincidence (IC)
 *
 * The IC measures the probability that two randomly chosen letters from
 * a text are the same. It is a powerful tool for determining whether
 * text is natural language or random gibberish.
 *
 * ## Formula
 *
 *     IC = sum(n_i * (n_i - 1)) / (N * (N - 1))
 *
 * Where:
 * - n_i = count of the i-th letter (A-Z)
 * - N = total number of letters
 *
 * ## Expected Values
 *
 * | Text Type       | IC Value |
 * |----------------|----------|
 * | English text    | ~0.0667  |
 * | Random (uniform)| ~0.0385 (= 1/26) |
 *
 * ## Why This Works
 *
 * English has uneven letter frequencies (E is ~12.7%, Z is ~0.07%).
 * This unevenness means repeated letters are more likely, pushing IC
 * above 1/26. A polyalphabetic cipher (like Vigenere) flattens the
 * distribution, pushing IC toward 1/26. Comparing IC values helps
 * determine whether a cipher is monoalphabetic or polyalphabetic.
 *
 * ## Example
 *
 *     indexOfCoincidence("AABB")
 *     // A appears 2 times, B appears 2 times, N = 4
 *     // IC = (2*1 + 2*1) / (4*3) = 4/12 = 0.333...
 *
 * @param text - Input text to analyze
 * @returns The index of coincidence (0.0 to 1.0)
 */
import { frequencyCount } from "./frequency_count.js";

export function indexOfCoincidence(text: string): number {
  const counts = frequencyCount(text);

  // N = total number of alphabetic characters.
  let n = 0;
  for (const count of counts.values()) {
    n += count;
  }

  // Need at least 2 characters to compute IC.
  if (n < 2) {
    return 0;
  }

  // Numerator: sum of n_i * (n_i - 1) for each letter.
  let numerator = 0;
  for (const count of counts.values()) {
    numerator += count * (count - 1);
  }

  // Denominator: N * (N - 1).
  const denominator = n * (n - 1);

  return numerator / denominator;
}
