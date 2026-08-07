// rng.hpp — deterministic pseudo-random number generators, in pure ISO C++17
// (header-only). A faithful port of the Rust `rng` crate, in namespace `ca::rng`.
// ===========================================================================
//
// Three classic non-cryptographic PRNGs, each fully deterministic given a seed:
//
//   ca::rng::Lcg        — a 64-bit Linear Congruential Generator (high 32 bits)
//   ca::rng::Xorshift64 — Marsaglia's Xorshift64 (three XOR-shifts)
//   ca::rng::Pcg32      — O'Neill's PCG32 (an LCG plus XSH-RR permutation)
//
// Each has: next_u32, next_u64, next_float (a double in [0, 1)), and
// next_int_in_range (rejection-sampled to avoid modulo bias).
//
// NOT cryptographically secure — do not use for keys or nonces.
//
// Portability: pure ISO C++17 — GCC, Clang, and MSVC with -pedantic-errors /
// /permissive- and warnings-as-errors. No extensions.
#ifndef RNG_HPP
#define RNG_HPP

#include <cstdint>

namespace ca {
namespace rng {

namespace detail {
constexpr std::uint64_t LCG_MULTIPLIER = 6364136223846793005ULL;
constexpr std::uint64_t LCG_INCREMENT = 1442695040888963407ULL;
constexpr double FLOAT_DIV = 4294967296.0; // 2^32

// Rejection-sampled uniform in [min, max]; `g` must have next_u32().
template <class Gen>
std::int64_t range_from(Gen &g, std::int64_t min, std::int64_t max) {
    if (min > max) {
        return min; // the Rust crate asserts min <= max
    }
    std::uint64_t range_size = (static_cast<std::uint64_t>(max) -
                               static_cast<std::uint64_t>(min)) + 1ULL;
    if (range_size == 0) { // full 64-bit range (Rust would divide by zero)
        return static_cast<std::int64_t>(static_cast<std::uint64_t>(min) +
                                         g.next_u32());
    }
    std::uint64_t threshold = (0ULL - range_size) % range_size;
    for (;;) {
        std::uint64_t r = g.next_u32();
        if (r >= threshold) {
            return static_cast<std::int64_t>(
                static_cast<std::uint64_t>(min) + (r % range_size));
        }
    }
}
} // namespace detail

class Lcg {
public:
    explicit Lcg(std::uint64_t seed) : state_(seed) {}

    std::uint32_t next_u32() {
        state_ = state_ * detail::LCG_MULTIPLIER + detail::LCG_INCREMENT;
        return static_cast<std::uint32_t>(state_ >> 32);
    }
    std::uint64_t next_u64() {
        std::uint64_t hi = next_u32();
        std::uint64_t lo = next_u32();
        return (hi << 32) | lo;
    }
    double next_float() {
        return static_cast<double>(next_u32()) / detail::FLOAT_DIV;
    }
    std::int64_t next_int_in_range(std::int64_t min, std::int64_t max) {
        return detail::range_from(*this, min, max);
    }

private:
    std::uint64_t state_;
};

class Xorshift64 {
public:
    // Seed 0 is replaced with 1 (0 is the all-zeros fixed point).
    explicit Xorshift64(std::uint64_t seed) : state_(seed == 0 ? 1ULL : seed) {}

    std::uint32_t next_u32() {
        std::uint64_t x = state_;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        state_ = x;
        return static_cast<std::uint32_t>(x);
    }
    std::uint64_t next_u64() {
        std::uint64_t hi = next_u32();
        std::uint64_t lo = next_u32();
        return (hi << 32) | lo;
    }
    double next_float() {
        return static_cast<double>(next_u32()) / detail::FLOAT_DIV;
    }
    std::int64_t next_int_in_range(std::int64_t min, std::int64_t max) {
        return detail::range_from(*this, min, max);
    }

private:
    std::uint64_t state_;
};

class Pcg32 {
public:
    explicit Pcg32(std::uint64_t seed)
        : state_(0), increment_(detail::LCG_INCREMENT | 1ULL) {
        state_ = state_ * detail::LCG_MULTIPLIER + increment_;
        state_ = state_ + seed;
        state_ = state_ * detail::LCG_MULTIPLIER + increment_;
    }

    std::uint32_t next_u32() {
        std::uint64_t old_state = state_;
        state_ = old_state * detail::LCG_MULTIPLIER + increment_;
        std::uint32_t xorshifted =
            static_cast<std::uint32_t>(((old_state >> 18) ^ old_state) >> 27);
        std::uint32_t rot = static_cast<std::uint32_t>(old_state >> 59);
        return static_cast<std::uint32_t>((xorshifted >> rot) |
                                          (xorshifted << ((32 - rot) & 31)));
    }
    std::uint64_t next_u64() {
        std::uint64_t hi = next_u32();
        std::uint64_t lo = next_u32();
        return (hi << 32) | lo;
    }
    double next_float() {
        return static_cast<double>(next_u32()) / detail::FLOAT_DIV;
    }
    std::int64_t next_int_in_range(std::int64_t min, std::int64_t max) {
        return detail::range_from(*this, min, max);
    }

private:
    std::uint64_t state_;
    std::uint64_t increment_;
};

} // namespace rng
} // namespace ca

#endif // RNG_HPP
