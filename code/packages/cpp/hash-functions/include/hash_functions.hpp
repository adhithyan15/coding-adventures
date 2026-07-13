// hash_functions.hpp — non-cryptographic hash functions, header-only ISO C++17.
// ============================================================================
//
// A faithful port of the Rust `hash-functions` crate (the "DT17" family): the
// well-known *non-cryptographic* hashes implemented from scratch, plus two
// quality-analysis helpers.
//
//   FNV-1a (32 & 64-bit), DJB2, polynomial rolling (Rabin-Karp), Murmur3-32,
//   and SipHash-2-4 (keyed).
//
// These are fast table hashes, NOT collision-resistant primitives — for
// security use the crypto digests (sha256/sha1/md5/hmac) this repo also ports.
//
// Design notes vs. the Rust crate:
//   * The `HashFunction` trait becomes an abstract base with concrete structs
//     (`Fnv1a32`, `Djb2`, `PolynomialRolling{base,modulus}`, ...), one per
//     algorithm, each exposing `hash()` and `output_bits()`.
//   * Rust's `u128` inside polynomial rolling is replaced by an exact,
//     overflow-safe `mulmod` — results match bit-for-bit for any 64-bit modulus.
//   * Rust's `avalanche_score` seeds itself from `getrandom` (OS entropy / FFI,
//     no pure-ISO equivalent); here `avalanche_score` takes a caller-supplied
//     fill callable, mirroring the crate's internal `avalanche_score_with_source`.
//
// Pure ISO C++17 — no <cmath>, no compiler extensions, no 128-bit integers.
#ifndef HASH_FUNCTIONS_HPP
#define HASH_FUNCTIONS_HPP

#include <array>
#include <cstddef>
#include <cstdint>
#include <string_view>
#include <vector>

namespace ca {
namespace hash_functions {

// ── Named constants (mirroring the Rust `pub const`s) ───────────────────────
inline constexpr std::uint64_t kDjb2OffsetBasis = 5381;
inline constexpr std::uint32_t kFnv32OffsetBasis = 0x811C9DC5u;
inline constexpr std::uint32_t kFnv32Prime = 0x01000193u;
inline constexpr std::uint64_t kFnv64OffsetBasis = 0xCBF29CE484222325ull;
inline constexpr std::uint64_t kFnv64Prime = 0x00000100000001B3ull;
inline constexpr std::uint64_t kPolynomialRollingDefaultBase = 31;
inline constexpr std::uint64_t kPolynomialRollingDefaultModulus =
    (std::uint64_t{1} << 61) - 1;

namespace detail {

inline std::uint32_t rotl32(std::uint32_t x, unsigned r) {
    return static_cast<std::uint32_t>((x << r) | (x >> (32 - r)));
}
inline std::uint64_t rotl64(std::uint64_t x, unsigned r) {
    return static_cast<std::uint64_t>((x << r) | (x >> (64 - r)));
}
// Population count without <bit>/builtins (Kernighan).
inline std::uint32_t popcount64(std::uint64_t x) {
    std::uint32_t n = 0;
    while (x != 0) {
        x &= x - 1;
        ++n;
    }
    return n;
}
inline std::uint32_t load_u32_le(const std::uint8_t *p) {
    return static_cast<std::uint32_t>(p[0]) |
           (static_cast<std::uint32_t>(p[1]) << 8) |
           (static_cast<std::uint32_t>(p[2]) << 16) |
           (static_cast<std::uint32_t>(p[3]) << 24);
}
inline std::uint64_t load_u64_le(const std::uint8_t *p) {
    std::uint64_t r = 0;
    for (int i = 0; i < 8; ++i)
        r |= static_cast<std::uint64_t>(p[i]) << (i * 8);
    return r;
}
// (a + b) mod m for a, b < m, never overflowing.
inline std::uint64_t addmod(std::uint64_t a, std::uint64_t b, std::uint64_t m) {
    if (a >= m - b) return a - (m - b);
    return a + b;
}
// (a * b) mod m, exact for any m < 2^64 (stands in for Rust's u128).
inline std::uint64_t mulmod(std::uint64_t a, std::uint64_t b, std::uint64_t m) {
    std::uint64_t result = 0;
    a %= m;
    while (b != 0) {
        if (b & 1u) result = addmod(result, a, m);
        a = addmod(a, a, m);
        b >>= 1;
    }
    return result;
}
inline std::uint32_t fmix32(std::uint32_t h) {
    h ^= h >> 16;
    h *= 0x85EBCA6Bu;
    h ^= h >> 13;
    h *= 0xC2B2AE35u;
    h ^= h >> 16;
    return h;
}
inline void sipround(std::uint64_t &v0, std::uint64_t &v1, std::uint64_t &v2,
                     std::uint64_t &v3) {
    v0 += v1;
    v1 = rotl64(v1, 13);
    v1 ^= v0;
    v0 = rotl64(v0, 32);
    v2 += v3;
    v3 = rotl64(v3, 16);
    v3 ^= v2;
    v0 += v3;
    v3 = rotl64(v3, 21);
    v3 ^= v0;
    v2 += v1;
    v1 = rotl64(v1, 17);
    v1 ^= v2;
    v2 = rotl64(v2, 32);
}

}  // namespace detail

// ── Free functions ──────────────────────────────────────────────────────────

inline std::uint32_t fnv1a_32(const std::uint8_t *data, std::size_t len) {
    std::uint32_t hash = kFnv32OffsetBasis;
    for (std::size_t i = 0; i < len; ++i) {
        hash ^= static_cast<std::uint32_t>(data[i]);
        hash *= kFnv32Prime;
    }
    return hash;
}
inline std::uint64_t fnv1a_64(const std::uint8_t *data, std::size_t len) {
    std::uint64_t hash = kFnv64OffsetBasis;
    for (std::size_t i = 0; i < len; ++i) {
        hash ^= static_cast<std::uint64_t>(data[i]);
        hash *= kFnv64Prime;
    }
    return hash;
}
inline std::uint64_t djb2(const std::uint8_t *data, std::size_t len) {
    std::uint64_t hash = kDjb2OffsetBasis;
    for (std::size_t i = 0; i < len; ++i)
        hash = (hash << 5) + hash + static_cast<std::uint64_t>(data[i]);
    return hash;
}
inline std::uint64_t polynomial_rolling_with_params(const std::uint8_t *data,
                                                    std::size_t len,
                                                    std::uint64_t base,
                                                    std::uint64_t modulus) {
    if (modulus == 0) return 0;  // Rust asserts modulus > 0.
    std::uint64_t hash = 0;
    for (std::size_t i = 0; i < len; ++i)
        hash = detail::addmod(detail::mulmod(hash, base, modulus),
                              static_cast<std::uint64_t>(data[i]) % modulus,
                              modulus);
    return hash;
}
inline std::uint64_t polynomial_rolling(const std::uint8_t *data,
                                        std::size_t len) {
    return polynomial_rolling_with_params(data, len,
                                          kPolynomialRollingDefaultBase,
                                          kPolynomialRollingDefaultModulus);
}
inline std::uint32_t murmur3_32_with_seed(const std::uint8_t *data,
                                          std::size_t len, std::uint32_t seed) {
    constexpr std::uint32_t c1 = 0xCC9E2D51u;
    constexpr std::uint32_t c2 = 0x1B873593u;
    std::uint32_t hash = seed;
    std::size_t nblocks = len / 4;
    for (std::size_t i = 0; i < nblocks; ++i) {
        std::uint32_t k = detail::load_u32_le(data + i * 4);
        k *= c1;
        k = detail::rotl32(k, 15);
        k *= c2;
        hash ^= k;
        hash = detail::rotl32(hash, 13);
        hash = hash * 5u + 0xE6546B64u;
    }
    std::size_t tail_len = len - nblocks * 4;
    if (tail_len != 0) {
        const std::uint8_t *tail = data + nblocks * 4;
        std::uint32_t k = 0;
        for (std::size_t i = 0; i < tail_len; ++i)
            k ^= static_cast<std::uint32_t>(tail[i]) << (i * 8);
        k *= c1;
        k = detail::rotl32(k, 15);
        k *= c2;
        hash ^= k;
    }
    hash ^= static_cast<std::uint32_t>(len);
    return detail::fmix32(hash);
}
inline std::uint32_t murmur3_32(const std::uint8_t *data, std::size_t len) {
    return murmur3_32_with_seed(data, len, 0);
}
inline std::uint64_t siphash_2_4(const std::uint8_t *data, std::size_t len,
                                 const std::array<std::uint8_t, 16> &key) {
    std::uint64_t k0 = detail::load_u64_le(key.data());
    std::uint64_t k1 = detail::load_u64_le(key.data() + 8);
    std::uint64_t v0 = 0x736F6D6570736575ull ^ k0;
    std::uint64_t v1 = 0x646F72616E646F6Dull ^ k1;
    std::uint64_t v2 = 0x6C7967656E657261ull ^ k0;
    std::uint64_t v3 = 0x7465646279746573ull ^ k1;

    std::size_t nblocks = len / 8;
    for (std::size_t i = 0; i < nblocks; ++i) {
        std::uint64_t m = detail::load_u64_le(data + i * 8);
        v3 ^= m;
        detail::sipround(v0, v1, v2, v3);
        detail::sipround(v0, v1, v2, v3);
        v0 ^= m;
    }
    std::uint64_t last = (static_cast<std::uint64_t>(len) & 0xff) << 56;
    std::size_t tail_len = len - nblocks * 8;
    const std::uint8_t *tail = data + nblocks * 8;
    for (std::size_t i = 0; i < tail_len; ++i)
        last |= static_cast<std::uint64_t>(tail[i]) << (i * 8);

    v3 ^= last;
    detail::sipround(v0, v1, v2, v3);
    detail::sipround(v0, v1, v2, v3);
    v0 ^= last;
    v2 ^= 0xff;
    detail::sipround(v0, v1, v2, v3);
    detail::sipround(v0, v1, v2, v3);
    detail::sipround(v0, v1, v2, v3);
    detail::sipround(v0, v1, v2, v3);
    return v0 ^ v1 ^ v2 ^ v3;
}

// String convenience overloads (hash the view's bytes).
inline std::uint32_t hash_str_fnv1a_32(std::string_view s) {
    return fnv1a_32(reinterpret_cast<const std::uint8_t *>(s.data()), s.size());
}
inline std::uint64_t hash_str_siphash(std::string_view s,
                                      const std::array<std::uint8_t, 16> &key) {
    return siphash_2_4(reinterpret_cast<const std::uint8_t *>(s.data()),
                       s.size(), key);
}

// ── HashFunction: the trait, as an abstract base + concrete structs ─────────

class HashFunction {
   public:
    virtual ~HashFunction() = default;
    virtual std::uint64_t hash(const std::uint8_t *data,
                               std::size_t len) const = 0;
    virtual std::uint32_t output_bits() const = 0;
};

struct Fnv1a32 final : HashFunction {
    std::uint64_t hash(const std::uint8_t *d, std::size_t n) const override {
        return fnv1a_32(d, n);
    }
    std::uint32_t output_bits() const override { return 32; }
};
struct Fnv1a64 final : HashFunction {
    std::uint64_t hash(const std::uint8_t *d, std::size_t n) const override {
        return fnv1a_64(d, n);
    }
    std::uint32_t output_bits() const override { return 64; }
};
struct Djb2 final : HashFunction {
    std::uint64_t hash(const std::uint8_t *d, std::size_t n) const override {
        return djb2(d, n);
    }
    std::uint32_t output_bits() const override { return 64; }
};
struct PolynomialRolling final : HashFunction {
    std::uint64_t base = kPolynomialRollingDefaultBase;
    std::uint64_t modulus = kPolynomialRollingDefaultModulus;
    PolynomialRolling() = default;
    PolynomialRolling(std::uint64_t b, std::uint64_t m) : base(b), modulus(m) {}
    std::uint64_t hash(const std::uint8_t *d, std::size_t n) const override {
        return polynomial_rolling_with_params(d, n, base, modulus);
    }
    std::uint32_t output_bits() const override { return 64; }
};
struct Murmur3_32 final : HashFunction {
    std::uint32_t seed = 0;
    Murmur3_32() = default;
    explicit Murmur3_32(std::uint32_t s) : seed(s) {}
    std::uint64_t hash(const std::uint8_t *d, std::size_t n) const override {
        return murmur3_32_with_seed(d, n, seed);
    }
    std::uint32_t output_bits() const override { return 32; }
};
struct SipHash24 final : HashFunction {
    std::array<std::uint8_t, 16> key{};
    SipHash24() = default;
    explicit SipHash24(const std::array<std::uint8_t, 16> &k) : key(k) {}
    std::uint64_t hash(const std::uint8_t *d, std::size_t n) const override {
        return siphash_2_4(d, n, key);
    }
    std::uint32_t output_bits() const override { return 64; }
};

// ── Analysis helpers (generic over the hash, like the Rust originals) ───────

// Estimate the average fraction of `output_bits` output bits that flip when a
// single input bit of an 8-byte input is toggled. `hash(data, len) -> u64` and
// `fill(std::uint8_t*, std::size_t)` are callables; `fill` supplies the input
// samples (Rust wires `getrandom` here — omitted, no pure-ISO entropy).
// Returns 0.0 on a contract violation.
template <class HashFn, class Fill>
double avalanche_score(HashFn hash, std::uint32_t output_bits,
                       std::size_t sample_size, Fill fill) {
    if (sample_size == 0 || output_bits == 0 || output_bits > 64) return 0.0;
    std::uint64_t total_bit_flips = 0;
    std::uint64_t total_trials = 0;
    std::array<std::uint8_t, 8> input{};
    for (std::size_t s = 0; s < sample_size; ++s) {
        fill(input.data(), input.size());
        std::uint64_t h1 = hash(input.data(), input.size());
        for (std::size_t bit_pos = 0; bit_pos < input.size() * 8; ++bit_pos) {
            std::size_t byte_idx = bit_pos / 8;
            std::uint8_t bit_mask =
                static_cast<std::uint8_t>(1u << (bit_pos % 8));
            std::array<std::uint8_t, 8> flipped = input;
            flipped[byte_idx] ^= bit_mask;
            std::uint64_t h2 = hash(flipped.data(), flipped.size());
            total_bit_flips += detail::popcount64(h1 ^ h2);
            total_trials += output_bits;
        }
    }
    return static_cast<double>(total_bit_flips) /
           static_cast<double>(total_trials);
}

// Chi-square statistic of how evenly `hash` spreads `inputs` across
// `num_buckets` buckets (0.0 = perfectly uniform). `inputs` is any range whose
// elements expose `.data()` and `.size()` (e.g. std::vector<std::uint8_t>,
// std::string_view). Returns -1.0 on a contract violation.
template <class HashFn, class InputRange>
double distribution_test(HashFn hash, const InputRange &inputs,
                         std::size_t num_buckets) {
    if (num_buckets == 0) return -1.0;
    std::vector<std::uint64_t> counts(num_buckets, 0);
    std::uint64_t total = 0;
    for (const auto &inp : inputs) {
        std::uint64_t h = hash(
            reinterpret_cast<const std::uint8_t *>(inp.data()), inp.size());
        counts[static_cast<std::size_t>(h % num_buckets)]++;
        ++total;
    }
    if (total == 0) return -1.0;
    double expected =
        static_cast<double>(total) / static_cast<double>(num_buckets);
    double chi2 = 0.0;
    for (std::uint64_t observed_count : counts) {
        double observed = static_cast<double>(observed_count);
        double delta = observed - expected;
        chi2 += delta * delta / expected;
    }
    return chi2;
}

}  // namespace hash_functions
}  // namespace ca

#endif  // HASH_FUNCTIONS_HPP
