// scytale_cipher.hpp — the Scytale transposition cipher, in pure ISO C++17,
// header-only, in namespace ca::scytale. A faithful port of the Rust
// `scytale-cipher` crate.
// ===========================================================================
//
// The Scytale (Sparta, ~700 BCE) is a transposition cipher: it reorders
// characters rather than replacing them. Encryption writes the text row-by-row
// into a grid `key` columns wide, pads the final row with spaces, then reads the
// grid column-by-column. Decryption rebuilds the grid (handling a short final
// row, as arises during brute force), reads it row-by-row, and strips the
// trailing pad spaces.
//
// Like the crate (which works on `char`s), this port transposes whole
// CHARACTERS, not bytes: the input is split into UTF-8 character units and the
// units are reordered, so multibyte characters stay intact. Malformed bytes are
// treated as single-byte units, so any input round-trips losslessly.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_SCYTALE_CIPHER_HPP
#define CA_SCYTALE_CIPHER_HPP

#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <vector>

namespace ca {
namespace scytale {

namespace detail {

// Byte length of the UTF-8 character led by `c` (a stray/continuation byte
// counts as a single-byte character).
inline std::size_t utf8_lead_len(unsigned char c) {
    if (c < 0x80) return 1;
    if ((c & 0xE0) == 0xC0) return 2;
    if ((c & 0xF0) == 0xE0) return 3;
    if ((c & 0xF8) == 0xF0) return 4;
    return 1;
}

// Split `text` into its UTF-8 character units (each unit is one character's
// bytes).
inline std::vector<std::string> split_units(const std::string& text) {
    std::vector<std::string> units;
    std::size_t i = 0, n = text.size();
    while (i < n) {
        std::size_t l = utf8_lead_len(static_cast<unsigned char>(text[i]));
        if (l > n - i) {
            l = n - i;
        }
        units.push_back(text.substr(i, l));
        i += l;
    }
    return units;
}

}  // namespace detail

// Encrypt `text` with `key`. An empty text yields an empty string; otherwise
// std::nullopt if the key is invalid (key < 2 or key > character count).
inline std::optional<std::string> encrypt(const std::string& text,
                                          std::size_t key) {
    if (text.empty()) {
        return std::string();
    }
    std::vector<std::string> units = detail::split_units(text);
    std::size_t n = units.size();
    if (key < 2 || key > n) {
        return std::nullopt;
    }
    std::size_t num_rows = n / key + (n % key ? 1 : 0);
    std::size_t padded_len = num_rows * key;
    units.resize(padded_len, " "); // pad the final row with spaces
    std::string result;
    for (std::size_t col = 0; col < key; ++col) {
        for (std::size_t row = 0; row < num_rows; ++row) {
            result += units[row * key + col];
        }
    }
    return result;
}

// Decrypt `text` with `key` (trailing pad spaces are stripped). Same contract.
inline std::optional<std::string> decrypt(const std::string& text,
                                          std::size_t key) {
    if (text.empty()) {
        return std::string();
    }
    std::vector<std::string> units = detail::split_units(text);
    std::size_t n = units.size();
    if (key < 2 || key > n) {
        return std::nullopt;
    }
    std::size_t num_rows = n / key + (n % key ? 1 : 0);
    std::size_t full_cols = (n % key == 0) ? key : (n % key);

    std::vector<std::size_t> col_starts(key), col_lens(key);
    std::size_t offset = 0;
    for (std::size_t col = 0; col < key; ++col) {
        std::size_t len =
            (n % key == 0 || col < full_cols) ? num_rows : num_rows - 1;
        col_starts[col] = offset;
        col_lens[col] = len;
        offset += len;
    }

    std::vector<std::string> ordered;
    ordered.reserve(n);
    for (std::size_t row = 0; row < num_rows; ++row) {
        for (std::size_t col = 0; col < key; ++col) {
            if (row < col_lens[col]) {
                ordered.push_back(units[col_starts[col] + row]);
            }
        }
    }
    std::size_t emit = ordered.size();
    while (emit > 0 && ordered[emit - 1] == " ") {
        --emit;
    }
    std::string result;
    for (std::size_t i = 0; i < emit; ++i) {
        result += ordered[i];
    }
    return result;
}

// One brute-force decryption: the tried key and the resulting plaintext.
struct BruteForceResult {
    std::size_t key;
    std::string text;
};

// Try every key from 2 to (character count)/2 and return the decryptions;
// empty when the text has fewer than 4 characters.
inline std::vector<BruteForceResult> brute_force(const std::string& text) {
    std::vector<BruteForceResult> results;
    std::size_t n = detail::split_units(text).size();
    if (n < 4) {
        return results;
    }
    std::size_t max_key = n / 2;
    for (std::size_t key = 2; key <= max_key; ++key) {
        std::optional<std::string> decrypted = decrypt(text, key);
        if (decrypted) {
            results.push_back({key, *decrypted});
        }
    }
    return results;
}

}  // namespace scytale
}  // namespace ca

#endif  // CA_SCYTALE_CIPHER_HPP
