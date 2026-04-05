/**
 * # Frequency Count
 *
 * Counts how many times each letter (A-Z) appears in the text.
 * Non-alphabetic characters are ignored. The count is case-insensitive:
 * both 'a' and 'A' increment the count for 'A'.
 *
 * ## Example
 *
 *     frequencyCount("Hello!")
 *     // => { H: 1, E: 1, L: 2, O: 1 }
 *
 * Note: Only letters that appear at least once are included in the result.
 * If you need all 26 letters (with zeros), use the result with a
 * fallback: `result.get(letter) ?? 0`.
 *
 * @param text - Input text to analyze
 * @returns A Map from uppercase letter to count
 */
export function frequencyCount(text: string): Map<string, number> {
  const counts = new Map<string, number>();

  for (const char of text) {
    // Convert to uppercase for case-insensitive counting.
    const upper = char.toUpperCase();

    // Only count A-Z. Skip digits, spaces, punctuation, etc.
    if (upper >= "A" && upper <= "Z") {
      counts.set(upper, (counts.get(upper) ?? 0) + 1);
    }
  }

  return counts;
}
