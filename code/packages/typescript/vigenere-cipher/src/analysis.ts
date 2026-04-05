/**
 * analysis.ts -- Cryptanalysis tools for breaking the Vigenere cipher.
 *
 * Breaking the Vigenere Cipher
 * ============================
 *
 * For 300 years the Vigenere cipher was considered unbreakable because
 * each letter uses a different shift, defeating simple frequency analysis.
 * Friedrich Kasiski (1863) and William Friedman (1920s) independently
 * developed methods to break it using two key insights:
 *
 * 1. **Index of Coincidence (IC)** -- A statistical measure of how
 *    "non-uniform" a text's letter distribution is. English text has
 *    IC ~0.0667 (letters are unevenly distributed: E is common, Z is
 *    rare). Random text has IC ~0.0385 (1/26). When you guess the
 *    correct key length and split the ciphertext into groups, each
 *    group is a simple Caesar cipher on English, so its IC should be
 *    near the English value.
 *
 * 2. **Chi-squared statistic** -- Once we know the key length, each
 *    position group is a Caesar cipher. We try all 26 shifts on each
 *    group and pick the one whose letter frequencies best match English.
 *    The chi-squared statistic measures how far observed frequencies
 *    deviate from expected English frequencies.
 *
 * The Algorithm
 * -------------
 *
 * Step 1: Find the key length
 *   - For each candidate key length k = 2, 3, ..., maxLength:
 *     - Split ciphertext letters into k groups (every k-th letter)
 *     - Calculate IC of each group
 *     - Average the ICs
 *   - The k with the highest average IC is our best guess
 *
 * Step 2: Find each key letter
 *   - For position i = 0, 1, ..., k-1:
 *     - Extract the group of every k-th letter starting at position i
 *     - For each candidate shift s = 0..25:
 *       - Decrypt the group by shifting back by s
 *       - Calculate chi-squared against English frequencies
 *     - The shift s with the *lowest* chi-squared is key letter i
 *
 * Step 3: Decrypt using the recovered key
 */

import { decrypt } from "./cipher.js";

/**
 * English letter frequencies (A-Z).
 *
 * These are the expected proportions of each letter in a large sample
 * of English text. Source: standard English frequency tables.
 *
 *   E ~12.7%, T ~9.1%, A ~8.2%, O ~7.5%, I ~7.0%, N ~6.7%, ...
 *   Z ~0.07%, Q ~0.10%, X ~0.15%, J ~0.15%
 */
const ENGLISH_FREQUENCIES: readonly number[] = [
  0.08167, // A
  0.01492, // B
  0.02782, // C
  0.04253, // D
  0.12702, // E
  0.02228, // F
  0.02015, // G
  0.06094, // H
  0.06966, // I
  0.00153, // J
  0.00772, // K
  0.04025, // L
  0.02406, // M
  0.06749, // N
  0.07507, // O
  0.01929, // P
  0.00095, // Q
  0.05987, // R
  0.06327, // S
  0.09056, // T
  0.02758, // U
  0.00978, // V
  0.02360, // W
  0.00150, // X
  0.01974, // Y
  0.00074, // Z
];

/**
 * Expected Index of Coincidence for English text.
 *
 * IC = sum(f_i * (f_i - 1)) / (N * (N - 1)) where f_i is the count
 * of letter i and N is the total letter count. For English, this is
 * approximately 0.0667.
 */
const ENGLISH_IC = 0.0667;

/**
 * Calculate the Index of Coincidence for a string of letters.
 *
 * The IC measures how likely it is that two randomly chosen letters
 * from the text are the same. Higher IC means more "structured"
 * (language-like) text; lower IC means more random/uniform.
 *
 *   IC = sum_{i=A}^{Z} count_i * (count_i - 1) / (N * (N - 1))
 *
 * @param letters - A string containing only uppercase letters.
 * @returns The index of coincidence (0.0 to 1.0).
 */
function indexOfCoincidence(letters: string): number {
  const n = letters.length;
  if (n <= 1) return 0;

  // Count frequency of each letter A-Z
  const counts = new Array(26).fill(0);
  for (const ch of letters) {
    counts[ch.charCodeAt(0) - 65]++;
  }

  // IC = sum(count_i * (count_i - 1)) / (N * (N - 1))
  let numerator = 0;
  for (const count of counts) {
    numerator += count * (count - 1);
  }

  return numerator / (n * (n - 1));
}

/**
 * Calculate the chi-squared statistic comparing observed letter counts
 * to expected English frequencies.
 *
 * Chi-squared = sum((observed_i - expected_i)^2 / expected_i)
 *
 * A *lower* chi-squared means the distribution is closer to English.
 * This is the key metric for determining which Caesar shift produces
 * the most English-like text.
 *
 * @param counts - Array of 26 letter counts (A=0, B=1, ..., Z=25).
 * @param total - Total number of letters.
 * @returns The chi-squared statistic.
 */
function chiSquared(counts: number[], total: number): number {
  let chi2 = 0;
  for (let i = 0; i < 26; i++) {
    const expected = ENGLISH_FREQUENCIES[i] * total;
    const diff = counts[i] - expected;
    chi2 += (diff * diff) / expected;
  }
  return chi2;
}

/**
 * Extract only the uppercase alphabetic characters from text.
 *
 * This is used to strip punctuation, spaces, and digits before
 * performing frequency analysis. All letters are converted to
 * uppercase so the analysis is case-insensitive.
 *
 * @param text - Any string.
 * @returns Uppercase letters only.
 */
function extractAlphaUpper(text: string): string {
  return text.replace(/[^a-zA-Z]/g, "").toUpperCase();
}

/**
 * Estimate the key length of a Vigenere-encrypted ciphertext.
 *
 * For each candidate key length k from 2 to maxLength:
 *   1. Split the ciphertext letters into k groups (group i contains
 *      every k-th letter starting at position i).
 *   2. Calculate the Index of Coincidence (IC) of each group.
 *   3. Average the ICs.
 *
 * The correct key length will produce groups that are each a simple
 * Caesar cipher on English text, so their IC will be near 0.0667.
 * Wrong key lengths will mix letters from different Caesar ciphers,
 * producing more uniform (random-like) distributions with IC near
 * 0.0385.
 *
 * @param ciphertext - The encrypted text to analyze.
 * @param maxLength - Maximum key length to consider (default 20).
 * @returns The estimated key length.
 */
export function findKeyLength(
  ciphertext: string,
  maxLength: number = 20,
): number {
  const letters = extractAlphaUpper(ciphertext);

  if (letters.length < 2) {
    return 1;
  }

  // Calculate average IC for each candidate key length
  const limit = Math.min(maxLength, Math.floor(letters.length / 2));
  const avgICs: number[] = new Array(limit + 1).fill(0);

  for (let k = 2; k <= limit; k++) {
    // Split into k groups: group i gets letters at positions i, i+k, i+2k, ...
    let totalIC = 0;
    let groupCount = 0;

    for (let i = 0; i < k; i++) {
      let group = "";
      for (let j = i; j < letters.length; j += k) {
        group += letters[j];
      }

      if (group.length > 1) {
        totalIC += indexOfCoincidence(group);
        groupCount++;
      }
    }

    avgICs[k] = groupCount > 0 ? totalIC / groupCount : 0;
  }

  // Find the key length with the highest average IC.
  let bestLength = 1;
  let bestIC = 0;

  for (let k = 2; k <= limit; k++) {
    if (avgICs[k] > bestIC) {
      bestIC = avgICs[k];
      bestLength = k;
    }
  }

  // Multiples of the true key length also produce high IC values.
  // To find the true (smallest) key length, we use chi-squared
  // validation: try the key at each candidate length that is a
  // divisor of the best length and check which actually recovers
  // a valid key. If a divisor produces a substantially worse
  // chi-squared fit, the true key is longer.
  //
  // Simpler approach: among all k values with IC >= 90% of the best
  // IC, pick the smallest that is NOT a proper divisor of a k with
  // even higher IC (unless that smaller k has IC very close to the
  // larger k). In practice, the true key length and its multiples
  // have similar IC, but the true length's IC is typically slightly
  // lower than the best multiple. We pick the smallest k whose IC
  // is within 90% of the best.
  const icThreshold = bestIC * 0.9;

  for (let k = 2; k <= limit; k++) {
    if (avgICs[k] >= icThreshold) {
      return k;
    }
  }

  return bestLength;
}

/**
 * Find the key letters given a known key length.
 *
 * For each position in the key (0 to keyLength-1):
 *   1. Extract the group of letters at that position.
 *   2. Try all 26 possible shifts (A through Z).
 *   3. For each shift, compute letter frequencies of the "decrypted"
 *      group and calculate chi-squared against English.
 *   4. The shift with the lowest chi-squared is the key letter.
 *
 * This works because each group is just a Caesar cipher, and the
 * correct Caesar shift will produce English-like frequency distribution.
 *
 * @param ciphertext - The encrypted text to analyze.
 * @param keyLength - The known or estimated key length.
 * @returns The recovered key as an uppercase string.
 */
export function findKey(ciphertext: string, keyLength: number): string {
  const letters = extractAlphaUpper(ciphertext);
  let key = "";

  for (let pos = 0; pos < keyLength; pos++) {
    // Extract every keyLength-th letter starting at position pos
    let group = "";
    for (let j = pos; j < letters.length; j += keyLength) {
      group += letters[j];
    }

    if (group.length === 0) {
      key += "A";
      continue;
    }

    // Try all 26 shifts and pick the one with lowest chi-squared
    let bestShift = 0;
    let bestChi2 = Infinity;

    for (let shift = 0; shift < 26; shift++) {
      // "Decrypt" the group by shifting back by `shift`
      const counts = new Array(26).fill(0);
      for (const ch of group) {
        const decrypted = (ch.charCodeAt(0) - 65 - shift + 26) % 26;
        counts[decrypted]++;
      }

      const chi2 = chiSquared(counts, group.length);
      if (chi2 < bestChi2) {
        bestChi2 = chi2;
        bestShift = shift;
      }
    }

    // The best shift value corresponds to the key letter
    // (shift of 0 = A, shift of 1 = B, ..., shift of 25 = Z)
    key += String.fromCharCode(65 + bestShift);
  }

  return key;
}

/**
 * Result of automatic cipher breaking.
 */
export interface BreakResult {
  key: string;
  plaintext: string;
}

/**
 * Automatically break a Vigenere cipher.
 *
 * This combines both steps of cryptanalysis:
 *   1. Find the key length using IC analysis.
 *   2. Find the key letters using chi-squared analysis.
 *   3. Decrypt using the recovered key.
 *
 * Requires sufficiently long ciphertext (~200+ characters of English)
 * for reliable results. Short ciphertexts may produce incorrect keys.
 *
 * @param ciphertext - The encrypted text to break.
 * @returns An object containing the recovered key and plaintext.
 */
export function breakCipher(ciphertext: string): BreakResult {
  const keyLength = findKeyLength(ciphertext);
  const key = findKey(ciphertext, keyLength);
  const plaintext = decrypt(ciphertext, key);

  return { key, plaintext };
}
