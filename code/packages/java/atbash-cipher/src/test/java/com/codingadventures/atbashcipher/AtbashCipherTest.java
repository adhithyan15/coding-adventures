// ============================================================================
// AtbashCipherTest.java — Unit Tests for AtbashCipher
// ============================================================================

package com.codingadventures.atbashcipher;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class AtbashCipherTest {

    // =========================================================================
    // 1. encrypt — basic correctness
    // =========================================================================

    @Test
    void encryptSingleUppercaseLetter() {
        assertEquals("Z", AtbashCipher.encrypt("A"));
        assertEquals("A", AtbashCipher.encrypt("Z"));
        assertEquals("N", AtbashCipher.encrypt("M"));
        assertEquals("M", AtbashCipher.encrypt("N"));
    }

    @Test
    void encryptSingleLowercaseLetter() {
        assertEquals("z", AtbashCipher.encrypt("a"));
        assertEquals("a", AtbashCipher.encrypt("z"));
        assertEquals("n", AtbashCipher.encrypt("m"));
        assertEquals("m", AtbashCipher.encrypt("n"));
    }

    @Test
    void encryptFullUppercaseAlphabet() {
        assertEquals("ZYXWVUTSRQPONMLKJIHGFEDCBA", AtbashCipher.encrypt("ABCDEFGHIJKLMNOPQRSTUVWXYZ"));
    }

    @Test
    void encryptFullLowercaseAlphabet() {
        assertEquals("zyxwvutsrqponmlkjihgfedcba", AtbashCipher.encrypt("abcdefghijklmnopqrstuvwxyz"));
    }

    @Test
    void encryptPreservesCase() {
        assertEquals("Svool", AtbashCipher.encrypt("Hello"));
        assertEquals("SVOOL", AtbashCipher.encrypt("HELLO"));
        assertEquals("svool", AtbashCipher.encrypt("hello"));
    }

    @Test
    void encryptHelloWorld() {
        assertEquals("Svool, Dliow!", AtbashCipher.encrypt("Hello, World!"));
    }

    @Test
    void encryptPassesThroughNonAlpha() {
        assertEquals("123", AtbashCipher.encrypt("123"));
        assertEquals("!@#", AtbashCipher.encrypt("!@#"));
        assertEquals(" ", AtbashCipher.encrypt(" "));
        assertEquals("A1B2", AtbashCipher.encrypt("Z1Y2"));
    }

    @Test
    void encryptEmptyString() {
        assertEquals("", AtbashCipher.encrypt(""));
    }

    @Test
    void encryptMixedContent() {
        // "Attack at Dawn":
        //   A→Z t→g t→g a→z c→x k→p ' ' a→z t→g ' ' D→W a→z w→d n→m
        assertEquals("Zggzxp zg Wzdm", AtbashCipher.encrypt("Attack at Dawn"));
    }

    // =========================================================================
    // 2. decrypt — same as encrypt (self-inverse)
    // =========================================================================

    @Test
    void decryptIsEncrypt() {
        String[] testCases = {
            "Hello, World!", "ABCXYZ", "The quick brown fox", "", "123!@#", "Svool"
        };
        for (String text : testCases) {
            assertEquals(
                AtbashCipher.encrypt(text),
                AtbashCipher.decrypt(text),
                "decrypt should equal encrypt for: " + text
            );
        }
    }

    @Test
    void decryptInvertsEncrypt() {
        String[] plaintexts = {
            "Hello, World!", "ATTACK AT DAWN", "The quick brown fox jumps over the lazy dog",
            "abcdefghijklmnopqrstuvwxyz", "ABCDEFGHIJKLMNOPQRSTUVWXYZ", "Mixed CASE text!"
        };
        for (String plaintext : plaintexts) {
            String roundtrip = AtbashCipher.decrypt(AtbashCipher.encrypt(plaintext));
            assertEquals(plaintext, roundtrip, "Roundtrip failed for: " + plaintext);
        }
    }

    @Test
    void decryptEmptyString() {
        assertEquals("", AtbashCipher.decrypt(""));
    }

    // =========================================================================
    // 3. Self-inverse property
    // =========================================================================

    @Test
    void selfInverseForEveryLetter() {
        // Applying Atbash twice to any single letter returns the original
        for (char c = 'A'; c <= 'Z'; c++) {
            String s = String.valueOf(c);
            assertEquals(s, AtbashCipher.encrypt(AtbashCipher.encrypt(s)),
                "Self-inverse failed for: " + c);
        }
        for (char c = 'a'; c <= 'z'; c++) {
            String s = String.valueOf(c);
            assertEquals(s, AtbashCipher.encrypt(AtbashCipher.encrypt(s)),
                "Self-inverse failed for: " + c);
        }
    }

    @Test
    void selfInverseForSentence() {
        String original = "The quick brown fox jumps over the lazy dog.";
        assertEquals(original, AtbashCipher.encrypt(AtbashCipher.encrypt(original)));
    }

    // =========================================================================
    // 4. Known test vectors (cross-language parity)
    // =========================================================================

    @Test
    void knownVectorAttack() {
        // "ATTACK" → each letter mirrored
        // A→Z, T→G, T→G, A→Z, C→X, K→P
        assertEquals("ZGGZXP", AtbashCipher.encrypt("ATTACK"));
    }

    @Test
    void knownVectorHello() {
        // H→S, E→V, L→O, L→O, O→L
        assertEquals("SVOOL", AtbashCipher.encrypt("HELLO"));
    }

    @Test
    void knownVectorMirrorPairs() {
        assertEquals("ABCXYZ", AtbashCipher.encrypt("ZYXCBA"));
        assertEquals("ZYXCBA", AtbashCipher.encrypt("ABCXYZ"));
    }

    // =========================================================================
    // 5. Unicode / non-ASCII pass-through
    // =========================================================================

    @Test
    void nonAsciiPassesThrough() {
        // Characters outside A-Z a-z pass through unchanged
        // "Xzuv" → X→C, z→a, u→f, v→e → "Cafe"
        assertEquals("Cafe", AtbashCipher.encrypt("Xzuv"));
        assertEquals("Svool Dliow", AtbashCipher.encrypt("Hello World"));
        // Digits and punctuation pass through
        assertEquals("C1f2", AtbashCipher.encrypt("X1u2"));
    }
}
