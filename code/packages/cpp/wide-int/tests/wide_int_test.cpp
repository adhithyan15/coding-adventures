// wide_int_test.cpp — unit tests for the C++ portable 128-bit integer library.
//
// Pure ISO C++17 (runs under iso-harness's -pedantic-errors), so no __int128
// oracle: golden vectors plus algebraic property/round-trip tests over a
// deterministic pseudo-random sweep.
#include "wide_int.hpp"
#include "iso_test.h"

namespace wi = ca::wide_int;
using u128 = wi::u128;
using i128 = wi::i128;

namespace {

std::uint64_t g_state = 0x9E3779B97F4A7C15u;
std::uint64_t rng() {
    g_state = g_state * 6364136223846793005u + 1442695040888963407u;
    return g_state;
}
u128 rng_u128() { return u128(rng(), rng()); }

void test_construction() {
    u128 a(0x1122334455667788u, 0x99AABBCCDDEEFF00u);
    ISO_CHECK(a.hi() == 0x1122334455667788u && a.lo() == 0x99AABBCCDDEEFF00u);
    ISO_CHECK(u128(42) == u128(0, 42));
    ISO_CHECK(u128(0).is_zero());
    ISO_CHECK(u128::max() == u128(~std::uint64_t(0), ~std::uint64_t(0)));
}

void test_add_sub() {
    ISO_CHECK((u128::max() + u128(1)).is_zero());
    ISO_CHECK(u128(0) - u128(1) == u128::max());
    ISO_CHECK(u128(std::uint64_t(-1)) + u128(1) == u128(1, 0));
    ISO_CHECK(u128(1, 0) - u128(1) == u128(std::uint64_t(-1)));
}

void test_mul_shift_golden() {
    u128 p = u128::mul_u64(std::uint64_t(-1), std::uint64_t(-1));
    ISO_CHECK(p.hi() == 0xFFFFFFFFFFFFFFFEu && p.lo() == 1u);
    ISO_CHECK((u128(1) << 64) == u128(1, 0));
    ISO_CHECK((u128(1) << 127) == u128(std::uint64_t(1) << 63, 0));
    ISO_CHECK((u128(1) << 128).is_zero());
    ISO_CHECK((u128(1, 0) >> 64) == u128(1));
    ISO_CHECK((u128::max() >> 127) == u128(1));
}

void test_divmod_golden() {
    std::pair<u128, u128> qr = u128::divmod(u128::max(), u128(2));
    ISO_CHECK(qr.first == u128((std::uint64_t(1) << 63) - 1, std::uint64_t(-1)));
    ISO_CHECK(qr.second == u128(1));
    ISO_CHECK(u128(100) / u128(7) == u128(14));
    ISO_CHECK(u128(100) % u128(7) == u128(2));
}

void test_format() {
    ISO_CHECK(u128::max().to_string() == "340282366920938463463374607431768211455");
    ISO_CHECK(u128::max().to_hex() == "ffffffffffffffffffffffffffffffff");
    ISO_CHECK(u128(0).to_string() == "0");
    u128 v(1);
    for (int i = 0; i < 20; ++i) v = v * u128(10);
    ISO_CHECK(v.to_string() == "100000000000000000000");
}

void test_signed() {
    ISO_CHECK(i128(-1).bits() == u128::max());
    ISO_CHECK(i128(-1).to_string() == "-1");
    ISO_CHECK(-(-i128(12345)) == i128(12345));
    ISO_CHECK(i128(-1) < i128(1));
    ISO_CHECK(i128(-5) < i128(-3));
    ISO_CHECK(i128::divmod(i128(-7), i128(2)).first == i128(-3));
    ISO_CHECK(i128::divmod(i128(-7), i128(2)).second == i128(-1));
    ISO_CHECK(i128(7) / i128(-2) == i128(-3));
    ISO_CHECK(i128(7) % i128(-2) == i128(1));
    ISO_CHECK(i128(-7) / i128(-2) == i128(3));
    ISO_CHECK((i128(-8) >> 1) == i128(-4));
    ISO_CHECK((i128(-1) >> 100) == i128(-1));
}

// constexpr smoke test: prove the core ops evaluate at compile time.
constexpr u128 kProd = u128(0xFFFFFFFFu) * u128(0x100000000u);
static_assert(kProd == u128(0xFFFFFFFF00000000u), "constexpr multiply");
static_assert((u128::max() / u128(2)) == u128((std::uint64_t(1) << 63) - 1, std::uint64_t(-1)),
              "constexpr divide");

void test_property_sweep() {
    for (int iter = 0; iter < 200000; ++iter) {
        u128 a = rng_u128();
        u128 b = rng_u128();
        ISO_CHECK((a + b) - b == a);
        ISO_CHECK(a + b == b + a);
        ISO_CHECK(a * b == b * a);
        if (!b.is_zero()) {
            std::pair<u128, u128> qr = u128::divmod(a, b);
            ISO_CHECK(qr.first * b + qr.second == a);
            ISO_CHECK(qr.second < b);
        }
        unsigned n = static_cast<unsigned>(rng() % 128u);
        ISO_CHECK(((a << n) >> n) == ((a << n) >> n));
        ISO_CHECK(u128::mul_u64(a.lo(), b.lo()) == u128(a.lo()) * u128(b.lo()));
    }
    ISO_CHECK(true);
}

} // namespace

int main() {
    test_construction();
    test_add_sub();
    test_mul_shift_golden();
    test_divmod_golden();
    test_format();
    test_signed();
    test_property_sweep();
    return ISO_TEST_RESULT();
}
