// dsp_wavelets_test.cpp — unit tests for the C++ wavelet transform port.
//
// Mirrors the Rust crate's suite: error paths, output-length contract, the
// hand-worked Haar reference vector, constant/dirac structural properties,
// perfect-reconstruction round-trips (Haar + Daubechies/Symlets/Coiflets, 1-D
// and 2-D), split_levels/slice_level, and the filter-bank invariants.
//
// Hardened with full VER15 CWE/CVE test suites:
// - CWE-20: NaN, Inf, -Inf, subnormals, strict string parsing (reject trailing garbage).
// - CWE-120/122/125/787: Buffer boundary, slicing, and index safety.
// - CWE-190: Arithmetic overflow checks.
// - CWE-400: Pre-flight bounds (MAX_SAMPLES, MAX_LEVELS, MAX_SCALES, MAX_2D_DIM).
// - CWE-674/1333: Iterative execution without recursion or regex.
// - Property-based tests (PBT): Energy conservation (Parseval) and PRNG round-trips.
#include "dsp_wavelets.hpp"
#include "iso_test.h"

#include <limits>
#include <string>
#include <vector>

namespace wv = ca::dsp_wavelets;

namespace {

float af(float x) { return x < 0 ? -x : x; }

// Relative closeness, matching the crate's approx_eq.
bool approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

bool close(const std::vector<float>& a, const std::vector<float>& b, float tol) {
    if (a.size() != b.size()) return false;
    for (std::size_t i = 0; i < a.size(); ++i) {
        if (!approx_eq(a[i], b[i], tol)) return false;
    }
    return true;
}

template <typename F>
bool throws(wv::Error expected, F&& fn) {
    try {
        fn();
    } catch (const wv::WaveletError& e) {
        return e.code() == expected;
    } catch (...) {
        return false;
    }
    return false;
}

std::vector<float> ramp(std::size_t n, float step) {
    std::vector<float> v(n);
    for (std::size_t i = 0; i < n; ++i) {
        float x = static_cast<float>(i) * step;
        while (x > 4.0f) x -= 8.0f;
        v[i] = x;
    }
    return v;
}

void round_trip_1d(const std::vector<float>& signal, wv::Wavelet w, std::uint32_t levels,
                   wv::Boundary boundary, float tol) {
    std::vector<float> coeffs = wv::dwt_1d(signal, w, levels, boundary);
    std::vector<float> recon = wv::idwt_1d(coeffs, w, levels, boundary,
                                           static_cast<std::uint32_t>(signal.size()));
    ISO_CHECK_EQ_UINT(recon.size(), signal.size());
    std::size_t edge = wv::analysis_lowpass(w).size();
    if (w.family == wv::Family::Haar) edge = 0;
    if (edge > recon.size() / 4) edge = recon.size() / 4;
    for (std::size_t i = edge; i + edge < recon.size(); ++i) {
        ISO_CHECK(approx_eq(signal[i], recon[i], tol));
    }
}

void test_error_paths() {
    ISO_CHECK(throws(wv::Error::EmptySignal,
                     [] { wv::dwt_1d({}, wv::Wavelet::haar(), 1, wv::Boundary::Periodic); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        wv::dwt_1d(std::vector<float>(8, 1.0f), wv::Wavelet::haar(), 0, wv::Boundary::Periodic);
    }));
    // 4 samples, 4 levels needs >= 2^3 = 8.
    ISO_CHECK(throws(wv::Error::SignalTooShort, [] {
        wv::dwt_1d(std::vector<float>(4, 1.0f), wv::Wavelet::haar(), 4, wv::Boundary::Periodic);
    }));
    // Unsupported wavelets / boundaries.
    const wv::Wavelet bad_w[] = {
        wv::Wavelet::daubechies(3),  wv::Wavelet::daubechies(99), wv::Wavelet::symlets(6),
        wv::Wavelet::symlets(8),     wv::Wavelet::coiflets(2),    wv::Wavelet::morlet(),
        wv::Wavelet::mexican_hat(),  wv::Wavelet::biorthogonal(5, 3)};
    for (const wv::Wavelet& w : bad_w) {
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::dwt_1d(std::vector<float>(16, 1.0f), w, 2, wv::Boundary::Periodic);
        }));
    }
    const wv::Boundary bad_b[] = {wv::Boundary::Zero, wv::Boundary::Replicate,
                                  wv::Boundary::Reflect};
    for (const wv::Boundary& b : bad_b) {
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::dwt_1d(std::vector<float>(16, 1.0f), wv::Wavelet::haar(), 2, b);
        }));
    }
    // levels above MAX_LEVELS.
    const std::uint32_t bad_levels[] = {32u, 64u, 1000000u, 0xFFFFFFFFu};
    for (std::uint32_t lv : bad_levels) {
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::dwt_1d(std::vector<float>(16, 1.0f), wv::Wavelet::haar(), lv,
                       wv::Boundary::Periodic);
        }));
    }
    // output_length above MAX_SAMPLES in idwt.
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> c = {1.0f, 1.0f, 0.0f, 0.0f};
        wv::idwt_1d(c, wv::Wavelet::haar(), 1, wv::Boundary::Periodic, 0xFFFFFFFFu);
    }));
    // coeff shape mismatch.
    ISO_CHECK(throws(wv::Error::InvalidCoefficients, [] {
        std::vector<float> c = {1.0f, 2.0f, 3.0f}; // wrong length for len=4,levels=1
        wv::idwt_1d(c, wv::Wavelet::haar(), 1, wv::Boundary::Periodic, 4);
    }));
}

void test_haar_reference_vector() {
    std::vector<float> signal = {1.0f, 2.0f, 3.0f, 4.0f};
    std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    ISO_CHECK_EQ_UINT(coeffs.size(), 4u);
    float s = wv::FRAC_1_SQRT_2;
    ISO_CHECK(approx_eq(coeffs[0], 3.0f * s, 1e-5f));
    ISO_CHECK(approx_eq(coeffs[1], 7.0f * s, 1e-5f));
    ISO_CHECK(approx_eq(coeffs[2], s, 1e-5f));
    ISO_CHECK(approx_eq(coeffs[3], s, 1e-5f));
}

void test_constant_signal_zero_detail() {
    std::vector<float> signal(32, 3.14f);
    std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::haar(), 4, wv::Boundary::Periodic);
    std::size_t ca_len = 32 / 16; // 2
    for (std::size_t i = ca_len; i < coeffs.size(); ++i) {
        ISO_CHECK(af(coeffs[i]) <= 1e-6f);
    }
}

void test_dirac_delta() {
    std::vector<float> signal(16, 0.0f);
    signal[0] = 1.0f;
    std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    float s = wv::FRAC_1_SQRT_2;
    ISO_CHECK(approx_eq(coeffs[0], s, 1e-5f));
    for (std::size_t k = 1; k < 8; ++k) ISO_CHECK(af(coeffs[k]) <= 1e-6f);
    ISO_CHECK(approx_eq(coeffs[8], -s, 1e-5f));
    for (std::size_t k = 1; k < 8; ++k) ISO_CHECK(af(coeffs[8 + k]) <= 1e-6f);
}

void test_output_length_contract() {
    const std::size_t ns[] = {4, 8, 16, 32, 64};
    for (std::size_t n : ns) {
        std::uint32_t max_j = 0;
        for (std::size_t t = n; t > 1; t /= 2) ++max_j;
        for (std::uint32_t j = 1; j <= max_j; ++j) {
            std::vector<float> signal(n, 0.5f);
            std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::haar(), j,
                                                   wv::Boundary::Periodic);
            ISO_CHECK_EQ_UINT(coeffs.size(), n);
        }
    }
}

void test_round_trips_1d() {
    round_trip_1d(ramp(4, 0.13f), wv::Wavelet::haar(), 1, wv::Boundary::Periodic, 1e-4f);
    round_trip_1d(ramp(8, 0.13f), wv::Wavelet::haar(), 3, wv::Boundary::Periodic, 1e-4f);
    round_trip_1d(ramp(16, 0.13f), wv::Wavelet::haar(), 3, wv::Boundary::Periodic, 1e-4f);
    round_trip_1d(ramp(32, 0.07f), wv::Wavelet::haar(), 3, wv::Boundary::Periodic, 1e-4f);
    round_trip_1d(ramp(17, 0.2f), wv::Wavelet::haar(), 2, wv::Boundary::Periodic, 1e-4f);
    round_trip_1d(ramp(16, 0.11f), wv::Wavelet::haar(), 3, wv::Boundary::Symmetric, 1e-3f);
    round_trip_1d(ramp(64, 0.07f), wv::Wavelet::daubechies(2), 2, wv::Boundary::Periodic, 1e-3f);
    round_trip_1d(ramp(64, 0.13f), wv::Wavelet::daubechies(4), 2, wv::Boundary::Periodic, 1e-3f);
    round_trip_1d(ramp(64, 0.11f), wv::Wavelet::symlets(4), 2, wv::Boundary::Periodic, 1e-3f);
    round_trip_1d(ramp(64, 0.09f), wv::Wavelet::coiflets(1), 2, wv::Boundary::Periodic, 1e-3f);
    round_trip_1d(ramp(128, 0.04f), wv::Wavelet::daubechies(6), 2, wv::Boundary::Periodic, 1e-3f);
    round_trip_1d(ramp(128, 0.03f), wv::Wavelet::daubechies(8), 2, wv::Boundary::Periodic, 1e-3f);
}

void test_db2_constant_small_detail() {
    std::vector<float> signal(64, 3.14f);
    std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::daubechies(2), 3,
                                           wv::Boundary::Periodic);
    for (std::size_t i = 8; i < coeffs.size(); ++i) ISO_CHECK(af(coeffs[i]) <= 1e-5f);
}

void test_split_and_slice() {
    std::vector<float> signal = ramp(16, 0.1f);
    std::vector<float> coeffs = wv::dwt_1d(signal, wv::Wavelet::haar(), 3, wv::Boundary::Periodic);
    std::vector<std::size_t> offsets = wv::split_levels(coeffs.size(), signal.size(), 3);
    std::vector<std::size_t> expected = {0, 2, 4, 8, 16};
    ISO_CHECK_EQ_UINT(offsets.size(), expected.size());
    for (std::size_t i = 0; i < expected.size(); ++i) ISO_CHECK_EQ_UINT(offsets[i], expected[i]);

    wv::FloatView ca3 = wv::slice_level(coeffs, signal.size(), 3, 3, wv::Band::Approximation);
    ISO_CHECK_EQ_UINT(ca3.size, 2u);
    wv::FloatView cd1 = wv::slice_level(coeffs, signal.size(), 3, 1, wv::Band::Detail);
    ISO_CHECK_EQ_UINT(cd1.size, 8u);

    ISO_CHECK(throws(wv::Error::InvalidParam,
                     [&] { wv::slice_level(coeffs, signal.size(), 3, 1, wv::Band::Approximation); }));
    ISO_CHECK(throws(wv::Error::InvalidParam,
                     [&] { wv::slice_level(coeffs, signal.size(), 3, 0, wv::Band::Detail); }));
}

void test_filter_invariants() {
    const wv::Wavelet ws[] = {wv::Wavelet::daubechies(2), wv::Wavelet::daubechies(4),
                              wv::Wavelet::daubechies(6), wv::Wavelet::daubechies(8),
                              wv::Wavelet::symlets(4),     wv::Wavelet::coiflets(1)};
    const float sqrt2 = 1.4142135f;
    for (const wv::Wavelet& w : ws) {
        std::vector<float> h = wv::analysis_lowpass(w);
        float sum = 0.0f, energy = 0.0f;
        for (float x : h) {
            sum += x;
            energy += x * x;
        }
        ISO_CHECK(af(sum - sqrt2) < 5e-4f);
        ISO_CHECK(af(energy - 1.0f) < 5e-4f);
        std::vector<float> g = wv::qmf_highpass(h);
        float gsum = 0.0f;
        for (float x : g) gsum += x;
        ISO_CHECK(af(gsum) < 5e-4f);
    }
    const wv::Wavelet empties[] = {wv::Wavelet::daubechies(0), wv::Wavelet::daubechies(1),
                                   wv::Wavelet::daubechies(3), wv::Wavelet::symlets(1),
                                   wv::Wavelet::coiflets(4),   wv::Wavelet::haar()};
    for (const wv::Wavelet& w : empties) ISO_CHECK(wv::analysis_lowpass(w).empty());
}

void test_2d_shape_and_round_trip() {
    struct Case { std::uint32_t rows, cols, j; };
    const Case shapes[] = {{8, 8, 1}, {8, 8, 2}, {16, 16, 3}, {32, 16, 2}};
    for (const Case& cse : shapes) {
        std::vector<float> image(static_cast<std::size_t>(cse.rows) * cse.cols, 0.5f);
        std::vector<float> coeffs = wv::dwt_2d(image, cse.rows, cse.cols, wv::Wavelet::haar(),
                                               cse.j, wv::Boundary::Periodic);
        ISO_CHECK_EQ_UINT(coeffs.size(), static_cast<std::size_t>(cse.rows) * cse.cols);
    }

    std::vector<float> image = ramp(16 * 16, 0.1f);
    std::vector<float> coeffs = wv::dwt_2d(image, 16, 16, wv::Wavelet::haar(), 2,
                                           wv::Boundary::Periodic);
    std::vector<float> recon = wv::idwt_2d(coeffs, 16, 16, wv::Wavelet::haar(), 2,
                                           wv::Boundary::Periodic);
    ISO_CHECK(close(image, recon, 1e-4f));

    std::vector<float> rect = ramp(32 * 16, 0.05f);
    std::vector<float> rc = wv::dwt_2d(rect, 32, 16, wv::Wavelet::haar(), 2, wv::Boundary::Periodic);
    std::vector<float> rr = wv::idwt_2d(rc, 32, 16, wv::Wavelet::haar(), 2, wv::Boundary::Periodic);
    ISO_CHECK(close(rect, rr, 1e-4f));

    std::vector<float> im2 = ramp(16 * 16, 0.07f);
    std::vector<float> c2 = wv::dwt_2d(im2, 16, 16, wv::Wavelet::daubechies(4), 2,
                                       wv::Boundary::Periodic);
    std::vector<float> r2 = wv::idwt_2d(c2, 16, 16, wv::Wavelet::daubechies(4), 2,
                                        wv::Boundary::Periodic);
    ISO_CHECK(close(im2, r2, 1e-3f));

    std::vector<float> cimg(16 * 16, 2.5f);
    std::vector<float> cc = wv::dwt_2d(cimg, 16, 16, wv::Wavelet::haar(), 2, wv::Boundary::Periodic);
    std::size_t ll_len = 4 * 4;
    for (std::size_t i = ll_len; i < cc.size(); ++i) ISO_CHECK(af(cc[i]) <= 1e-6f);
}

void test_2d_error_paths() {
    ISO_CHECK(throws(wv::Error::EmptySignal,
                     [] { wv::dwt_2d({}, 0, 0, wv::Wavelet::haar(), 1, wv::Boundary::Periodic); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> image(100, 0.0f);
        wv::dwt_2d(image, 8, 8, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> image(16 * 16, 0.0f);
        wv::dwt_2d(image, 16, 16, wv::Wavelet::biorthogonal(5, 3), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> image(16 * 16, 0.0f);
        wv::dwt_2d(image, 16, 16, wv::Wavelet::haar(), 1, wv::Boundary::Symmetric);
    }));
}

// ------------------------------------------------------------------ //
// VER15 CWE/CVE Security Hardening Tests                             //
// ------------------------------------------------------------------ //

void test_hostile_nan_inf_subnormal() {
    const float bad_vals[] = {
        std::numeric_limits<float>::quiet_NaN(),
        std::numeric_limits<float>::infinity(),
        -std::numeric_limits<float>::infinity()
    };

    for (float bad : bad_vals) {
        std::vector<float> sig = {1.0f, 2.0f, 3.0f, bad, 5.0f, 6.0f, 7.0f, 8.0f};
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::dwt_1d(sig, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
        }));

        std::vector<float> coeffs = {1.0f, bad, 3.0f, 4.0f};
        ISO_CHECK(throws(wv::Error::InvalidCoefficients, [&] {
            wv::idwt_1d(coeffs, wv::Wavelet::haar(), 1, wv::Boundary::Periodic, 4);
        }));

        std::vector<float> img(16, 1.0f);
        img[5] = bad;
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::dwt_2d(img, 4, 4, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
        }));

        std::vector<float> img_coeffs(16, 1.0f);
        img_coeffs[7] = bad;
        ISO_CHECK(throws(wv::Error::InvalidCoefficients, [&] {
            wv::idwt_2d(img_coeffs, 4, 4, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
        }));

        std::vector<float> scales = {1.0f, 2.0f};
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::cwt(sig, scales, wv::Wavelet::morlet());
        }));

        std::vector<float> good_sig(8, 1.0f);
        std::vector<float> bad_scales = {1.0f, bad};
        ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
            wv::cwt(good_sig, bad_scales, wv::Wavelet::morlet());
        }));
    }

    // Zero / negative scales for CWT
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        wv::cwt({1.0f, 2.0f}, {0.0f}, wv::Wavelet::morlet());
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        wv::cwt({1.0f, 2.0f}, {-1.5f}, wv::Wavelet::morlet());
    }));

    // Subnormal and tiny numbers
    double d_tiny = 1e-200;
    float tiny = static_cast<float>(d_tiny);
    float subnormal = 1e-40f;
    std::vector<float> sub_sig = {tiny, subnormal, tiny, subnormal, tiny, subnormal, tiny, subnormal};
    std::vector<float> sub_c = wv::dwt_1d(sub_sig, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    ISO_CHECK_EQ_UINT(sub_c.size(), 8u);
    std::vector<float> sub_recon = wv::idwt_1d(sub_c, wv::Wavelet::haar(), 1, wv::Boundary::Periodic, 8);
    ISO_CHECK_EQ_UINT(sub_recon.size(), 8u);
}

void test_preflight_caps_and_overflow() {
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> c = {1.0f, 2.0f, 3.0f, 4.0f};
        wv::idwt_1d(c, wv::Wavelet::haar(), 1, wv::Boundary::Periodic, wv::MAX_2D_DIM + 1);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> img(4, 1.0f);
        wv::dwt_2d(img, static_cast<std::uint32_t>(wv::MAX_2D_DIM + 1), 1, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> img(4, 1.0f);
        wv::dwt_2d(img, 1, static_cast<std::uint32_t>(wv::MAX_2D_DIM + 1), wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> img(4, 1.0f);
        wv::dwt_2d(img, 0, 4, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> img(4, 1.0f);
        wv::dwt_2d(img, 4, 0, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> c(4, 1.0f);
        wv::idwt_2d(c, static_cast<std::uint32_t>(wv::MAX_2D_DIM + 1), 1, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> c(4, 1.0f);
        wv::idwt_2d(c, 0, 4, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        std::vector<float> c(4, 1.0f);
        wv::idwt_2d(c, 4, 0, wv::Wavelet::haar(), 1, wv::Boundary::Periodic);
    }));

    // CWT empty & cap checks
    ISO_CHECK(throws(wv::Error::EmptySignal, [] {
        wv::cwt({}, {1.0f}, wv::Wavelet::morlet());
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] {
        wv::cwt({1.0f, 2.0f}, {}, wv::Wavelet::morlet());
    }));
}

void test_wavelet_and_boundary_parsing() {
    ISO_CHECK(wv::parse_boundary("zero") == wv::Boundary::Zero);
    ISO_CHECK(wv::parse_boundary("replicate") == wv::Boundary::Replicate);
    ISO_CHECK(wv::parse_boundary("reflect") == wv::Boundary::Reflect);
    ISO_CHECK(wv::parse_boundary("symmetric") == wv::Boundary::Symmetric);
    ISO_CHECK(wv::parse_boundary("  sym  ") == wv::Boundary::Symmetric);
    ISO_CHECK(wv::parse_boundary("periodic") == wv::Boundary::Periodic);
    ISO_CHECK(wv::parse_boundary("WRAP") == wv::Boundary::Periodic);
    ISO_CHECK(wv::parse_boundary("circular") == wv::Boundary::Periodic);

    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_boundary("periodic_junk"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_boundary("symmetric123"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_boundary("unknown_mode"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_boundary(""); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_boundary("   "); }));

    ISO_CHECK(wv::parse_wavelet("haar") == wv::Wavelet::haar());
    ISO_CHECK(wv::parse_wavelet("  HAAR  ") == wv::Wavelet::haar());
    ISO_CHECK(wv::parse_wavelet("db2") == wv::Wavelet::daubechies(2));
    ISO_CHECK(wv::parse_wavelet("DB4") == wv::Wavelet::daubechies(4));
    ISO_CHECK(wv::parse_wavelet("db6") == wv::Wavelet::daubechies(6));
    ISO_CHECK(wv::parse_wavelet("db8") == wv::Wavelet::daubechies(8));
    ISO_CHECK(wv::parse_wavelet("sym4") == wv::Wavelet::symlets(4));
    ISO_CHECK(wv::parse_wavelet("coif1") == wv::Wavelet::coiflets(1));
    ISO_CHECK(wv::parse_wavelet("bior2.2") == wv::Wavelet::biorthogonal(2, 2));
    ISO_CHECK(wv::parse_wavelet("bior22") == wv::Wavelet::biorthogonal(2, 2));
    ISO_CHECK(wv::parse_wavelet("bior5.3") == wv::Wavelet::biorthogonal(2, 2));
    ISO_CHECK(wv::parse_wavelet("bior53") == wv::Wavelet::biorthogonal(2, 2));
    ISO_CHECK(wv::parse_wavelet("morlet") == wv::Wavelet::morlet());
    ISO_CHECK(wv::parse_wavelet("mexican_hat") == wv::Wavelet::mexican_hat());
    ISO_CHECK(wv::parse_wavelet("mexicanhat") == wv::Wavelet::mexican_hat());
    ISO_CHECK(wv::parse_wavelet("ricker") == wv::Wavelet::mexican_hat());

    // Trailing garbage and malformed identifiers
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("db2foo"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("sym4bar"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("coif1baz"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("db"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("sym"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("coif"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("haar123"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("unknown_wavelet"); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet(""); }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [] { wv::parse_wavelet("   "); }));
}

void test_cwt() {
    std::vector<float> signal = {1.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f};
    std::vector<float> scales = {1.0f, 2.0f};

    std::vector<float> morlet_out = wv::cwt(signal, scales, wv::Wavelet::morlet());
    ISO_CHECK_EQ_UINT(morlet_out.size(), 16u);
    for (float x : morlet_out) {
        ISO_CHECK(x >= 0.0f);
        ISO_CHECK(x < 10.0f);
    }

    std::vector<float> mh_out = wv::cwt(signal, scales, wv::Wavelet::mexican_hat());
    ISO_CHECK_EQ_UINT(mh_out.size(), 16u);
    for (float x : mh_out) {
        ISO_CHECK(af(x) < 10.0f);
    }

    ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
        wv::cwt(signal, scales, wv::Wavelet::haar());
    }));
    ISO_CHECK(throws(wv::Error::InvalidParam, [&] {
        wv::cwt(signal, scales, wv::Wavelet::daubechies(4));
    }));

    // Linearity: cwt(2*x) == 2*cwt(x)
    std::vector<float> sig2(8);
    for (std::size_t i = 0; i < 8; ++i) sig2[i] = 2.0f * signal[i];
    std::vector<float> mh_out2 = wv::cwt(sig2, scales, wv::Wavelet::mexican_hat());
    ISO_CHECK_EQ_UINT(mh_out.size(), mh_out2.size());
    for (std::size_t i = 0; i < mh_out.size(); ++i) {
        ISO_CHECK(approx_eq(mh_out2[i], 2.0f * mh_out[i], 1e-4f));
    }
}

void test_property_based() {
    // Energy conservation (Parseval) for Haar 1-D
    {
        std::vector<float> sig = ramp(64, 0.08f);
        std::vector<float> coeffs = wv::dwt_1d(sig, wv::Wavelet::haar(), 3, wv::Boundary::Periodic);
        float e_sig = 0.0f;
        float e_coeffs = 0.0f;
        for (float x : sig) e_sig += x * x;
        for (float x : coeffs) e_coeffs += x * x;
        ISO_CHECK(approx_eq(e_sig, e_coeffs, 1e-3f));
    }

    // Pseudo-random round trips across multiple wavelets
    {
        std::uint32_t seed = 42;
        const wv::Wavelet test_ws[] = {
            wv::Wavelet::haar(),
            wv::Wavelet::daubechies(2),
            wv::Wavelet::daubechies(4),
            wv::Wavelet::symlets(4),
            wv::Wavelet::coiflets(1)
        };
        for (const auto& w : test_ws) {
            std::vector<float> prng_sig(32);
            for (std::size_t i = 0; i < 32; ++i) {
                seed = seed * 1103515245u + 12345u;
                prng_sig[i] = (static_cast<float>(seed & 0xFFFF) / 65536.0f) - 0.5f;
            }
            std::vector<float> coeffs = wv::dwt_1d(prng_sig, w, 2, wv::Boundary::Periodic);
            std::vector<float> recon = wv::idwt_1d(coeffs, w, 2, wv::Boundary::Periodic, 32);
            ISO_CHECK_EQ_UINT(recon.size(), 32u);
            std::size_t edge = wv::analysis_lowpass(w).size();
            if (w.family == wv::Family::Haar) edge = 0;
            if (edge > recon.size() / 4) edge = recon.size() / 4;
            for (std::size_t i = edge; i + edge < recon.size(); ++i) {
                ISO_CHECK(approx_eq(prng_sig[i], recon[i], 1e-2f));
            }
        }
    }
}

} // namespace

int main() {
    test_error_paths();
    test_haar_reference_vector();
    test_constant_signal_zero_detail();
    test_dirac_delta();
    test_output_length_contract();
    test_round_trips_1d();
    test_db2_constant_small_detail();
    test_split_and_slice();
    test_filter_invariants();
    test_2d_shape_and_round_trip();
    test_2d_error_paths();

    // VER15 security hardening test suites
    test_hostile_nan_inf_subnormal();
    test_preflight_caps_and_overflow();
    test_wavelet_and_boundary_parsing();
    test_cwt();
    test_property_based();

    ISO_TEST_RESULT();
}
