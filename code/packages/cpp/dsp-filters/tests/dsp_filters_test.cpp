// dsp_filters_test.cpp — unit, hostile-input, and SplitMix64 PBT tests for cpp/dsp-filters.
#include "dsp_filters.hpp"
#include "iso_test.h"

#include <algorithm>
#include <cfloat>
#include <cmath>
#include <cstdint>
#include <limits>
#include <vector>

namespace df = ca::dsp_filters;

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

double steady_rms(const std::vector<float>& x, std::size_t start, std::size_t end) {
    double acc = 0.0;
    for (std::size_t i = start; i < end; ++i) {
        acc += static_cast<double>(x[i]) * static_cast<double>(x[i]);
    }
    return acc / static_cast<double>(end - start);
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
df::Error expect_filter_error(Fn fn) {
    try {
        fn();
        ISO_CHECK_MSG(false, "expected FilterError");
        return df::Error::InvalidCoefficient;
    } catch (const df::FilterError& e) {
        return e.code();
    }
}

void test_fir_and_design() {
    std::vector<float> sig = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f, 6.0f, 7.0f, 8.0f};
    std::vector<float> ker = {0.25f, 0.5f, 0.25f};
    check_close_vec(df::fir(sig, ker), df::fir_fft(sig, ker), 1e-4);

    auto lp = df::design_low_pass(0.2f, 33u, df::WindowType::Hamming);
    double sum = 0.0;
    for (std::size_t i = 0; i < lp.size(); ++i) {
        sum += lp[i];
        ISO_CHECK_EQ_DBL(lp[i], lp[lp.size() - 1 - i], 1e-6);
    }
    ISO_CHECK_EQ_DBL(sum, 1.0, 1e-5);

    auto hp = df::design_high_pass(0.2f, 33u, df::WindowType::Hann);
    sum = 0.0;
    for (float v : hp) sum += v;
    ISO_CHECK_EQ_DBL(sum, 0.0, 1e-5);

    auto bp = df::design_band_pass(0.15f, 0.35f, 65u, df::WindowType::Blackman);
    sum = 0.0;
    for (float v : bp) sum += v;
    ISO_CHECK(std::abs(sum) < 0.05);

    auto bs = df::design_band_stop(0.15f, 0.35f, 65u, df::WindowType::Rectangular);
    sum = 0.0;
    for (float v : bs) sum += v;
    ISO_CHECK_EQ_DBL(sum, 1.0, 0.05);
}

void test_iir_biquad_butterworth_and_zero_phase() {
    std::vector<float> step(200, 1.0f);
    auto out = df::iir(step, {1.0f}, {1.0f, -0.9f});
    ISO_CHECK_EQ_DBL(out[0], 1.0, 1e-5);
    ISO_CHECK_EQ_DBL(out[1], 1.9, 1e-5);
    ISO_CHECK_EQ_DBL(out[199], 10.0, 1e-3);

    // Failure-atomic Biquad
    float qnan = std::numeric_limits<float>::quiet_NaN();
    auto bq = df::Biquad::lowpass(0.2f);
    (void)bq.process({1.0f, 0.5f, -0.25f});
    float z1_saved = bq.z1();
    float z2_saved = bq.z2();
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([&bq, qnan] {
        bq.process({1.0f, qnan, 0.5f});
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_DBL(bq.z1(), z1_saved, 0.0);
    ISO_CHECK_EQ_DBL(bq.z2(), z2_saved, 0.0);
    bq.reset();
    ISO_CHECK_EQ_DBL(bq.z1(), 0.0, 0.0);

    // Cookbook constructors
    (void)df::Biquad::highpass(0.1f);
    (void)df::Biquad::bandpass(0.2f);
    (void)df::Biquad::notch(0.25f);
    (void)df::Biquad::allpass(0.25f);
    (void)df::Biquad::peak(0.25f, 6.0f);

    // Butterworth LP, HP, BP, BS SOS + zero-phase sosfiltfilt
    std::vector<float> sig_lo(256), sig_hi(256), sig_mid(256);
    for (std::size_t i = 0; i < 256; ++i) {
        sig_lo[i] = static_cast<float>(std::sin(2.0 * TEST_PI * 0.03 * static_cast<double>(i)));
        sig_hi[i] = static_cast<float>(std::sin(2.0 * TEST_PI * 0.42 * static_cast<double>(i)));
        sig_mid[i] = static_cast<float>(std::sin(2.0 * TEST_PI * 0.25 * static_cast<double>(i)));
    }

    auto lp_sos = df::butterworth_sos(df::ButterworthType::Lowpass, 4u, 0.15f);
    ISO_CHECK(steady_rms(df::sosfilt(lp_sos, sig_lo), 64, 256) > 0.4);
    ISO_CHECK(steady_rms(df::sosfilt(lp_sos, sig_hi), 64, 256) < 0.01);

    auto hp_sos = df::butterworth_sos(df::ButterworthType::Highpass, 4u, 0.30f);
    ISO_CHECK(steady_rms(df::sosfilt(hp_sos, sig_hi), 64, 256) > 0.4);
    ISO_CHECK(steady_rms(df::sosfilt(hp_sos, sig_lo), 64, 256) < 0.01);

    auto bp_sos = df::butterworth_sos(df::ButterworthType::Bandpass, 3u, 0.18f, 0.32f);
    ISO_CHECK(steady_rms(df::sosfilt(bp_sos, sig_mid), 64, 256) > 0.4);
    ISO_CHECK(steady_rms(df::sosfilt(bp_sos, sig_lo), 64, 256) < 0.01);
    ISO_CHECK(steady_rms(df::sosfilt(bp_sos, sig_hi), 64, 256) < 0.01);

    auto bs_sos = df::butterworth_sos(df::ButterworthType::Bandstop, 2u, 0.18f, 0.32f);
    ISO_CHECK(steady_rms(df::sosfilt(bs_sos, sig_mid), 64, 256) < 0.02);
    ISO_CHECK(steady_rms(df::sosfilt(bs_sos, sig_lo), 64, 256) > 0.4);
    ISO_CHECK(steady_rms(df::sosfilt(bs_sos, sig_hi), 64, 256) > 0.4);

    // Zero-phase alignment check
    auto zp = df::sosfiltfilt(bs_sos, sig_lo);
    double dot = 0.0;
    for (std::size_t i = 32; i < 224; ++i) dot += static_cast<double>(zp[i]) * static_cast<double>(sig_lo[i]);
    ISO_CHECK(dot > 80.0);

    // (b, a) vs SOS + filtfilt parity
    auto ba = df::butterworth_lowpass(2u, 0.2f);
    auto lp2_sos = df::butterworth_sos(df::ButterworthType::Lowpass, 2u, 0.2f);
    check_close_vec(df::iir(sig_lo, ba.first, ba.second), df::sosfilt(lp2_sos, sig_lo), 1e-4);
    check_close_vec(df::filtfilt(sig_lo, ba.first, ba.second), df::sosfiltfilt(lp2_sos, sig_lo), 1e-4);

    (void)df::butterworth_highpass(1u, 0.2f);
    (void)df::butterworth_bandpass(2u, 0.15f, 0.35f);
    (void)df::butterworth_bandstop(1u, 0.15f, 0.35f);

    // In-place aliasing `out == signal`
    std::vector<float> buf(sig_lo.begin(), sig_lo.begin() + 32);
    auto ref = df::sosfiltfilt(lp2_sos, buf);
    df::sosfiltfilt_into(lp2_sos.data(), lp2_sos.size(), buf.data(), buf.size(), buf.data());
    check_close_vec(buf, ref, 1e-6);

    // Parsers
    ISO_CHECK(df::parse_window(" blackman ") == df::WindowType::Blackman);
    ISO_CHECK(df::parse_butterworth_type("BANDPASS") == df::ButterworthType::Bandpass);
}

void test_hostile_inputs() {
    float qnan = std::numeric_limits<float>::quiet_NaN();
    float pinf = std::numeric_limits<float>::infinity();

    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] { df::fir({}, {1.0f}); })),
                     static_cast<int>(df::Error::EmptySignal));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] { df::fir({1.0f}, {}); })),
                     static_cast<int>(df::Error::EmptyKernel));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] { df::iir({}, {1.0f}, {1.0f}); })),
                     static_cast<int>(df::Error::EmptySignal));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] { df::iir({1.0f}, {1.0f}, {0.0f, 0.5f}); })),
                     static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::design_low_pass(0.2f, 5u, static_cast<df::WindowType>(99));
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::butterworth_sos(static_cast<df::ButterworthType>(99), 2u, 0.2f);
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::butterworth_sos(df::ButterworthType::Lowpass, 0u, 0.2f);
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::butterworth_sos(df::ButterworthType::Bandpass, 2u, 0.3f, 0.2f);
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([qnan] {
        df::fir({1.0f, qnan}, {1.0f});
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::fir({FLT_MAX, FLT_MAX}, {FLT_MAX, FLT_MAX});
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::Biquad(FLT_MAX, 0.0f, 0.0f, 1e-30f, 0.0f, 0.0f);
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::Biquad::peak(0.2f, 1e6f, 1.0f);
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([pinf] {
        df::sosfiltfilt({}, {1.0f, pinf});
    })), static_cast<int>(df::Error::InvalidCoefficient));

    // Separately rounded f32 DFII-T overflow & validation order (DSP03 AF-03)
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::sosfilt({{2.0f, 0.0f, 0.0f, 1.0f, 0.0f, 0.0f},
                     {0.5f, 0.0f, 0.0f, 1.0f, 0.0f, 0.0f}}, {FLT_MAX});
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::sosfilt({{0.0f, 2.0f, 0.0f, 1.0f, 0.0f, 0.0f}}, {FLT_MAX});
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([] {
        df::sosfilt({{FLT_MAX, 0.0f, 0.0f, 1.0f, 1.0f, 0.0f}}, {1.0f, 2.0f});
    })), static_cast<int>(df::Error::InvalidCoefficient));

    df::Biquad bq_fma(FLT_MAX, 0.0f, 0.0f, 1.0f, 1.0f, 0.0f);
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([&] {
        bq_fma.process({1.0f, 2.0f});
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_DBL(bq_fma.z1(), 0.0, 0.0);
    ISO_CHECK_EQ_DBL(bq_fma.z2(), 0.0, 0.0);
    ISO_CHECK_EQ_DBL(bq_fma.process_sample(1.0f), static_cast<double>(FLT_MAX), 0.0);
    ISO_CHECK_EQ_DBL(bq_fma.z1(), -static_cast<double>(FLT_MAX), 0.0);
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([&] {
        bq_fma.process_sample(2.0f);
    })), static_cast<int>(df::Error::InvalidCoefficient));
    ISO_CHECK_EQ_DBL(bq_fma.z1(), -static_cast<double>(FLT_MAX), 0.0);

    float scratch = 0.0f;
    float dummy_in = 1.0f;
    ISO_CHECK_EQ_INT(static_cast<int>(expect_filter_error([&] {
        df::sosfiltfilt_into(nullptr, 1, &dummy_in, 0, &scratch);
    })), static_cast<int>(df::Error::EmptySignal));
}

void test_pbt_splitmix64() {
    SplitMix64 rng(0x123456789ABCDEF0ULL);
    const std::size_t pairs[5][2] = {{8, 3}, {16, 5}, {32, 8}, {64, 17}, {100, 20}};
    for (const auto& nk : pairs) {
        std::vector<float> sig(nk[0]), ker(nk[1]);
        for (float& x : sig) x = rng.sample_f32();
        for (float& h : ker) h = rng.sample_f32();
        check_close_vec(df::fir(sig, ker), df::fir_fft(sig, ker), 1e-4);
    }
}

} // namespace

int main() {
    test_fir_and_design();
    test_iir_biquad_butterworth_and_zero_phase();
    test_hostile_inputs();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
