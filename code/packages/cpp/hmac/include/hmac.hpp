// hmac.hpp — HMAC (keyed-hash message authentication, RFC 2104), in pure ISO
// C++17 (header-only). A faithful port of the Rust `hmac` crate's generic
// construction.
// ===========================================================================
//
//     K0    = H(key) if len(key) > B else key, right-padded with zeros to B
//     HMAC  = H( (K0 ^ opad) || H( (K0 ^ ipad) || message ) )
//
// with ipad = 0x36, opad = 0x5c. Hash-agnostic: pass a hash callable
// `std::vector<uint8_t>(const std::vector<uint8_t>&)` and the hash's block size
// B. The tests instantiate it with the sibling `sha256` package and the RFC 4231
// HMAC-SHA256 vectors.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef HMAC_HPP
#define HMAC_HPP

#include <cstddef>
#include <cstdint>
#include <vector>

namespace ca {

// hmac — HashFn is any callable taking and returning std::vector<std::uint8_t>
// (a one-shot hash). `block_size` is the hash's block size in bytes.
template <typename HashFn>
std::vector<std::uint8_t> hmac(HashFn hash, std::size_t block_size,
                               const std::vector<std::uint8_t> &key,
                               const std::vector<std::uint8_t> &message) {
    constexpr std::uint8_t ipad = 0x36, opad = 0x5c;

    // Normalize the key to exactly block_size bytes (hash if too long, then
    // zero-pad).
    std::vector<std::uint8_t> k0(block_size, 0);
    if (key.size() > block_size) {
        std::vector<std::uint8_t> hk = hash(key);
        for (std::size_t i = 0; i < hk.size() && i < block_size; i++) {
            k0[i] = hk[i];
        }
    } else {
        for (std::size_t i = 0; i < key.size(); i++) {
            k0[i] = key[i];
        }
    }

    // inner = H( (K0 ^ ipad) || message )
    std::vector<std::uint8_t> inner_in;
    inner_in.reserve(block_size + message.size());
    for (std::size_t i = 0; i < block_size; i++) {
        inner_in.push_back(static_cast<std::uint8_t>(k0[i] ^ ipad));
    }
    inner_in.insert(inner_in.end(), message.begin(), message.end());
    std::vector<std::uint8_t> inner = hash(inner_in);

    // HMAC = H( (K0 ^ opad) || inner )
    std::vector<std::uint8_t> outer_in;
    outer_in.reserve(block_size + inner.size());
    for (std::size_t i = 0; i < block_size; i++) {
        outer_in.push_back(static_cast<std::uint8_t>(k0[i] ^ opad));
    }
    outer_in.insert(outer_in.end(), inner.begin(), inner.end());
    return hash(outer_in);
}

// hmac_verify — constant-time equality (does not short-circuit).
inline bool hmac_verify(const std::uint8_t *a, const std::uint8_t *b,
                        std::size_t len) {
    std::uint8_t diff = 0;
    for (std::size_t i = 0; i < len; i++) {
        diff = static_cast<std::uint8_t>(diff | (a[i] ^ b[i]));
    }
    return diff == 0;
}

inline bool hmac_verify(const std::vector<std::uint8_t> &a,
                        const std::vector<std::uint8_t> &b) {
    if (a.size() != b.size()) {
        return false;
    }
    return hmac_verify(a.data(), b.data(), a.size());
}

} // namespace ca

#endif // HMAC_HPP
