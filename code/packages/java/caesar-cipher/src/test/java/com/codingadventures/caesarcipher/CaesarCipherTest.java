// ============================================================================
// CaesarCipherTest.java — Unit Tests for CaesarCipher
// ============================================================================

package com.codingadventures.caesarcipher;

import org.junit.jupiter.api.Test;
import java.util.List;
import static org.junit.jupiter.api.Assertions.*;

class CaesarCipherTest {

    // =========================================================================
    // 1. encrypt — basic correctness
    // =========================================================================

    @Test
    void encryptShift0IsIdentity() {
        assertEquals("Hello", CaesarCipher.encrypt("Hello", 0));
        assertEquals("ABCXYZ", CaesarCipher.encrypt("ABCXYZ", 0));
    }

    @Test
    void encryptShift1() {
        assertEquals("B", CaesarCipher.encrypt("A", 1));
        assertEquals("A", CaesarCipher.encrypt("Z", 1));  // wraps
    }

    @Test
    void encryptShift3Classic() {
        assertEquals("KHOOR", CaesarCipher.encrypt("HELLO", 3));
        assertEquals("Khoor, Zruog!", CaesarCipher.encrypt("Hello, World!", 3));
    }

    @Test
    void encryptShift13Rot13() {
        assertEquals("URYYB", CaesarCipher.encrypt("HELLO", 13));
    }

    @Test
    void encryptShift25() {
        // Shift 25 = shift -1 (back one)
        assertEquals("Z", CaesarCipher.encrypt("A", 25));
        assertEquals("ABCDE", CaesarCipher.encrypt("BCDEF", 25));
    }

    @Test
    void encryptPreservesCase() {
        assertEquals("Khoor", CaesarCipher.encrypt("Hello", 3));
        assertEquals("KHOOR", CaesarCipher.encrypt("HELLO", 3));
        assertEquals("khoor", CaesarCipher.encrypt("hello", 3));
    }

    @Test
    void encryptNonAlphaPassesThrough() {
        assertEquals("Khoor, Zruog!", CaesarCipher.encrypt("Hello, World!", 3));
        assertEquals("123 !@#", CaesarCipher.encrypt("123 !@#", 7));
    }

    @Test
    void encryptEmptyString() {
        assertEquals("", CaesarCipher.encrypt("", 5));
    }

    // =========================================================================
    // 2. shift arithmetic — mod 26 and negatives
    // =========================================================================

    @Test
    void encryptShift26IsIdentity() {
        assertEquals("HELLO", CaesarCipher.encrypt("HELLO", 26));
        assertEquals("HELLO", CaesarCipher.encrypt("HELLO", 52));
    }

    @Test
    void encryptNegativeShift() {
        assertEquals("HELLO", CaesarCipher.encrypt("KHOOR", -3));
    }

    @Test
    void encryptLargePositiveShift() {
        // Shift 29 = shift 3 (29 mod 26)
        assertEquals(CaesarCipher.encrypt("HELLO", 3), CaesarCipher.encrypt("HELLO", 29));
    }

    // =========================================================================
    // 3. decrypt
    // =========================================================================

    @Test
    void decryptShift3() {
        assertEquals("HELLO", CaesarCipher.decrypt("KHOOR", 3));
        assertEquals("Hello, World!", CaesarCipher.decrypt("Khoor, Zruog!", 3));
    }

    @Test
    void decryptRoundtrip() {
        String[] texts = { "Hello, World!", "ATTACK AT DAWN", "The quick brown fox.", "" };
        int[] shifts = { 1, 3, 13, 25 };
        for (String text : texts) {
            for (int shift : shifts) {
                assertEquals(text, CaesarCipher.decrypt(CaesarCipher.encrypt(text, shift), shift),
                    "Roundtrip failed for text='" + text + "', shift=" + shift);
            }
        }
    }

    // =========================================================================
    // 4. ROT13
    // =========================================================================

    @Test
    void rot13BasicTest() {
        assertEquals("URYYB", CaesarCipher.rot13("HELLO"));
        assertEquals("HELLO", CaesarCipher.rot13("URYYB"));
    }

    @Test
    void rot13SelfInverse() {
        String[] tests = { "Hello, World!", "Why did the chicken cross the road?", "ABCxyz" };
        for (String text : tests) {
            assertEquals(text, CaesarCipher.rot13(CaesarCipher.rot13(text)),
                "ROT13 self-inverse failed for: " + text);
        }
    }

    @Test
    void rot13NonAlphaUnchanged() {
        assertEquals("Uryyb, Jbeyq!", CaesarCipher.rot13("Hello, World!"));
    }

    // =========================================================================
    // 5. bruteForce
    // =========================================================================

    @Test
    void bruteForceReturns25Results() {
        List<CaesarCipher.BruteForceResult> results = CaesarCipher.bruteForce("KHOOR");
        assertEquals(25, results.size());
    }

    @Test
    void bruteForceShiftsAre1Through25() {
        List<CaesarCipher.BruteForceResult> results = CaesarCipher.bruteForce("KHOOR");
        for (int i = 0; i < 25; i++) {
            assertEquals(i + 1, results.get(i).shift);
        }
    }

    @Test
    void bruteForceContainsCorrectDecryption() {
        // "KHOOR" encrypted with shift 3 → plaintext is "HELLO" at shift 3
        List<CaesarCipher.BruteForceResult> results = CaesarCipher.bruteForce("KHOOR");
        boolean found = results.stream().anyMatch(r -> r.shift == 3 && r.text.equals("HELLO"));
        assertTrue(found, "brute force should include shift=3, text=HELLO");
    }

    @Test
    void bruteForceEmptyString() {
        List<CaesarCipher.BruteForceResult> results = CaesarCipher.bruteForce("");
        assertEquals(25, results.size());
        for (CaesarCipher.BruteForceResult r : results) {
            assertEquals("", r.text);
        }
    }

    // =========================================================================
    // 6. frequencyAnalysis
    // =========================================================================

    @Test
    void frequencyAnalysisEmptyString() {
        CaesarCipher.FrequencyResult result = CaesarCipher.frequencyAnalysis("");
        assertEquals(0, result.shift);
        assertEquals("", result.text);
    }

    @Test
    void frequencyAnalysisLongEnglishText() {
        // Encrypt a long English text with shift=13, then recover it
        String plaintext = "The quick brown fox jumps over the lazy dog. " +
            "Pack my box with five dozen liquor jugs. " +
            "How vexingly quick daft zebras jump.";
        String ciphertext = CaesarCipher.encrypt(plaintext, 13);
        CaesarCipher.FrequencyResult result = CaesarCipher.frequencyAnalysis(ciphertext);
        assertEquals(13, result.shift, "Frequency analysis should recover shift=13");
        assertEquals(plaintext, result.text);
    }

    @Test
    void frequencyAnalysisShift3() {
        String plaintext = "In the beginning God created the heavens and the earth. " +
            "The earth was without form and void and darkness was over the face of the deep.";
        String ciphertext = CaesarCipher.encrypt(plaintext, 3);
        CaesarCipher.FrequencyResult result = CaesarCipher.frequencyAnalysis(ciphertext);
        assertEquals(3, result.shift, "Frequency analysis should recover shift=3");
    }

    // =========================================================================
    // 7. ENGLISH_FREQUENCIES constant
    // =========================================================================

    @Test
    void englishFrequenciesLength() {
        assertEquals(26, CaesarCipher.ENGLISH_FREQUENCIES.length);
    }

    @Test
    void englishFrequenciesSumToOne() {
        double sum = 0.0;
        for (double f : CaesarCipher.ENGLISH_FREQUENCIES) sum += f;
        assertEquals(1.0, sum, 0.001, "Frequencies should sum to ~1.0");
    }

    @Test
    void eIsTheMostCommonLetter() {
        // E (index 4) should have the highest frequency
        double eFreq = CaesarCipher.ENGLISH_FREQUENCIES[4];  // 'E'
        for (int i = 0; i < 26; i++) {
            if (i != 4) {
                assertTrue(eFreq >= CaesarCipher.ENGLISH_FREQUENCIES[i],
                    "E should be most frequent; index " + i + " had higher freq");
            }
        }
    }
}
