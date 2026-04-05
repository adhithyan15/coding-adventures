/**
 * # Frequency Distribution
 *
 * Converts raw letter counts into proportions (0.0 to 1.0). This
 * normalizes the data so texts of different lengths can be compared.
 *
 * ## Formula
 *
 *     proportion(letter) = count(letter) / total_letter_count
 *
 * ## Example
 *
 *     frequencyDistribution("AABB")
 *     // => { A: 0.5, B: 0.5 }
 *
 *     frequencyDistribution("AAAB")
 *     // => { A: 0.75, B: 0.25 }
 *
 * @param text - Input text to analyze
 * @returns A Map from uppercase letter to proportion (0.0 - 1.0)
 */
import { frequencyCount } from "./frequency_count.js";

export function frequencyDistribution(text: string): Map<string, number> {
  const counts = frequencyCount(text);

  // Total is the sum of all letter counts (not the text length,
  // because non-alpha characters are excluded).
  let total = 0;
  for (const count of counts.values()) {
    total += count;
  }

  // Convert each count to a proportion.
  const distribution = new Map<string, number>();
  if (total > 0) {
    for (const [letter, count] of counts) {
      distribution.set(letter, count / total);
    }
  }

  return distribution;
}
