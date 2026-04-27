// ============================================================================
// VigenereCipher.kt — The polyalphabetic substitution cipher
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
//   key position counter.  This matches the Java/Python/Rust/TypeScript
//   implementations.
// - The key is case-insensitive (normalised to uppercase internally).
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
// L groups (position 0, L, 2L, ...; position 1, L+1, 2L+1, ...; etc.).  Each
// group was encrypted with the SAME shift, so it behaves like a Caesar cipher.
// The group IC will be close to English IC (0.065) when the key length is
// correct and close to random IC (0.038) when it's wrong.
//
// Once we have the key length, each group is broken with chi-squared against
// English letter frequencies — the same technique used in CaesarCipher.
//

package com.codingadventures.vigenerecipher

/**
 * The Vigenère cipher — a polyalphabetic substitution cipher.
 *
 * Uses a keyword to apply different Caesar shifts at each position.
 * Non-alphabetic characters pass through unchanged without advancing the key.
 *
 * Includes automatic ciphertext-only cryptanalysis via IC and chi-squared.
 *
 * ```kotlin
 * VigenereCipher.encrypt("ATTACKATDAWN", "LEMON")  // → "LXFOPVEFRNHR"
 * VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON")  // → "ATTACKATDAWN"
 *
 * // Automatic attack (needs ≥ 200 chars for reliability)
 * val r = VigenereCipher.breakCipher(ciphertext)
 * println("Key: ${r.key}, Plaintext: ${r.plaintext}")
 * ```
 */
object VigenereCipher {

    // =========================================================================
    // English letter frequencies
    // =========================================================================
    //
    // Indexed 0=A … 25=Z.  Same table as CaesarCipher.

    /** English letter frequencies indexed A=0 … Z=25. */
    val ENGLISH_FREQUENCIES: DoubleArray = doubleArrayOf(
        0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015,
        0.06094, 0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749,
        0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056, 0.02758,
        0.00978, 0.02360, 0.00150, 0.01974, 0.00074,
    )

    // =========================================================================
    // Key validation
    // =========================================================================

    /**
     * Validate and normalise the keyword.
     *
     * @throws IllegalArgumentException if the key is empty or contains non-alpha
     */
    private fun validateKey(key: String): String {
        require(key.isNotEmpty()) { "Key must not be empty" }
        val upper = key.uppercase()
        require(upper.all { it in 'A'..'Z' }) {
            "Key must contain only letters, got: '$key'"
        }
        return upper
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt [plaintext] using the Vigenère cipher.
     *
     * Advances the key position only on alphabetic characters.  Non-alpha
     * characters are copied unchanged.
     *
     * Known test vector: `encrypt("ATTACKATDAWN", "LEMON")` → `"LXFOPVEFRNHR"`.
     *
     * @param plaintext the text to encrypt
     * @param key       the keyword (letters only, case-insensitive)
     * @return the ciphertext
     * @throws IllegalArgumentException if key is invalid
     */
    fun encrypt(plaintext: String, key: String): String {
        val k = validateKey(key)
        val kLen = k.length
        var kPos = 0
        return buildString(plaintext.length) {
            for (c in plaintext) {
                when {
                    c in 'A'..'Z' -> {
                        val shift = k[kPos % kLen].code - 'A'.code
                        append(('A'.code + (c.code - 'A'.code + shift) % 26).toChar())
                        kPos++
                    }
                    c in 'a'..'z' -> {
                        val shift = k[kPos % kLen].code - 'A'.code
                        append(('a'.code + (c.code - 'a'.code + shift) % 26).toChar())
                        kPos++
                    }
                    else -> append(c)
                }
            }
        }
    }

    /**
     * Decrypt [ciphertext] encrypted with the Vigenère cipher.
     *
     * @param ciphertext the text to decrypt
     * @param key        the keyword used for encryption (letters only, case-insensitive)
     * @return the original plaintext
     * @throws IllegalArgumentException if key is invalid
     */
    fun decrypt(ciphertext: String, key: String): String {
        val k = validateKey(key)
        val kLen = k.length
        var kPos = 0
        return buildString(ciphertext.length) {
            for (c in ciphertext) {
                when {
                    c in 'A'..'Z' -> {
                        val shift = k[kPos % kLen].code - 'A'.code
                        append(('A'.code + (c.code - 'A'.code - shift + 26) % 26).toChar())
                        kPos++
                    }
                    c in 'a'..'z' -> {
                        val shift = k[kPos % kLen].code - 'A'.code
                        append(('a'.code + (c.code - 'a'.code - shift + 26) % 26).toChar())
                        kPos++
                    }
                    else -> append(c)
                }
            }
        }
    }

    // =========================================================================
    // Cryptanalysis — Index of Coincidence key-length finder
    // =========================================================================

    /**
     * Estimate the key length using the Index of Coincidence.
     *
     * Tries candidate lengths from 2 to [maxLength] (default 20).  For each
     * candidate L, splits the ciphertext into L groups and computes the average
     * IC across groups.  The correct key length produces groups that look like
     * Caesar-cipher ciphertext (IC ≈ 0.065) rather than random text (IC ≈ 0.038).
     *
     * Returns the smallest candidate length within 5% of the best average IC,
     * after filtering out any candidates that are multiples of smaller
     * qualifying candidates (since k, 2k, 3k … all produce high IC).
     *
     * @param ciphertext the ciphertext (only alphabetic characters are analysed)
     * @param maxLength  maximum key length to try (must be ≥ 2)
     * @return the estimated key length
     */
    fun findKeyLength(ciphertext: String, maxLength: Int = 20): Int {
        // Extract only alphabetic characters, uppercase.
        val s = ciphertext.filter { it.isLetter() }.uppercase()
        val n = s.length

        val limit = minOf(maxLength, n / 2)
        val avgIcs = DoubleArray(limit + 1)
        var bestAvgIc = -1.0

        for (l in 2..limit) {
            var totalIc = 0.0
            var validGroups = 0
            for (g in 0 until l) {
                // Extract every l-th character starting at position g.
                val counts = IntArray(26)
                var groupLen = 0
                var pos = g
                while (pos < n) {
                    counts[s[pos].code - 'A'.code]++
                    groupLen++
                    pos += l
                }
                if (groupLen < 2) continue
                // IC = Σ n_i(n_i - 1) / (N(N - 1))
                val numerator = counts.sumOf { c -> c.toLong() * (c - 1) }
                totalIc += numerator.toDouble() / (groupLen.toLong() * (groupLen - 1))
                validGroups++
            }
            if (validGroups > 0) {
                avgIcs[l] = totalIc / validGroups
                if (avgIcs[l] > bestAvgIc) bestAvgIc = avgIcs[l]
            }
        }

        // Collect all candidates within 5% of the best average IC.
        val threshold = bestAvgIc * 0.95
        val candidates = (2..limit).filter { avgIcs[it] >= threshold }.toMutableList()
        if (candidates.isEmpty()) return 2

        // Remove any candidate that is a multiple of a smaller candidate in the
        // list.  Multiples of the true key length also score well; the smallest
        // residual is the true key length.
        for (smaller in candidates.toList()) {
            candidates.removeAll { bigger -> bigger != smaller && bigger % smaller == 0 }
        }
        return candidates.first()
    }

    // =========================================================================
    // Cryptanalysis — chi-squared key recovery
    // =========================================================================

    /**
     * Recover the keyword given the ciphertext and the correct key length.
     *
     * Splits the ciphertext into [keyLength] groups (one per key position),
     * then runs chi-squared analysis on each group to find the Caesar shift —
     * which equals the corresponding keyword letter.
     *
     * After recovering all characters, checks whether the key has a repeating
     * sub-period and returns the minimal period.  This makes [findKey] robust
     * even when [findKeyLength] returns a multiple of the true key length.
     *
     * @param ciphertext the ciphertext
     * @param keyLength  the key length (from [findKeyLength])
     * @return the recovered keyword (uppercase)
     */
    fun findKey(ciphertext: String, keyLength: Int): String {
        val s = ciphertext.filter { it.isLetter() }.uppercase()
        val keyChars = CharArray(keyLength)

        for (g in 0 until keyLength) {
            // Build letter-frequency counts for group g.
            val counts = IntArray(26)
            var groupLen = 0
            var pos = g
            while (pos < s.length) {
                counts[s[pos].code - 'A'.code]++
                groupLen++
                pos += keyLength
            }
            if (groupLen == 0) { keyChars[g] = 'A'; continue }

            // Chi-squared against English frequencies for each shift.
            var bestScore = Double.MAX_VALUE
            var bestShift = 0
            for (k in 0 until 26) {
                var chiSq = 0.0
                for (i in 0 until 26) {
                    val plainIdx = (i - k + 26) % 26
                    val expected = ENGLISH_FREQUENCIES[plainIdx] * groupLen
                    val diff = counts[i] - expected
                    chiSq += (diff * diff) / expected
                }
                if (chiSq < bestScore) {
                    bestScore = chiSq
                    bestShift = k
                }
            }
            keyChars[g] = ('A'.code + bestShift).toChar()
        }

        // Return the minimal period of the key.  If the IC estimator returned
        // a multiple of the true key (e.g. 10 for LEMON), the full key is
        // LEMONLEMON — which has minimal period 5, giving back "LEMON".
        val fullKey = String(keyChars)
        for (p in 1..keyLength / 2) {
            if (keyLength % p != 0) continue
            val base = fullKey.substring(0, p)
            if ((p until keyLength).all { i -> fullKey[i] == fullKey[i % p] }) return base
        }
        return fullKey
    }

    // =========================================================================
    // Full automatic attack
    // =========================================================================

    /**
     * Fully automatic ciphertext-only attack.
     *
     * Chains [findKeyLength] → [findKey] → [decrypt].
     *
     * Works best on ciphertexts of 200+ alphabetic characters.
     *
     * @param ciphertext the ciphertext to break
     * @return a [BreakResult] with the recovered key and plaintext
     */
    fun breakCipher(ciphertext: String): BreakResult {
        val keyLen = findKeyLength(ciphertext)
        val key = findKey(ciphertext, keyLen)
        val plaintext = decrypt(ciphertext, key)
        return BreakResult(key, plaintext)
    }

    // =========================================================================
    // Result type
    // =========================================================================

    /** The result of [breakCipher]: recovered key and decrypted plaintext. */
    data class BreakResult(
        /** The recovered keyword (uppercase). */
        val key: String,
        /** The decrypted plaintext. */
        val plaintext: String,
    )
}
