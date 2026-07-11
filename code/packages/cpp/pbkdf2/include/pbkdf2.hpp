// pbkdf2.hpp — PBKDF2, Password-Based Key Derivation Function 2 (RFC 8018 §
// 5.2), in pure ISO C++17 (header-only), in namespace ca. A faithful port of the
// Rust `pbkdf2` crate.
// ===========================================================================
//
// PBKDF2 stretches a password into a cryptographic key by applying a
// pseudorandom function (here HMAC) `iterations` times per output block. The
// iteration count is the tunable cost: every brute-force guess pays the same
// price, so a large count slows attackers down.
//
//   DK   = T_1 || T_2 || ... || T_n            (first key_length bytes)
//   T_i  = U_1 XOR U_2 XOR ... XOR U_c
//   U_1  = PRF(Password, Salt || INT_32_BE(i))
//   U_j  = PRF(Password, U_{j-1})              for j = 2..c
//
// The block index `i` is appended to the salt as a 4-byte big-endian integer,
// making each block's first U value distinct.
//
// Hash-agnostic core: pass a hash callable `std::vector<uint8_t>(const
// std::vector<uint8_t>&)` plus its block and digest sizes; convenience wrappers
// fix HMAC-SHA1 / -SHA256 / -SHA512. Built on the sibling header-only `hmac` and
// `sha*` packages.
//
// Validation failures throw std::invalid_argument (empty password, zero
// iterations, zero / oversized key length).
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. Standard library only.
#ifndef CA_PBKDF2_HPP
#define CA_PBKDF2_HPP

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <string>
#include <vector>

#include "hmac.hpp"    // sibling header-only package (include path via run.sh)
#include "sha1.hpp"
#include "sha256.hpp"
#include "sha512.hpp"

namespace ca {

// The practical maximum key length (1 MiB): bounds memory and keeps the block
// counter within 32 bits, matching the Rust crate.
inline constexpr std::size_t pbkdf2_max_key_length = std::size_t(1) << 20;

// pbkdf2 — the generic core. `hash` produces `digest_size` bytes; the PRF is
// HMAC(hash, block_size). Derives `key_length` bytes. An empty password throws
// unless `allow_empty_password` is true.
template <typename HashFn>
std::vector<std::uint8_t> pbkdf2(HashFn hash, std::size_t digest_size,
                                 std::size_t block_size,
                                 const std::vector<std::uint8_t>& password,
                                 const std::vector<std::uint8_t>& salt,
                                 std::size_t iterations, std::size_t key_length,
                                 bool allow_empty_password = false) {
    if (digest_size == 0) {
        throw std::invalid_argument("pbkdf2: digest_size must be positive");
    }
    if (password.empty() && !allow_empty_password) {
        throw std::invalid_argument("pbkdf2: password must not be empty");
    }
    if (iterations == 0) {
        throw std::invalid_argument("pbkdf2: iterations must be positive");
    }
    if (key_length == 0) {
        throw std::invalid_argument("pbkdf2: key_length must be positive");
    }
    if (key_length > pbkdf2_max_key_length) {
        throw std::invalid_argument(
            "pbkdf2: key_length must not exceed 2^20 (1 MiB)");
    }

    std::size_t num_blocks = (key_length + digest_size - 1) / digest_size;
    std::vector<std::uint8_t> dk;
    dk.reserve(num_blocks * digest_size);

    for (std::uint32_t i = 1; i <= static_cast<std::uint32_t>(num_blocks); ++i) {
        // Seed = Salt || INT_32_BE(i).
        std::vector<std::uint8_t> seed(salt);
        seed.push_back(static_cast<std::uint8_t>((i >> 24) & 0xFF));
        seed.push_back(static_cast<std::uint8_t>((i >> 16) & 0xFF));
        seed.push_back(static_cast<std::uint8_t>((i >> 8) & 0xFF));
        seed.push_back(static_cast<std::uint8_t>(i & 0xFF));

        // U_1 = PRF(Password, Seed); T = U_1; prev = U_1.
        std::vector<std::uint8_t> u = hmac(hash, block_size, password, seed);
        std::vector<std::uint8_t> t = u;
        std::vector<std::uint8_t> prev = std::move(u);

        // U_j = PRF(Password, U_{j-1}); T ^= U_j, for j = 2..iterations.
        for (std::size_t j = 1; j < iterations; ++j) {
            std::vector<std::uint8_t> next = hmac(hash, block_size, password,
                                                  prev);
            for (std::size_t k = 0; k < t.size() && k < next.size(); ++k) {
                t[k] ^= next[k];
            }
            prev = std::move(next);
        }

        dk.insert(dk.end(), t.begin(), t.end());
    }

    dk.resize(key_length);  // truncate the final block to key_length
    return dk;
}

// The SHA family wrapped as `std::vector<uint8_t>(const std::vector<uint8_t>&)`
// callables for the HMAC PRF.
namespace detail {
inline std::vector<std::uint8_t> sha1_vec(const std::vector<std::uint8_t>& d) {
    sha1_digest h = ca::sha1(d.data(), d.size());
    return std::vector<std::uint8_t>(h.begin(), h.end());
}
inline std::vector<std::uint8_t> sha256_vec(const std::vector<std::uint8_t>& d) {
    sha256_digest h = ca::sha256(d.data(), d.size());
    return std::vector<std::uint8_t>(h.begin(), h.end());
}
inline std::vector<std::uint8_t> sha512_vec(const std::vector<std::uint8_t>& d) {
    sha512_digest h = ca::sha512(d.data(), d.size());
    return std::vector<std::uint8_t>(h.begin(), h.end());
}
}  // namespace detail

inline std::vector<std::uint8_t> pbkdf2_hmac_sha1(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t iterations,
    std::size_t key_length, bool allow_empty_password = false) {
    return pbkdf2(detail::sha1_vec, 20, 64, password, salt, iterations,
                  key_length, allow_empty_password);
}

inline std::vector<std::uint8_t> pbkdf2_hmac_sha256(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t iterations,
    std::size_t key_length, bool allow_empty_password = false) {
    return pbkdf2(detail::sha256_vec, 32, 64, password, salt, iterations,
                  key_length, allow_empty_password);
}

inline std::vector<std::uint8_t> pbkdf2_hmac_sha512(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t iterations,
    std::size_t key_length, bool allow_empty_password = false) {
    return pbkdf2(detail::sha512_vec, 64, 128, password, salt, iterations,
                  key_length, allow_empty_password);
}

// Lowercase-hex convenience wrappers.
inline std::string to_hex(const std::vector<std::uint8_t>& bytes) {
    static const char* digits = "0123456789abcdef";
    std::string out;
    out.reserve(bytes.size() * 2);
    for (std::uint8_t b : bytes) {
        out.push_back(digits[b >> 4]);
        out.push_back(digits[b & 0x0F]);
    }
    return out;
}

inline std::string pbkdf2_hmac_sha1_hex(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t iterations,
    std::size_t key_length, bool allow_empty_password = false) {
    return to_hex(pbkdf2_hmac_sha1(password, salt, iterations, key_length,
                                   allow_empty_password));
}

inline std::string pbkdf2_hmac_sha256_hex(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t iterations,
    std::size_t key_length, bool allow_empty_password = false) {
    return to_hex(pbkdf2_hmac_sha256(password, salt, iterations, key_length,
                                     allow_empty_password));
}

inline std::string pbkdf2_hmac_sha512_hex(
    const std::vector<std::uint8_t>& password,
    const std::vector<std::uint8_t>& salt, std::size_t iterations,
    std::size_t key_length, bool allow_empty_password = false) {
    return to_hex(pbkdf2_hmac_sha512(password, salt, iterations, key_length,
                                     allow_empty_password));
}

}  // namespace ca

#endif  // CA_PBKDF2_HPP
