// ============================================================================
// ScytaleCipher.kt — The ancient Greek transposition cipher
// ============================================================================
//
// The scytale (pronounced "SKIT-uh-lee") was used by the Spartans around
// 7th century BCE. A strip of parchment was wound helically around a wooden
// staff, and the message was written lengthwise. When unwound, the strip
// appeared as a jumble of characters — unreadable without a staff of the
// same diameter to re-wind it.
//
// The key is the diameter of the staff, translated to the number of columns
// in a grid:
//
//   Plaintext: "HELLOSPARTANS"   key = 4 (columns)
//
//   Write row-by-row into 4 columns, padding with spaces:
//     Row 0:  H  E  L  L
//     Row 1:  O  S  P  A
//     Row 2:  R  T  A  N
//     Row 3:  S  _  _  _
//
//   Read column-by-column:
//     Col 0: H O R S
//     Col 1: E S T _
//     Col 2: L P A _
//     Col 3: L A N _
//
//   Ciphertext: "HORSEST LPA LAN "
//
// This is a pure transposition cipher — letters are not changed, only their
// positions are rearranged.
//

package com.codingadventures.scytalecipher

/**
 * The Scytale cipher — a columnar transposition cipher.
 *
 * Encrypts by writing plaintext row-by-row into a grid of [key] columns,
 * then reading column-by-column. Padding spaces fill the last row.
 *
 * Decrypts by writing ciphertext column-by-column, reading row-by-row,
 * then stripping trailing padding spaces.
 *
 * ```kotlin
 * ScytaleCipher.encrypt("HELLOSPARTANS", 4)  // → "HORSEST LPA LAN "
 * ScytaleCipher.decrypt("HORSEST LPA LAN ", 4)  // → "HELLOSPARTANS"
 * ```
 */
object ScytaleCipher {

    const val MAX_BRUTE_FORCE_TEXT_LENGTH: Int = 4096

    // =========================================================================
    // Validation helper
    // =========================================================================

    private fun validateKey(key: Int, textLen: Int) {
        require(key >= 2) { "key must be at least 2, got: $key" }
        require(textLen == 0 || key <= textLen) {
            "key ($key) must not exceed text length ($textLen)"
        }
    }

    // =========================================================================
    // Public API
    // =========================================================================

    /**
     * Encrypt plaintext using the Scytale cipher.
     *
     * Writes text row-by-row into a grid of [key] columns, padding the last
     * row with spaces, then reads column-by-column.
     *
     * @param text the plaintext
     * @param key  the number of columns (≥ 2, ≤ text length if non-empty)
     * @return the ciphertext
     * @throws IllegalArgumentException if key is invalid
     */
    fun encrypt(text: String, key: Int): String {
        if (text.isEmpty()) return ""
        val scalars = text.codePoints().toArray()
        validateKey(key, scalars.size)

        val rows = (scalars.size + key - 1) / key
        val paddedLen = rows * key
        val padded = scalars.copyOf(paddedLen)
        for (index in scalars.size until paddedLen) padded[index] = 0x20

        return buildString(paddedLen) {
            for (col in 0 until key) {
                for (row in 0 until rows) {
                    appendCodePoint(padded[row * key + col])
                }
            }
        }
    }

    /**
     * Decrypt ciphertext that was encrypted with the Scytale cipher.
     *
     * Writes ciphertext column-by-column into a grid, reads row-by-row,
     * then strips trailing padding spaces.
     *
     * @param text the ciphertext
     * @param key  the number of columns used during encryption (≥ 2)
     * @return the original plaintext (trailing padding spaces stripped)
     * @throws IllegalArgumentException if key is invalid
     */
    fun decrypt(text: String, key: Int): String {
        if (text.isEmpty()) return ""
        val scalars = text.codePoints().toArray()
        validateKey(key, scalars.size)

        val len = scalars.size
        val rows = (len + key - 1) / key
        val remainder = len % key
        val columnStarts = IntArray(key)
        val columnLengths = IntArray(key)
        var offset = 0

        for (column in 0 until key) {
            columnStarts[column] = offset
            columnLengths[column] = if (remainder == 0 || column < remainder) rows else rows - 1
            offset += columnLengths[column]
        }

        val plaintext = IntArray(len)
        var output = 0
        for (row in 0 until rows) {
            for (column in 0 until key) {
                if (row < columnLengths[column]) {
                    plaintext[output++] = scalars[columnStarts[column] + row]
                }
            }
        }
        while (output > 0 && plaintext[output - 1] == 0x20) output--
        return String(plaintext, 0, output)
    }

    /**
     * Brute-force attack: try all valid key values from 2 to text.length / 2.
     *
     * Returns an empty list if the text is too short (fewer than 4 characters).
     *
     * @param text the ciphertext
     * @return list of [BruteForceResult] for all valid keys
     */
    fun bruteForce(text: String): List<BruteForceResult> {
        val scalarLength = text.codePointCount(0, text.length)
        require(scalarLength <= MAX_BRUTE_FORCE_TEXT_LENGTH) { "scytale-brute-force-limit" }
        if (scalarLength < 4) return emptyList()
        return (2..scalarLength / 2).map { key -> BruteForceResult(key, decrypt(text, key)) }
    }

    // =========================================================================
    // Result type
    // =========================================================================

    /** A (key, plaintext) pair returned by [bruteForce]. */
    data class BruteForceResult(val key: Int, val text: String)
}
