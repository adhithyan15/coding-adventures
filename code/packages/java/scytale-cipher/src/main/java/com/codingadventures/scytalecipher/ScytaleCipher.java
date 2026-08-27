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
//   Ciphertext: "HORSEST LPA LAN "
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
import java.util.Arrays;
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
 * ScytaleCipher.encrypt("HELLOSPARTANS", 4)  // → "HORSEST LPA LAN "
 * ScytaleCipher.decrypt("HORSEST LPA LAN ", 4)  // → "HELLOSPARTANS"
 * }</pre>
 *
 * <p>All methods are static — pure utility class.
 */
public final class ScytaleCipher {

    public static final int MAX_BRUTE_FORCE_TEXT_LENGTH = 4096;

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
     *   Output: "HORSEST LPA LAN "
     * </pre>
     *
     * @param text the plaintext (any characters, including spaces/punctuation)
     * @param key  the number of columns (≥ 2, ≤ text length if non-empty)
     * @return the ciphertext
     * @throws IllegalArgumentException if key is invalid
     */
    public static String encrypt(String text, int key) {
        if (text.isEmpty()) return "";
        int[] scalars = text.codePoints().toArray();
        validateKey(key, scalars.length);

        // Number of rows needed (ceiling division)
        int rows = (scalars.length + key - 1) / key;
        int paddedLen = rows * key;

        // Build the padded scalar grid. A supplementary-plane scalar occupies
        // one cell even though Java stores it as two UTF-16 code units.
        int[] padded = Arrays.copyOf(scalars, paddedLen);
        Arrays.fill(padded, scalars.length, paddedLen, 0x20);

        // Read column-by-column
        StringBuilder sb = new StringBuilder(paddedLen);
        for (int col = 0; col < key; col++) {
            for (int row = 0; row < rows; row++) {
                sb.appendCodePoint(padded[row * key + col]);
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
        int[] scalars = text.codePoints().toArray();
        validateKey(key, scalars.length);

        int len = scalars.length;
        int rows = (len + key - 1) / key;
        int remainder = len % key;
        int[] columnStarts = new int[key];
        int[] columnLengths = new int[key];
        int offset = 0;
        for (int column = 0; column < key; column++) {
            columnStarts[column] = offset;
            columnLengths[column] = remainder == 0 || column < remainder ? rows : rows - 1;
            offset += columnLengths[column];
        }

        int[] plaintext = new int[len];
        int output = 0;
        for (int row = 0; row < rows; row++) {
            for (int column = 0; column < key; column++) {
                if (row < columnLengths[column]) {
                    plaintext[output++] = scalars[columnStarts[column] + row];
                }
            }
        }
        while (output > 0 && plaintext[output - 1] == 0x20) output--;
        return new String(plaintext, 0, output);
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
        int scalarLength = text.codePointCount(0, text.length());
        if (scalarLength > MAX_BRUTE_FORCE_TEXT_LENGTH) {
            throw new IllegalArgumentException("scytale-brute-force-limit");
        }
        if (scalarLength < 4) return results;
        for (int key = 2; key <= scalarLength / 2; key++) {
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
