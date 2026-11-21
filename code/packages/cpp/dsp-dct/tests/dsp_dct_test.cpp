// dsp_dct_test.cpp — unit, hostile-input, and SplitMix64 PBT tests for cpp/dsp-dct.
#include "dsp_dct.hpp"
#include "iso_test.h"

#include <algorithm>
#include <cfloat>
#include <cmath>
#include <cstdint>
#include <limits>
#include <vector>

namespace dd = ca::dsp_dct;

namespace {

constexpr double TEST_PI = 3.14159265358979323846;

void check_close_vec(const std::vector<float>& a, const std::vector<float>& b, double tol) {
    ISO_CHECK_EQ_UINT(a.size(), b.size());
    if (a.size() != b.size()) return;
    for (std::size_t i = 0; i < a.size(); ++i) {
        double da = static_cast<double>(a[i]);
        double db = static_cast<double>(b[i]);
        double scale = std::max({std::abs(da), std::abs(db), 1.0});
        ISO_CHECK_EQ_DBL(da, db, scale * tol);
    }
}

bool all_finite(const std::vector<float>& v) {
    for (float x : v) {
        if (!std::isfinite(x)) return false;
    }
    return true;
}

struct SplitMix64 {
    std::uint64_t state;
    explicit SplitMix64(std::uint64_t seed) : state(seed) {}
    std::uint64_t next() {
        state += 0x9e3779b97f4a7c15ULL;
        std::uint64_t z = state;
        z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL;
        z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL;
        return z ^ (z >> 31);
    }
    float sample_f32() {
        std::uint64_t u = next();
        float m = static_cast<float>(u & 0x00ffffffULL) / static_cast<float>(0x01000000ULL);
        return (m * 2.0f) - 1.0f;
    }
};

template <typename Fn>
dd::Error expect_dct_error(Fn fn) {
    try {
        fn();
        ISO_CHECK_MSG(false, "expected DctError");
        return dd::Error::InvalidInput;
    } catch (const dd::DctError& e) {
        return e.code();
    }
}

void test_closed_form_and_naive() {
    std::vector<float> imp(8, 0.0f);
    imp[0] = 1.0f;
    auto c_imp = dd::dct(imp, dd::DctType::II, dd::DctNorm::None);
    for (std::size_t k = 0; k < 8; ++k) {
        double expected = 2.0 * std::cos(TEST_PI * static_cast<double>(k) / 16.0);
        ISO_CHECK_EQ_DBL(c_imp[k], expected, 1e-5);
    }

    std::vector<float> dc(16, 1.0f);
    auto c_dc = dd::dct(dc, dd::DctType::II, dd::DctNorm::Ortho);
    ISO_CHECK_EQ_DBL(c_dc[0], std::sqrt(16.0), 1e-5);
    for (std::size_t k = 1; k < 16; ++k) {
        ISO_CHECK_EQ_DBL(c_dc[k], 0.0, 1e-5);
    }

    // Naive oracle parity for DCT-II and DCT-III across pow2 and non-pow2
    for (std::size_t n : {2u, 3u, 4u, 5u, 8u, 16u}) {
        std::vector<float> sig(n);
        for (std::size_t m = 0; m < n; ++m) {
            sig[m] = std::sin(0.3f * static_cast<float>(m) - 0.7f);
        }
        std::vector<float> naive_ii(n, 0.0f);
        std::vector<float> naive_iii(n, 0.0f);
        for (std::size_t k = 0; k < n; ++k) {
            double a2 = 0.0;
            double a3 = static_cast<double>(sig[0]);
            for (std::size_t m = 0; m < n; ++m) {
                a2 += static_cast<double>(sig[m]) *
                      std::cos(TEST_PI * static_cast<double>(k) * (2.0 * static_cast<double>(m) + 1.0) /
                               (2.0 * static_cast<double>(n)));
            }
            for (std::size_t m = 1; m < n; ++m) {
                a3 += 2.0 * static_cast<double>(sig[m]) *
                      std::cos(TEST_PI * static_cast<double>(m) * (2.0 * static_cast<double>(k) + 1.0) /
                               (2.0 * static_cast<double>(n)));
            }
            naive_ii[k] = static_cast<float>(2.0 * a2);
            naive_iii[k] = static_cast<float>(a3);
        }
        check_close_vec(dd::dct(sig, dd::DctType::II, dd::DctNorm::None), naive_ii, 1e-4);
        check_close_vec(dd::dct(sig, dd::DctType::III, dd::DctNorm::None), naive_iii, 1e-4);
    }
}

void test_all_types_and_2d() {
    std::vector<float> sig = {1.5f, -2.25f, 3.0f, 0.5f, -1.0f, 4.25f, -0.75f};
    const dd::DctType df[4] = {dd::DctType::I, dd::DctType::II, dd::DctType::III, dd::DctType::IV};
    const dd::DctType di[4] = {dd::DctType::I, dd::DctType::III, dd::DctType::II, dd::DctType::IV};
    const dd::DstType sf[4] = {dd::DstType::I, dd::DstType::II, dd::DstType::III, dd::DstType::IV};
    const dd::DstType si[4] = {dd::DstType::I, dd::DstType::III, dd::DstType::II, dd::DstType::IV};

    for (int i = 0; i < 4; ++i) {
        for (dd::DctNorm nm : {dd::DctNorm::None, dd::DctNorm::Ortho}) {
            auto dc = dd::dct(sig, df[i], nm);
            auto dr = dd::idct(dc, di[i], nm);
            check_close_vec(dr, sig, 1e-4);

            auto sc = dd::dst(sig, sf[i], nm);
            auto sr = dd::idst(sc, si[i], nm);
            check_close_vec(sr, sig, 1e-4);
        }
    }

    // In-place aliasing `out == signal`
    std::vector<float> buf = sig;
    dd::dct_into(buf.data(), buf.size(), dd::DctType::II, dd::DctNorm::Ortho, buf.data());
    dd::idct_into(buf.data(), buf.size(), dd::DctType::III, dd::DctNorm::Ortho, buf.data());
    check_close_vec(buf, sig, 1e-4);

    // 2-D DC block & non-square 3x5
    std::vector<float> dc2d(64, 1.0f);
    auto c2d = dd::dct_2d(dc2d, 8, 8, dd::DctType::II, dd::DctNorm::None);
    ISO_CHECK_EQ_DBL(c2d[0], 256.0, 1e-3);
    for (std::size_t i = 1; i < 64; ++i) ISO_CHECK_EQ_DBL(c2d[i], 0.0, 1e-3);
    check_close_vec(dd::idct_2d(c2d, 8, 8, dd::DctType::III, dd::DctNorm::None), dc2d, 1e-4);

    std::vector<float> img(15);
    for (std::size_t i = 0; i < 15; ++i) img[i] = static_cast<float>(i) * 0.25f - 1.5f;
    check_close_vec(dd::idct_2d(dd::dct_2d(img, 3, 5), 3, 5), img, 1e-4);
    check_close_vec(dd::idst_2d(dd::dst_2d(img, 3, 5), 3, 5), img, 1e-4);

    // Parsers
    ISO_CHECK(dd::parse_dct_type(" iv ") == dd::DctType::IV);
    ISO_CHECK(dd::parse_dst_type("DST-II") == dd::DstType::II);
    ISO_CHECK(dd::parse_norm(" ortho ") == dd::DctNorm::Ortho);
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::parse_dct_type("bad"); })),
                     static_cast<int>(dd::Error::InvalidInput));
}

void test_hostile_inputs() {
    float qnan = std::numeric_limits<float>::quiet_NaN();
    float pinf = std::numeric_limits<float>::infinity();
    float ninf = -pinf;

    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::dct({}); })),
                     static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::idct({}); })),
                     static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::dst({}); })),
                     static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::idst({}); })),
                     static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::dct_2d({}, 4, 4); })),
                     static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct({}, static_cast<dd::DctType>(99), static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::idct({}, static_cast<dd::DctType>(99), static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dst({}, static_cast<dd::DstType>(99), static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::idst({}, static_cast<dd::DstType>(99), static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct_2d({}, 4, 4, static_cast<dd::DctType>(99), static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::EmptyInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct_2d({}, 0, 4, static_cast<dd::DctType>(99), static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::InvalidInput));

    // Invalid enum domains
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct({1.0f, 2.0f}, static_cast<dd::DctType>(99), dd::DctNorm::Ortho);
    })), static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct({1.0f, 2.0f}, dd::DctType::II, static_cast<dd::DctNorm>(99));
    })), static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dst({1.0f, 2.0f}, static_cast<dd::DstType>(0), dd::DctNorm::Ortho);
    })), static_cast<int>(dd::Error::InvalidInput));

    // DCT-I / DST-I require N >= 2
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct({1.0f}, dd::DctType::I, dd::DctNorm::Ortho);
    })), static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dst({1.0f}, dd::DstType::I, dd::DctNorm::Ortho);
    })), static_cast<int>(dd::Error::InvalidInput));

    // Non-finite inputs
    for (float bad : {qnan, pinf, ninf}) {
        ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([bad] {
            dd::dct({1.0f, bad, 3.0f});
        })), static_cast<int>(dd::Error::InvalidInput));
        ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([bad] {
            dd::dst({1.0f, bad, 3.0f});
        })), static_cast<int>(dd::Error::InvalidInput));
        ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([bad] {
            dd::dct_2d({1.0f, bad, 3.0f, 4.0f}, 2, 2);
        })), static_cast<int>(dd::Error::InvalidInput));
    }

    // Contractive N=1 Ortho vs genuine FLT_MAX overflow
    auto ok1 = dd::dct({2.0e38f}, dd::DctType::II, dd::DctNorm::Ortho);
    ISO_CHECK_EQ_DBL(ok1[0], 2.0e38f, 1e32);
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct({FLT_MAX}, dd::DctType::II, dd::DctNorm::None);
    })), static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct({FLT_MAX, FLT_MAX, FLT_MAX, FLT_MAX}, dd::DctType::II, dd::DctNorm::Ortho);
    })), static_cast<int>(dd::Error::InvalidInput));

    // Subnormal stability
    float sub = FLT_MIN / 4.0f;
    auto sub_c = dd::dct({sub, -sub, sub * 2.0f, 0.0f});
    ISO_CHECK(all_finite(sub_c));

    // 2-D dimensions & CWE-190 / CWE-400
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::dct_2d({1.0f}, 0, 1); })),
                     static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::dct_2d({1.0f}, 1, 0); })),
                     static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] { dd::dct_2d({1.0f, 2.0f}, 2, 2); })),
                     static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct_2d({1.0f, 2.0f}, UINT32_MAX, UINT32_MAX);
    })), static_cast<int>(dd::Error::InvalidInput));

    // In-place null & aliased error preservation
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([] {
        dd::dct_into(nullptr, 2, dd::DctType::II, dd::DctNorm::Ortho, nullptr);
    })), static_cast<int>(dd::Error::InvalidInput));
    std::vector<float> aliased_bad = {1.0f, qnan, 3.0f};
    ISO_CHECK_EQ_INT(static_cast<int>(expect_dct_error([&aliased_bad] {
        dd::dct_into(aliased_bad.data(), aliased_bad.size(), dd::DctType::II, dd::DctNorm::Ortho, aliased_bad.data());
    })), static_cast<int>(dd::Error::InvalidInput));
    ISO_CHECK_EQ_DBL(aliased_bad[0], 1.0, 0.0);
    ISO_CHECK_EQ_DBL(aliased_bad[2], 3.0, 0.0);
}

void test_pbt_splitmix64() {
    SplitMix64 rng(0xA1B2C3D4E5F60718ULL);
    for (std::size_t n : {1u, 2u, 3u, 4u, 5u, 7u, 8u, 15u, 23u, 32u}) {
        std::vector<float> sig(n);
        double e_time = 0.0;
        for (std::size_t i = 0; i < n; ++i) {
            sig[i] = rng.sample_f32();
            e_time += static_cast<double>(sig[i]) * static_cast<double>(sig[i]);
        }
        auto check_energy = [e_time](const std::vector<float>& c) {
            double e_freq = 0.0;
            for (float v : c) e_freq += static_cast<double>(v) * static_cast<double>(v);
            ISO_CHECK_EQ_DBL(e_time, e_freq, std::max(e_time, 1.0) * 1e-4);
        };

        auto c2 = dd::dct(sig, dd::DctType::II, dd::DctNorm::Ortho);
        check_close_vec(dd::idct(c2, dd::DctType::III, dd::DctNorm::Ortho), sig, 1e-4);
        check_energy(c2);

        auto c3 = dd::dct(sig, dd::DctType::III, dd::DctNorm::Ortho);
        check_close_vec(dd::idct(c3, dd::DctType::II, dd::DctNorm::Ortho), sig, 1e-4);
        check_energy(c3);

        auto c4 = dd::dct(sig, dd::DctType::IV, dd::DctNorm::Ortho);
        check_close_vec(dd::idct(c4, dd::DctType::IV, dd::DctNorm::Ortho), sig, 1e-4);
        check_energy(c4);

        auto s2 = dd::dst(sig, dd::DstType::II, dd::DctNorm::Ortho);
        check_close_vec(dd::idst(s2, dd::DstType::III, dd::DctNorm::Ortho), sig, 1e-4);
        check_energy(s2);

        auto s3 = dd::dst(sig, dd::DstType::III, dd::DctNorm::Ortho);
        check_close_vec(dd::idst(s3, dd::DstType::II, dd::DctNorm::Ortho), sig, 1e-4);
        check_energy(s3);

        auto s4 = dd::dst(sig, dd::DstType::IV, dd::DctNorm::Ortho);
        check_close_vec(dd::idst(s4, dd::DstType::IV, dd::DctNorm::Ortho), sig, 1e-4);
        check_energy(s4);

        if (n >= 2) {
            auto c1 = dd::dct(sig, dd::DctType::I, dd::DctNorm::Ortho);
            check_close_vec(dd::idct(c1, dd::DctType::I, dd::DctNorm::Ortho), sig, 1e-4);
            check_energy(c1);

            auto s1 = dd::dst(sig, dd::DstType::I, dd::DctNorm::Ortho);
            check_close_vec(dd::idst(s1, dd::DstType::I, dd::DctNorm::Ortho), sig, 1e-4);
            check_energy(s1);
        }
    }

    SplitMix64 rng2(0xCA11AB1ED00DFEEDULL);
    const std::uint32_t dims[6][2] = {{1, 1}, {2, 2}, {3, 4}, {5, 3}, {8, 8}, {7, 6}};
    for (const auto& hw : dims) {
        std::size_t total = static_cast<std::size_t>(hw[0]) * static_cast<std::size_t>(hw[1]);
        std::vector<float> img(total);
        double e_time = 0.0;
        for (std::size_t i = 0; i < total; ++i) {
            img[i] = rng2.sample_f32();
            e_time += static_cast<double>(img[i]) * static_cast<double>(img[i]);
        }
        auto c2d = dd::dct_2d(img, hw[0], hw[1], dd::DctType::II, dd::DctNorm::Ortho);
        check_close_vec(dd::idct_2d(c2d, hw[0], hw[1], dd::DctType::III, dd::DctNorm::Ortho), img, 1e-4);
        double e_freq = 0.0;
        for (float v : c2d) e_freq += static_cast<double>(v) * static_cast<double>(v);
        ISO_CHECK_EQ_DBL(e_time, e_freq, std::max(e_time, 1.0) * 1e-4);
    }

    // Large-N O(N log N) verification for all 8 transforms (N = 16384)
    {
        std::vector<float> big(16384);
        for (float& x : big) x = rng.sample_f32();
        auto check_prefix = [&big](const std::vector<float>& r) {
            for (std::size_t i = 0; i < 64; ++i) {
                ISO_CHECK_EQ_DBL(r[i], big[i], 1e-3);
            }
        };
        check_prefix(dd::idct(dd::dct(big, dd::DctType::I), dd::DctType::I));
        check_prefix(dd::idct(dd::dct(big, dd::DctType::IV), dd::DctType::IV));
        check_prefix(dd::idst(dd::dst(big, dd::DstType::I), dd::DstType::I));
        check_prefix(dd::idst(dd::dst(big, dd::DstType::II), dd::DstType::III));
        check_prefix(dd::idst(dd::dst(big, dd::DstType::IV), dd::DstType::IV));
    }
}

} // namespace

int main() {
    test_closed_form_and_naive();
    test_all_types_and_2d();
    test_hostile_inputs();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
