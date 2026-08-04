// dsp_wavelets_test.cpp — unit tests for the C++ wavelet transform port.
//
// Mirrors the Rust crate's suite: error paths, output-length contract, the
// hand-worked Haar reference vector, constant/dirac structural properties,
// perfect-reconstruction round-trips (Haar + Daubechies/Symlets/Coiflets, 1-D
// and 2-D), split_levels/slice_level, and the filter-bank invariants.
//
// Test signals use simple ramps / arithmetic rather than sin/cos so the lane
// stays free of <cmath>; perfect reconstruction holds for any input signal.
#include "dsp_wavelets.hpp"
#include "iso_test.h"

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

// A deterministic pseudo-signal (ramp with a wrap) so round-trips exercise
// non-trivial data without <cmath>.
std::vector<float> ramp(std::size_t n, float step) {
    std::vector<float> v(n);
    for (std::size_t i = 0; i < n; ++i) {
        float x = static_cast<float>(i) * step;
        // fold into a small range to keep magnitudes modest
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
    // Periodic round-trips are exact for orthogonal wavelets; check central
    // region for longer filters (edge effects skipped) as the crate does.
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
    for (wv::Boundary b : bad_b) {
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
    // pywt.dwt([1,2,3,4], 'haar', periodization) magnitudes; our sign
    // convention gives cD = +1/√2 (see crate doc).
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
    // Symmetric Haar round trip.
    round_trip_1d(ramp(16, 0.11f), wv::Wavelet::haar(), 3, wv::Boundary::Symmetric, 1e-3f);
    // Daubechies / Symlets / Coiflets under Periodic (central region).
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
    // Unsupported (family, order) pairs return an empty filter.
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

    // Haar 2-D round trip.
    std::vector<float> image = ramp(16 * 16, 0.1f);
    std::vector<float> coeffs = wv::dwt_2d(image, 16, 16, wv::Wavelet::haar(), 2,
                                           wv::Boundary::Periodic);
    std::vector<float> recon = wv::idwt_2d(coeffs, 16, 16, wv::Wavelet::haar(), 2,
                                           wv::Boundary::Periodic);
    ISO_CHECK(close(image, recon, 1e-4f));

    // Rectangular Haar round trip.
    std::vector<float> rect = ramp(32 * 16, 0.05f);
    std::vector<float> rc = wv::dwt_2d(rect, 32, 16, wv::Wavelet::haar(), 2, wv::Boundary::Periodic);
    std::vector<float> rr = wv::idwt_2d(rc, 32, 16, wv::Wavelet::haar(), 2, wv::Boundary::Periodic);
    ISO_CHECK(close(rect, rr, 1e-4f));

    // Db4 2-D round trip.
    std::vector<float> im2 = ramp(16 * 16, 0.07f);
    std::vector<float> c2 = wv::dwt_2d(im2, 16, 16, wv::Wavelet::daubechies(4), 2,
                                       wv::Boundary::Periodic);
    std::vector<float> r2 = wv::idwt_2d(c2, 16, 16, wv::Wavelet::daubechies(4), 2,
                                        wv::Boundary::Periodic);
    ISO_CHECK(close(im2, r2, 1e-3f));

    // Constant image → zero detail.
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
    ISO_TEST_RESULT();
}
