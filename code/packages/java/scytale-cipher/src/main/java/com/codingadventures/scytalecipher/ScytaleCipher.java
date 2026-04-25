// ============================================================================
// ScytaleCipher.java — The ancient Greek transposition cipher
// ============================================================================
//
// The scytale (pronounced "SKIT-uh-lee") was used by the Spartans around
// 7th century BCE. A strip of parchment was wound helically around a wooden
// staff (the scytale), and the message was written lengthwise. When unwound,
// the strip appeared as a jumble of characters — unreadable without a staff of
// the same diameter to re-wind it.
//
// The key is the diameter of the staff, which determines the number of
// columns in a grid when translated to a paper cipher:
//
//   Plaintext: "HELLOSPARTANS"   key = 4 (columns)
//
//   Write row-by-row into 4 columns, padding with spaces:
//
//     Row 0:  H  E  L  L
//     Row 1:  O  S  P  A
//     Row 2:  R  T  A  N
//     Row 3:  S  _  _  _     ← '_' represents space padding
//
//   Read column-by-column (down each column):
//     Col 0: H O R S
//     Col 1: E S T _
//     Col 2: L P A _
//     Col 3: L A N _
//
//   Ciphertext: "HORSEST LPAAL N "
//
//   To decrypt: write the ciphertext column-by-column into the same grid,
//   then read row-by-row, stripping trailing padding spaces.
//
// This is a pure transposition cipher — the letters are not changed, only
// their positions. Frequency analysis of individual letters is therefore
// ineffective (the frequency distribution matches plaintext), but n-gram
// analysis on the decrypted text breaks it easily.
//

package com.codingadventures.scytalecipher;

import java.util.ArrayList;
import java.util.List;

/**
 * The Scytale cipher — a columnar transposition cipher.
 *
 * <p>Encrypts by writing plaintext row-by-row into a grid of {@code key} columns,
 * then reading column-by-column. Padding spaces fill the last row.
 *
 * <p>Decrypts by writing ciphertext column-by-column, reading row-by-row,
 * then stripping trailing padding spaces.
 *
 * <pre>{@code
 * ScytaleCipher.encrypt("HELLOSPARTANS", 4)  // → "HORSEST LPAAL N "
 * ScytaleCipher.decrypt("HORSEST LPAAL N ", 4)  // → "HELLOSPARTANS"
 * }</pre>
 *
 * <p>All methods are static — pure utility class.
 */
public final class ScytaleCipher {

    // Private constructor.
    private ScytaleCipher() {}

    // =========================================================================
    // Validation helper
    // =========================================================================

    /**
     * Validate that {@code key} is a usable cipher key for the given text length.
     *
     * @param key  the number of columns
     * @param textLen the length of the text being processed
     * @throws IllegalArgumentException if key < 2 or key > textLen (and textLen > 0)
     */
    private static void validateKey(int key, int textLen) {
        if (key < 2) {
            throw new IllegalArgumentException("key must be at least 2, got: " + key);
        }
        if (textLen > 0 && key > textLen) {
            throw new IllegalArgumentException(
                "key (" + key + ") must not exceed text length (" + textLen + ")");
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt plaintext using the Scytale cipher.
     *
     * <p>Writes text row-by-row into a grid of {@code key} columns, padding
     * the last row with spaces, then reads column-by-column.
     *
     * <p>Example (key=4):
     * <pre>
     *   Input:   "HELLOSPARTANS"
     *   Grid:    H E L L
     *            O S P A
     *            R T A N
     *            S _ _ _   ← spaces
     *   Output: "HORSEST LPAAL N "
     * </pre>
     *
     * @param text the plaintext (any characters, including spaces/punctuation)
     * @param key  the number of columns (≥ 2, ≤ text length if non-empty)
     * @return the ciphertext
     * @throws IllegalArgumentException if key is invalid
     */
    public static String encrypt(String text, int key) {
        if (text.isEmpty()) return "";
        validateKey(key, text.length());

        // Number of rows needed (ceiling division)
        int rows = (text.length() + key - 1) / key;
        int paddedLen = rows * key;

        // Build the padded character grid
        char[] padded = new char[paddedLen];
        for (int i = 0; i < text.length(); i++) padded[i] = text.charAt(i);
        for (int i = text.length(); i < paddedLen; i++) padded[i] = ' ';

        // Read column-by-column
        StringBuilder sb = new StringBuilder(paddedLen);
        for (int col = 0; col < key; col++) {
            for (int row = 0; row < rows; row++) {
                sb.append(padded[row * key + col]);
            }
        }
        return sb.toString();
    }

    /**
     * Decrypt ciphertext that was encrypted with the Scytale cipher.
     *
     * <p>Writes ciphertext column-by-column into a grid, reads row-by-row,
     * then strips trailing padding spaces.
     *
     * @param text the ciphertext
     * @param key  the number of columns used during encryption (≥ 2)
     * @return the original plaintext (trailing padding spaces stripped)
     * @throws IllegalArgumentException if key is invalid
     */
    public static String decrypt(String text, int key) {
        if (text.isEmpty()) return "";
        validateKey(key, text.length());

        int len = text.length();
        int rows = (len + key - 1) / key;

        // Ciphertext was read column-by-column: ciphertext[c*rows + r] = grid[r][c]
        // To reverse: grid[r][c] = ciphertext[c*rows + r]
        // Read grid row-by-row: grid[r][c] = ciphertext[c*rows + r]
        char[] grid = new char[len];
        for (int col = 0; col < key; col++) {
            for (int row = 0; row < rows; row++) {
                int cipherIdx = col * rows + row;
                int gridIdx   = row * key  + col;
                if (cipherIdx < len && gridIdx < len) {
                    grid[gridIdx] = text.charAt(cipherIdx);
                }
            }
        }

        // Strip trailing padding spaces
        String result = new String(grid);
        int end = result.length();
        while (end > 0 && result.charAt(end - 1) == ' ') end--;
        return result.substring(0, end);
    }

    /**
     * Brute-force attack: try all valid key values.
     *
     * <p>Tries key values from 2 up to {@code text.length() / 2}, inclusive.
     * Returns an empty list if the text is too short (fewer than 4 characters).
     *
     * @param text the ciphertext
     * @return list of {@link BruteForceResult} for all valid keys
     */
    public static List<BruteForceResult> bruteForce(String text) {
        List<BruteForceResult> results = new ArrayList<>();
        if (text.length() < 4) return results;
        for (int key = 2; key <= text.length() / 2; key++) {
            results.add(new BruteForceResult(key, decrypt(text, key)));
        }
        return results;
    }

    // =========================================================================
    // Result type
    // =========================================================================

    /** A (key, plaintext) pair returned by {@link #bruteForce}. */
    public static final class BruteForceResult {
        /** The key tried. */
        public final int key;
        /** The plaintext produced by this key. */
        public final String text;

        public BruteForceResult(int key, String text) {
            this.key  = key;
            this.text = text;
        }

        @Override
        public String toString() {
            return "BruteForceResult{key=" + key + ", text=\"" + text + "\"}";
        }
    }
}
