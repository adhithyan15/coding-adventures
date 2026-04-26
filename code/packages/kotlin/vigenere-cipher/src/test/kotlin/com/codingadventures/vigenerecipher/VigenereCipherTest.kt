// ============================================================================
// VigenereCipherTest.kt — Unit Tests for VigenereCipher
// ============================================================================

package com.codingadventures.vigenerecipher

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class VigenereCipherTest {

    // =========================================================================
    // 1. encrypt — known test vectors
    // =========================================================================

    @Test
    fun encryptClassicLemon() {
        // Cross-language test vector from the Python/Java implementations
        assertEquals("LXFOPVEFRNHR", VigenereCipher.encrypt("ATTACKATDAWN", "LEMON"))
    }

    @Test
    fun encryptMixedCase() {
        // Key is case-insensitive; output preserves input case
        assertEquals("Rijvs, Uyvjn!", VigenereCipher.encrypt("Hello, World!", "key"))
    }

    @Test
    fun encryptKeyRepeats() {
        // Key "AB" means alternating shifts 0 and 1
        // A+0=A, B+1=C, C+0=C, D+1=E, F+0=F
        assertEquals("ACCEF", VigenereCipher.encrypt("ABCDF", "AB"))
        // A+0=A, B+1=C, C+0=C, D+1=E, E+0=E
        assertEquals("ACCEE", VigenereCipher.encrypt("ABCDE", "AB"))
    }

    @Test
    fun encryptNonAlphaPassesThrough() {
        assertEquals("Rijvs, Uyvjn!", VigenereCipher.encrypt("Hello, World!", "key"))
    }

    @Test
    fun encryptNonAlphaDoesNotAdvanceKey() {
        // "A B" with key "B": 'A' shifted by B(1) → B, ' ' passes through,
        // 'B' also shifted by B(1) → C (key length=1, every letter shifts by 1)
        assertEquals("B C", VigenereCipher.encrypt("A B", "B"))
    }

    @Test
    fun encryptEmptyString() {
        assertEquals("", VigenereCipher.encrypt("", "KEY"))
    }

    @Test
    fun encryptKeyLongerThanText() {
        // Only the first key letters are used
        assertEquals("L", VigenereCipher.encrypt("A", "LEMON"))  // A+L=L
    }

    // =========================================================================
    // 2. decrypt — known test vectors
    // =========================================================================

    @Test
    fun decryptClassicLemon() {
        assertEquals("ATTACKATDAWN", VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON"))
    }

    @Test
    fun decryptMixedCase() {
        assertEquals("Hello, World!", VigenereCipher.decrypt("Rijvs, Uyvjn!", "key"))
    }

    @Test
    fun decryptEmptyString() {
        assertEquals("", VigenereCipher.decrypt("", "KEY"))
    }

    // =========================================================================
    // 3. roundtrip
    // =========================================================================

    @Test
    fun roundtripVariousKeys() {
        val texts = listOf(
            "ATTACKATDAWN", "Hello, World!", "The quick brown fox jumps over the lazy dog.",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
        )
        val keys = listOf("KEY", "LEMON", "A", "SECRETKEY")
        for (text in texts) {
            for (key in keys) {
                assertEquals(
                    text,
                    VigenereCipher.decrypt(VigenereCipher.encrypt(text, key), key),
                    "Roundtrip failed for text='$text', key=$key",
                )
            }
        }
    }

    @Test
    fun roundtripSingleCharKey() {
        // Single-char key is equivalent to a Caesar cipher
        val text = "Hello World"
        assertEquals(text, VigenereCipher.decrypt(VigenereCipher.encrypt(text, "C"), "C"))
    }

    // =========================================================================
    // 4. Input validation
    // =========================================================================

    @Test
    fun encryptRejectsEmptyKey() {
        assertFailsWith<IllegalArgumentException> { VigenereCipher.encrypt("Hello", "") }
    }

    @Test
    fun encryptRejectsNonAlphaKey() {
        assertFailsWith<IllegalArgumentException> { VigenereCipher.encrypt("Hello", "KEY1") }
        assertFailsWith<IllegalArgumentException> { VigenereCipher.encrypt("Hello", "KE Y") }
    }

    @Test
    fun decryptRejectsEmptyKey() {
        assertFailsWith<IllegalArgumentException> { VigenereCipher.decrypt("HELLO", "") }
    }

    // =========================================================================
    // 5. findKeyLength
    // =========================================================================

    @Test
    fun findKeyLengthForLongText() {
        // Use 300+ chars of natural English prose for reliable IC analysis
        val plaintext =
            "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
            "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
            "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
            "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS."
        val ciphertext = VigenereCipher.encrypt(plaintext, "LEMON")
        val keyLen = VigenereCipher.findKeyLength(ciphertext)
        assertEquals(5, keyLen)
    }

    @Test
    fun findKeyLengthKey3() {
        val plaintext =
            "THE HISTORY OF CRYPTOGRAPHY IS THE HISTORY OF ATTEMPTS TO COMMUNICATE " +
            "PRIVATELY IN THE PRESENCE OF ADVERSARIES. SINCE THE EARLIEST RECORDED " +
            "HISTORY MILITARY COMMANDERS AND DIPLOMATS HAVE USED SECRET CODES TO " +
            "PROTECT SENSITIVE INFORMATION FROM ENEMIES AND RIVALS WHO MIGHT INTERCEPT."
        val ciphertext = VigenereCipher.encrypt(plaintext, "KEY")
        val keyLen = VigenereCipher.findKeyLength(ciphertext)
        assertEquals(3, keyLen)
    }

    // =========================================================================
    // 6. findKey
    // =========================================================================

    @Test
    fun findKeyForLongTextLemon() {
        val plaintext =
            "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
            "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
            "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
            "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS."
        val ciphertext = VigenereCipher.encrypt(plaintext, "LEMON")
        val recoveredKey = VigenereCipher.findKey(ciphertext, 5)
        assertEquals("LEMON", recoveredKey)
    }

    @Test
    fun findKeyForKey3() {
        val plaintext =
            "THE HISTORY OF CRYPTOGRAPHY IS THE HISTORY OF ATTEMPTS TO COMMUNICATE " +
            "PRIVATELY IN THE PRESENCE OF ADVERSARIES. SINCE THE EARLIEST RECORDED " +
            "HISTORY MILITARY COMMANDERS AND DIPLOMATS HAVE USED SECRET CODES TO " +
            "PROTECT SENSITIVE INFORMATION FROM ENEMIES AND RIVALS WHO MIGHT INTERCEPT."
        val ciphertext = VigenereCipher.encrypt(plaintext, "KEY")
        val recoveredKey = VigenereCipher.findKey(ciphertext, 3)
        assertEquals("KEY", recoveredKey)
    }

    // =========================================================================
    // 7. breakCipher — full automatic attack
    // =========================================================================

    @Test
    fun breakCipherEndToEnd() {
        val plaintext =
            "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
            "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
            "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
            "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS. " +
            "CHARLES BABBAGE BROKE THE CIPHER IN THE EIGHTEEN FIFTIES USING THE INDEX."
        val ciphertext = VigenereCipher.encrypt(plaintext, "LEMON")
        val result = VigenereCipher.breakCipher(ciphertext)
        assertEquals("LEMON", result.key)
        assertEquals(plaintext, result.plaintext)
    }

    // =========================================================================
    // 8. ENGLISH_FREQUENCIES
    // =========================================================================

    @Test
    fun englishFrequenciesLength() {
        assertEquals(26, VigenereCipher.ENGLISH_FREQUENCIES.size)
    }

    @Test
    fun englishFrequenciesSumToOne() {
        val sum = VigenereCipher.ENGLISH_FREQUENCIES.sum()
        assertEquals(1.0, sum, 0.001)
    }

    // =========================================================================
    // 9. BreakResult data class
    // =========================================================================

    @Test
    fun breakResultDataClass() {
        val r = VigenereCipher.BreakResult("LEMON", "ATTACKATDAWN")
        assertEquals("LEMON", r.key)
        assertEquals("ATTACKATDAWN", r.plaintext)
        // Data class equality
        val r2 = VigenereCipher.BreakResult("LEMON", "ATTACKATDAWN")
        assertEquals(r, r2)
    }

    @Test
    fun breakResultToString() {
        val r = VigenereCipher.BreakResult("KEY", "HELLO")
        val s = r.toString()
        assert(s.contains("KEY")) { "toString should include key: $s" }
        assert(s.contains("HELLO")) { "toString should include plaintext: $s" }
    }
}
