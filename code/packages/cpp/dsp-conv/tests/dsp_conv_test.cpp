// dsp_conv_test.cpp — unit, Family-1/4 invariant, hostile-input, and SplitMix64 PBT
// tests for cpp/dsp-conv (pure ISO C++17).
#include "dsp_conv.hpp"
#include "iso_test.h"

#include <cfloat>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <string>
#include <vector>

namespace dc = ca::dsp_conv;

namespace {

float af(float x) { return x < 0.0f ? -x : x; }

bool approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

void check_close(const std::vector<float>& a, const std::vector<float>& b, float tol) {
    ISO_CHECK_EQ_UINT(a.size(), b.size());
    for (std::size_t i = 0; i < a.size(); ++i) {
        ISO_CHECK(approx_eq(a[i], b[i], tol));
    }
}

template <typename F>
bool throws_err(dc::Error expected, F&& fn) {
    try {
        fn();
    } catch (const dc::ConvError& e) {
        return e.code() == expected;
    } catch (...) {
        return false;
    }
    return false;
}

struct SplitMix64 {
    std::uint64_t state;
    explicit SplitMix64(std::uint64_t seed) : state(seed) {}
    std::uint64_t next() {
        state += UINT64_C(0x9E3779B97F4A7C15);
        std::uint64_t z = state;
        z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
        return z ^ (z >> 31);
    }
    float sample(float min_v, float max_v) {
        double unit = static_cast<double>(next() >> 11) / 9007199254740992.0;
        return static_cast<float>(static_cast<double>(min_v) +
                                  (static_cast<double>(max_v) - static_cast<double>(min_v)) * unit);
    }
};

void test_index_and_parsing() {
    ISO_CHECK(dc::extend_index(2, 5, dc::BoundaryMode::Zero).value() == 2u);
    ISO_CHECK(!dc::extend_index(-1, 5, dc::BoundaryMode::Zero).has_value());
    ISO_CHECK(dc::extend_index(-3, 5, dc::BoundaryMode::Replicate).value() == 0u);
    ISO_CHECK(dc::extend_index(7, 5, dc::BoundaryMode::Replicate).value() == 4u);
    ISO_CHECK(dc::extend_index(-1, 5, dc::BoundaryMode::Wrap).value() == 4u);
    ISO_CHECK(dc::extend_index(6, 5, dc::BoundaryMode::Wrap).value() == 1u);
    ISO_CHECK(dc::extend_index(-1, 4, dc::BoundaryMode::Reflect).value() == 1u);
    ISO_CHECK(dc::extend_index(4, 4, dc::BoundaryMode::Reflect).value() == 2u);
    ISO_CHECK(dc::extend_index(3, 1, dc::BoundaryMode::Reflect).value() == 0u);

    std::vector<float> sig = {10.0f, 20.0f, 30.0f};
    ISO_CHECK(approx_eq(dc::sample(sig, -1, dc::BoundaryMode::Zero), 0.0f, 1e-7f));
    ISO_CHECK(approx_eq(dc::sample(sig, -1, dc::BoundaryMode::Replicate), 10.0f, 1e-7f));
    ISO_CHECK(approx_eq(dc::sample(sig, 3, dc::BoundaryMode::Replicate), 30.0f, 1e-7f));

    ISO_CHECK_EQ_UINT(dc::next_pow2(0), 1);
    ISO_CHECK_EQ_UINT(dc::next_pow2(3), 4);
    ISO_CHECK_EQ_UINT(dc::next_pow2(100), 128);

    ISO_CHECK(dc::parse_boundary_mode("  Reflect \n") == dc::BoundaryMode::Reflect);
    ISO_CHECK(dc::parse_conv_mode(" VALID ") == dc::ConvMode::Valid);
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] { dc::parse_boundary_mode("zero extra"); }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] { dc::parse_conv_mode("full!"); }));
}

void test_conv1d_fft_and_ola() {
    std::vector<float> sig = {1.0f, -2.0f, 3.5f, 0.5f, -1.0f, 2.0f, 4.0f, -3.0f};
    std::vector<float> ker = {0.1f, 0.2f, 0.4f, 0.2f, 0.1f};

    for (auto mode : {dc::ConvMode::Full, dc::ConvMode::Same, dc::ConvMode::Valid}) {
        auto d = dc::convolve(sig, ker, mode, dc::BoundaryMode::Zero);
        auto f = dc::convolve_fft(sig, ker, mode);
        auto f_alias = dc::fft_convolve(sig, ker, mode);
        auto o_auto = dc::overlap_add(sig, ker, std::nullopt, mode);
        auto o_b4 = dc::overlap_add(sig, ker, std::size_t(4), mode);
        check_close(d, f, 1e-5f);
        check_close(d, f_alias, 1e-5f);
        check_close(d, o_auto, 1e-5f);
        check_close(d, o_b4, 1e-5f);
    }

    auto corr = dc::correlate({1.0f, 2.0f, 3.0f, 4.0f, 5.0f}, {2.0f, 3.0f}, dc::ConvMode::Valid);
    check_close(corr, {8.0f, 13.0f, 18.0f, 23.0f}, 1e-6f);
}

void test_2d_and_kernels() {
    std::vector<float> img = {1, 2, 3, 4, 5, 6, 7, 8, 9};
    std::vector<float> h_ker = {0.25f, 0.5f, 0.25f};
    std::vector<float> v_ker = {0.2f, 0.6f, 0.2f};
    std::vector<float> outer(9);
    for (std::size_t r = 0; r < 3; ++r) {
        for (std::size_t c = 0; c < 3; ++c) {
            outer[r * 3 + c] = v_ker[r] * h_ker[c];
        }
    }
    auto r2d = dc::conv2d(img, outer, 3, 3, 3, 3, dc::BoundaryMode::Replicate);
    auto rsep = dc::sep_conv2d(img, h_ker, v_ker, 3, 3, dc::BoundaryMode::Replicate);
    check_close(r2d, rsep, 1e-5f);

    std::vector<float> k2x2 = {0.0f, 1.0f, 1.0f, 0.0f};
    auto rfull = dc::convolve_2d(img, k2x2, 3, 3, 2, 2, dc::ConvMode::Full, dc::BoundaryMode::Zero);
    ISO_CHECK_EQ_UINT(rfull.size(), 16);
    auto rvalid = dc::convolve2d(img, k2x2, 3, 3, 2, 2, dc::ConvMode::Valid, dc::BoundaryMode::Zero);
    ISO_CHECK_EQ_UINT(rvalid.size(), 4);

    auto gk = dc::gaussian_blur_kernel(1.5f, 9u);
    ISO_CHECK_EQ_UINT(gk.size(), 9);
    double gsum = 0.0;
    for (float v : gk) gsum += v;
    ISO_CHECK(approx_eq(static_cast<float>(gsum), 1.0f, 1e-5f));
    auto sh = dc::sharpen_kernel(1.5f);
    double ssum = 0.0;
    for (float v : sh) ssum += v;
    ISO_CHECK(approx_eq(static_cast<float>(ssum), 1.0f, 1e-5f));
}

void test_hostile_and_cwe() {
    float nan_v = std::numeric_limits<float>::quiet_NaN();
    float inf_v = std::numeric_limits<float>::infinity();
    float ninf_v = -std::numeric_limits<float>::infinity();
    float sub_v = std::numeric_limits<float>::denorm_min();

    ISO_CHECK(throws_err(dc::Error::EmptySignal, [] { dc::conv1d({}, {1.0f}); }));
    ISO_CHECK(throws_err(dc::Error::EmptyKernel, [] { dc::conv1d({1.0f}, {}); }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] {
        dc::conv1d({1.0f}, {1.0f}, static_cast<dc::BoundaryMode>(99));
    }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] {
        dc::convolve({1.0f}, {1.0f}, static_cast<dc::ConvMode>(99));
    }));

    for (float bad : {nan_v, inf_v, ninf_v}) {
        ISO_CHECK(throws_err(dc::Error::InvalidInput, [=] { dc::conv1d({1.0f, bad}, {1.0f}); }));
        ISO_CHECK(throws_err(dc::Error::InvalidInput, [=] { dc::convolve_fft({1.0f}, {bad}); }));
        ISO_CHECK(throws_err(dc::Error::InvalidInput, [=] { dc::overlap_add({bad}, {1.0f}); }));
        ISO_CHECK(throws_err(dc::Error::InvalidInput, [=] { dc::gaussian_blur_kernel(bad, 3u); }));
        ISO_CHECK(throws_err(dc::Error::InvalidInput, [=] { dc::sharpen_kernel(bad); }));
    }

    ISO_CHECK(throws_err(dc::Error::InvalidInput, [=] { dc::gaussian_blur_kernel(sub_v, 5u); }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] { dc::gaussian_blur_kernel(FLT_MAX, 5u); }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] { dc::gaussian_blur_kernel(1.0f, 4u); }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] { dc::sharpen_kernel(FLT_MAX); }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] {
        dc::conv1d({FLT_MAX, FLT_MAX}, {2.0f, 2.0f}, dc::BoundaryMode::Replicate);
    }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] {
        dc::overlap_add({1.0f, 2.0f}, {1.0f}, std::size_t(0));
    }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] {
        dc::overlap_add({1.0f, 2.0f}, {1.0f, 2.0f, 3.0f}, std::size_t(0), dc::ConvMode::Valid);
    }));
    ISO_CHECK(throws_err(dc::Error::ImageSizeMismatch, [] {
        dc::convolve_2d({}, {1.0f}, 0u, 2u, 1u, 1u, static_cast<dc::ConvMode>(99));
    }));
    ISO_CHECK(throws_err(dc::Error::EmptyKernel, [] {
        dc::convolve_2d({1.0f, 2.0f, 3.0f, 4.0f}, {}, 2u, 2u, 0u, 1u, static_cast<dc::ConvMode>(99));
    }));
    ISO_CHECK(throws_err(dc::Error::KernelTooLarge, [] {
        dc::convolve({1.0f, 2.0f}, {1.0f, 2.0f, 3.0f}, dc::ConvMode::Valid);
    }));
    ISO_CHECK(throws_err(dc::Error::ImageSizeMismatch, [] {
        dc::conv2d({1.0f, 2.0f}, {1.0f}, 100u, 100u, 1u, 1u);
    }));
    ISO_CHECK(throws_err(dc::Error::InvalidInput, [] {
        dc::conv2d({1.0f}, {1.0f}, UINT32_MAX, UINT32_MAX, 1u, 1u);
    }) || throws_err(dc::Error::ImageSizeMismatch, [] {
        dc::conv2d({1.0f}, {1.0f}, UINT32_MAX, UINT32_MAX, 1u, 1u);
    }));
}

void test_pbt_splitmix64_invariants() {
    SplitMix64 rng(UINT64_C(0xCAFEBABE01234567));
    for (int iter = 0; iter < 15; ++iter) {
        std::size_t len_a = 4u + static_cast<std::size_t>(rng.next() % 24u);
        std::size_t len_b = 3u + static_cast<std::size_t>(rng.next() % 15u);
        std::vector<float> a(len_a), b(len_b);
        for (float& v : a) v = rng.sample(-5.0f, 5.0f);
        for (float& v : b) v = rng.sample(-5.0f, 5.0f);

        auto ab = dc::convolve(a, b, dc::ConvMode::Full);
        auto ba = dc::convolve(b, a, dc::ConvMode::Full);
        auto fft_ab = dc::convolve_fft(a, b, dc::ConvMode::Full);
        auto ola_ab = dc::overlap_add(a, b, std::nullopt, dc::ConvMode::Full);

        check_close(ab, ba, 1e-4f);
        check_close(ab, fft_ab, 1e-4f);
        check_close(ab, ola_ab, 1e-4f);
    }
}

} // namespace

int main() {
    test_index_and_parsing();
    test_conv1d_fft_and_ola();
    test_2d_and_kernels();
    test_hostile_and_cwe();
    test_pbt_splitmix64_invariants();
    return ISO_TEST_RESULT();
}
