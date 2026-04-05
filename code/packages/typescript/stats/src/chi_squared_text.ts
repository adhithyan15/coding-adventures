/**
 * # Chi-Squared for Text
 *
 * A convenience function that computes the chi-squared statistic for a
 * text against an expected frequency table (like ENGLISH_FREQUENCIES).
 *
 * ## How It Works
 *
 * 1. Count the letters in the text.
 * 2. For each letter A-Z, compute the expected count: total * freq[letter].
 * 3. Run chi-squared on the 26-element observed vs expected arrays.
 *
 * ## Use in Cryptanalysis
 *
 * To break a Caesar cipher, try all 26 shifts and pick the one that
 * produces the lowest chi-squared value against English frequencies.
 * The lowest value indicates the shift that makes the decrypted text
 * most resemble natural English.
 *
 * @param text - Ciphertext or plaintext to analyze
 * @param expectedFreq - Expected frequency table (letter -> proportion)
 * @returns The chi-squared statistic
 */
import { frequencyCount } from "./frequency_count.js";

export function chiSquaredText(
  text: string,
  expectedFreq: Record<string, number>
): number {
  const counts = frequencyCount(text);

  // Total number of alphabetic characters in the text.
  let total = 0;
  for (const count of counts.values()) {
    total += count;
  }

  if (total === 0) {
    return 0;
  }

  // Build 26-element parallel arrays for observed and expected counts.
  let chi2 = 0;
  for (let code = 65; code <= 90; code++) {
    const letter = String.fromCharCode(code);
    const observed = counts.get(letter) ?? 0;
    const expected = total * (expectedFreq[letter] ?? 0);

    // Skip letters with zero expected frequency to avoid division by zero.
    if (expected > 0) {
      const diff = observed - expected;
      chi2 += (diff * diff) / expected;
    }
  }

  return chi2;
}
