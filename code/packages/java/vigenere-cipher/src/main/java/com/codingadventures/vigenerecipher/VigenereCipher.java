// ============================================================================
// VigenereCipher.java — The polyalphabetic substitution cipher
// ============================================================================
//
// The Vigenère cipher was described by Giovan Battista Bellaso in 1553 and
// later misattributed to Blaise de Vigenère. It was called "le chiffre
// indéchiffrable" (the unbreakable cipher) for three centuries until Charles
// Babbage broke it around 1854 using the index of coincidence — a method
// rediscovered independently by Friedrich Kasiski in 1863.
//
// The key idea:
// -------------
// Instead of one fixed shift (as in Caesar), the Vigenère cipher uses a
// keyword to determine a different shift for each letter position.
//
//   keyword:    L E M O N L E M O N L E M O N ...  (repeats)
//   plaintext:  A T T A C K A T D A W N
//   shifts:     11 4 12 14 13 11 4 12 14 13 11 4
//   ciphertext: L X F O P V E F R N H R
//
// Encryption formula:
//   C[i] = (P[i] + K[i mod len(K)]) mod 26     (for alphabetic chars only)
//
// Decryption formula:
//   P[i] = (C[i] - K[i mod len(K)] + 26) mod 26
//
// Key properties:
// ---------------
// - Non-alphabetic characters pass through unchanged and do NOT advance the
//   key position counter. This matches the Python/Rust/TypeScript implementations.
// - The key is case-insensitive (normalized to uppercase internally).
// - An empty or non-alpha key raises IllegalArgumentException.
//
// Breaking Vigenère — Index of Coincidence method:
// ------------------------------------------------
// The Index of Coincidence (IC) of a text is the probability that two
// randomly chosen characters are the same:
//
//   IC = Σ n_i(n_i - 1) / (N(N - 1))
//
// where n_i is the count of letter i and N is the total count of letters.
//
// For random text: IC ≈ 0.038
// For English text: IC ≈ 0.065
//
// Key insight: if we know the key length L, we can split the ciphertext into
// L groups (position 0, L, 2L, ...; position 1, L+1, 2L+1, ...; etc.). Each
// group was encrypted with the SAME shift, so it behaves like a Caesar cipher.
// The group IC will be close to English IC (0.065) when the key length is
// correct and close to random IC (0.038) when it's wrong.
//
// Once we have the key length, each group is broken with chi-squared against
// English letter frequencies — the same technique used in CaesarCipher.
//

package com.codingadventures.vigenerecipher;

/**
 * The Vigenère cipher — a polyalphabetic substitution cipher.
 *
 * <p>Uses a keyword to apply different Caesar shifts at each position.
 * Non-alphabetic characters pass through unchanged without advancing the key.
 *
 * <p>Includes automatic ciphertext-only cryptanalysis via IC and chi-squared.
 *
 * <pre>{@code
 * VigenereCipher.encrypt("ATTACKATDAWN", "LEMON")  // → "LXFOPVEFRNHR"
 * VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON")  // → "ATTACKATDAWN"
 *
 * // Automatic attack (needs ≥ 200 chars for reliability)
 * VigenereCipher.BreakResult r = VigenereCipher.breakCipher(ciphertext);
 * System.out.println("Key: " + r.key + ", Plaintext: " + r.plaintext);
 * }</pre>
 */
public final class VigenereCipher {

    // Private constructor.
    private VigenereCipher() {}

    // =========================================================================
    // English letter frequencies
    // =========================================================================
    //
    // Indexed 0=A … 25=Z. Same table as CaesarCipher.

    /** English letter frequencies indexed A=0 … Z=25. */
    public static final double[] ENGLISH_FREQUENCIES = {
        0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015,
        0.06094, 0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749,
        0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056, 0.02758,
        0.00978, 0.02360, 0.00150, 0.01974, 0.00074,
    };

    // =========================================================================
    // Key validation
    // =========================================================================

    /**
     * Validate and normalise the keyword.
     *
     * @param key the raw keyword string
     * @return the normalised keyword (uppercase, letters only confirmed)
     * @throws IllegalArgumentException if the key is empty or contains non-alpha
     */
    private static String validateKey(String key) {
        if (key == null || key.isEmpty()) {
            throw new IllegalArgumentException("Key must not be empty");
        }
        String upper = key.toUpperCase();
        for (int i = 0; i < upper.length(); i++) {
            char c = upper.charAt(i);
            if (c < 'A' || c > 'Z') {
                throw new IllegalArgumentException(
                    "Key must contain only letters, got: '" + key + "'");
            }
        }
        return upper;
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt plaintext using the Vigenère cipher.
     *
     * <p>Advances the key position only on alphabetic characters. Non-alpha
     * characters are copied unchanged.
     *
     * <p>Known test vector: {@code encrypt("ATTACKATDAWN", "LEMON")} → {@code "LXFOPVEFRNHR"}.
     *
     * @param plaintext the text to encrypt
     * @param key       the keyword (letters only, case-insensitive)
     * @return the ciphertext
     * @throws IllegalArgumentException if key is invalid
     */
    public static String encrypt(String plaintext, String key) {
        String k = validateKey(key);
        int kLen = k.length();
        int kPos = 0;
        StringBuilder sb = new StringBuilder(plaintext.length());
        for (int i = 0; i < plaintext.length(); i++) {
            char c = plaintext.charAt(i);
            if (c >= 'A' && c <= 'Z') {
                int shift = k.charAt(kPos % kLen) - 'A';
                sb.append((char) ('A' + (c - 'A' + shift) % 26));
                kPos++;
            } else if (c >= 'a' && c <= 'z') {
                int shift = k.charAt(kPos % kLen) - 'A';
                sb.append((char) ('a' + (c - 'a' + shift) % 26));
                kPos++;
            } else {
                sb.append(c);
            }
        }
        return sb.toString();
    }

    /**
     * Decrypt ciphertext encrypted with the Vigenère cipher.
     *
     * @param ciphertext the text to decrypt
     * @param key        the keyword used for encryption (letters only, case-insensitive)
     * @return the original plaintext
     * @throws IllegalArgumentException if key is invalid
     */
    public static String decrypt(String ciphertext, String key) {
        String k = validateKey(key);
        int kLen = k.length();
        int kPos = 0;
        StringBuilder sb = new StringBuilder(ciphertext.length());
        for (int i = 0; i < ciphertext.length(); i++) {
            char c = ciphertext.charAt(i);
            if (c >= 'A' && c <= 'Z') {
                int shift = k.charAt(kPos % kLen) - 'A';
                sb.append((char) ('A' + (c - 'A' - shift + 26) % 26));
                kPos++;
            } else if (c >= 'a' && c <= 'z') {
                int shift = k.charAt(kPos % kLen) - 'A';
                sb.append((char) ('a' + (c - 'a' - shift + 26) % 26));
                kPos++;
            } else {
                sb.append(c);
            }
        }
        return sb.toString();
    }

    // =========================================================================
    // Cryptanalysis — Index of Coincidence key-length finder
    // =========================================================================

    /**
     * Estimate the key length using the Index of Coincidence.
     *
     * <p>Tries candidate lengths from 2 to {@code maxLength} (default 20).
     * For each candidate L, splits the ciphertext into L groups and computes
     * the average IC across groups. The correct key length produces groups
     * that look like Caesar-cipher ciphertext (IC ≈ 0.065) rather than random
     * text (IC ≈ 0.038).
     *
     * <p>Returns the smallest candidate length within 5% of the best average IC.
     *
     * @param ciphertext the ciphertext (only alphabetic characters are analysed)
     * @param maxLength  maximum key length to try (must be ≥ 2)
     * @return the estimated key length
     */
    public static int findKeyLength(String ciphertext, int maxLength) {
        // Extract only alphabetic characters
        StringBuilder alphaOnly = new StringBuilder();
        for (int i = 0; i < ciphertext.length(); i++) {
            char c = ciphertext.charAt(i);
            if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')) {
                alphaOnly.append(Character.toUpperCase(c));
            }
        }
        String s = alphaOnly.toString();
        int n = s.length();

        int limit = Math.min(maxLength, n / 2);
        double[] avgIcs = new double[limit + 1];
        double bestAvgIc = -1.0;

        for (int L = 2; L <= limit; L++) {
            double totalIc = 0.0;
            int validGroups = 0;
            for (int g = 0; g < L; g++) {
                // Extract every L-th character starting at position g
                int[] counts = new int[26];
                int groupLen = 0;
                for (int pos = g; pos < n; pos += L) {
                    counts[s.charAt(pos) - 'A']++;
                    groupLen++;
                }
                // Compute IC for this group
                if (groupLen < 2) continue;
                long numerator = 0;
                for (int i = 0; i < 26; i++) {
                    numerator += (long) counts[i] * (counts[i] - 1);
                }
                totalIc += (double) numerator / ((long) groupLen * (groupLen - 1));
                validGroups++;
            }
            if (validGroups > 0) {
                avgIcs[L] = totalIc / validGroups;
                if (avgIcs[L] > bestAvgIc) bestAvgIc = avgIcs[L];
            }
        }

        // Return the SMALLEST candidate within 5% of the best average IC,
        // excluding multiples of smaller candidates that also scored well.
        //
        // Multiples of the true key length also score well (k, 2k, 3k all
        // produce Caesar-cipher groups). The divisor-filtering step below
        // prevents returning 2k or 3k when k is already a qualifying candidate.
        double threshold = bestAvgIc * 0.95;

        // Collect all qualifying key-length candidates in ascending order.
        java.util.List<Integer> candidates = new java.util.ArrayList<>();
        for (int L = 2; L <= limit; L++) {
            if (avgIcs[L] >= threshold) candidates.add(L);
        }
        if (candidates.isEmpty()) return 2;

        // Remove any candidate that is a multiple of a smaller candidate in
        // the list.  The smallest residual candidate is the true key length.
        for (int i = 0; i < candidates.size(); i++) {
            int smaller = candidates.get(i);
            candidates.removeIf(bigger -> bigger != smaller && bigger % smaller == 0);
        }
        return candidates.get(0);
    }

    /** {@link #findKeyLength(String, int)} with default maxLength = 20. */
    public static int findKeyLength(String ciphertext) {
        return findKeyLength(ciphertext, 20);
    }

    // =========================================================================
    // Cryptanalysis — chi-squared key recovery
    // =========================================================================

    /**
     * Recover the keyword given the ciphertext and the correct key length.
     *
     * <p>Splits the ciphertext into {@code keyLength} groups (one per key position),
     * then runs chi-squared analysis on each group to find the Caesar shift —
     * which equals the corresponding keyword letter.
     *
     * @param ciphertext the ciphertext
     * @param keyLength  the key length (from {@link #findKeyLength})
     * @return the recovered keyword (uppercase)
     */
    public static String findKey(String ciphertext, int keyLength) {
        // Extract alphabetic characters only, uppercase
        StringBuilder alphaOnly = new StringBuilder();
        for (int i = 0; i < ciphertext.length(); i++) {
            char c = ciphertext.charAt(i);
            if ((c >= 'A' && c <= 'Z') || (c >= 'a' && c <= 'z')) {
                alphaOnly.append(Character.toUpperCase(c));
            }
        }
        String s = alphaOnly.toString();

        char[] keyChars = new char[keyLength];
        for (int g = 0; g < keyLength; g++) {
            // Extract every keyLength-th character
            int[] counts = new int[26];
            int groupLen = 0;
            for (int pos = g; pos < s.length(); pos += keyLength) {
                counts[s.charAt(pos) - 'A']++;
                groupLen++;
            }
            if (groupLen == 0) { keyChars[g] = 'A'; continue; }

            // Chi-squared against English frequencies for each shift
            double bestScore = Double.MAX_VALUE;
            int bestShift = 0;
            for (int k = 0; k < 26; k++) {
                double chiSq = 0.0;
                for (int i = 0; i < 26; i++) {
                    int plainIdx = (i - k + 26) % 26;
                    double expected = ENGLISH_FREQUENCIES[plainIdx] * groupLen;
                    double diff = counts[i] - expected;
                    chiSq += (diff * diff) / expected;
                }
                if (chiSq < bestScore) {
                    bestScore = chiSq;
                    bestShift = k;
                }
            }
            keyChars[g] = (char) ('A' + bestShift);
        }

        // If the recovered key has a repeating sub-period, return the minimal
        // period.  This handles cases where the IC key-length estimator found
        // a multiple of the true key length (e.g. 10 instead of 5 for LEMON).
        String fullKey = new String(keyChars);
        for (int p = 1; p <= keyLength / 2; p++) {
            if (keyLength % p != 0) continue;
            String base = fullKey.substring(0, p);
            boolean repeated = true;
            for (int i = p; i < keyLength; i++) {
                if (fullKey.charAt(i) != fullKey.charAt(i % p)) {
                    repeated = false;
                    break;
                }
            }
            if (repeated) return base;
        }
        return fullKey;
    }

    // =========================================================================
    // Full automatic attack
    // =========================================================================

    /**
     * Fully automatic ciphertext-only attack.
     *
     * <p>Chains {@link #findKeyLength} → {@link #findKey} → {@link #decrypt}.
     *
     * <p>Works best on ciphertexts of 200+ alphabetic characters.
     *
     * @param ciphertext the ciphertext to break
     * @return a {@link BreakResult} with the recovered key and plaintext
     */
    public static BreakResult breakCipher(String ciphertext) {
        int keyLen = findKeyLength(ciphertext);
        String key = findKey(ciphertext, keyLen);
        String plaintext = decrypt(ciphertext, key);
        return new BreakResult(key, plaintext);
    }

    // =========================================================================
    // Result type
    // =========================================================================

    /** The result of {@link #breakCipher}: recovered key and decrypted plaintext. */
    public static final class BreakResult {
        /** The recovered keyword (uppercase). */
        public final String key;
        /** The decrypted plaintext. */
        public final String plaintext;

        public BreakResult(String key, String plaintext) {
            this.key       = key;
            this.plaintext = plaintext;
        }

        @Override
        public String toString() {
            return "BreakResult{key=\"" + key + "\", plaintext=\"" + plaintext + "\"}";
        }
    }
}
