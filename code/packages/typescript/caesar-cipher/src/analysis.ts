/**
 * # Caesar Cipher -- Cryptanalysis Tools
 *
 * The Caesar cipher is famously weak. With only 25 possible keys (shifts 1-25),
 * it can be broken by simply trying all of them (brute force). Even better, we
 * can use statistical properties of natural language to identify the correct
 * shift without trying them all.
 *
 * This module provides two cryptanalysis techniques:
 *
 * 1. **Brute Force** -- Try all 25 non-trivial shifts and return every possibility.
 *    A human can then scan the results and pick the one that makes sense.
 *
 * 2. **Frequency Analysis** -- Compare the letter frequency distribution of the
 *    ciphertext against known English letter frequencies. The shift that produces
 *    the best match is likely the correct key.
 *
 * ## Why Frequency Analysis Works
 *
 * In any sufficiently long English text, certain letters appear much more often
 * than others. The letter 'E' is the most common (~12.7%), followed by 'T' (~9.1%),
 * 'A' (~8.2%), and so on. The Caesar cipher preserves these frequencies -- it just
 * shifts which letter carries each frequency.
 *
 * If we encrypt English text with shift 3, then the most common letter in the
 * ciphertext will be 'H' (which was 'E' in the plaintext). By measuring the
 * frequency of each letter in the ciphertext and comparing it to the expected
 * English frequencies, we can deduce the shift.
 *
 * The mathematical tool we use is the **chi-squared statistic**, which measures
 * how far an observed distribution deviates from an expected one. The shift that
 * minimizes chi-squared is our best guess.
 *
 * ## Chi-Squared Statistic
 *
 * The formula is:
 *
 *   chi2 = SUM over all letters of: (observed_count - expected_count)^2 / expected_count
 *
 * - A chi-squared of 0 means the distributions match perfectly.
 * - Higher values mean more deviation from the expected English frequencies.
 * - We try all 26 shifts and pick the one with the lowest chi-squared value.
 *
 * @module analysis
 */

import { decrypt } from "./cipher.js";

// ─── English Letter Frequencies ────────────────────────────────────────────────
//
// These frequencies are derived from large corpora of English text. They represent
// the probability of each letter appearing in a typical English passage. Different
// sources give slightly different numbers, but these are widely accepted values.
//
// Source: https://en.wikipedia.org/wiki/Letter_frequency
//
// The frequencies sum to approximately 1.0 (100%). They are expressed as
// proportions, not percentages (so 0.08167 means 8.167%).

/**
 * Expected frequency of each letter in English text, expressed as a proportion
 * (0.0 to 1.0). For example, `ENGLISH_FREQUENCIES["e"]` is approximately 0.12702,
 * meaning 'e' makes up about 12.7% of English text.
 *
 * These values are used by the frequency analysis algorithm to identify the
 * most likely Caesar cipher shift.
 */
export const ENGLISH_FREQUENCIES: Record<string, number> = {
  a: 0.08167,
  b: 0.01492,
  c: 0.02782,
  d: 0.04253,
  e: 0.12702,
  f: 0.02228,
  g: 0.02015,
  h: 0.06094,
  i: 0.06966,
  j: 0.00153,
  k: 0.00772,
  l: 0.04025,
  m: 0.02406,
  n: 0.06749,
  o: 0.07507,
  p: 0.01929,
  q: 0.00095,
  r: 0.05987,
  s: 0.06327,
  t: 0.09056,
  u: 0.02758,
  v: 0.00978,
  w: 0.02360,
  x: 0.00150,
  y: 0.01974,
  z: 0.00074,
};

// ─── Types ─────────────────────────────────────────────────────────────────────

/**
 * Represents one candidate decryption from a brute-force attack.
 *
 * When brute-forcing a Caesar cipher, we try every possible shift (1 through 25)
 * and return the result for each. The correct plaintext will be one of these
 * 25 entries -- a human or a frequency analysis algorithm can identify which one.
 */
export interface BruteForceResult {
  /** The shift value that was applied to produce this decryption (1-25). */
  shift: number;
  /** The plaintext produced by decrypting with this shift. */
  plaintext: string;
}

// ─── Brute Force ───────────────────────────────────────────────────────────────
//
// The simplest possible attack. Since there are only 26 possible shifts (0-25),
// and shift 0 is the identity (no change), we only need to try 25 shifts.
// We return all 25 results and let the caller pick the right one.
//
// Time complexity: O(25 * n) where n is the length of the ciphertext.
// This is effectively O(n) -- trivial even for very long messages.

/**
 * Performs a brute-force attack on Caesar cipher encrypted text.
 *
 * Tries every possible shift from 1 to 25 and returns all candidate
 * decryptions. Shift 0 is omitted because it would return the original
 * ciphertext unchanged.
 *
 * @param ciphertext - The encrypted text to attack.
 * @returns An array of 25 `BruteForceResult` objects, one for each
 *   possible shift. The correct plaintext will be among them.
 *
 * @example
 * ```ts
 * const results = bruteForce("KHOOR");
 * // results[2] will be { shift: 3, plaintext: "HELLO" }
 * // because "KHOOR" was encrypted with shift 3
 * ```
 */
export function bruteForce(ciphertext: string): BruteForceResult[] {
  const results: BruteForceResult[] = [];

  // Try every non-trivial shift. Shift 0 is skipped because it's the identity
  // transformation (the ciphertext IS the plaintext if shift is 0).
  for (let shift = 1; shift <= 25; shift++) {
    results.push({
      shift,
      plaintext: decrypt(ciphertext, shift),
    });
  }

  return results;
}

// ─── Frequency Analysis ────────────────────────────────────────────────────────
//
// This is the intelligent approach. Instead of presenting 25 options to a human,
// we use statistics to automatically identify the most likely shift.
//
// Algorithm:
// 1. Count the frequency of each letter in the ciphertext.
// 2. For each possible shift (0-25):
//    a. "Unshift" the frequency distribution by that amount.
//    b. Compute the chi-squared statistic comparing unshifted frequencies
//       to expected English frequencies.
// 3. The shift with the lowest chi-squared is our best guess.
//
// Note: This works well for longer texts (50+ characters) but may fail on
// very short messages where the frequency distribution is too sparse to be
// statistically meaningful.

/**
 * Counts the frequency of each letter (a-z) in the given text.
 *
 * All letters are counted in lowercase form. Non-alphabetic characters
 * are ignored. Returns an array of 26 counts, where index 0 is 'a',
 * index 1 is 'b', and so on.
 *
 * @param text - The text to analyze.
 * @returns An array of 26 letter counts.
 *
 * @example
 * ```ts
 * const counts = countLetters("Hello");
 * // counts[4] is 1  (one 'e')
 * // counts[7] is 1  (one 'h')
 * // counts[11] is 2 (two 'l's)
 * // counts[14] is 1 (one 'o')
 * ```
 */
function countLetters(text: string): number[] {
  // Initialize all 26 counters to zero.
  const counts = new Array<number>(26).fill(0);

  // The character code for lowercase 'a' -- our anchor point.
  const LOWER_A = 97;
  const LOWER_Z = 122;
  const UPPER_A = 65;
  const UPPER_Z = 90;

  for (let i = 0; i < text.length; i++) {
    const code = text.charCodeAt(i);

    if (code >= LOWER_A && code <= LOWER_Z) {
      // Lowercase letter: subtract 'a' to get 0-25 index.
      counts[code - LOWER_A]++;
    } else if (code >= UPPER_A && code <= UPPER_Z) {
      // Uppercase letter: subtract 'A' to get 0-25 index.
      counts[code - UPPER_A]++;
    }
    // Non-alphabetic characters are silently skipped.
  }

  return counts;
}

/**
 * Computes the chi-squared statistic comparing observed letter counts to
 * expected English frequencies.
 *
 * The chi-squared statistic measures how well an observed frequency
 * distribution matches an expected distribution. Lower values indicate
 * a better match.
 *
 * Formula: chi2 = SUM((observed - expected)^2 / expected)
 *
 * @param observed - Array of 26 observed letter counts.
 * @param totalLetters - Total number of letters in the sample.
 * @returns The chi-squared statistic (lower is better).
 */
function chiSquared(observed: number[], totalLetters: number): number {
  // If there are no letters, we can't compute a meaningful statistic.
  if (totalLetters === 0) return Infinity;

  let chi2 = 0;
  const letters = "abcdefghijklmnopqrstuvwxyz";

  for (let i = 0; i < 26; i++) {
    // The expected count is the English frequency times the total letter count.
    // For example, if we have 100 letters and 'e' has frequency 0.127, we
    // expect about 12.7 occurrences of 'e'.
    const expected = ENGLISH_FREQUENCIES[letters[i]] * totalLetters;

    // The chi-squared formula: (observed - expected)^2 / expected
    // This penalizes large deviations from the expected distribution.
    // Dividing by `expected` normalizes the contribution so rare letters
    // don't dominate the statistic.
    if (expected > 0) {
      const diff = observed[i] - expected;
      chi2 += (diff * diff) / expected;
    }
  }

  return chi2;
}

/**
 * Uses frequency analysis to determine the most likely Caesar cipher shift.
 *
 * Compares the letter frequency distribution of the ciphertext against known
 * English letter frequencies for each possible shift (0-25). The shift that
 * produces the closest match (lowest chi-squared statistic) is returned along
 * with the decrypted plaintext.
 *
 * This method works best on longer texts (50+ characters). For very short
 * messages, brute force with human inspection may be more reliable.
 *
 * @param ciphertext - The encrypted text to analyze.
 * @returns An object containing the detected `shift` and the decrypted `plaintext`.
 *
 * @example
 * ```ts
 * const result = frequencyAnalysis("KHOOR ZRUOG");
 * // result.shift === 3
 * // result.plaintext === "HELLO WORLD"
 * ```
 */
export function frequencyAnalysis(ciphertext: string): {
  shift: number;
  plaintext: string;
} {
  // Step 1: Count letter frequencies in the ciphertext.
  const cipherCounts = countLetters(ciphertext);
  const totalLetters = cipherCounts.reduce((sum, count) => sum + count, 0);

  // Step 2: Try every possible shift and compute chi-squared for each.
  let bestShift = 0;
  let bestChi2 = Infinity;

  for (let shift = 0; shift < 26; shift++) {
    // "Unshift" the frequency distribution: if the cipher used shift S,
    // then ciphertext letter at position i originally came from position
    // (i - S) mod 26. We create a new frequency array that represents
    // what the plaintext frequencies would look like if this shift is correct.
    const unshifted = new Array<number>(26);
    for (let i = 0; i < 26; i++) {
      // Where did letter i in the ciphertext come from?
      // It came from position (i - shift) mod 26 in the plaintext.
      // So the count at position (i - shift) mod 26 in the unshifted
      // distribution should match the count at position i in the ciphertext.
      unshifted[((i - shift) % 26 + 26) % 26] = cipherCounts[i];
    }

    // Compare the unshifted distribution to expected English frequencies.
    const chi2 = chiSquared(unshifted, totalLetters);

    if (chi2 < bestChi2) {
      bestChi2 = chi2;
      bestShift = shift;
    }
  }

  // Step 3: Decrypt with the best shift and return the result.
  return {
    shift: bestShift,
    plaintext: decrypt(ciphertext, bestShift),
  };
}
