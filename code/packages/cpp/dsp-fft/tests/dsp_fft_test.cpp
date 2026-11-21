// dsp_fft_test.cpp — Unit, hostile-input, invariant, and SplitMix64 PBT tests for cpp/dsp-fft.
#include "dsp_fft.hpp"
#include "iso_test.h"

#include <cmath>
#include <cstdint>
#include <limits>
#include <vector>

namespace cx = ca::dsp_complex;
namespace fft = ca::dsp_fft;

namespace {

float af(float x) { return x < 0.0f ? -x : x; }

bool approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

template <typename F>
bool throws(fft::Error expected, F&& fn) {
    try {
        fn();
    } catch (const fft::FftError& e) {
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

std::vector<float> naive_dft(const std::vector<float>& interleaved, bool inverse) {
    constexpr double TWO_PI = 6.28318530717958647692;
    std::size_t n = interleaved.size() / 2u;
    double sign = inverse ? 1.0 : -1.0;
    double scale = inverse ? (1.0 / static_cast<double>(n)) : 1.0;
    std::vector<float> out(2u * n);
    for (std::size_t k = 0u; k < n; ++k) {
        double sum_re = 0.0;
        double sum_im = 0.0;
        for (std::size_t j = 0u; j < n; ++j) {
            double theta = sign * TWO_PI * static_cast<double>(k * j) / static_cast<double>(n);
            double c = std::cos(theta);
            double s = std::sin(theta);
            double xr = static_cast<double>(interleaved[2u * j]);
            double xi = static_cast<double>(interleaved[2u * j + 1u]);
            sum_re += xr * c - xi * s;
            sum_im += xr * s + xi * c;
        }
        out[2u * k] = static_cast<float>(sum_re * scale);
        out[2u * k + 1u] = static_cast<float>(sum_im * scale);
    }
    return out;
}

void test_closed_form_vectors() {
    for (std::size_t n : {std::size_t{1}, std::size_t{5}, std::size_t{8}, std::size_t{12}}) {
        std::vector<float> imp(n, 0.0f);
        imp[0] = 1.0f;
        auto spec = fft::fft(imp);
        ISO_CHECK_EQ_UINT(spec.len(), n);
        for (std::size_t k = 0u; k < n; ++k) {
            ISO_CHECK(approx_eq(spec.at(k).re, 1.0f, 1e-4f));
            ISO_CHECK(approx_eq(spec.at(k).im, 0.0f, 1e-4f));
        }

        std::vector<float> dc(n, 1.0f);
        auto dc_spec = fft::fft(dc);
        ISO_CHECK(approx_eq(dc_spec.at(0u).re, static_cast<float>(n), 1e-4f));
        ISO_CHECK(approx_eq(dc_spec.at(0u).im, 0.0f, 1e-4f));
        for (std::size_t k = 1u; k < n; ++k) {
            ISO_CHECK(approx_eq(dc_spec.at(k).re, 0.0f, 1e-3f));
            ISO_CHECK(approx_eq(dc_spec.at(k).im, 0.0f, 1e-3f));
        }
    }

    constexpr double TWO_PI = 6.28318530717958647692;
    std::vector<float> cos_sig(8u);
    for (std::size_t i = 0u; i < 8u; ++i) {
        cos_sig[i] = static_cast<float>(std::cos(TWO_PI * static_cast<double>(i) / 8.0));
    }
    auto spec = fft::fft(cos_sig);
    ISO_CHECK(approx_eq(spec.at(1u).re, 4.0f, 1e-5f) && approx_eq(spec.at(1u).im, 0.0f, 1e-5f));
    ISO_CHECK(approx_eq(spec.at(7u).re, 4.0f, 1e-5f) && approx_eq(spec.at(7u).im, 0.0f, 1e-5f));
}

void test_dft_oracle_and_roundtrips() {
    for (std::size_t n = 1u; n <= 32u; ++n) {
        std::vector<float> sig(2u * n);
        std::vector<float> rsig(n);
        for (std::size_t i = 0u; i < n; ++i) {
            double s = std::sin(static_cast<double>(i) * 0.37);
            double c = std::cos(static_cast<double>(i) * 0.37) * 0.5;
            sig[2u * i] = static_cast<float>(s);
            sig[2u * i + 1u] = static_cast<float>(c);
            rsig[i] = static_cast<float>(s);
        }
        auto oracle = naive_dft(sig, false);
        auto spec = fft::fft(sig, true);
        auto recon = fft::ifft(spec);
        for (std::size_t i = 0u; i < 2u * n; ++i) {
            ISO_CHECK(approx_eq(spec.as_interleaved()[i], oracle[i], 1e-3f));
            ISO_CHECK(approx_eq(recon.as_interleaved()[i], sig[i], 1e-3f));
        }

        auto half = fft::rfft(rsig);
        ISO_CHECK_EQ_UINT(half.len(), n / 2u + 1u);
        auto rrec = fft::irfft(half, static_cast<std::uint32_t>(n));
        ISO_CHECK_EQ_UINT(rrec.size(), n);
        for (std::size_t i = 0u; i < n; ++i) {
            ISO_CHECK(approx_eq(rrec[i], rsig[i], 1e-3f));
        }
    }
}

void test_family_invariants() {
    for (std::size_t n : {std::size_t{16}, std::size_t{13}}) {
        std::vector<float> x(2u * n), y(2u * n), xy(2u * n);
        constexpr float alpha = 1.5f;
        constexpr float beta = -0.75f;
        double e_time = 0.0;
        for (std::size_t i = 0u; i < n; ++i) {
            x[2u * i] = static_cast<float>(i + 1u) * 0.25f;
            x[2u * i + 1u] = -0.5f;
            y[2u * i] = 1.0f - static_cast<float>(i) * 0.1f;
            y[2u * i + 1u] = 0.3f * static_cast<float>(i);
            xy[2u * i] = alpha * x[2u * i] + beta * y[2u * i];
            xy[2u * i + 1u] = alpha * x[2u * i + 1u] + beta * y[2u * i + 1u];
            e_time += static_cast<double>(x[2u * i]) * x[2u * i] +
                      static_cast<double>(x[2u * i + 1u]) * x[2u * i + 1u];
        }
        auto fx = fft::fft(x, true);
        auto fy = fft::fft(y, true);
        auto fxy = fft::fft(xy, true);
        double e_freq = 0.0;
        for (std::size_t i = 0u; i < n; ++i) {
            float lre = alpha * fx.at(i).re + beta * fy.at(i).re;
            float lim = alpha * fx.at(i).im + beta * fy.at(i).im;
            ISO_CHECK(approx_eq(fxy.at(i).re, lre, 1e-3f));
            ISO_CHECK(approx_eq(fxy.at(i).im, lim, 1e-3f));
            e_freq += static_cast<double>(fx.at(i).re) * fx.at(i).re +
                      static_cast<double>(fx.at(i).im) * fx.at(i).im;
        }
        ISO_CHECK(approx_eq(static_cast<float>(e_time),
                            static_cast<float>(e_freq / static_cast<double>(n)), 1e-3f));
    }
}

void test_hostile_cwe_guards() {
    const float qnan = std::numeric_limits<float>::quiet_NaN();
    const float inf = std::numeric_limits<float>::infinity();
    const float ninf = -std::numeric_limits<float>::infinity();

    /* CWE-476: nullptr */
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::fft(nullptr, 4u, false); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::fft_scalar(nullptr, 4u); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::rfft(nullptr, 4u); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::irfft_scalar(nullptr, 4u, 2u); }));

    /* CWE-20: zero, odd, non-pow2, non-finite, irfft mismatch */
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::fft(std::vector<float>{}); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::fft({1.0f, 2.0f, 3.0f}, true); }));
    ISO_CHECK(throws(fft::Error::NotPowerOfTwo, [] {
        fft::fft_scalar({1.0f, 0.0f, 2.0f, 0.0f, 3.0f, 0.0f});
    }));
    ISO_CHECK(throws(fft::Error::NotPowerOfTwo, [] {
        fft::ifft_scalar({1.0f, 0.0f, 2.0f, 0.0f, 3.0f, 0.0f});
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [&] { fft::fft({qnan, 0.0f}); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [&] { fft::fft({inf, 0.0f}); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [&] { fft::fft({ninf, 0.0f}); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [&] { fft::rfft({inf, 0.0f}); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] { fft::ifft(cx::ComplexTensor()); }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::irfft_scalar({1.0f, 0.0f, 2.0f, 0.0f}, 0u);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::irfft_scalar({1.0f, 0.0f, 2.0f, 0.0f}, 8u);
    }));

    /* CWE-190 / CWE-400: bounds */
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        float x = 1.0f;
        fft::fft(&x, fft::MAX_FFT_LEN + 1u, false);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        float x[2] = {0.0f, 0.0f};
        fft::bluestein_scalar(x, ((fft::MAX_FFT_LEN / 2u) + 1u) * 2u, fft::Direction::Forward);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        float x[2] = {0.0f, 0.0f};
        fft::fft(x, ((fft::MAX_FFT_LEN / 2u) + 1u) * 2u, true);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        float x[2] = {0.0f, 0.0f};
        fft::fft(x, (fft::MAX_FFT_LEN / 2u) + 1u, false);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        float x[2] = {0.0f, 0.0f};
        fft::rfft_scalar(x, (fft::MAX_FFT_LEN / 2u) + 1u);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        float x[2] = {0.0f, 0.0f};
        fft::irfft_scalar(x, (((fft::MAX_FFT_LEN / 2u) + 1u) / 2u + 1u) * 2u,
                          static_cast<std::uint32_t>((fft::MAX_FFT_LEN / 2u) + 1u));
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::irfft_scalar({1.0f, 0.0f, 2.0f, 0.0f},
                          static_cast<std::uint32_t>(fft::MAX_FFT_LEN + 1u));
    }));

    /* CWE-20: invalid direction enum and near-FLT_MAX arithmetic overflow throw FftError (never ComplexError) */
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::bluestein_scalar({1.0f, 0.0f, 2.0f, 0.0f, 3.0f, 0.0f},
                              static_cast<fft::Direction>(99));
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::fft_scalar({2.0e38f, 0.0f, 2.0e38f, 0.0f});
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::bluestein_scalar({2.0e38f, 0.0f, 2.0e38f, 0.0f, 2.0e38f, 0.0f},
                              fft::Direction::Forward);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::fft({2.0e38f, 0.0f, 2.0e38f, 0.0f}, true);
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::rfft({2.0e38f, 2.0e38f});
    }));
    ISO_CHECK(throws(fft::Error::InvalidInput, [] {
        fft::irfft_scalar({2.0e38f, 0.0f, 2.0e38f, 0.0f}, 2u);
    }));
}

void test_splitmix64_pbt() {
    std::uint64_t seed = 0x13579BDF2468ACE0ULL;
    for (int trial = 0; trial < 32; ++trial) {
        std::size_t n = static_cast<std::size_t>(splitmix64(seed) % 45u) + 1u;
        std::vector<float> sig(2u * n);
        std::vector<float> rsig(n);
        double e_time = 0.0;
        for (std::size_t i = 0u; i < n; ++i) {
            sig[2u * i] = rand_unit_f(seed) * 2.0f;
            sig[2u * i + 1u] = rand_unit_f(seed) * 2.0f;
            rsig[i] = sig[2u * i];
            e_time += static_cast<double>(sig[2u * i]) * sig[2u * i] +
                      static_cast<double>(sig[2u * i + 1u]) * sig[2u * i + 1u];
        }
        auto oracle = naive_dft(sig, false);
        auto spec = fft::fft(sig, true);
        auto recon = fft::ifft(spec);
        double e_freq = 0.0;
        for (std::size_t i = 0u; i < n; ++i) {
            ISO_CHECK(approx_eq(spec.at(i).re, oracle[2u * i], 1e-3f));
            ISO_CHECK(approx_eq(spec.at(i).im, oracle[2u * i + 1u], 1e-3f));
            ISO_CHECK(approx_eq(recon.at(i).re, sig[2u * i], 1e-3f));
            ISO_CHECK(approx_eq(recon.at(i).im, sig[2u * i + 1u], 1e-3f));
            e_freq += static_cast<double>(spec.at(i).re) * spec.at(i).re +
                      static_cast<double>(spec.at(i).im) * spec.at(i).im;
        }
        ISO_CHECK(approx_eq(static_cast<float>(e_time),
                            static_cast<float>(e_freq / static_cast<double>(n)), 1e-3f));

        auto half = fft::rfft(rsig);
        auto rrec = fft::irfft(half, static_cast<std::uint32_t>(n));
        for (std::size_t i = 0u; i < n; ++i) {
            ISO_CHECK(approx_eq(rrec[i], rsig[i], 1e-3f));
        }
    }
}

} // namespace

int main() {
    test_closed_form_vectors();
    test_dft_oracle_and_roundtrips();
    test_family_invariants();
    test_hostile_cwe_guards();
    test_splitmix64_pbt();
    return ISO_TEST_RESULT();
}
