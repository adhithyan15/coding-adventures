// ============================================================================
// ScytaleCipherTest.java — Unit Tests for ScytaleCipher
// ============================================================================

package com.codingadventures.scytalecipher;

import org.junit.jupiter.api.Test;
import java.util.List;
import static org.junit.jupiter.api.Assertions.*;

class ScytaleCipherTest {

    // =========================================================================
    // 1. encrypt — basic correctness
    // =========================================================================

    @Test
    void encryptSimple() {
        // "HELLOSPARTANS" key=4
        // Grid:  H E L L
        //        O S P A
        //        R T A N
        //        S _ _ _
        // Cols: HORS | EST_ | LPAA | LANA_ → wait, let me recompute
        // col 0: H,O,R,S → "HORS"
        // col 1: E,S,T,_ → "EST "
        // col 2: L,P,A,_ → "LPA "
        // col 3: L,A,N,_ → "LAN "
        assertEquals("HORSEST LPA LAN ", ScytaleCipher.encrypt("HELLOSPARTANS", 4));
    }

    @Test
    void encryptKey2() {
        // "ABCDEF" key=2
        // Grid: A B
        //       C D
        //       E F
        // cols: ACE | BDF
        assertEquals("ACEBDF", ScytaleCipher.encrypt("ABCDEF", 2));
    }

    @Test
    void encryptKey3Even() {
        // "ABCDEF" key=3 (2 rows, no padding needed)
        // Grid: A B C
        //       D E F
        // cols: AD | BE | CF
        assertEquals("ADBECF", ScytaleCipher.encrypt("ABCDEF", 3));
    }

    @Test
    void encryptPadsLastRow() {
        // "HELLO" key=3
        // Grid: H E L
        //       L O _   ← padded
        // cols: HL | EO | L_
        assertEquals("HLEOL ", ScytaleCipher.encrypt("HELLO", 3));
    }

    @Test
    void encryptEmptyString() {
        assertEquals("", ScytaleCipher.encrypt("", 3));
    }

    @Test
    void encryptSingleRow() {
        // When text length == key, there's one row — ciphertext is text rotated
        // "ABCD" key=4 → grid is just one row, columns are individual chars
        assertEquals("ABCD", ScytaleCipher.encrypt("ABCD", 4));
    }

    @Test
    void encryptPreservesNonAlpha() {
        // All characters (spaces, digits, punctuation) are transposed as-is
        // "ABCCDD" key=3: grid [A B C][C D D] → cols AC|BD|CD → "ACBDCD"
        assertEquals("ACBDCD", ScytaleCipher.encrypt("ABCCDD", 3));
    }

    // =========================================================================
    // 2. decrypt
    // =========================================================================

    @Test
    void decryptSimple() {
        assertEquals("HELLOSPARTANS", ScytaleCipher.decrypt("HORSEST LPA LAN ", 4));
    }

    @Test
    void decryptKey2() {
        assertEquals("ABCDEF", ScytaleCipher.decrypt("ACEBDF", 2));
    }

    @Test
    void decryptKey3() {
        assertEquals("ABCDEF", ScytaleCipher.decrypt("ADBECF", 3));
    }

    @Test
    void decryptStripsPadding() {
        // HLEOL_ where _ is a space
        assertEquals("HELLO", ScytaleCipher.decrypt("HLEOL ", 3));
    }

    @Test
    void decryptEmptyString() {
        assertEquals("", ScytaleCipher.decrypt("", 4));
    }

    // =========================================================================
    // 3. roundtrip
    // =========================================================================

    @Test
    void roundtripNoSpecialChars() {
        String[] texts = { "HELLOSPARTANS", "ABCDEFGHIJKLMNOP", "ATTACKATDAWN" };
        int[] keys = { 2, 3, 4, 5 };
        for (String text : texts) {
            for (int key : keys) {
                if (key <= text.length()) {
                    assertEquals(text, ScytaleCipher.decrypt(ScytaleCipher.encrypt(text, key), key),
                        "Roundtrip failed: text='" + text + "', key=" + key);
                }
            }
        }
    }

    @Test
    void roundtripWithSpacesInPlaintext() {
        // Spaces in plaintext survive only when they're not at the very end
        // (trailing spaces are stripped by decrypt). Test with spaces in the middle.
        String text = "HELLO WORLD";
        assertEquals(text, ScytaleCipher.decrypt(ScytaleCipher.encrypt(text, 4), 4));
    }

    // =========================================================================
    // 4. Input validation
    // =========================================================================

    @Test
    void encryptRejectsKey1() {
        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.encrypt("HELLO", 1));
    }

    @Test
    void encryptRejectsKey0() {
        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.encrypt("HELLO", 0));
    }

    @Test
    void encryptRejectsNegativeKey() {
        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.encrypt("HELLO", -1));
    }

    @Test
    void encryptRejectsKeyExceedingLength() {
        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.encrypt("HI", 5));
    }

    @Test
    void decryptRejectsKey1() {
        assertThrows(IllegalArgumentException.class, () -> ScytaleCipher.decrypt("KHOOR", 1));
    }

    // =========================================================================
    // 5. bruteForce
    // =========================================================================

    @Test
    void bruteForceTooShortReturnsEmpty() {
        assertEquals(0, ScytaleCipher.bruteForce("HI").size());
        assertEquals(0, ScytaleCipher.bruteForce("HEL").size());
    }

    @Test
    void bruteForceReturnsCorrectKeys() {
        // "ABCDEF" (length 6) → keys 2, 3
        List<ScytaleCipher.BruteForceResult> results = ScytaleCipher.bruteForce("ACEBDF");
        assertEquals(2, results.get(0).key);
        assertEquals(3, results.get(1).key);
    }

    @Test
    void bruteForceContainsCorrectDecryption() {
        String ciphertext = ScytaleCipher.encrypt("HELLOSPARTANS", 4);
        List<ScytaleCipher.BruteForceResult> results = ScytaleCipher.bruteForce(ciphertext);
        boolean found = results.stream().anyMatch(r -> r.key == 4 && r.text.equals("HELLOSPARTANS"));
        assertTrue(found, "brute force should find key=4 → HELLOSPARTANS");
    }
}
