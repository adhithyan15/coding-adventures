// scrypt.hpp — scrypt, the sequential memory-hard password-based key derivation
// function (RFC 7914), in pure ISO C++17 (header-only), in namespace ca. A
// faithful port of the Rust `scrypt` crate.
// ===========================================================================
//
// PBKDF2 and bcrypt can be parallelised cheaply on GPUs / FPGAs. scrypt adds
// *memory hardness*: it deliberately allocates a large random-access working set
// (N * 128 * r bytes) and reads it in a data-dependent order, so an attacker
// cannot trade memory for speed.
//
//   scrypt(P, S, N, r, p, dkLen):
//     1. B  = PBKDF2-HMAC-SHA256(P, S, 1, p*128*r)   -- expand into p blocks
//     2. B[i] = ROMix(B[i], N)   for each 128*r-byte block   -- memory-hard
//     3. DK = PBKDF2-HMAC-SHA256(P, B, 1, dkLen)     -- extract the key
//
// ROMix fills a table V of N snapshots (BlockMix run N times), then does N more
// BlockMix steps each XORing in a data-chosen V entry (integerify % N). BlockMix
// mixes 2r 64-byte blocks with the Salsa20/8 core.
//
// Parameters: N (CPU/memory cost, a power of two >= 2, <= 2^20), r (block-size
// multiplier >= 1), p (parallelisation >= 1), dk_len (output length, <= 2^20).
// Memory: N*128*r bytes. Built on the sibling header-only `pbkdf2` package.
//
// Invalid parameters throw std::invalid_argument. If the N*128*r working set
// cannot be allocated, the underlying std::vector throws std::bad_alloc (the
// idiomatic C++ analogue of the C port's SCRYPT_ALLOC_ERROR).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_SCRYPT_HPP
#define CA_SCRYPT_HPP

#include <cstddef>
#include <cstdint>
#include <cstring>
#include <limits>
#include <stdexcept>
#include <string>
#include <vector>

#include "pbkdf2.hpp"  // sibling header-only package (include path via run.sh)

namespace ca {

inline constexpr std::size_t scrypt_max_n = std::size_t(1) << 20;
inline constexpr std::size_t scrypt_max_dk_len = std::size_t(1) << 20;

namespace detail {

inline std::uint32_t scrypt_rotl32(std::uint32_t v, unsigned c) {
    return (v << c) | (v >> (32 - c));
}

// Salsa20/8 core over a 64-byte block. `in` and `out` must not alias.
inline void salsa20_8(const std::uint8_t in[64], std::uint8_t out[64]) {
    std::uint32_t x[16];
    std::uint32_t z[16];
    for (int i = 0; i < 16; ++i) {
        x[i] = static_cast<std::uint32_t>(in[i * 4]) |
               (static_cast<std::uint32_t>(in[i * 4 + 1]) << 8) |
               (static_cast<std::uint32_t>(in[i * 4 + 2]) << 16) |
               (static_cast<std::uint32_t>(in[i * 4 + 3]) << 24);
        z[i] = x[i];
    }
    auto qr = [&x](int a, int b, int c, int d) {
        x[b] ^= scrypt_rotl32(x[a] + x[d], 7);
        x[c] ^= scrypt_rotl32(x[b] + x[a], 9);
        x[d] ^= scrypt_rotl32(x[c] + x[b], 13);
        x[a] ^= scrypt_rotl32(x[d] + x[c], 18);
    };
    for (int round = 0; round < 4; ++round) {
        qr(0, 4, 8, 12);
        qr(5, 9, 13, 1);
        qr(10, 14, 2, 6);
        qr(15, 3, 7, 11);
        qr(0, 1, 2, 3);
        qr(5, 6, 7, 4);
        qr(10, 11, 8, 9);
        qr(15, 12, 13, 14);
    }
    for (int i = 0; i < 16; ++i) {
        std::uint32_t v = x[i] + z[i];  // uint32 wraps (well-defined)
        out[i * 4] = static_cast<std::uint8_t>(v & 0xFF);
        out[i * 4 + 1] = static_cast<std::uint8_t>((v >> 8) & 0xFF);
        out[i * 4 + 2] = static_cast<std::uint8_t>((v >> 16) & 0xFF);
        out[i * 4 + 3] = static_cast<std::uint8_t>((v >> 24) & 0xFF);
    }
}

// BlockMix over 2r 64-byte blocks. `in` and `out` are each 128*r bytes and must
// not alias; output uses the RFC 7914 even-then-odd interleaving.
inline void block_mix(const std::vector<std::uint8_t>& in,
                      std::vector<std::uint8_t>& out, std::size_t r) {
    std::size_t two_r = 2 * r;
    std::uint8_t x[64];
    std::uint8_t xored[64];
    std::memcpy(x, in.data() + (two_r - 1) * 64, 64);
    for (std::size_t i = 0; i < two_r; ++i) {
        for (std::size_t k = 0; k < 64; ++k) {
            xored[k] = static_cast<std::uint8_t>(x[k] ^ in[i * 64 + k]);
        }
        salsa20_8(xored, x);
        std::size_t dst = (i % 2 == 0) ? (i / 2) : (r + i / 2);
        std::memcpy(out.data() + dst * 64, x, 64);
    }
}

// integerify: little-endian integer from the last 64-byte block (low 8 bytes).
inline std::uint64_t integerify(const std::vector<std::uint8_t>& blocks,
                                std::size_t r) {
    const std::uint8_t* last = blocks.data() + (2 * r - 1) * 64;
    std::uint64_t v = 0;
    for (int i = 0; i < 8; ++i) {
        v |= static_cast<std::uint64_t>(last[i]) << (8 * i);
    }
    return v;
}

// ROMix on a 128*r-byte block, in place.
inline void ro_mix(std::vector<std::uint8_t>& block, std::size_t n,
                   std::size_t r) {
    std::size_t block_len = 128 * r;
    std::vector<std::uint8_t> v(n * block_len);
    std::vector<std::uint8_t> x = block;
    std::vector<std::uint8_t> t(block_len);

    for (std::size_t i = 0; i < n; ++i) {
        std::memcpy(v.data() + i * block_len, x.data(), block_len);
        block_mix(x, t, r);
        x.swap(t);
    }
    for (std::size_t i = 0; i < n; ++i) {
        std::size_t j = static_cast<std::size_t>(integerify(x, r) %
                                                 static_cast<std::uint64_t>(n));
        const std::uint8_t* vj = v.data() + j * block_len;
        for (std::size_t k = 0; k < block_len; ++k) {
            x[k] ^= vj[k];
        }
        block_mix(x, t, r);
        x.swap(t);
    }
    block = std::move(x);
}

}  // namespace detail

// scrypt — derive `dk_len` bytes from `password` and `salt`. `n` must be a power
// of two in [2, 2^20]; `r`, `p` >= 1. Throws std::invalid_argument on any
// invalid parameter. An empty password/salt is permitted (RFC 7914 vector 1).
inline std::vector<std::uint8_t> scrypt(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t n, std::size_t r,
    std::size_t p, std::size_t dk_len) {
    if (n > scrypt_max_n) {
        throw std::invalid_argument("scrypt: N must not exceed 2^20");
    }
    if (n < 2 || (n & (n - 1)) != 0) {
        throw std::invalid_argument("scrypt: N must be a power of two >= 2");
    }
    if (r == 0) {
        throw std::invalid_argument("scrypt: r must be positive");
    }
    if (p == 0) {
        throw std::invalid_argument("scrypt: p must be positive");
    }
    if (dk_len == 0) {
        throw std::invalid_argument("scrypt: dk_len must be positive");
    }
    if (dk_len > scrypt_max_dk_len) {
        throw std::invalid_argument("scrypt: dk_len must not exceed 2^20");
    }
    // p*r <= 2^30 (r >= 1 here, so `p > 2^30/r` is exactly `p*r > 2^30`).
    if (p > (std::size_t(1) << 30) / r) {
        throw std::invalid_argument("scrypt: p*r must not exceed 2^30");
    }
    std::size_t pr = p * r;
    if (pr > (std::size_t(1) << 30) / 128) {
        throw std::invalid_argument("scrypt: p*128*r must not exceed 2^30");
    }
    std::size_t b_len = 128 * pr;

    // Guard the ROMix working-set size N*128*r against size_t overflow. The C
    // port gets this free from calloc's checked multiply; here we must check
    // explicitly, or a 32-bit wrap would under-allocate the V table and corrupt
    // the heap. (128*r == b_len/p <= 2^30 already cannot overflow.)
    std::size_t block_len = 128 * r;
    if (block_len != 0 &&
        n > (std::numeric_limits<std::size_t>::max)() / block_len) {
        throw std::invalid_argument("scrypt: N*128*r working set exceeds size_t");
    }

    // Step 1: expand the password into B (p blocks of 128*r bytes).
    std::vector<std::uint8_t> b =
        pbkdf2_hmac_sha256(password, salt, 1, b_len, /*allow_empty=*/true);

    // Step 2: ROMix each 128*r-byte block independently.
    for (std::size_t i = 0; i < p; ++i) {
        std::vector<std::uint8_t> chunk(b.begin() + static_cast<std::ptrdiff_t>(
                                                        i * 128 * r),
                                        b.begin() + static_cast<std::ptrdiff_t>(
                                                        (i + 1) * 128 * r));
        detail::ro_mix(chunk, n, r);
        std::memcpy(b.data() + i * 128 * r, chunk.data(), 128 * r);
    }

    // Step 3: extract the final key (salt = B).
    return pbkdf2_hmac_sha256(password, b, 1, dk_len, /*allow_empty=*/true);
}

// scrypt_hex — like scrypt but returns a lowercase hex string.
inline std::string scrypt_hex(const std::vector<std::uint8_t>& password,
                              const std::vector<std::uint8_t>& salt,
                              std::size_t n, std::size_t r, std::size_t p,
                              std::size_t dk_len) {
    return to_hex(scrypt(password, salt, n, r, p, dk_len));
}

}  // namespace ca

#endif  // CA_SCRYPT_HPP
