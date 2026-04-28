// ============================================================================
// VigenereCipherTest.java — Unit Tests for VigenereCipher
// ============================================================================

package com.codingadventures.vigenerecipher;

import org.junit.jupiter.api.Test;
import static org.junit.jupiter.api.Assertions.*;

class VigenereCipherTest {

    // =========================================================================
    // 1. encrypt — known test vectors
    // =========================================================================

    @Test
    void encryptClassicLemon() {
        // Cross-language test vector from the Python implementation
        assertEquals("LXFOPVEFRNHR", VigenereCipher.encrypt("ATTACKATDAWN", "LEMON"));
    }

    @Test
    void encryptMixedCase() {
        // Key is case-insensitive; output preserves input case
        assertEquals("Rijvs, Uyvjn!", VigenereCipher.encrypt("Hello, World!", "key"));
    }

    @Test
    void encryptKeyRepeats() {
        // Key "AB" means alternating shifts 0 and 1
        // A+0=A, B+1=C, C+0=C, D+1=E, F+0=F
        assertEquals("ACCEF", VigenereCipher.encrypt("ABCDF", "AB"));
        // A+0=A, B+1=C, C+0=C, D+1=E, E+0=E
        assertEquals("ACCEE", VigenereCipher.encrypt("ABCDE", "AB"));
    }

    @Test
    void encryptNonAlphaPassesThrough() {
        assertEquals("Rijvs, Uyvjn!", VigenereCipher.encrypt("Hello, World!", "key"));
    }

    @Test
    void encryptNonAlphaDoesNotAdvanceKey() {
        // "A B" with key "B": 'A' gets shifted by B(1)→B, ' ' passes through,
        // 'B' also gets shifted by B(1)→C (key position still 1 → wraps to key[1%1=0] = B)
        // key "B" length=1: every letter shifts by 1
        assertEquals("B C", VigenereCipher.encrypt("A B", "B"));
    }

    @Test
    void encryptEmptyString() {
        assertEquals("", VigenereCipher.encrypt("", "KEY"));
    }

    @Test
    void encryptKeyLongerThanText() {
        // Only the first key letters are used
        assertEquals("L", VigenereCipher.encrypt("A", "LEMON"));  // A+L=L
    }

    // =========================================================================
    // 2. decrypt — known test vectors
    // =========================================================================

    @Test
    void decryptClassicLemon() {
        assertEquals("ATTACKATDAWN", VigenereCipher.decrypt("LXFOPVEFRNHR", "LEMON"));
    }

    @Test
    void decryptMixedCase() {
        assertEquals("Hello, World!", VigenereCipher.decrypt("Rijvs, Uyvjn!", "key"));
    }

    @Test
    void decryptEmptyString() {
        assertEquals("", VigenereCipher.decrypt("", "KEY"));
    }

    // =========================================================================
    // 3. roundtrip
    // =========================================================================

    @Test
    void roundtripVariousKeys() {
        String[] texts = {
            "ATTACKATDAWN", "Hello, World!", "The quick brown fox jumps over the lazy dog.",
            "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
        };
        String[] keys = { "KEY", "LEMON", "A", "SECRETKEY" };
        for (String text : texts) {
            for (String key : keys) {
                assertEquals(text, VigenereCipher.decrypt(VigenereCipher.encrypt(text, key), key),
                    "Roundtrip failed for text='" + text + "', key=" + key);
            }
        }
    }

    @Test
    void roundtripSingleCharKey() {
        // Single-char key is equivalent to a Caesar cipher
        String text = "Hello World";
        assertEquals(text, VigenereCipher.decrypt(VigenereCipher.encrypt(text, "C"), "C"));
    }

    // =========================================================================
    // 4. Input validation
    // =========================================================================

    @Test
    void encryptRejectsEmptyKey() {
        assertThrows(IllegalArgumentException.class, () -> VigenereCipher.encrypt("Hello", ""));
    }

    @Test
    void encryptRejectsNonAlphaKey() {
        assertThrows(IllegalArgumentException.class, () -> VigenereCipher.encrypt("Hello", "KEY1"));
        assertThrows(IllegalArgumentException.class, () -> VigenereCipher.encrypt("Hello", "KE Y"));
    }

    @Test
    void decryptRejectsEmptyKey() {
        assertThrows(IllegalArgumentException.class, () -> VigenereCipher.decrypt("HELLO", ""));
    }

    // =========================================================================
    // 5. findKeyLength
    // =========================================================================

    @Test
    void findKeyLengthForLongText() {
        // Use 300+ chars of natural English prose for reliable IC analysis
        String plaintext =
            "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
            "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
            "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
            "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS.";
        String ciphertext = VigenereCipher.encrypt(plaintext, "LEMON");
        int keyLen = VigenereCipher.findKeyLength(ciphertext);
        assertEquals(5, keyLen);
    }

    @Test
    void findKeyLengthKey3() {
        String plaintext =
            "THE HISTORY OF CRYPTOGRAPHY IS THE HISTORY OF ATTEMPTS TO COMMUNICATE " +
            "PRIVATELY IN THE PRESENCE OF ADVERSARIES. SINCE THE EARLIEST RECORDED " +
            "HISTORY MILITARY COMMANDERS AND DIPLOMATS HAVE USED SECRET CODES TO " +
            "PROTECT SENSITIVE INFORMATION FROM ENEMIES AND RIVALS WHO MIGHT INTERCEPT.";
        String ciphertext = VigenereCipher.encrypt(plaintext, "KEY");
        int keyLen = VigenereCipher.findKeyLength(ciphertext);
        assertEquals(3, keyLen);
    }

    // =========================================================================
    // 6. findKey
    // =========================================================================

    @Test
    void findKeyForLongTextLemon() {
        String plaintext =
            "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
            "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
            "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
            "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS.";
        String ciphertext = VigenereCipher.encrypt(plaintext, "LEMON");
        String recoveredKey = VigenereCipher.findKey(ciphertext, 5);
        assertEquals("LEMON", recoveredKey);
    }

    @Test
    void findKeyForKey3() {
        String plaintext =
            "THE HISTORY OF CRYPTOGRAPHY IS THE HISTORY OF ATTEMPTS TO COMMUNICATE " +
            "PRIVATELY IN THE PRESENCE OF ADVERSARIES. SINCE THE EARLIEST RECORDED " +
            "HISTORY MILITARY COMMANDERS AND DIPLOMATS HAVE USED SECRET CODES TO " +
            "PROTECT SENSITIVE INFORMATION FROM ENEMIES AND RIVALS WHO MIGHT INTERCEPT.";
        String ciphertext = VigenereCipher.encrypt(plaintext, "KEY");
        String recoveredKey = VigenereCipher.findKey(ciphertext, 3);
        assertEquals("KEY", recoveredKey);
    }

    // =========================================================================
    // 7. breakCipher — full automatic attack
    // =========================================================================

    @Test
    void breakCipherEndToEnd() {
        String plaintext =
            "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
            "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
            "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
            "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS. " +
            "CHARLES BABBAGE BROKE THE CIPHER IN THE EIGHTEEN FIFTIES USING THE INDEX.";
        String ciphertext = VigenereCipher.encrypt(plaintext, "LEMON");
        VigenereCipher.BreakResult result = VigenereCipher.breakCipher(ciphertext);
        assertEquals("LEMON", result.key);
        assertEquals(plaintext, result.plaintext);
    }

    // =========================================================================
    // 8. ENGLISH_FREQUENCIES
    // =========================================================================

    @Test
    void englishFrequenciesLength() {
        assertEquals(26, VigenereCipher.ENGLISH_FREQUENCIES.length);
    }

    @Test
    void englishFrequenciesSumToOne() {
        double sum = 0.0;
        for (double f : VigenereCipher.ENGLISH_FREQUENCIES) sum += f;
        assertEquals(1.0, sum, 0.001);
    }
}
