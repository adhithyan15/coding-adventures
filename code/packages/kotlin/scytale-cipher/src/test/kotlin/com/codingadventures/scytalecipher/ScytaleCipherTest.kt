// ============================================================================
// ScytaleCipherTest.kt — Unit Tests for ScytaleCipher
// ============================================================================

package com.codingadventures.scytalecipher

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ScytaleCipherTest {

    // =========================================================================
    // 1. encrypt
    // =========================================================================

    @Test
    fun encryptSimple() =
        // "HELLOSPARTANS" key=4 → cols HORS|EST_|LPA_|LAN_ → "HORSEST LPA LAN "
        assertEquals("HORSEST LPA LAN ", ScytaleCipher.encrypt("HELLOSPARTANS", 4))

    @Test fun encryptKey2() = assertEquals("ACEBDF", ScytaleCipher.encrypt("ABCDEF", 2))

    @Test fun encryptKey3Even() = assertEquals("ADBECF", ScytaleCipher.encrypt("ABCDEF", 3))

    @Test
    fun encryptPadsLastRow() =
        // "HELLO" key=3: [H E L][L O _] → cols HL|EO|L_ → "HLEOL "
        assertEquals("HLEOL ", ScytaleCipher.encrypt("HELLO", 3))

    @Test fun encryptEmptyString() = assertEquals("", ScytaleCipher.encrypt("", 3))

    @Test fun encryptSingleRow() = assertEquals("ABCD", ScytaleCipher.encrypt("ABCD", 4))

    @Test
    fun encryptPreservesNonAlpha() =
        // "ABCCDD" key=3: [A B C][C D D] → cols AC|BD|CD → "ACBDCD"
        assertEquals("ACBDCD", ScytaleCipher.encrypt("ABCCDD", 3))

    // =========================================================================
    // 2. decrypt
    // =========================================================================

    @Test fun decryptSimple() = assertEquals("HELLOSPARTANS", ScytaleCipher.decrypt("HORSEST LPA LAN ", 4))
    @Test fun decryptKey2()  = assertEquals("ABCDEF", ScytaleCipher.decrypt("ACEBDF", 2))
    @Test fun decryptKey3()  = assertEquals("ABCDEF", ScytaleCipher.decrypt("ADBECF", 3))

    @Test
    fun decryptStripsPadding() = assertEquals("HELLO", ScytaleCipher.decrypt("HLEOL ", 3))

    @Test fun decryptEmptyString() = assertEquals("", ScytaleCipher.decrypt("", 4))

    // =========================================================================
    // 3. roundtrip
    // =========================================================================

    @Test
    fun roundtripNoSpecialChars() {
        val texts = listOf("HELLOSPARTANS", "ABCDEFGHIJKLMNOP", "ATTACKATDAWN")
        val keys  = listOf(2, 3, 4, 5)
        for (text in texts) for (key in keys) {
            if (key <= text.length) {
                assertEquals(
                    text,
                    ScytaleCipher.decrypt(ScytaleCipher.encrypt(text, key), key),
                    "Roundtrip failed: text='$text', key=$key"
                )
            }
        }
    }

    @Test
    fun roundtripWithSpacesInMiddle() {
        val text = "HELLO WORLD"
        assertEquals(text, ScytaleCipher.decrypt(ScytaleCipher.encrypt(text, 4), 4))
    }

    // =========================================================================
    // 4. Input validation
    // =========================================================================

    @Test fun encryptRejectsKey1()       = assertThrows<IllegalArgumentException> { ScytaleCipher.encrypt("HELLO", 1) }
    @Test fun encryptRejectsKey0()       = assertThrows<IllegalArgumentException> { ScytaleCipher.encrypt("HELLO", 0) }
    @Test fun encryptRejectsNegativeKey()= assertThrows<IllegalArgumentException> { ScytaleCipher.encrypt("HELLO", -1) }
    @Test fun encryptRejectsKeyTooLarge()= assertThrows<IllegalArgumentException> { ScytaleCipher.encrypt("HI", 5) }
    @Test fun decryptRejectsKey1()       = assertThrows<IllegalArgumentException> { ScytaleCipher.decrypt("KHOOR", 1) }

    // =========================================================================
    // 5. bruteForce
    // =========================================================================

    @Test fun bruteForceTooShortReturnsEmpty() {
        assertEquals(0, ScytaleCipher.bruteForce("HI").size)
        assertEquals(0, ScytaleCipher.bruteForce("HEL").size)
    }

    @Test
    fun bruteForceReturnsCorrectKeys() {
        val results = ScytaleCipher.bruteForce("ACEBDF")  // length 6 → keys 2, 3
        assertEquals(2, results[0].key)
        assertEquals(3, results[1].key)
    }

    @Test
    fun bruteForceContainsCorrectDecryption() {
        val ciphertext = ScytaleCipher.encrypt("HELLOSPARTANS", 4)
        val results = ScytaleCipher.bruteForce(ciphertext)
        assertTrue(results.any { it.key == 4 && it.text == "HELLOSPARTANS" },
            "brute force should find key=4 → HELLOSPARTANS")
    }
}
