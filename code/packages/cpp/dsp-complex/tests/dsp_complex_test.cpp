// dsp_complex_test.cpp — Unit, hostile-input, and SplitMix64 PBT tests for cpp/dsp-complex.
#include "dsp_complex.hpp"
#include "iso_test.h"

#include <cstdint>
#include <limits>
#include <vector>

namespace cx = ca::dsp_complex;

namespace {

float af(float x) { return x < 0.0f ? -x : x; }

bool approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

template <typename F>
bool throws(cx::Error expected, F&& fn) {
    try {
        fn();
    } catch (const cx::ComplexError& e) {
        return e.code() == expected;
    } catch (...) {
        return false;
    }
    return false;
}

std::uint64_t splitmix64(std::uint64_t& state) {
    std::uint64_t z = (state += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

float rand_unit_f(std::uint64_t& state) {
    std::uint32_t bits = static_cast<std::uint32_t>(splitmix64(state) >> 40);
    return (static_cast<float>(bits) / 16777216.0f) * 2.0f - 1.0f;
}

void test_constructors_and_projections() {
    std::vector<float> real = {3.0f, 0.0f, -1.0f, 1.0f};
    std::vector<float> imag = {4.0f, 5.0f, 0.0f, -1.0f};
    constexpr float PI_F = 3.14159265f;

    auto t = cx::ComplexTensor::from_real_imag(real, imag);
    ISO_CHECK_EQ_UINT(t.len(), 4u);
    ISO_CHECK(!t.is_empty());
    ISO_CHECK_EQ_UINT(t.shape()[0], 4u);
    ISO_CHECK_EQ_UINT(t.shape()[1], 2u);

    auto r = t.real();
    auto im = t.imag();
    auto mag = t.magnitude();
    auto ph = t.phase();
    ISO_CHECK(approx_eq(r[0], 3.0f, 1e-6f) && approx_eq(r[2], -1.0f, 1e-6f));
    ISO_CHECK(approx_eq(im[0], 4.0f, 1e-6f) && approx_eq(im[3], -1.0f, 1e-6f));
    ISO_CHECK(approx_eq(mag[0], 5.0f, 1e-6f) && approx_eq(mag[3], 1.41421356f, 1e-5f));
    ISO_CHECK(approx_eq(ph[1], PI_F * 0.5f, 1e-5f));
    ISO_CHECK(approx_eq(ph[2], PI_F, 1e-5f));
    ISO_CHECK(approx_eq(ph[3], -PI_F * 0.25f, 1e-5f));

    auto empty = cx::ComplexTensor::from_real({});
    ISO_CHECK(empty.is_empty());
    ISO_CHECK_EQ_UINT(empty.len(), 0u);
    ISO_CHECK(empty.real().empty());

    auto z = cx::ComplexTensor::zeros(3u);
    ISO_CHECK_EQ_UINT(z.len(), 3u);
    ISO_CHECK(z.at(2u) == cx::Complex(0.0f, 0.0f));
}

void test_elementwise_and_indexing() {
    auto a = cx::ComplexTensor::from_interleaved({1.0f, 2.0f, -3.0f, 4.0f});
    auto b = cx::ComplexTensor::from_interleaved({5.0f, -1.0f, 2.0f, 2.0f});

    auto sum = a.add(b);
    ISO_CHECK(approx_eq(sum.at(0u).re, 6.0f, 1e-6f) && approx_eq(sum.at(0u).im, 1.0f, 1e-6f));

    auto prod = a.mul(b);
    ISO_CHECK(approx_eq(prod.at(0u).re, 7.0f, 1e-6f) && approx_eq(prod.at(0u).im, 9.0f, 1e-6f));

    auto quot = a.div(b);
    ISO_CHECK(approx_eq(quot.at(1u).re, 0.25f, 1e-5f) && approx_eq(quot.at(1u).im, 1.75f, 1e-5f));

    a.set(1u, cx::Complex(9.0f, -8.0f));
    ISO_CHECK(approx_eq(a.at(1u).re, 9.0f, 1e-6f) && approx_eq(a.at(1u).im, -8.0f, 1e-6f));
}

void test_hostile_cwe_guards() {
    const float qnan = std::numeric_limits<float>::quiet_NaN();
    const float inf = std::numeric_limits<float>::infinity();
    const float ninf = -std::numeric_limits<float>::infinity();

    /* CWE-476: nullptr with non-zero count */
    ISO_CHECK(throws(cx::Error::InvalidInput, [] { cx::ComplexTensor::from_real(nullptr, 2u); }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] {
        float x[2] = {1.0f, 2.0f};
        cx::ComplexTensor::from_real_imag(nullptr, 2u, x, 2u);
    }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] { cx::ComplexTensor::from_interleaved(nullptr, 2u); }));

    /* CWE-20: non-finite, odd interleaved, length mismatch, div-by-zero */
    ISO_CHECK(throws(cx::Error::InvalidInput, [&] { cx::ComplexTensor::from_real({1.0f, qnan}); }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [&] { cx::ComplexTensor::from_real({inf, 0.0f}); }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [&] { cx::ComplexTensor::from_real({0.0f, ninf}); }));
    ISO_CHECK(throws(cx::Error::LengthMismatch, [] {
        cx::ComplexTensor::from_real_imag({1.0f, 2.0f}, {1.0f});
    }));
    ISO_CHECK(throws(cx::Error::OddInterleavedLength, [] {
        cx::ComplexTensor::from_interleaved({1.0f, 2.0f, 3.0f});
    }));

    auto t = cx::ComplexTensor::from_interleaved({1.0f, 2.0f});
    auto z = cx::ComplexTensor::zeros(1u);
    ISO_CHECK(throws(cx::Error::InvalidInput, [&] { t.set(0u, cx::Complex(qnan, 0.0f)); }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [&] { t.scale(inf); }));
    ISO_CHECK(throws(cx::Error::DivisionByZero, [&] { t.div(z); }));
    ISO_CHECK(throws(cx::Error::DivisionByZero, [] {
        (void)(cx::Complex(1.0f, 0.0f) / cx::Complex(0.0f, 0.0f));
    }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [&] {
        (void)(cx::Complex(qnan, 0.0f) / cx::Complex(1.0f, 0.0f));
    }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] {
        (void)(cx::Complex(2.0e38f, 0.0f) / cx::Complex(1.0e-38f, 0.0f));
    }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] {
        auto big = cx::ComplexTensor::from_interleaved({3.0e38f, 3.0e38f});
        (void)big.magnitude();
    }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] {
        auto num = cx::ComplexTensor::from_interleaved({2.0e38f, 0.0f});
        auto den = cx::ComplexTensor::from_interleaved({1.0e-38f, 0.0f});
        (void)num.div(den);
    }));

    /* CWE-125 / CWE-787: out-of-range indexing */
    ISO_CHECK(throws(cx::Error::OutOfRange, [&] { (void)t.at(1u); }));
    ISO_CHECK(throws(cx::Error::OutOfRange, [&] { t.set(1u, cx::Complex(0.0f, 0.0f)); }));

    /* CWE-190 / CWE-400: overflow & MAX_SAMPLES cap */
    std::size_t sz = 0;
    ISO_CHECK(!cx::detail::checked_add(std::numeric_limits<std::size_t>::max(), 1u, sz));
    ISO_CHECK(!cx::detail::checked_mul(std::numeric_limits<std::size_t>::max(), 2u, sz));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] { cx::ComplexTensor::zeros(cx::MAX_SAMPLES + 1u); }));
    ISO_CHECK(throws(cx::Error::InvalidInput, [] {
        float x = 1.0f;
        cx::ComplexTensor::from_real(&x, cx::MAX_SAMPLES + 1u);
    }));
}

void test_splitmix64_pbt() {
    std::uint64_t seed = 0xCAFEBABE12345678ULL;
    for (int trial = 0; trial < 40; ++trial) {
        std::size_t n = static_cast<std::size_t>(splitmix64(seed) % 32u) + 1u;
        std::vector<float> ra(n), ia(n), rb(n), ib(n);
        for (std::size_t i = 0; i < n; ++i) {
            ra[i] = rand_unit_f(seed) * 4.0f;
            ia[i] = rand_unit_f(seed) * 4.0f;
            rb[i] = rand_unit_f(seed) * 3.0f + (i % 2u == 0u ? 4.5f : -4.5f);
            ib[i] = rand_unit_f(seed) * 3.0f;
        }
        auto a = cx::ComplexTensor::from_real_imag(ra, ia);
        auto b = cx::ComplexTensor::from_real_imag(rb, ib);

        ISO_CHECK(a.conjugate().conjugate() == a);

        auto prod = a.mul(b);
        auto quot = prod.div(b);
        auto ma = a.magnitude();
        auto mb = b.magnitude();
        auto mp = prod.magnitude();
        for (std::size_t i = 0; i < n; ++i) {
            ISO_CHECK(approx_eq(mp[i], ma[i] * mb[i], 1e-4f));
            ISO_CHECK(approx_eq(quot.at(i).re, a.at(i).re, 1e-4f));
            ISO_CHECK(approx_eq(quot.at(i).im, a.at(i).im, 1e-4f));
        }
    }
}

} // namespace

int main() {
    test_constructors_and_projections();
    test_elementwise_and_indexing();
    test_hostile_cwe_guards();
    test_splitmix64_pbt();
    return ISO_TEST_RESULT();
}
