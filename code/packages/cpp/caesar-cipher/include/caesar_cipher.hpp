// caesar_cipher.hpp — the Caesar cipher (encrypt / decrypt / ROT13) plus
// brute-force and frequency-analysis attacks, in pure ISO C++17 (header-only).
// A faithful port of the Rust `caesar-cipher` crate.
// ===========================================================================
//
// The Caesar cipher replaces each letter with the letter a fixed number of
// positions further along the alphabet, wrapping at the end:
//
//     Plain:   A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
//     Shift 3: D E F G H I J K L M N O P Q R S T U V W X Y Z A B C
//
// Only ASCII letters shift; digits, punctuation, and spaces pass through, and
// case is preserved. Decryption is encryption by the negative shift.
//
// Unlike the C port, C++ has std::string, so encrypt/decrypt/rot13 simply
// return a new string — mirroring the Rust API closely, including brute_force
// (all 25 candidate decryptions) and frequency_analysis (the best shift plus
// its plaintext).
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef CAESAR_CIPHER_HPP
#define CAESAR_CIPHER_HPP

#include <array>
#include <cstddef>
#include <limits>
#include <string>
#include <utility>
#include <vector>

namespace caesar_cipher {

namespace detail {

inline bool is_upper_ascii(char ch) { return ch >= 'A' && ch <= 'Z'; }
inline bool is_lower_ascii(char ch) { return ch >= 'a' && ch <= 'z'; }

// Standard English letter frequencies (A..Z), matching the Rust crate. Used to
// score how English-like a decryption is.
inline const std::array<double, 26> &english_frequencies() {
    static const std::array<double, 26> freqs = {
        0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015, 0.06094,
        0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749, 0.07507, 0.01929,
        0.00095, 0.05987, 0.06327, 0.09056, 0.02758, 0.00978, 0.02360, 0.00150,
        0.01974, 0.00074};
    return freqs;
}

// Normalise any (possibly negative or large) shift into 0..25.
inline int normalise_shift(int shift) { return ((shift % 26) + 26) % 26; }

inline char shift_char(char ch, int normalised_shift) {
    if (is_upper_ascii(ch)) {
        int position = ch - 'A';
        return static_cast<char>('A' + (position + normalised_shift) % 26);
    }
    if (is_lower_ascii(ch)) {
        int position = ch - 'a';
        return static_cast<char>('a' + (position + normalised_shift) % 26);
    }
    return ch; // non-letters pass through unchanged
}

} // namespace detail

// encrypt — shift every ASCII letter of `text` forward by `shift` positions
// (mod 26; negative shifts allowed), returning the transformed string.
inline std::string encrypt(const std::string &text, int shift) {
    int normalised = detail::normalise_shift(shift);
    std::string out;
    out.reserve(text.size());
    for (char ch : text) {
        out.push_back(detail::shift_char(ch, normalised));
    }
    return out;
}

// decrypt — the inverse of encrypt (shift backwards).
inline std::string decrypt(const std::string &text, int shift) {
    return encrypt(text, -shift);
}

// rot13 — encrypt with a shift of 13; its own inverse.
inline std::string rot13(const std::string &text) { return encrypt(text, 13); }

// letter_counts — how many times each letter A–Z appears (case-insensitive);
// index 0 is 'A' … 25 is 'Z'. Non-letters are ignored.
inline std::array<std::size_t, 26> letter_counts(const std::string &text) {
    std::array<std::size_t, 26> counts = {};
    for (char ch : text) {
        if (detail::is_upper_ascii(ch)) {
            counts[static_cast<std::size_t>(ch - 'A')]++;
        } else if (detail::is_lower_ascii(ch)) {
            counts[static_cast<std::size_t>(ch - 'a')]++;
        }
    }
    return counts;
}

// chi_squared — chi-squared statistic of `text`'s letter distribution against
// English frequencies. Lower means a better fit. Returns a large sentinel when
// there are no letters.
inline double chi_squared(const std::string &text) {
    std::array<std::size_t, 26> counts = letter_counts(text);
    std::size_t total = 0;
    for (std::size_t c : counts) {
        total += c;
    }
    if (total == 0) {
        return std::numeric_limits<double>::max();
    }
    double total_f = static_cast<double>(total);
    double sum = 0.0;
    for (std::size_t i = 0; i < 26; i++) {
        double expected = total_f * detail::english_frequencies()[i];
        if (expected < 1e-10) {
            continue;
        }
        double diff = static_cast<double>(counts[i]) - expected;
        sum += diff * diff / expected;
    }
    return sum;
}

// One decryption candidate produced by a brute-force attack.
struct BruteForceResult {
    int shift;
    std::string plaintext;
};

// brute_force — decrypt `ciphertext` with every shift 1..25 and return all 25
// candidates, in shift order.
inline std::vector<BruteForceResult> brute_force(const std::string &ciphertext) {
    std::vector<BruteForceResult> results;
    results.reserve(25);
    for (int shift = 1; shift <= 25; shift++) {
        results.push_back(BruteForceResult{shift, decrypt(ciphertext, shift)});
    }
    return results;
}

// frequency_analysis — break the cipher without knowing the shift: pick the
// decryption whose letter distribution best fits English (lowest chi-squared).
// Returns {best shift (1..25), best plaintext}. Seeds with shift 1 so ties
// (e.g. no letters) still yield a valid result.
inline std::pair<int, std::string>
frequency_analysis(const std::string &ciphertext) {
    std::string best_plaintext = decrypt(ciphertext, 1);
    int best_shift = 1;
    double best_score = chi_squared(best_plaintext);
    for (int shift = 2; shift <= 25; shift++) {
        std::string candidate = decrypt(ciphertext, shift);
        double score = chi_squared(candidate);
        if (score < best_score) {
            best_score = score;
            best_shift = shift;
            best_plaintext = std::move(candidate);
        }
    }
    return {best_shift, best_plaintext};
}

} // namespace caesar_cipher

#endif // CAESAR_CIPHER_HPP
