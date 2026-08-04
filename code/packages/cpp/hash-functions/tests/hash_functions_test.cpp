// Tests for hash-functions, using the header-only iso_test.h harness (pure ISO).
// Vectors mirror the Rust crate's own unit tests. 64-bit values are checked
// full-width via ISO_CHECK (ISO_CHECK_EQ_UINT narrows to `unsigned long`, which
// is 32-bit on LLP64/Windows).
#include "iso_test.h"

#include <array>
#include <cstdint>
#include <string_view>
#include <vector>

#include "hash_functions.hpp"

namespace hf = ca::hash_functions;

// Hash a string literal's bytes (excluding the terminating NUL).
static std::uint32_t b32(std::string_view s) {
    return hf::fnv1a_32(reinterpret_cast<const std::uint8_t *>(s.data()),
                        s.size());
}
static const std::uint8_t *bp(std::string_view s) {
    return reinterpret_cast<const std::uint8_t *>(s.data());
}

int main() {
    using std::string_view;

    // ── FNV-1a 32 ────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_UINT(b32(""), 0x811C9DC5u);
    ISO_CHECK_EQ_UINT(b32("a"), 0xE40C292Cu);
    ISO_CHECK_EQ_UINT(b32("abc"), 0x1A47E90Bu);
    ISO_CHECK_EQ_UINT(b32("hello"), 1335831723u);
    ISO_CHECK_EQ_UINT(b32("foobar"), 3214735720u);

    // ── FNV-1a 64 ────────────────────────────────────────────────────────────
    ISO_CHECK(hf::fnv1a_64(bp(""), 0) == 0xCBF29CE484222325ull);
    ISO_CHECK(hf::fnv1a_64(bp("a"), 1) == 0xAF63DC4C8601EC8Cull);
    ISO_CHECK(hf::fnv1a_64(bp("abc"), 3) == 0xE71FA2190541574Bull);
    ISO_CHECK(hf::fnv1a_64(bp("hello"), 5) == 0xA430D84680AABD0Bull);

    // ── DJB2 ─────────────────────────────────────────────────────────────────
    ISO_CHECK(hf::djb2(bp(""), 0) == 5381ull);
    ISO_CHECK(hf::djb2(bp("a"), 1) == 177670ull);
    ISO_CHECK(hf::djb2(bp("abc"), 3) == 193485963ull);

    // ── Polynomial rolling ───────────────────────────────────────────────────
    ISO_CHECK(hf::polynomial_rolling(bp(""), 0) == 0ull);
    ISO_CHECK(hf::polynomial_rolling(bp("a"), 1) == 97ull);
    ISO_CHECK(hf::polynomial_rolling(bp("ab"), 2) == 3105ull);
    ISO_CHECK(hf::polynomial_rolling(bp("abc"), 3) == 96354ull);
    ISO_CHECK(hf::polynomial_rolling_with_params(bp("abc"), 3, 37,
                                                 1000000007ull) ==
              static_cast<std::uint64_t>(((97ull * 37 + 98) * 37 + 99)));
    {
        std::uint64_t big_mod = (std::uint64_t{1} << 62) - 57;
        std::uint64_t h =
            hf::polynomial_rolling_with_params(bp("hello"), 5, 1000003ull,
                                               big_mod);
        ISO_CHECK(h < big_mod);
    }
    ISO_CHECK(hf::polynomial_rolling_with_params(bp("abc"), 3, 31, 0) == 0ull);

    // ── Murmur3 (32-bit) ─────────────────────────────────────────────────────
    ISO_CHECK_EQ_UINT(hf::murmur3_32(bp(""), 0), 0u);
    ISO_CHECK_EQ_UINT(hf::murmur3_32_with_seed(bp(""), 0, 1), 0x514E28B7u);
    ISO_CHECK_EQ_UINT(hf::murmur3_32(bp("a"), 1), 0x3C2569B2u);
    ISO_CHECK_EQ_UINT(hf::murmur3_32(bp("abc"), 3), 0xB3DD93FAu);
    ISO_CHECK_EQ_UINT(hf::murmur3_32(bp("abcd"), 4), 0x43ED676Au);

    // ── SipHash-2-4 ──────────────────────────────────────────────────────────
    {
        std::array<std::uint8_t, 16> key{};
        for (std::size_t i = 0; i < 16; ++i)
            key[i] = static_cast<std::uint8_t>(i);
        ISO_CHECK(hf::siphash_2_4(bp(""), 0, key) == 0x726FDB47DD0E0E31ull);
        std::array<std::uint8_t, 1> one_zero{0x00};
        ISO_CHECK(hf::siphash_2_4(one_zero.data(), 1, key) ==
                  0x74F839C593DC67FDull);
    }

    // ── String helpers ───────────────────────────────────────────────────────
    {
        std::array<std::uint8_t, 16> zero_key{};
        ISO_CHECK_EQ_UINT(hf::hash_str_fnv1a_32("hello"), b32("hello"));
        ISO_CHECK(hf::hash_str_siphash("hello", zero_key) ==
                  hf::siphash_2_4(bp("hello"), 5, zero_key));
    }

    // ── HashFunction dispatch forwards to the free functions ─────────────────
    {
        std::array<std::uint8_t, 16> zero_key{};
        hf::Fnv1a32 fnv32;
        hf::Fnv1a64 fnv64;
        hf::Djb2 djb;
        hf::PolynomialRolling poly;
        hf::Murmur3_32 murmur;
        hf::SipHash24 sip{zero_key};

        ISO_CHECK(fnv32.hash(bp("abc"), 3) ==
                  static_cast<std::uint64_t>(b32("abc")));
        ISO_CHECK(fnv64.hash(bp("abc"), 3) == hf::fnv1a_64(bp("abc"), 3));
        ISO_CHECK(djb.hash(bp("abc"), 3) == hf::djb2(bp("abc"), 3));
        ISO_CHECK(poly.hash(bp("abc"), 3) ==
                  hf::polynomial_rolling(bp("abc"), 3));
        ISO_CHECK(murmur.hash(bp("abc"), 3) ==
                  static_cast<std::uint64_t>(hf::murmur3_32(bp("abc"), 3)));
        ISO_CHECK(sip.hash(bp("abc"), 3) ==
                  hf::siphash_2_4(bp("abc"), 3, zero_key));

        ISO_CHECK_EQ_UINT(fnv32.output_bits(), 32u);
        ISO_CHECK_EQ_UINT(fnv64.output_bits(), 64u);
        ISO_CHECK_EQ_UINT(djb.output_bits(), 64u);
        ISO_CHECK_EQ_UINT(poly.output_bits(), 64u);
        ISO_CHECK_EQ_UINT(murmur.output_bits(), 32u);
        ISO_CHECK_EQ_UINT(sip.output_bits(), 64u);

        // Dispatch through a base-class reference (the trait-object analog).
        const hf::HashFunction &as_trait = poly;
        ISO_CHECK(as_trait.hash(bp("abc"), 3) ==
                  hf::polynomial_rolling(bp("abc"), 3));
    }
    // Non-default polynomial constructor carries its params.
    {
        hf::PolynomialRolling poly{37, 1000000007ull};
        ISO_CHECK(poly.hash(bp("abc"), 3) ==
                  hf::polynomial_rolling_with_params(bp("abc"), 3, 37,
                                                     1000000007ull));
    }

    // ── Analysis: avalanche ──────────────────────────────────────────────────
    {
        // Deterministic LCG fill matching the Rust test's `deterministic_fill`.
        std::uint64_t seed = 1;
        auto fill = [&seed](std::uint8_t *buf, std::size_t len) {
            for (std::size_t i = 0; i < len; ++i) {
                seed = seed * 6364136223846793005ull + 1u;
                buf[i] = static_cast<std::uint8_t>(seed >> 24);
            }
        };
        auto zero_hash = [](const std::uint8_t *, std::size_t) -> std::uint64_t {
            return 0;
        };
        ISO_CHECK_EQ_DBL(hf::avalanche_score(zero_hash, 32, 4, fill), 0.0,
                         1e-12);
        // Contract violations return 0.0.
        ISO_CHECK_EQ_DBL(hf::avalanche_score(zero_hash, 32, 0, fill), 0.0,
                         1e-12);
        ISO_CHECK_EQ_DBL(hf::avalanche_score(zero_hash, 65, 4, fill), 0.0,
                         1e-12);
    }

    // ── Analysis: distribution ───────────────────────────────────────────────
    {
        auto zero_hash = [](const std::uint8_t *, std::size_t) -> std::uint64_t {
            return 0;
        };
        std::vector<std::string_view> inputs = {"a", "b", "c", "d"};
        // All in bucket 0: counts=[4,0,0,0], expected=1 → chi2 = 9+1+1+1 = 12.
        ISO_CHECK_EQ_DBL(hf::distribution_test(zero_hash, inputs, 4), 12.0,
                         1e-9);
        auto len_hash = [](const std::uint8_t *,
                           std::size_t n) -> std::uint64_t {
            return static_cast<std::uint64_t>(n);
        };
        std::vector<std::string_view> two = {"hello", "world"};
        ISO_CHECK(hf::distribution_test(len_hash, two, 4) >= 0.0);
        // Contract violation.
        ISO_CHECK(hf::distribution_test(zero_hash, inputs, 0) < 0.0);
    }

    return ISO_TEST_RESULT();
}
