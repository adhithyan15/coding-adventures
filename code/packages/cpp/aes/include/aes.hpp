// aes.hpp — the AES block cipher (FIPS 197), in pure ISO C++17 (header-only). A
// faithful port of the Rust `aes` crate, in namespace `ca::aes`.
// ===========================================================================
//
// AES encrypts a 128-bit block under a 128-, 192-, or 256-bit key through 10,
// 12, or 14 rounds of SubBytes / ShiftRows / MixColumns / AddRoundKey (the last
// round omits MixColumns). The S-box is built from the GF(2^8) inverse (AES
// polynomial 0x11B) plus an affine transform, computed via the sibling `gf256`
// header — exactly as the Rust crate uses `gf256::Field`.
//
//   ca::aes::encrypt_block / decrypt_block — the raw 16-byte block cipher
//   ca::aes::sbox / inv_sbox               — the S-box tables
//
// Output matches the FIPS 197 known-answer vectors (Appendices B and C).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef AES_HPP
#define AES_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <cstring>
#include <optional>
#include <vector>

#include "gf256.hpp"

namespace ca {
namespace aes {

using block_t = std::array<std::uint8_t, 16>;

namespace detail {

inline constexpr std::uint8_t RCON[15] = {0x00, 0x01, 0x02, 0x04, 0x08,
                                          0x10, 0x20, 0x40, 0x80, 0x1B,
                                          0x36, 0x6C, 0xD8, 0xAB, 0x4D};

inline std::uint8_t rotl8(std::uint8_t x, unsigned n) {
    return static_cast<std::uint8_t>((x << n) | (x >> (8 - n)));
}
inline std::uint8_t affine_transform(std::uint8_t b) {
    return static_cast<std::uint8_t>(b ^ rotl8(b, 1) ^ rotl8(b, 2) ^
                                     rotl8(b, 3) ^ rotl8(b, 4) ^ 0x63);
}

struct sboxes {
    std::array<std::uint8_t, 256> sbox;
    std::array<std::uint8_t, 256> inv;
};

inline const sboxes &get_sboxes() {
    static const sboxes s = [] {
        sboxes out{};
        ca::gf256::Field field(0x11B);
        for (int b = 0; b < 256; b++) {
            std::uint8_t inv =
                (b == 0) ? 0 : field.inverse(static_cast<std::uint8_t>(b));
            out.sbox[static_cast<std::size_t>(b)] = affine_transform(inv);
        }
        for (int b = 0; b < 256; b++) {
            out.inv[out.sbox[static_cast<std::size_t>(b)]] =
                static_cast<std::uint8_t>(b);
        }
        return out;
    }();
    return s;
}

inline std::uint8_t xtime(std::uint8_t b) {
    std::uint8_t shifted = static_cast<std::uint8_t>(b << 1);
    return (b & 0x80) ? static_cast<std::uint8_t>(shifted ^ 0x1B) : shifted;
}
inline std::uint8_t gmul(std::uint8_t a, std::uint8_t b) {
    std::uint8_t result = 0, aa = a, bb = b;
    for (int i = 0; i < 8; i++) {
        if (bb & 1) {
            result = static_cast<std::uint8_t>(result ^ aa);
        }
        std::uint8_t hi = static_cast<std::uint8_t>(aa & 0x80);
        aa = static_cast<std::uint8_t>(aa << 1);
        if (hi) {
            aa = static_cast<std::uint8_t>(aa ^ 0x1B);
        }
        bb = static_cast<std::uint8_t>(bb >> 1);
    }
    return result;
}

// Round keys packed as round_keys[rk][row][col].
using round_keys_t = std::array<std::array<std::array<std::uint8_t, 4>, 4>, 15>;

inline bool expand_key(const std::vector<std::uint8_t> &key, round_keys_t &rk,
                       int &nr_out) {
    std::size_t key_len = key.size();
    if (key_len != 16 && key_len != 24 && key_len != 32) {
        return false;
    }
    const sboxes &sb = get_sboxes();
    std::size_t nk = key_len / 4;
    int nr = (nk == 4) ? 10 : (nk == 6) ? 12 : 14;
    std::size_t total = 4 * (static_cast<std::size_t>(nr) + 1);
    std::array<std::array<std::uint8_t, 4>, 60> w{};
    for (std::size_t i = 0; i < nk; i++) {
        for (int j = 0; j < 4; j++) {
            w[i][static_cast<std::size_t>(j)] = key[4 * i + static_cast<std::size_t>(j)];
        }
    }
    for (std::size_t i = nk; i < total; i++) {
        std::array<std::uint8_t, 4> temp = w[i - 1];
        if (i % nk == 0) {
            std::uint8_t t0 = temp[0];
            temp[0] = temp[1];
            temp[1] = temp[2];
            temp[2] = temp[3];
            temp[3] = t0;
            for (int j = 0; j < 4; j++) {
                temp[static_cast<std::size_t>(j)] =
                    sb.sbox[temp[static_cast<std::size_t>(j)]];
            }
            temp[0] ^= RCON[i / nk];
        } else if (nk == 8 && i % nk == 4) {
            for (int j = 0; j < 4; j++) {
                temp[static_cast<std::size_t>(j)] =
                    sb.sbox[temp[static_cast<std::size_t>(j)]];
            }
        }
        for (int j = 0; j < 4; j++) {
            w[i][static_cast<std::size_t>(j)] = static_cast<std::uint8_t>(
                w[i - nk][static_cast<std::size_t>(j)] ^
                temp[static_cast<std::size_t>(j)]);
        }
    }
    for (int r = 0; r <= nr; r++) {
        for (int col = 0; col < 4; col++) {
            for (int row = 0; row < 4; row++) {
                rk[static_cast<std::size_t>(r)][static_cast<std::size_t>(row)]
                  [static_cast<std::size_t>(col)] =
                    w[static_cast<std::size_t>(4 * r + col)]
                     [static_cast<std::size_t>(row)];
            }
        }
    }
    nr_out = nr;
    return true;
}

using state_t = std::uint8_t[4][4];

inline void bytes_to_state(const block_t &block, state_t s) {
    for (int col = 0; col < 4; col++) {
        for (int row = 0; row < 4; row++) {
            s[row][col] = block[static_cast<std::size_t>(row + 4 * col)];
        }
    }
}
inline block_t state_to_bytes(const state_t s) {
    block_t out{};
    for (int col = 0; col < 4; col++) {
        for (int row = 0; row < 4; row++) {
            out[static_cast<std::size_t>(row + 4 * col)] = s[row][col];
        }
    }
    return out;
}
inline void add_round_key(state_t s,
                          const std::array<std::array<std::uint8_t, 4>, 4> &rk) {
    for (int r = 0; r < 4; r++) {
        for (int c = 0; c < 4; c++) {
            s[r][c] = static_cast<std::uint8_t>(
                s[r][c] ^ rk[static_cast<std::size_t>(r)][static_cast<std::size_t>(c)]);
        }
    }
}
inline void sub_bytes(state_t s, bool inverse) {
    const sboxes &sb = get_sboxes();
    for (int r = 0; r < 4; r++) {
        for (int c = 0; c < 4; c++) {
            s[r][c] = inverse ? sb.inv[s[r][c]] : sb.sbox[s[r][c]];
        }
    }
}
inline void shift_rows(state_t s, bool inverse) {
    std::uint8_t t[4][4];
    for (int r = 0; r < 4; r++) {
        for (int c = 0; c < 4; c++) {
            int src = inverse ? (c + 4 - r) % 4 : (c + r) % 4;
            t[r][c] = s[r][src];
        }
    }
    std::memcpy(s, t, 16);
}
inline void mix_columns(state_t s) {
    std::uint8_t t[4][4];
    for (int col = 0; col < 4; col++) {
        std::uint8_t s0 = s[0][col], s1 = s[1][col], s2 = s[2][col],
                     s3 = s[3][col];
        t[0][col] = static_cast<std::uint8_t>(xtime(s0) ^ (xtime(s1) ^ s1) ^ s2 ^ s3);
        t[1][col] = static_cast<std::uint8_t>(s0 ^ xtime(s1) ^ (xtime(s2) ^ s2) ^ s3);
        t[2][col] = static_cast<std::uint8_t>(s0 ^ s1 ^ xtime(s2) ^ (xtime(s3) ^ s3));
        t[3][col] = static_cast<std::uint8_t>((xtime(s0) ^ s0) ^ s1 ^ s2 ^ xtime(s3));
    }
    std::memcpy(s, t, 16);
}
inline void inv_mix_columns(state_t s) {
    std::uint8_t t[4][4];
    for (int col = 0; col < 4; col++) {
        std::uint8_t s0 = s[0][col], s1 = s[1][col], s2 = s[2][col],
                     s3 = s[3][col];
        t[0][col] = static_cast<std::uint8_t>(gmul(0x0e, s0) ^ gmul(0x0b, s1) ^
                                              gmul(0x0d, s2) ^ gmul(0x09, s3));
        t[1][col] = static_cast<std::uint8_t>(gmul(0x09, s0) ^ gmul(0x0e, s1) ^
                                              gmul(0x0b, s2) ^ gmul(0x0d, s3));
        t[2][col] = static_cast<std::uint8_t>(gmul(0x0d, s0) ^ gmul(0x09, s1) ^
                                              gmul(0x0e, s2) ^ gmul(0x0b, s3));
        t[3][col] = static_cast<std::uint8_t>(gmul(0x0b, s0) ^ gmul(0x0d, s1) ^
                                              gmul(0x09, s2) ^ gmul(0x0e, s3));
    }
    std::memcpy(s, t, 16);
}

} // namespace detail

inline const std::array<std::uint8_t, 256> &sbox() {
    return detail::get_sboxes().sbox;
}
inline const std::array<std::uint8_t, 256> &inv_sbox() {
    return detail::get_sboxes().inv;
}

// Encrypt one 16-byte block under `key` (16/24/32 bytes). nullopt on bad key.
inline std::optional<block_t> encrypt_block(const block_t &block,
                                            const std::vector<std::uint8_t> &key) {
    detail::round_keys_t rk;
    int nr;
    if (!detail::expand_key(key, rk, nr)) {
        return std::nullopt;
    }
    detail::state_t state;
    detail::bytes_to_state(block, state);
    detail::add_round_key(state, rk[0]);
    for (int rnd = 1; rnd < nr; rnd++) {
        detail::sub_bytes(state, false);
        detail::shift_rows(state, false);
        detail::mix_columns(state);
        detail::add_round_key(state, rk[static_cast<std::size_t>(rnd)]);
    }
    detail::sub_bytes(state, false);
    detail::shift_rows(state, false);
    detail::add_round_key(state, rk[static_cast<std::size_t>(nr)]);
    return detail::state_to_bytes(state);
}

// Decrypt one 16-byte block. nullopt on bad key.
inline std::optional<block_t> decrypt_block(const block_t &block,
                                            const std::vector<std::uint8_t> &key) {
    detail::round_keys_t rk;
    int nr;
    if (!detail::expand_key(key, rk, nr)) {
        return std::nullopt;
    }
    detail::state_t state;
    detail::bytes_to_state(block, state);
    detail::add_round_key(state, rk[static_cast<std::size_t>(nr)]);
    for (int rnd = nr - 1; rnd >= 1; rnd--) {
        detail::shift_rows(state, true);
        detail::sub_bytes(state, true);
        detail::add_round_key(state, rk[static_cast<std::size_t>(rnd)]);
        detail::inv_mix_columns(state);
    }
    detail::shift_rows(state, true);
    detail::sub_bytes(state, true);
    detail::add_round_key(state, rk[0]);
    return detail::state_to_bytes(state);
}

} // namespace aes
} // namespace ca

#endif // AES_HPP
