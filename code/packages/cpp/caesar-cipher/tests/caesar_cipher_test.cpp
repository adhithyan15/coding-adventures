// caesar_cipher_test.cpp — behavioral tests for the C++ Caesar cipher, using
// the header-only iso_test.h harness (pure ISO). Mirrors the Rust crate's
// tests: shift-3 example, wraparound, ROT13 round-trips, normalisation,
// letter counts, brute-force, and the frequency-analysis attack.
#include "iso_test.h"

#include <climits> // INT_MIN, INT_MAX — extreme-shift edge cases

#include "caesar_cipher.hpp"

int main() {
    using namespace caesar_cipher;

    // Classic example and its inverse.
    ISO_CHECK_STR_EQ(encrypt("Hello, World!", 3).c_str(), "Khoor, Zruog!");
    ISO_CHECK_STR_EQ(decrypt("Khoor, Zruog!", 3).c_str(), "Hello, World!");

    // Wraparound preserves case; non-letters pass through.
    ISO_CHECK_STR_EQ(encrypt("XYZ xyz", 3).c_str(), "ABC abc");

    // ROT13 is its own inverse.
    ISO_CHECK_STR_EQ(rot13(rot13("Hello")).c_str(), "Hello");
    ISO_CHECK_STR_EQ(rot13("Why did the chicken?").c_str(),
                     "Jul qvq gur puvpxra?");

    // Shift normalisation: 29 ≡ 3, -23 ≡ 3, 26 ≡ 0 (identity).
    ISO_CHECK_STR_EQ(encrypt("abc", 29).c_str(), "def");
    ISO_CHECK_STR_EQ(encrypt("abc", -23).c_str(), "def");
    ISO_CHECK_STR_EQ(encrypt("unchanged 123!", 26).c_str(), "unchanged 123!");

    // Extreme shifts must not invoke UB (INT_MIN negation) and must round-trip.
    ISO_CHECK_STR_EQ(decrypt(encrypt("Round Trip!", INT_MIN), INT_MIN).c_str(),
                     "Round Trip!");
    ISO_CHECK_STR_EQ(decrypt(encrypt("Round Trip!", INT_MAX), INT_MAX).c_str(),
                     "Round Trip!");

    // Letter counts (case-insensitive, non-letters ignored).
    auto counts = letter_counts("Hello, World!");
    ISO_CHECK_EQ_UINT(counts['l' - 'a'], 3);
    ISO_CHECK_EQ_UINT(counts['o' - 'a'], 2);
    ISO_CHECK_EQ_UINT(counts['z' - 'a'], 0);

    // chi-squared: English scores lower than a scrambled shift; no letters → max.
    ISO_CHECK(chi_squared("the quick brown fox jumps over the lazy dog") <
              chi_squared("wkh txlfn eurzq"));
    ISO_CHECK(chi_squared("12345 !!!") > 1e300);

    // brute_force yields 25 candidates; the shift-3 entry undoes a shift-3
    // encryption.
    auto plain = encrypt("Attack at dawn", 3);
    auto candidates = brute_force(plain);
    ISO_CHECK_EQ_UINT(candidates.size(), 25);
    ISO_CHECK_EQ_INT(candidates[2].shift, 3); // index 2 → shift 3
    ISO_CHECK_STR_EQ(candidates[2].plaintext.c_str(), "Attack at dawn");

    // frequency_analysis recovers the plaintext without knowing the shift.
    auto cipher = encrypt("the quick brown fox jumps over the lazy dog", 7);
    auto result = frequency_analysis(cipher);
    ISO_CHECK_EQ_INT(result.first, 7);
    ISO_CHECK_STR_EQ(result.second.c_str(),
                     "the quick brown fox jumps over the lazy dog");

    return ISO_TEST_RESULT();
}
