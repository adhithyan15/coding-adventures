// ============================================================================
// CaesarCipher.kt — The classical shift cipher
// ============================================================================
//
// The Caesar cipher is named after Julius Caesar, who reportedly used a shift
// of 3 to protect messages of military significance.
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
// ROT13 is a special case with shift = 13. Because the alphabet has 26 letters,
// shifting by 13 is self-inverse: ROT13(ROT13("HELLO")) = "HELLO".
//
// Security:
// ---------
// The Caesar cipher has only 26 possible keys (shifts 0–25). An attacker can
// try all 26 in under a second. Even without brute force, frequency analysis
// works instantly. This cipher provides no real security.
//

package com.codingadventures.caesarcipher

/**
 * The Caesar cipher — a classical shift cipher.
 *
 * Shifts every letter forward in the alphabet by a fixed amount. Non-alphabetic
 * characters pass through unchanged. Case is preserved.
 *
 * Includes ROT13 (shift=13), brute-force decryption (all 25 shifts), and
 * frequency analysis for automatic ciphertext-only attack.
 *
 * ```kotlin
 * CaesarCipher.encrypt("Hello, World!", 3)  // → "Khoor, Zruog!"
 * CaesarCipher.decrypt("Khoor, Zruog!", 3)  // → "Hello, World!"
 * CaesarCipher.rot13("Hello")               // → "Uryyb"
 * ```
 */
object CaesarCipher {

    // =========================================================================
    // English letter frequencies (for chi-squared analysis)
    // =========================================================================
    //
    // The frequency of each letter A–Z in typical English text.
    // Key insight: 'E' (index 4) is most common at ~12.7%.

    /** English letter frequencies indexed A=0 … Z=25. */
    val ENGLISH_FREQUENCIES: DoubleArray = doubleArrayOf(
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
    )

    // =========================================================================
    // Core shift
    // =========================================================================

    /** Shift a single alphabetic character. Non-alpha returned unchanged. */
    private fun shiftChar(c: Char, shift: Int): Char {
        val normalizedShift = ((shift % 26) + 26) % 26
        return when {
            c in 'A'..'Z' -> ('A'.code + (c.code - 'A'.code + normalizedShift) % 26).toChar()
            c in 'a'..'z' -> ('a'.code + (c.code - 'a'.code + normalizedShift) % 26).toChar()
            else -> c
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt a string by shifting every letter forward by [shift] positions.
     *
     * Shift is taken modulo 26, so values outside 0–25 work correctly.
     * Negative shifts are allowed (equivalent to decrypting with that shift).
     *
     * @param text  the plaintext
     * @param shift any integer — taken mod 26 internally
     * @return the ciphertext
     */
    fun encrypt(text: String, shift: Int): String =
        text.map { shiftChar(it, shift) }.joinToString("")

    /**
     * Decrypt a string by shifting every letter backward by [shift] positions.
     *
     * Equivalent to `encrypt(text, -shift)`.
     *
     * @param text  the ciphertext
     * @param shift the shift used during encryption
     * @return the original plaintext
     */
    fun decrypt(text: String, shift: Int): String = encrypt(text, -shift)

    /**
     * ROT13 — Caesar cipher with shift = 13.
     *
     * Self-inverse: `rot13(rot13(text)) == text`. Widely used online to
     * obscure spoilers and punchlines.
     *
     * @param text any string
     * @return the ROT13 transformation
     */
    fun rot13(text: String): String = encrypt(text, 13)

    /**
     * Try all 25 non-trivial shifts and return every possible decryption.
     *
     * @param ciphertext the ciphertext to crack
     * @return list of [BruteForceResult] for shifts 1–25
     */
    fun bruteForce(ciphertext: String): List<BruteForceResult> =
        (1..25).map { shift -> BruteForceResult(shift, decrypt(ciphertext, shift)) }

    /**
     * Automatic frequency-analysis attack — find the most likely shift.
     *
     * Counts letters in the ciphertext, computes chi-squared against the
     * expected English distribution for each possible shift, and returns
     * the shift with the lowest chi-squared score.
     *
     * @param ciphertext the ciphertext (only alphabetic characters are analysed)
     * @return the best (shift, plaintext) guess; shift=0 for empty input
     */
    fun frequencyAnalysis(ciphertext: String): FrequencyResult {
        val counts = IntArray(26)
        var total = 0
        for (c in ciphertext) {
            when {
                c in 'A'..'Z' -> { counts[c - 'A']++; total++ }
                c in 'a'..'z' -> { counts[c - 'a']++; total++ }
            }
        }
        if (total == 0) return FrequencyResult(0, ciphertext)

        var bestScore = Double.MAX_VALUE
        var bestShift = 0
        for (k in 0 until 26) {
            var chiSq = 0.0
            for (i in 0 until 26) {
                val plainIdx = (i - k + 26) % 26
                val expected = ENGLISH_FREQUENCIES[plainIdx] * total
                val diff = counts[i] - expected
                chiSq += diff * diff / expected
            }
            if (chiSq < bestScore) {
                bestScore = chiSq
                bestShift = k
            }
        }
        return FrequencyResult(bestShift, decrypt(ciphertext, bestShift))
    }

    // =========================================================================
    // Result types
    // =========================================================================

    /** A (shift, plaintext) pair returned by [bruteForce]. */
    data class BruteForceResult(val shift: Int, val text: String)

    /** A (shift, plaintext) pair returned by [frequencyAnalysis]. */
    data class FrequencyResult(val shift: Int, val text: String)
}
