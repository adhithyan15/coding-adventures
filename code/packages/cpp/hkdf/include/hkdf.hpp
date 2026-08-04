// hkdf.hpp — HKDF, the HMAC-based key derivation function (RFC 5869), in pure
// ISO C++17 (header-only). A faithful port of the Rust `hkdf` crate.
// ===========================================================================
//
//     extract:  PRK = HMAC(salt, IKM)          (empty salt → zeros)
//     expand:   OKM = T(1) || T(2) || …         (truncated to `length`)
//               T(i) = HMAC(PRK, T(i-1) || info || i)
//
// Hash-agnostic: pass a hash callable `std::vector<uint8_t>(const
// std::vector<uint8_t>&)` plus the hash's block and digest sizes. Built on the
// sibling header-only `hmac`. Tests use SHA-256 and the RFC 5869 vectors.
//
// Portability: pure ISO C++17. Compiles clean under GCC, Clang, and MSVC with
// -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
#ifndef HKDF_HPP
#define HKDF_HPP

#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <vector>

#include "hmac.hpp" // sibling header-only package (include path via run.sh)

namespace ca {

// hkdf_extract — PRK = HMAC(salt, IKM). An empty salt becomes digest_size zeros.
template <typename HashFn>
std::vector<std::uint8_t> hkdf_extract(HashFn hash, std::size_t block_size,
                                       std::size_t digest_size,
                                       const std::vector<std::uint8_t> &salt,
                                       const std::vector<std::uint8_t> &ikm) {
    std::vector<std::uint8_t> effective_salt =
        salt.empty() ? std::vector<std::uint8_t>(digest_size, 0) : salt;
    return hmac(hash, block_size, effective_salt, ikm);
}

// hkdf_expand — expand `prk` to `length` bytes mixing in `info`. Throws
// std::invalid_argument on a zero or too-large length (> 255 * digest_size).
template <typename HashFn>
std::vector<std::uint8_t> hkdf_expand(HashFn hash, std::size_t block_size,
                                      std::size_t digest_size,
                                      const std::vector<std::uint8_t> &prk,
                                      const std::vector<std::uint8_t> &info,
                                      std::size_t length) {
    if (length == 0) {
        throw std::invalid_argument("hkdf_expand: length must be > 0");
    }
    if (digest_size == 0 || length > static_cast<std::size_t>(255) * digest_size) {
        throw std::invalid_argument("hkdf_expand: length too large");
    }
    std::size_t n = (length + digest_size - 1) / digest_size;

    std::vector<std::uint8_t> okm;
    okm.reserve(n * digest_size);
    std::vector<std::uint8_t> t_prev; // T(0) = empty
    for (std::size_t i = 1; i <= n; i++) {
        std::vector<std::uint8_t> message;
        message.reserve(t_prev.size() + info.size() + 1);
        message.insert(message.end(), t_prev.begin(), t_prev.end());
        message.insert(message.end(), info.begin(), info.end());
        message.push_back(static_cast<std::uint8_t>(i));
        t_prev = hmac(hash, block_size, prk, message);
        okm.insert(okm.end(), t_prev.begin(), t_prev.end());
    }
    okm.resize(length);
    return okm;
}

// hkdf — the full extract-then-expand.
template <typename HashFn>
std::vector<std::uint8_t> hkdf(HashFn hash, std::size_t block_size,
                               std::size_t digest_size,
                               const std::vector<std::uint8_t> &salt,
                               const std::vector<std::uint8_t> &ikm,
                               const std::vector<std::uint8_t> &info,
                               std::size_t length) {
    std::vector<std::uint8_t> prk =
        hkdf_extract(hash, block_size, digest_size, salt, ikm);
    return hkdf_expand(hash, block_size, digest_size, prk, info, length);
}

} // namespace ca

#endif // HKDF_HPP
