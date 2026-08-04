// ct_compare.hpp — constant-time byte comparison, in pure ISO C++17,
// header-only. A faithful port of the Rust `ct-compare` crate.
// ===========================================================================
//
// A naive `memcmp` (or an `==` loop that returns early on the first mismatch)
// takes a different amount of time depending on WHERE two values first differ.
// For secrets — MAC/auth tags, derived keys, password hashes — that timing is a
// side channel an attacker can exploit to recover the secret byte by byte.
//
// These routines instead do the SAME work for every byte regardless of its
// value: they fold all differences into an accumulator with no data-dependent
// branch, then check the accumulator once at the end.
//
//   ct_eq         — equal-length? and equal bytes? (length is treated as public)
//   ct_eq_fixed   — equal bytes over a compile-time length (std::array, no check)
//   ct_select_bytes — branchless select between two equal-length buffers
//   ct_eq_u64     — constant-time equality of two 64-bit values
//
// The Rust crate uses `core::hint::black_box` as an optimiser barrier so the
// loop is not folded back into an early-exit; the pure-ISO equivalent here is a
// read through a `volatile` object.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef CA_CT_COMPARE_HPP
#define CA_CT_COMPARE_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <stdexcept>
#include <vector>

namespace ca {
namespace ct_compare {

namespace detail {
// Optimiser barriers: read through a volatile object so the compiler cannot
// fold the accumulation back into an early-exit.
inline std::uint8_t barrier(std::uint8_t x) {
    volatile std::uint8_t v = x;
    return v;
}
inline std::uint64_t barrier(std::uint64_t x) {
    volatile std::uint64_t v = x;
    return v;
}
}  // namespace detail

// ct_eq — true iff the two byte sequences have the same length AND the same
// bytes. The length comparison is early (length is public); the byte comparison
// is constant-time.
inline bool ct_eq(const std::vector<std::uint8_t>& a,
                  const std::vector<std::uint8_t>& b) {
    if (a.size() != b.size()) {
        return false;
    }
    std::uint8_t acc = 0;
    for (std::size_t i = 0; i < a.size(); ++i) {
        acc = static_cast<std::uint8_t>(acc | (a[i] ^ b[i]));
    }
    return detail::barrier(acc) == 0;
}

// ct_eq_fixed — true iff the N bytes of `a` and `b` are equal. No length check
// (the length is a compile-time constant); constant-time over the N bytes.
template <std::size_t N>
inline bool ct_eq_fixed(const std::array<std::uint8_t, N>& a,
                        const std::array<std::uint8_t, N>& b) {
    std::uint8_t acc = 0;
    for (std::size_t i = 0; i < N; ++i) {
        acc = static_cast<std::uint8_t>(acc | (a[i] ^ b[i]));
    }
    return detail::barrier(acc) == 0;
}

// ct_select_bytes — branchless select: returns a copy of `a` if `choice` is
// true, else a copy of `b`. No instruction branches on `choice`. Throws
// std::invalid_argument if the inputs differ in length (the crate panics).
inline std::vector<std::uint8_t> ct_select_bytes(
    const std::vector<std::uint8_t>& a, const std::vector<std::uint8_t>& b,
    bool choice) {
    if (a.size() != b.size()) {
        throw std::invalid_argument(
            "ct_select_bytes requires equal-length inputs");
    }
    // mask = 0xFF when choice is true, else 0x00 — with no branch.
    const std::uint8_t mask =
        static_cast<std::uint8_t>(0u - static_cast<unsigned>(choice ? 1u : 0u));
    std::vector<std::uint8_t> out(a.size());
    for (std::size_t i = 0; i < a.size(); ++i) {
        // b ^ ((a ^ b) & mask): a when mask=0xFF, b when mask=0x00.
        out[i] = static_cast<std::uint8_t>(
            b[i] ^ (static_cast<std::uint8_t>(a[i] ^ b[i]) & mask));
    }
    return out;
}

// ct_eq_u64 — true iff a == b, computed without a data-dependent branch.
inline bool ct_eq_u64(std::uint64_t a, std::uint64_t b) {
    const std::uint64_t diff = a ^ b;
    // Fold every bit of diff into the top bit: 0 iff diff == 0.
    const std::uint64_t folded = (diff | (0u - diff)) >> 63;
    return detail::barrier(folded) == 0;
}

}  // namespace ct_compare
}  // namespace ca

#endif  // CA_CT_COMPARE_HPP
