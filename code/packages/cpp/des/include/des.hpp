// des.hpp — the DES block cipher (FIPS 46) and Triple DES (NIST SP 800-67), in
// pure ISO C++17 (header-only). A faithful port of the Rust `des` crate, in
// namespace `ca::des`.
// ===========================================================================
//
// DES encrypts a 64-bit block under a 64-bit key (56 key bits + 8 parity bits)
// through a 16-round Feistel network — the archetypal block cipher. Output
// matches the FIPS 46 and NIST SP 800-20 known-answer test vectors.
//
//   ca::des::encrypt_block / decrypt_block   — the raw 8-byte block cipher
//   ca::des::ecb_encrypt / ecb_decrypt       — ECB mode with PKCS#7 padding
//   ca::des::tdea_encrypt_block / tdea_decrypt_block — Triple DES (EDE)
//
// Security note: DES and 3DES are cryptographically broken for modern use. This
// is a faithful implementation for study and legacy interop, not a
// recommendation.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef DES_HPP
#define DES_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <optional>
#include <utility>
#include <vector>

namespace ca {
namespace des {

using block_t = std::array<std::uint8_t, 8>;

namespace detail {

inline constexpr std::uint8_t IP[64] = {
    57, 49, 41, 33, 25, 17, 9,  1,  59, 51, 43, 35, 27, 19, 11, 3,
    61, 53, 45, 37, 29, 21, 13, 5,  63, 55, 47, 39, 31, 23, 15, 7,
    56, 48, 40, 32, 24, 16, 8,  0,  58, 50, 42, 34, 26, 18, 10, 2,
    60, 52, 44, 36, 28, 20, 12, 4,  62, 54, 46, 38, 30, 22, 14, 6};

inline constexpr std::uint8_t FP[64] = {
    39, 7, 47, 15, 55, 23, 63, 31, 38, 6, 46, 14, 54, 22, 62, 30,
    37, 5, 45, 13, 53, 21, 61, 29, 36, 4, 44, 12, 52, 20, 60, 28,
    35, 3, 43, 11, 51, 19, 59, 27, 34, 2, 42, 10, 50, 18, 58, 26,
    33, 1, 41, 9,  49, 17, 57, 25, 32, 0, 40, 8,  48, 16, 56, 24};

inline constexpr std::uint8_t PC1[56] = {
    56, 48, 40, 32, 24, 16, 8,  0,  57, 49, 41, 33, 25, 17,
    9,  1,  58, 50, 42, 34, 26, 18, 10, 2,  59, 51, 43, 35,
    62, 54, 46, 38, 30, 22, 14, 6,  61, 53, 45, 37, 29, 21,
    13, 5,  60, 52, 44, 36, 28, 20, 12, 4,  27, 19, 11, 3};

inline constexpr std::uint8_t PC2[48] = {
    13, 16, 10, 23, 0,  4,  2,  27, 14, 5,  20, 9,  22, 18, 11, 3,
    25, 7,  15, 6,  26, 19, 12, 1,  40, 51, 30, 36, 46, 54, 29, 39,
    50, 44, 32, 47, 43, 48, 38, 55, 33, 52, 45, 41, 49, 35, 28, 31};

inline constexpr std::uint8_t E[48] = {
    31, 0,  1,  2,  3,  4,  3,  4,  5,  6,  7,  8,  7,  8,  9,  10,
    11, 12, 11, 12, 13, 14, 15, 16, 15, 16, 17, 18, 19, 20, 19, 20,
    21, 22, 23, 24, 23, 24, 25, 26, 27, 28, 27, 28, 29, 30, 31, 0};

inline constexpr std::uint8_t P[32] = {15, 6,  19, 20, 28, 11, 27, 16,
                                       0,  14, 22, 25, 4,  17, 30, 9,
                                       1,  7,  23, 13, 31, 26, 2,  8,
                                       18, 12, 29, 5,  21, 10, 3,  24};

inline constexpr std::uint8_t SHIFTS[16] = {1, 1, 2, 2, 2, 2, 2, 2,
                                            1, 2, 2, 2, 2, 2, 2, 1};

inline constexpr std::uint8_t SBOXES[8][4][16] = {
    {{14, 4, 13, 1, 2, 15, 11, 8, 3, 10, 6, 12, 5, 9, 0, 7},
     {0, 15, 7, 4, 14, 2, 13, 1, 10, 6, 12, 11, 9, 5, 3, 8},
     {4, 1, 14, 8, 13, 6, 2, 11, 15, 12, 9, 7, 3, 10, 5, 0},
     {15, 12, 8, 2, 4, 9, 1, 7, 5, 11, 3, 14, 10, 0, 6, 13}},
    {{15, 1, 8, 14, 6, 11, 3, 4, 9, 7, 2, 13, 12, 0, 5, 10},
     {3, 13, 4, 7, 15, 2, 8, 14, 12, 0, 1, 10, 6, 9, 11, 5},
     {0, 14, 7, 11, 10, 4, 13, 1, 5, 8, 12, 6, 9, 3, 2, 15},
     {13, 8, 10, 1, 3, 15, 4, 2, 11, 6, 7, 12, 0, 5, 14, 9}},
    {{10, 0, 9, 14, 6, 3, 15, 5, 1, 13, 12, 7, 11, 4, 2, 8},
     {13, 7, 0, 9, 3, 4, 6, 10, 2, 8, 5, 14, 12, 11, 15, 1},
     {13, 6, 4, 9, 8, 15, 3, 0, 11, 1, 2, 12, 5, 10, 14, 7},
     {1, 10, 13, 0, 6, 9, 8, 7, 4, 15, 14, 3, 11, 5, 2, 12}},
    {{7, 13, 14, 3, 0, 6, 9, 10, 1, 2, 8, 5, 11, 12, 4, 15},
     {13, 8, 11, 5, 6, 15, 0, 3, 4, 7, 2, 12, 1, 10, 14, 9},
     {10, 6, 9, 0, 12, 11, 7, 13, 15, 1, 3, 14, 5, 2, 8, 4},
     {3, 15, 0, 6, 10, 1, 13, 8, 9, 4, 5, 11, 12, 7, 2, 14}},
    {{2, 12, 4, 1, 7, 10, 11, 6, 8, 5, 3, 15, 13, 0, 14, 9},
     {14, 11, 2, 12, 4, 7, 13, 1, 5, 0, 15, 10, 3, 9, 8, 6},
     {4, 2, 1, 11, 10, 13, 7, 8, 15, 9, 12, 5, 6, 3, 0, 14},
     {11, 8, 12, 7, 1, 14, 2, 13, 6, 15, 0, 9, 10, 4, 5, 3}},
    {{12, 1, 10, 15, 9, 2, 6, 8, 0, 13, 3, 4, 14, 7, 5, 11},
     {10, 15, 4, 2, 7, 12, 9, 5, 6, 1, 13, 14, 0, 11, 3, 8},
     {9, 14, 15, 5, 2, 8, 12, 3, 7, 0, 4, 10, 1, 13, 11, 6},
     {4, 3, 2, 12, 9, 5, 15, 10, 11, 14, 1, 7, 6, 0, 8, 13}},
    {{4, 11, 2, 14, 15, 0, 8, 13, 3, 12, 9, 7, 5, 10, 6, 1},
     {13, 0, 11, 7, 4, 9, 1, 10, 14, 3, 5, 12, 2, 15, 8, 6},
     {1, 4, 11, 13, 12, 3, 7, 14, 10, 15, 6, 8, 0, 5, 9, 2},
     {6, 11, 13, 8, 1, 4, 10, 7, 9, 5, 0, 15, 14, 2, 3, 12}},
    {{13, 2, 8, 4, 6, 15, 11, 1, 10, 9, 3, 14, 5, 0, 12, 7},
     {1, 15, 13, 8, 10, 3, 7, 4, 12, 5, 6, 11, 0, 14, 9, 2},
     {7, 11, 4, 1, 9, 12, 14, 2, 0, 6, 10, 13, 15, 3, 5, 8},
     {2, 1, 14, 7, 4, 10, 8, 13, 15, 12, 9, 0, 3, 5, 6, 11}}};

inline void bytes_to_bits(const std::uint8_t *data, std::size_t nbytes,
                          std::uint8_t *bits) {
    for (std::size_t b = 0; b < nbytes; b++) {
        for (std::size_t k = 0; k < 8; k++) {
            bits[b * 8 + k] = static_cast<std::uint8_t>((data[b] >> (7 - k)) & 1);
        }
    }
}
inline void bits_to_bytes(const std::uint8_t *bits, std::size_t nbits,
                          std::uint8_t *bytes) {
    for (std::size_t b = 0; b < nbits / 8; b++) {
        std::uint8_t byte = 0;
        for (std::size_t k = 0; k < 8; k++) {
            byte = static_cast<std::uint8_t>((byte << 1) | bits[b * 8 + k]);
        }
        bytes[b] = byte;
    }
}
inline void permute(const std::uint8_t *in, const std::uint8_t *table,
                    std::size_t tlen, std::uint8_t *out) {
    for (std::size_t i = 0; i < tlen; i++) {
        out[i] = in[table[i]];
    }
}
inline void left_rotate_28(std::uint8_t half[28], unsigned n) {
    std::uint8_t tmp[28];
    for (unsigned i = 0; i < 28; i++) {
        tmp[i] = half[(i + n) % 28];
    }
    for (unsigned i = 0; i < 28; i++) {
        half[i] = tmp[i];
    }
}

using subkeys_t = std::array<std::array<std::uint8_t, 6>, 16>;

inline subkeys_t expand_key(const block_t &key) {
    std::uint8_t key_bits[64], permuted[56], c[28], d[28];
    bytes_to_bits(key.data(), 8, key_bits);
    permute(key_bits, PC1, 56, permuted);
    for (unsigned i = 0; i < 28; i++) {
        c[i] = permuted[i];
        d[i] = permuted[28 + i];
    }
    subkeys_t subkeys{};
    for (unsigned i = 0; i < 16; i++) {
        std::uint8_t cd[56], subkey_bits[48];
        left_rotate_28(c, SHIFTS[i]);
        left_rotate_28(d, SHIFTS[i]);
        for (unsigned j = 0; j < 28; j++) {
            cd[j] = c[j];
            cd[28 + j] = d[j];
        }
        permute(cd, PC2, 48, subkey_bits);
        bits_to_bytes(subkey_bits, 48, subkeys[i].data());
    }
    return subkeys;
}

inline void feistel_f(const std::uint8_t right[32], const std::uint8_t subkey[6],
                      std::uint8_t out[32]) {
    std::uint8_t expanded[48], sk_bits[48], xored[48], sbox_out[32];
    permute(right, E, 48, expanded);
    bytes_to_bits(subkey, 6, sk_bits);
    for (unsigned i = 0; i < 48; i++) {
        xored[i] = static_cast<std::uint8_t>(expanded[i] ^ sk_bits[i]);
    }
    for (unsigned box = 0; box < 8; box++) {
        const std::uint8_t *chunk = &xored[box * 6];
        unsigned row = static_cast<unsigned>((chunk[0] << 1) | chunk[5]);
        unsigned col = static_cast<unsigned>((chunk[1] << 3) | (chunk[2] << 2) |
                                             (chunk[3] << 1) | chunk[4]);
        std::uint8_t val = SBOXES[box][row][col];
        for (unsigned k = 0; k < 4; k++) {
            sbox_out[box * 4 + k] =
                static_cast<std::uint8_t>((val >> (3 - k)) & 1);
        }
    }
    permute(sbox_out, P, 32, out);
}

inline block_t des_block(const block_t &block, const subkeys_t &subkeys) {
    std::uint8_t bits[64], perm[64], left[32], right[32];
    bytes_to_bits(block.data(), 8, bits);
    permute(bits, IP, 64, perm);
    for (unsigned i = 0; i < 32; i++) {
        left[i] = perm[i];
        right[i] = perm[32 + i];
    }
    for (unsigned r = 0; r < 16; r++) {
        std::uint8_t f_out[32], new_right[32];
        feistel_f(right, subkeys[r].data(), f_out);
        for (unsigned i = 0; i < 32; i++) {
            new_right[i] = static_cast<std::uint8_t>(left[i] ^ f_out[i]);
        }
        for (unsigned i = 0; i < 32; i++) {
            left[i] = right[i];
            right[i] = new_right[i];
        }
    }
    for (unsigned i = 0; i < 32; i++) {
        perm[i] = right[i];
        perm[32 + i] = left[i];
    }
    std::uint8_t result_bits[64];
    permute(perm, FP, 64, result_bits);
    block_t out{};
    bits_to_bytes(result_bits, 64, out.data());
    return out;
}

inline subkeys_t reversed(subkeys_t subkeys) {
    for (unsigned i = 0; i < 8; i++) {
        std::swap(subkeys[i], subkeys[15 - i]);
    }
    return subkeys;
}

} // namespace detail

// Encrypt one 8-byte block under `key`.
inline block_t encrypt_block(const block_t &block, const block_t &key) {
    return detail::des_block(block, detail::expand_key(key));
}

// Decrypt one 8-byte block (encryption with the subkeys reversed).
inline block_t decrypt_block(const block_t &block, const block_t &key) {
    return detail::des_block(block, detail::reversed(detail::expand_key(key)));
}

// ECB mode with PKCS#7 padding. Returns ciphertext (a multiple of 8 bytes).
inline std::vector<std::uint8_t> ecb_encrypt(
    const std::vector<std::uint8_t> &plaintext, const block_t &key) {
    detail::subkeys_t subkeys = detail::expand_key(key);
    std::size_t pad = 8 - (plaintext.size() % 8);
    std::vector<std::uint8_t> padded = plaintext;
    padded.insert(padded.end(), pad, static_cast<std::uint8_t>(pad));
    std::vector<std::uint8_t> out;
    out.reserve(padded.size());
    for (std::size_t off = 0; off < padded.size(); off += 8) {
        block_t blk{};
        for (unsigned i = 0; i < 8; i++) {
            blk[i] = padded[off + i];
        }
        block_t enc = detail::des_block(blk, subkeys);
        out.insert(out.end(), enc.begin(), enc.end());
    }
    return out;
}

// ECB decrypt + PKCS#7 unpad. std::nullopt on a bad length or bad padding.
inline std::optional<std::vector<std::uint8_t>> ecb_decrypt(
    const std::vector<std::uint8_t> &ciphertext, const block_t &key) {
    if (ciphertext.empty() || ciphertext.size() % 8 != 0) {
        return std::nullopt;
    }
    detail::subkeys_t subkeys = detail::reversed(detail::expand_key(key));
    std::vector<std::uint8_t> plain;
    plain.reserve(ciphertext.size());
    for (std::size_t off = 0; off < ciphertext.size(); off += 8) {
        block_t blk{};
        for (unsigned i = 0; i < 8; i++) {
            blk[i] = ciphertext[off + i];
        }
        block_t dec = detail::des_block(blk, subkeys);
        plain.insert(plain.end(), dec.begin(), dec.end());
    }
    std::size_t pad = plain.back();
    if (pad == 0 || pad > 8 || pad > plain.size()) {
        return std::nullopt;
    }
    for (std::size_t i = 0; i < pad; i++) {
        if (plain[plain.size() - 1 - i] != static_cast<std::uint8_t>(pad)) {
            return std::nullopt;
        }
    }
    plain.resize(plain.size() - pad);
    return plain;
}

// Triple DES (EDE): C = E_k1(D_k2(E_k3(P))).
inline block_t tdea_encrypt_block(const block_t &block, const block_t &k1,
                                  const block_t &k2, const block_t &k3) {
    return encrypt_block(decrypt_block(encrypt_block(block, k3), k2), k1);
}

// Triple DES (DED): P = D_k3(E_k2(D_k1(C))).
inline block_t tdea_decrypt_block(const block_t &block, const block_t &k1,
                                  const block_t &k2, const block_t &k3) {
    return decrypt_block(encrypt_block(decrypt_block(block, k1), k2), k3);
}

} // namespace des
} // namespace ca

#endif // DES_HPP
