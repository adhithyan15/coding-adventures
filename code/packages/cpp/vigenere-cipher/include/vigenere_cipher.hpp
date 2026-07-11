// vigenere_cipher.hpp — the Vigenere polyalphabetic cipher with cryptanalysis,
// in pure ISO C++17, header-only, in namespace ca::vigenere. A faithful port of
// the Rust `vigenere-cipher` crate.
// ===========================================================================
//
// The Vigenere cipher (Bellaso, 1553) shifts each plaintext letter by a
// different amount taken from a repeating keyword. It resisted cryptanalysis
// for 300 years until Kasiski (1863) and Friedman (1920s) broke it with two
// statistical tools, both provided here:
//
//   Index of Coincidence — measures how "English-like" a letter distribution
//     is; splitting ciphertext by the true key length makes each group a Caesar
//     cipher on English, whose IC (~0.067) stands out from random (~0.038).
//     This reveals the KEY LENGTH.
//
//   Chi-squared — with the key length known, each position-group is a Caesar
//     cipher; the shift whose decrypted letter frequencies best match English
//     (lowest chi-squared) is that key letter. This reveals the KEY.
//
// Character handling (matching the crate): letters are shifted preserving case;
// non-alphabetic characters pass through and do NOT advance the key position;
// the key must be non-empty and all ASCII letters (else encrypt/decrypt return
// std::nullopt).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No libm (the statistics use only
// +, -, *, /); std::numeric_limits<double>::max() stands in for f64::INFINITY.
#ifndef CA_VIGENERE_CIPHER_HPP
#define CA_VIGENERE_CIPHER_HPP

#include <array>
#include <cstddef>
#include <limits>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace vigenere {

namespace detail {

inline bool is_upper(unsigned char c) { return c >= 'A' && c <= 'Z'; }
inline bool is_lower(unsigned char c) { return c >= 'a' && c <= 'z'; }
inline bool is_alpha(unsigned char c) { return is_upper(c) || is_lower(c); }
inline unsigned char to_upper(unsigned char c) {
    return is_lower(c) ? static_cast<unsigned char>(c - 'a' + 'A') : c;
}

// English letter frequencies (A-Z) for chi-squared analysis.
inline constexpr std::array<double, 26> kEnglishFrequencies = {
    0.08167, 0.01492, 0.02782, 0.04253, 0.12702, 0.02228, 0.02015,
    0.06094, 0.06966, 0.00153, 0.00772, 0.04025, 0.02406, 0.06749,
    0.07507, 0.01929, 0.00095, 0.05987, 0.06327, 0.09056, 0.02758,
    0.00978, 0.02360, 0.00150, 0.01974, 0.00074};

// The ASCII letters of `text`, upper-cased.
inline std::vector<unsigned char> extract_alpha_upper(const std::string& text) {
    std::vector<unsigned char> out;
    for (char rc : text) {
        unsigned char c = static_cast<unsigned char>(rc);
        if (is_alpha(c)) {
            out.push_back(to_upper(c));
        }
    }
    return out;
}

// Chi-squared of `counts` (26 bins, `total` letters > 0) against English.
inline double chi_squared(const std::array<std::size_t, 26>& counts,
                          std::size_t total) {
    double chi2 = 0.0;
    for (std::size_t i = 0; i < 26; ++i) {
        double expected = kEnglishFrequencies[i] * static_cast<double>(total);
        double diff = static_cast<double>(counts[i]) - expected;
        chi2 += (diff * diff) / expected;
    }
    return chi2;
}

// Shared engine for encrypt (forward = true) and decrypt (forward = false).
inline std::optional<std::string> transform(const std::string& text,
                                            const std::string& key,
                                            bool forward);

}  // namespace detail

// True iff `key` is non-empty and all ASCII letters.
inline bool key_valid(const std::string& key) {
    if (key.empty()) {
        return false;
    }
    for (char c : key) {
        if (!detail::is_alpha(static_cast<unsigned char>(c))) {
            return false;
        }
    }
    return true;
}

// ---- core cipher ------------------------------------------------------

inline std::optional<std::string> encrypt(const std::string& plaintext,
                                          const std::string& key) {
    return detail::transform(plaintext, key, true);
}

inline std::optional<std::string> decrypt(const std::string& ciphertext,
                                          const std::string& key) {
    return detail::transform(ciphertext, key, false);
}

inline std::optional<std::string> detail::transform(const std::string& text,
                                                    const std::string& key,
                                                    bool forward) {
    if (!key_valid(key)) {
        return std::nullopt;
    }
    std::string kup;
    kup.reserve(key.size());
    for (char c : key) {
        kup.push_back(static_cast<char>(to_upper(static_cast<unsigned char>(c))));
    }

    std::string result;
    result.reserve(text.size());
    std::size_t ki = 0;
    for (char rc : text) {
        unsigned char ch = static_cast<unsigned char>(rc);
        if (is_upper(ch) || is_lower(ch)) {
            int base = is_upper(ch) ? 'A' : 'a';
            int shift = static_cast<unsigned char>(kup[ki % kup.size()]) - 'A';
            int off = forward ? (ch - base + shift) : (ch - base + 26 - shift);
            result.push_back(static_cast<char>((off % 26) + base));
            ++ki;
        } else {
            result.push_back(static_cast<char>(ch));
        }
    }
    return result;
}

// ---- cryptanalysis ----------------------------------------------------

// Estimate the key length by Index of Coincidence over candidate lengths
// 2..max_length, returning the smallest whose average IC is within 90% of the
// best. Returns 1 for text with fewer than two letters.
inline std::size_t find_key_length(const std::string& ciphertext,
                                   std::size_t max_length) {
    std::vector<unsigned char> letters = detail::extract_alpha_upper(ciphertext);
    std::size_t n = letters.size();
    if (n < 2) {
        return 1;
    }
    std::size_t limit = (max_length < n / 2) ? max_length : n / 2;
    std::vector<double> avg_ics(limit + 1, 0.0);

    for (std::size_t k = 2; k <= limit; ++k) {
        double total_ic = 0.0;
        std::size_t group_count = 0;
        for (std::size_t i = 0; i < k; ++i) {
            std::array<std::size_t, 26> counts = {};
            std::size_t gn = 0;
            for (std::size_t j = i; j < n; j += k) {
                ++counts[letters[j] - 'A'];
                ++gn;
            }
            if (gn > 1) {
                std::size_t num = 0;
                for (std::size_t t = 0; t < 26; ++t) {
                    num += counts[t] * (counts[t] > 0 ? counts[t] - 1 : 0);
                }
                total_ic += static_cast<double>(num) /
                            static_cast<double>(gn * (gn - 1));
                ++group_count;
            }
        }
        if (group_count > 0) {
            avg_ics[k] = total_ic / static_cast<double>(group_count);
        }
    }

    double best_ic = 0.0;
    for (double ic : avg_ics) {
        if (ic > best_ic) {
            best_ic = ic;
        }
    }
    if (best_ic > 0.0) {
        double threshold = best_ic * 0.9;
        for (std::size_t k = 2; k <= limit; ++k) {
            if (avg_ics[k] >= threshold) {
                return k;
            }
        }
    }
    return 1;
}

// Recover the key for a known key length by chi-squared frequency analysis.
inline std::string find_key(const std::string& ciphertext,
                            std::size_t key_length) {
    std::vector<unsigned char> letters = detail::extract_alpha_upper(ciphertext);
    std::size_t n = letters.size();
    std::string key;
    key.reserve(key_length);

    for (std::size_t pos = 0; pos < key_length; ++pos) {
        std::size_t gn = 0;
        for (std::size_t j = pos; j < n; j += key_length) {  // key_length >= 1
            ++gn;
        }
        if (gn == 0) {
            key.push_back('A');
            continue;
        }
        unsigned int best_shift = 0;
        double best_chi2 = std::numeric_limits<double>::max();
        for (unsigned int shift = 0; shift < 26; ++shift) {
            std::array<std::size_t, 26> counts = {};
            for (std::size_t j = pos; j < n; j += key_length) {
                unsigned int dec = (letters[j] - 'A' + 26 - shift) % 26;
                ++counts[dec];
            }
            double chi2 = detail::chi_squared(counts, gn);
            if (chi2 < best_chi2) {
                best_chi2 = chi2;
                best_shift = shift;
            }
        }
        key.push_back(static_cast<char>('A' + best_shift));
    }
    return key;
}

struct BreakResult {
    std::string key;
    std::string plaintext;
};

// Automatically break the ciphertext: find the key length, then the key, then
// decrypt (empty plaintext if decryption somehow fails, matching the crate).
inline BreakResult break_cipher(const std::string& ciphertext) {
    std::size_t key_length = find_key_length(ciphertext, 20);
    std::string key = find_key(ciphertext, key_length);
    std::optional<std::string> plaintext = decrypt(ciphertext, key);
    return BreakResult{key, plaintext.value_or(std::string())};
}

}  // namespace vigenere
}  // namespace ca

#endif  // CA_VIGENERE_CIPHER_HPP
