// ============================================================================
// CaesarCipher.java — The classical shift cipher
// ============================================================================
//
// The Caesar cipher is named after Julius Caesar, who reportedly used a shift
// of 3 to protect messages of military significance. Suetonius describes it:
// "If he had anything confidential to say, he wrote it in cipher, that is, by
// so changing the order of the letters of the alphabet, that not a word could
// be made out."
//
// The principle is simple: shift every letter forward in the alphabet by a
// fixed number of positions, wrapping around from Z back to A.
//
//   shift = 3:   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//                ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓ ↓
//                D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
//
//   encrypt("HELLO", 3) = "KHOOR"
//   decrypt("KHOOR", 3) = "HELLO"
//
// ROT13:
// ------
// ROT13 is a special case of Caesar cipher with shift = 13. Because the
// alphabet has 26 letters, shifting by 13 is self-inverse (just like Atbash):
//   ROT13(ROT13("HELLO")) = "HELLO"
// ROT13 is widely used on the internet to obscure spoilers and jokes.
//
// Security:
// ---------
// The Caesar cipher has only 26 possible keys (shifts 0–25). An attacker can
// try all 26 in under a second. Even without brute force, frequency analysis
// works: 'E' is the most common English letter. If 'H' appears most often in
// the ciphertext, the shift is probably 3 (H - E = 3). This cipher provides
// no real security and should never be used for sensitive data.
//

package com.codingadventures.caesarcipher;

import java.util.ArrayList;
import java.util.List;

/**
 * The Caesar cipher — a classical shift cipher.
 *
 * <p>Shifts every letter forward in the alphabet by a fixed amount. Non-alphabetic
 * characters pass through unchanged. Case is preserved.
 *
 * <p>Includes ROT13 (shift=13), brute-force decryption (all 25 shifts), and
 * frequency analysis for automatic ciphertext-only attack.
 *
 * <pre>{@code
 * CaesarCipher.encrypt("Hello, World!", 3)  // → "Khoor, Zruog!"
 * CaesarCipher.decrypt("Khoor, Zruog!", 3)  // → "Hello, World!"
 * CaesarCipher.rot13("Hello")               // → "Uryyb"
 * }</pre>
 */
public final class CaesarCipher {

    // Private constructor: pure utility class.
    private CaesarCipher() {}

    // =========================================================================
    // English letter frequencies (for chi-squared analysis)
    // =========================================================================
    //
    // The frequency of each letter A–Z in typical English text, as proportions
    // (values sum to approximately 1.0). Source: Lewand (2000), "Cryptological
    // Mathematics," based on large English corpora.
    //
    // Key insight: 'E' (index 4) is most common at ~12.7%, followed by 'T' at
    // ~9.1%. If ciphertext has a different letter as the most common, the shift
    // is the distance from that letter back to 'E'.

    /** English letter frequencies indexed A=0 … Z=25. */
    public static final double[] ENGLISH_FREQUENCIES = {
        0.08167, // A
        0.01492, // B
        0.02782, // C
        0.04253, // D
        0.12702, // E  ← most common
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
    };

    // =========================================================================
    // Core shift
    // =========================================================================

    /**
     * Shift a single alphabetic character by {@code shift} positions, wrapping.
     * Returns non-alpha characters unchanged.
     */
    private static char shiftChar(char c, int shift) {
        if (c >= 'A' && c <= 'Z') {
            return (char) ('A' + ((c - 'A' + shift % 26 + 26) % 26));
        } else if (c >= 'a' && c <= 'z') {
            return (char) ('a' + ((c - 'a' + shift % 26 + 26) % 26));
        } else {
            return c;
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt a string by shifting every letter forward by {@code shift} positions.
     *
     * <p>Shift is taken modulo 26, so values outside 0–25 work correctly.
     * Negative shifts are allowed (equivalent to decrypting with that shift).
     *
     * @param text  the plaintext
     * @param shift any integer — taken mod 26 internally
     * @return the ciphertext
     */
    public static String encrypt(String text, int shift) {
        StringBuilder sb = new StringBuilder(text.length());
        for (int i = 0; i < text.length(); i++) {
            sb.append(shiftChar(text.charAt(i), shift));
        }
        return sb.toString();
    }

    /**
     * Decrypt a string by shifting every letter backward by {@code shift} positions.
     *
     * <p>Equivalent to {@code encrypt(text, -shift)}.
     *
     * @param text  the ciphertext
     * @param shift the shift used during encryption
     * @return the original plaintext
     */
    public static String decrypt(String text, int shift) {
        return encrypt(text, -shift);
    }

    /**
     * ROT13 — Caesar cipher with shift = 13.
     *
     * <p>Self-inverse: {@code rot13(rot13(text)) == text}. Applying ROT13 to
     * plaintext encrypts it; applying it again decrypts it.
     *
     * <p>Widely used online to obscure spoilers and punchlines.
     *
     * @param text any string
     * @return the ROT13 transformation
     */
    public static String rot13(String text) {
        return encrypt(text, 13);
    }

    /**
     * Try all 25 non-trivial shifts and return every possible decryption.
     *
     * <p>Returns a list of 25 {@code int[2]}-style records represented as
     * {@code int[]{shift, index}} — actually a list of {@link BruteForceResult}.
     * Shifts 1–25 are tried; shift 0 (identity) is omitted.
     *
     * @param ciphertext the ciphertext to crack
     * @return list of (shift, plaintext) pairs for all 25 non-trivial shifts
     */
    public static List<BruteForceResult> bruteForce(String ciphertext) {
        List<BruteForceResult> results = new ArrayList<>(25);
        for (int shift = 1; shift <= 25; shift++) {
            results.add(new BruteForceResult(shift, decrypt(ciphertext, shift)));
        }
        return results;
    }

    /**
     * Automatic frequency-analysis attack — find the most likely shift.
     *
     * <p>Counts letters in the ciphertext, computes the chi-squared statistic
     * against the expected English distribution for each of the 26 possible
     * shifts, and returns the shift with the lowest chi-squared score.
     *
     * <p>Reliability increases with ciphertext length. For texts of fewer than
     * ~20 alphabetic characters the result may be unreliable.
     *
     * @param ciphertext the ciphertext (only alphabetic characters are analysed)
     * @return the best (shift, plaintext) guess; shift=0 for empty input
     */
    public static FrequencyResult frequencyAnalysis(String ciphertext) {
        // Count each letter (case-insensitive).
        int[] counts = new int[26];
        int total = 0;
        for (int i = 0; i < ciphertext.length(); i++) {
            char c = ciphertext.charAt(i);
            if (c >= 'A' && c <= 'Z') { counts[c - 'A']++; total++; }
            else if (c >= 'a' && c <= 'z') { counts[c - 'a']++; total++; }
        }
        if (total == 0) return new FrequencyResult(0, ciphertext);

        // For each candidate shift k, compute chi-squared against English freq.
        // Chi-squared: Σ (observed - expected)² / expected
        // We assume shift k means ciphertext letter i actually represents letter (i - k + 26) % 26.
        double bestScore = Double.MAX_VALUE;
        int bestShift = 0;
        for (int k = 0; k < 26; k++) {
            double chiSq = 0.0;
            for (int i = 0; i < 26; i++) {
                int plainIdx = (i - k + 26) % 26;
                double expected = ENGLISH_FREQUENCIES[plainIdx] * total;
                double diff = counts[i] - expected;
                chiSq += (diff * diff) / expected;
            }
            if (chiSq < bestScore) {
                bestScore = chiSq;
                bestShift = k;
            }
        }
        return new FrequencyResult(bestShift, decrypt(ciphertext, bestShift));
    }

    // =========================================================================
    // Result types
    // =========================================================================

    /** A (shift, plaintext) pair returned by {@link #bruteForce}. */
    public static final class BruteForceResult {
        /** The shift tried (1–25). */
        public final int shift;
        /** The plaintext produced by this shift. */
        public final String text;

        public BruteForceResult(int shift, String text) {
            this.shift = shift;
            this.text  = text;
        }

        @Override
        public String toString() {
            return "BruteForceResult{shift=" + shift + ", text=\"" + text + "\"}";
        }
    }

    /** A (shift, plaintext) pair returned by {@link #frequencyAnalysis}. */
    public static final class FrequencyResult {
        /** The shift determined by frequency analysis. */
        public final int shift;
        /** The plaintext produced by applying this shift. */
        public final String text;

        public FrequencyResult(int shift, String text) {
            this.shift = shift;
            this.text  = text;
        }

        @Override
        public String toString() {
            return "FrequencyResult{shift=" + shift + ", text=\"" + text + "\"}";
        }
    }
}
