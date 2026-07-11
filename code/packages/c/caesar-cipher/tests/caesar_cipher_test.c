/*
 * caesar_cipher_test.c — behavioral tests for the C Caesar cipher, using the
 * header-only iso_test.h harness (pure ISO). Mirrors the Rust crate's tests:
 * the classic shift-3 example, wraparound, ROT13 round-trips, case/non-letter
 * preservation, negative and >26 shifts, letter counts, and the
 * frequency-analysis attack recovering a known plaintext.
 */
#include "iso_test.h"

#include <limits.h> /* INT_MIN, INT_MAX — extreme-shift edge cases */

#include "caesar_cipher.h"

int main(void) {
    char buf[128];
    char round[128];
    size_t counts[26];
    int shift;

    /* Classic example: "Hello, World!" shifted by 3 → "Khoor, Zruog!"
     * (letters shift, punctuation/space/case are preserved). */
    caesar_encrypt("Hello, World!", 3, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "Khoor, Zruog!");

    /* Decrypt is the inverse. */
    caesar_decrypt(buf, 3, round, sizeof round);
    ISO_CHECK_STR_EQ(round, "Hello, World!");

    /* Wraparound: X/Y/Z shift 3 → A/B/C, preserving case. */
    caesar_encrypt("XYZ xyz", 3, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "ABC abc");

    /* ROT13 is its own inverse. */
    caesar_rot13("Hello", buf, sizeof buf);
    caesar_rot13(buf, round, sizeof round);
    ISO_CHECK_STR_EQ(round, "Hello");
    caesar_rot13("Why did the chicken?", buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "Jul qvq gur puvpxra?");

    /* Shift normalisation: 29 ≡ 3, and -23 ≡ 3 (mod 26). */
    caesar_encrypt("abc", 29, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "def");
    caesar_encrypt("abc", -23, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "def");

    /* A shift of 0 (or any multiple of 26) is the identity. */
    caesar_encrypt("unchanged 123!", 26, buf, sizeof buf);
    ISO_CHECK_STR_EQ(buf, "unchanged 123!");

    /* Extreme shifts must not invoke UB (INT_MIN negation) and must round-trip:
     * encrypt then decrypt with the same extreme shift restores the plaintext. */
    caesar_encrypt("Round Trip!", INT_MIN, buf, sizeof buf);
    caesar_decrypt(buf, INT_MIN, round, sizeof round);
    ISO_CHECK_STR_EQ(round, "Round Trip!");
    caesar_encrypt("Round Trip!", INT_MAX, buf, sizeof buf);
    caesar_decrypt(buf, INT_MAX, round, sizeof round);
    ISO_CHECK_STR_EQ(round, "Round Trip!");

    /* Return value is the character count (excludes the NUL). */
    ISO_CHECK_EQ_INT(caesar_encrypt("abcde", 1, buf, sizeof buf), 5);

    /* Too-small buffer is reported, not overflowed. */
    ISO_CHECK_EQ_INT(caesar_encrypt("abcde", 1, buf, 3), -1);

    /* Letter counts are case-insensitive and ignore non-letters. */
    caesar_letter_counts("Hello, World!", counts);
    ISO_CHECK_EQ_UINT(counts['l' - 'a'], 3); /* l/L: He(ll)o, Wor(l)d */
    ISO_CHECK_EQ_UINT(counts['o' - 'a'], 2); /* o/O */
    ISO_CHECK_EQ_UINT(counts['z' - 'a'], 0);

    /* chi-squared: plain English scores much lower than a scrambled shift. */
    ISO_CHECK(caesar_chi_squared("the quick brown fox jumps over the lazy dog") <
              caesar_chi_squared("wkh txlfn eurzq"));
    /* No letters → the large sentinel. */
    ISO_CHECK(caesar_chi_squared("12345 !!!") > 1e300);

    /* Frequency analysis recovers the plaintext without knowing the shift.
     * Encrypt a sufficiently English sentence, then attack it. */
    caesar_encrypt("the quick brown fox jumps over the lazy dog", 7, buf,
                   sizeof buf);
    shift = caesar_frequency_analysis(buf, round, sizeof round);
    ISO_CHECK_EQ_INT(shift, 7);
    ISO_CHECK_STR_EQ(round, "the quick brown fox jumps over the lazy dog");

    return ISO_TEST_RESULT();
}
