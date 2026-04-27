// ============================================================================
// CaesarCipherTest.kt — Unit Tests for CaesarCipher
// ============================================================================

package com.codingadventures.caesarcipher

import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class CaesarCipherTest {

    // =========================================================================
    // 1. encrypt — basic correctness
    // =========================================================================

    @Test fun encryptShift0IsIdentity() {
        assertEquals("Hello", CaesarCipher.encrypt("Hello", 0))
        assertEquals("ABCXYZ", CaesarCipher.encrypt("ABCXYZ", 0))
    }

    @Test fun encryptShift1Wrap() {
        assertEquals("B", CaesarCipher.encrypt("A", 1))
        assertEquals("A", CaesarCipher.encrypt("Z", 1))  // wraps
    }

    @Test fun encryptShift3Classic() {
        assertEquals("KHOOR", CaesarCipher.encrypt("HELLO", 3))
        assertEquals("Khoor, Zruog!", CaesarCipher.encrypt("Hello, World!", 3))
    }

    @Test fun encryptShift13Rot13() =
        assertEquals("URYYB", CaesarCipher.encrypt("HELLO", 13))

    @Test fun encryptShift25() {
        assertEquals("Z", CaesarCipher.encrypt("A", 25))
        assertEquals("ABCDE", CaesarCipher.encrypt("BCDEF", 25))
    }

    @Test fun encryptPreservesCase() {
        assertEquals("Khoor", CaesarCipher.encrypt("Hello", 3))
        assertEquals("KHOOR", CaesarCipher.encrypt("HELLO", 3))
        assertEquals("khoor", CaesarCipher.encrypt("hello", 3))
    }

    @Test fun encryptNonAlphaPassesThrough() {
        assertEquals("Khoor, Zruog!", CaesarCipher.encrypt("Hello, World!", 3))
        assertEquals("123 !@#", CaesarCipher.encrypt("123 !@#", 7))
    }

    @Test fun encryptEmptyString() = assertEquals("", CaesarCipher.encrypt("", 5))

    // =========================================================================
    // 2. shift arithmetic — mod 26 and negatives
    // =========================================================================

    @Test fun encryptShift26IsIdentity() {
        assertEquals("HELLO", CaesarCipher.encrypt("HELLO", 26))
        assertEquals("HELLO", CaesarCipher.encrypt("HELLO", 52))
    }

    @Test fun encryptNegativeShift() =
        assertEquals("HELLO", CaesarCipher.encrypt("KHOOR", -3))

    @Test fun encryptLargePositiveShift() =
        assertEquals(CaesarCipher.encrypt("HELLO", 3), CaesarCipher.encrypt("HELLO", 29))

    // =========================================================================
    // 3. decrypt
    // =========================================================================

    @Test fun decryptShift3() {
        assertEquals("HELLO", CaesarCipher.decrypt("KHOOR", 3))
        assertEquals("Hello, World!", CaesarCipher.decrypt("Khoor, Zruog!", 3))
    }

    @Test fun decryptRoundtrip() {
        val texts = listOf("Hello, World!", "ATTACK AT DAWN", "The quick brown fox.", "")
        val shifts = listOf(1, 3, 13, 25)
        for (text in texts) {
            for (shift in shifts) {
                assertEquals(
                    text,
                    CaesarCipher.decrypt(CaesarCipher.encrypt(text, shift), shift),
                    "Roundtrip failed for text='$text', shift=$shift"
                )
            }
        }
    }

    // =========================================================================
    // 4. ROT13
    // =========================================================================

    @Test fun rot13Basic() {
        assertEquals("URYYB", CaesarCipher.rot13("HELLO"))
        assertEquals("HELLO", CaesarCipher.rot13("URYYB"))
    }

    @Test fun rot13SelfInverse() {
        listOf("Hello, World!", "Why did the chicken cross the road?", "ABCxyz")
            .forEach { text ->
                assertEquals(
                    text,
                    CaesarCipher.rot13(CaesarCipher.rot13(text)),
                    "ROT13 self-inverse failed for: $text"
                )
            }
    }

    @Test fun rot13NonAlphaUnchanged() =
        assertEquals("Uryyb, Jbeyq!", CaesarCipher.rot13("Hello, World!"))

    // =========================================================================
    // 5. bruteForce
    // =========================================================================

    @Test fun bruteForceReturns25Results() =
        assertEquals(25, CaesarCipher.bruteForce("KHOOR").size)

    @Test fun bruteForceShiftsAre1Through25() {
        CaesarCipher.bruteForce("KHOOR").forEachIndexed { i, r ->
            assertEquals(i + 1, r.shift)
        }
    }

    @Test fun bruteForceContainsCorrectDecryption() {
        val results = CaesarCipher.bruteForce("KHOOR")
        assertTrue(results.any { it.shift == 3 && it.text == "HELLO" },
            "brute force should include shift=3, text=HELLO")
    }

    @Test fun bruteForceEmptyString() {
        val results = CaesarCipher.bruteForce("")
        assertEquals(25, results.size)
        assertTrue(results.all { it.text == "" })
    }

    // =========================================================================
    // 6. frequencyAnalysis
    // =========================================================================

    @Test fun frequencyAnalysisEmptyString() {
        val result = CaesarCipher.frequencyAnalysis("")
        assertEquals(0, result.shift)
        assertEquals("", result.text)
    }

    @Test fun frequencyAnalysisLongEnglishText() {
        val plaintext = "The quick brown fox jumps over the lazy dog. " +
            "Pack my box with five dozen liquor jugs. " +
            "How vexingly quick daft zebras jump."
        val ciphertext = CaesarCipher.encrypt(plaintext, 13)
        val result = CaesarCipher.frequencyAnalysis(ciphertext)
        assertEquals(13, result.shift, "Frequency analysis should recover shift=13")
        assertEquals(plaintext, result.text)
    }

    @Test fun frequencyAnalysisShift3() {
        val plaintext = "In the beginning God created the heavens and the earth. " +
            "The earth was without form and void and darkness was over the face of the deep."
        val ciphertext = CaesarCipher.encrypt(plaintext, 3)
        val result = CaesarCipher.frequencyAnalysis(ciphertext)
        assertEquals(3, result.shift, "Frequency analysis should recover shift=3")
    }

    // =========================================================================
    // 7. ENGLISH_FREQUENCIES constant
    // =========================================================================

    @Test fun englishFrequenciesLength() =
        assertEquals(26, CaesarCipher.ENGLISH_FREQUENCIES.size)

    @Test fun englishFrequenciesSumToOne() {
        val sum = CaesarCipher.ENGLISH_FREQUENCIES.sum()
        assertEquals(1.0, sum, 0.001, "Frequencies should sum to ~1.0")
    }

    @Test fun eIsTheMostCommonLetter() {
        val eFreq = CaesarCipher.ENGLISH_FREQUENCIES[4]  // 'E'
        for (i in 0 until 26) {
            if (i != 4) assertTrue(eFreq >= CaesarCipher.ENGLISH_FREQUENCIES[i],
                "E should be most frequent; index $i had higher freq")
        }
    }
}
