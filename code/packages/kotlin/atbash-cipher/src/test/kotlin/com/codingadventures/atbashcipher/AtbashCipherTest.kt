// ============================================================================
// AtbashCipherTest.kt — Unit Tests for AtbashCipher
// ============================================================================

package com.codingadventures.atbashcipher

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals

class AtbashCipherTest {

    // =========================================================================
    // 1. encrypt — basic correctness
    // =========================================================================

    @Test fun encryptSingleUpperA() = assertEquals("Z", AtbashCipher.encrypt("A"))
    @Test fun encryptSingleUpperZ() = assertEquals("A", AtbashCipher.encrypt("Z"))
    @Test fun encryptSingleUpperM() = assertEquals("N", AtbashCipher.encrypt("M"))
    @Test fun encryptSingleUpperN() = assertEquals("M", AtbashCipher.encrypt("N"))

    @Test fun encryptSingleLowerA() = assertEquals("z", AtbashCipher.encrypt("a"))
    @Test fun encryptSingleLowerZ() = assertEquals("a", AtbashCipher.encrypt("z"))
    @Test fun encryptSingleLowerM() = assertEquals("n", AtbashCipher.encrypt("m"))
    @Test fun encryptSingleLowerN() = assertEquals("m", AtbashCipher.encrypt("n"))

    @Test
    fun encryptFullUppercaseAlphabet() =
        assertEquals("ZYXWVUTSRQPONMLKJIHGFEDCBA", AtbashCipher.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"))

    @Test
    fun encryptFullLowercaseAlphabet() =
        assertEquals("zyxwvutsrqponmlkjihgfedcba", AtbashCipher.encrypt("abcdefghijklmnopqrstuvwxyz"))

    @Test
    fun encryptPreservesCase() {
        assertEquals("Svool", AtbashCipher.encrypt("Hello"))
        assertEquals("SVOOL", AtbashCipher.encrypt("HELLO"))
        assertEquals("svool", AtbashCipher.encrypt("hello"))
    }

    @Test
    fun encryptHelloWorld() = assertEquals("Svool, Dliow!", AtbashCipher.encrypt("Hello, World!"))

    @Test
    fun encryptPassesThroughNonAlpha() {
        assertEquals("123", AtbashCipher.encrypt("123"))
        assertEquals("!@#", AtbashCipher.encrypt("!@#"))
        assertEquals(" ", AtbashCipher.encrypt(" "))
        assertEquals("A1B2", AtbashCipher.encrypt("Z1Y2"))
    }

    @Test fun encryptEmptyString() = assertEquals("", AtbashCipher.encrypt(""))

    @Test
    fun encryptMixedContent() =
        // A→Z t→g t→g a→z c→x k→p ' ' a→z t→g ' ' D→W a→z w→d n→m
        assertEquals("Zggzxp zg Wzdm", AtbashCipher.encrypt("Attack at Dawn"))

    // =========================================================================
    // 2. decrypt — same as encrypt (self-inverse)
    // =========================================================================

    @Test
    fun decryptIsEncrypt() {
        listOf("Hello, World!", "ABCXYZ", "The quick brown fox", "", "123!@#", "Svool")
            .forEach { text ->
                assertEquals(
                    AtbashCipher.encrypt(text),
                    AtbashCipher.decrypt(text),
                    "decrypt should equal encrypt for: $text"
                )
            }
    }

    @Test
    fun decryptInvertsEncrypt() {
        listOf(
            "Hello, World!", "ATTACK AT DAWN",
            "The quick brown fox jumps over the lazy dog",
            "abcdefghijklmnopqrstuvwxyz", "ABCDEFGHIJKLMNOPQRSTUVWXYZ",
            "Mixed CASE text!"
        ).forEach { plaintext ->
            assertEquals(
                plaintext,
                AtbashCipher.decrypt(AtbashCipher.encrypt(plaintext)),
                "Roundtrip failed for: $plaintext"
            )
        }
    }

    @Test fun decryptEmptyString() = assertEquals("", AtbashCipher.decrypt(""))

    // =========================================================================
    // 3. Self-inverse property
    // =========================================================================

    @Test
    fun selfInverseForEveryLetter() {
        for (c in 'A'..'Z') {
            val s = c.toString()
            assertEquals(s, AtbashCipher.encrypt(AtbashCipher.encrypt(s)), "Failed for: $c")
        }
        for (c in 'a'..'z') {
            val s = c.toString()
            assertEquals(s, AtbashCipher.encrypt(AtbashCipher.encrypt(s)), "Failed for: $c")
        }
    }

    @Test
    fun selfInverseForSentence() {
        val original = "The quick brown fox jumps over the lazy dog."
        assertEquals(original, AtbashCipher.encrypt(AtbashCipher.encrypt(original)))
    }

    // =========================================================================
    // 4. Known test vectors (cross-language parity)
    // =========================================================================

    @Test fun knownVectorAttack() = assertEquals("ZGGZXP", AtbashCipher.encrypt("ATTACK"))
    @Test fun knownVectorHello()  = assertEquals("SVOOL",  AtbashCipher.encrypt("HELLO"))

    @Test
    fun knownVectorMirrorPairs() {
        assertEquals("ABCXYZ", AtbashCipher.encrypt("ZYXCBA"))
        assertEquals("ZYXCBA", AtbashCipher.encrypt("ABCXYZ"))
    }

    // =========================================================================
    // 5. Non-ASCII pass-through
    // =========================================================================

    @Test
    fun nonAsciiPassesThrough() {
        // "Xzuv" → X→C, z→a, u→f, v→e → "Cafe"
        assertEquals("Cafe", AtbashCipher.encrypt("Xzuv"))
        assertEquals("Svool Dliow", AtbashCipher.encrypt("Hello World"))
        assertEquals("C1f2", AtbashCipher.encrypt("X1u2"))
    }
}
