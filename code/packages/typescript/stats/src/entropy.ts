/**
 * # Shannon Entropy
 *
 * Shannon entropy measures the average "information content" or
 * "surprise" in a message. It answers: "How many bits do we need,
 * on average, to encode each symbol?"
 *
 * ## Formula
 *
 *     H = -sum(p_i * log2(p_i))
 *
 * Where p_i is the proportion of each letter.
 *
 * ## Expected Values
 *
 * | Distribution     | Entropy          |
 * |-----------------|------------------|
 * | Uniform 26 chars | log2(26) ~ 4.700 |
 * | English text     | ~4.0 - 4.5       |
 * | Single letter    | 0.0              |
 *
 * ## Intuition
 *
 * - If a text uses all 26 letters equally, entropy is maximized at
 *   log2(26) ~ 4.7 bits. You need maximum bits because every letter
 *   is equally likely — no prediction shortcuts.
 * - If a text uses only one letter, entropy is 0. You already know
 *   what every character will be — no information content.
 * - Natural English falls in between: some letters are predictable (E
 *   is common) but there's still variety.
 *
 * @param text - Input text to analyze
 * @returns Shannon entropy in bits
 */
import { frequencyDistribution } from "./frequency_distribution.js";

export function entropy(text: string): number {
  const distribution = frequencyDistribution(text);

  let h = 0;
  for (const p of distribution.values()) {
    // Skip zero probabilities: 0 * log2(0) is defined as 0 by convention
    // (the limit as p -> 0 of p * log2(p) is 0).
    if (p > 0) {
      h -= p * Math.log2(p);
    }
  }

  return h;
}
