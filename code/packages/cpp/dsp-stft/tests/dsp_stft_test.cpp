// dsp_stft_test.cpp — unit, Family-1/4 invariant, hostile-input, and SplitMix64 PBT
// tests for cpp/dsp-stft (pure ISO C++17).
#include "dsp_stft.hpp"
#include "iso_test.h"

#include <cfloat>
#include <cstddef>
#include <cstdint>
#include <limits>
#include <string>
#include <vector>

namespace ds = ca::dsp_stft;

namespace {

float af(float x) { return x < 0.0f ? -x : x; }

bool approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

template <typename F>
bool throws_err(ds::Error expected, F&& fn) {
    try {
        fn();
    } catch (const ds::StftError& e) {
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

void test_windows_and_parsing() {
    ISO_CHECK(ds::parse_window_type(" hann ") == ds::WindowType::Hann);
    ISO_CHECK(ds::parse_window_type("Hanning") == ds::WindowType::Hann);
    ISO_CHECK(ds::parse_window_type("HAMMING") == ds::WindowType::Hamming);
    ISO_CHECK(ds::parse_window_type("blackman") == ds::WindowType::Blackman);
    ISO_CHECK(ds::parse_window_type("boxcar") == ds::WindowType::Rectangular);
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::parse_window_type("hann extra"); }));

    auto w = ds::hann_window(16u);
    ISO_CHECK_EQ_UINT(w.size(), 16u);
    ISO_CHECK(approx_eq(w[0], 0.0f, 1e-6f));
    ISO_CHECK(approx_eq(w[15], 0.0f, 1e-6f));
    auto w1 = ds::hamming_window(1u);
    ISO_CHECK(approx_eq(w1[0], 1.0f, 1e-6f));
    auto wb = ds::blackman_window(9u);
    ISO_CHECK(approx_eq(wb[4], 1.0f, 1e-5f));
    auto wr = ds::rectangular_window(8u);
    ISO_CHECK(approx_eq(wr[3], 1.0f, 1e-6f));
}

void test_stft_istft_ola_and_griffin_lim() {
    const std::size_t n = 1024u;
    std::vector<float> sig(n);
    for (std::size_t i = 0; i < n; ++i) {
        sig[i] = static_cast<float>(0.7 * ((i % 17) / 17.0 - 0.5) +
                                    0.3 * (((i * 3) % 29) / 29.0 - 0.5));
    }

    auto spec = ds::stft(sig, 128u, 64u, ds::WindowType::Hann);
    ISO_CHECK_EQ_UINT(spec.size(), 15u * 65u * 2u);
    auto recon = ds::istft(spec, 128u, 64u, ds::WindowType::Hann, static_cast<std::uint32_t>(n));
    ISO_CHECK_EQ_UINT(recon.size(), n);
    for (std::size_t i = 128u; i < n - 128u; ++i) {
        ISO_CHECK(approx_eq(recon[i], sig[i], 1e-3f));
    }

    // Non-power-of-two Bluestein STFT/ISTFT
    std::vector<float> sig480(sig.begin(), sig.begin() + 480);
    auto spec_np2 = ds::stft(sig480, 60u, 30u, ds::WindowType::Hann);
    auto recon_np2 = ds::istft(spec_np2, 60u, 30u, ds::WindowType::Hann, 480u);
    for (std::size_t i = 60u; i < 420u; ++i) {
        ISO_CHECK(approx_eq(recon_np2[i], sig480[i], 1e-3f));
    }

    // OLA / COLA checks
    auto ola_h = ds::check_ola(128u, 64u, ds::WindowType::Hann);
    ISO_CHECK(ola_h.is_nola && ola_h.min_norm > 0.1f);
    auto ola_r = ds::check_ola(128u, 128u, ds::WindowType::Rectangular);
    ISO_CHECK(ola_r.is_nola && ola_r.is_cola);
    auto ola_bad = ds::check_ola(64u, 128u, ds::WindowType::Hann);
    ISO_CHECK(!ola_bad.is_nola && !ola_bad.is_cola);

    // Griffin-Lim
    std::vector<float> sig512(sig.begin(), sig.begin() + 512);
    auto mag = ds::magnitude_spectrogram(sig512, 64u, 16u, ds::WindowType::Hann);
    auto gl = ds::griffin_lim(mag, 64u, 16u, ds::WindowType::Hann, 12u, 512u);
    auto gl_mag = ds::magnitude_spectrogram(gl, 64u, 16u, ds::WindowType::Hann);
    double num = 0.0, den = 0.0;
    for (std::size_t i = 0; i < mag.size(); ++i) {
        double d = static_cast<double>(gl_mag[i]) - static_cast<double>(mag[i]);
        num += d * d;
        den += static_cast<double>(mag[i]) * static_cast<double>(mag[i]);
    }
    ISO_CHECK(den > 0.0 && (num / den) < 0.05);
}

void test_spectrograms_mel_and_mfcc() {
    const std::size_t n = 1024u;
    std::vector<float> sig(n);
    std::vector<float> zeros(n, 0.0f);
    for (std::size_t i = 0; i < n; ++i) {
        sig[i] = static_cast<float>(static_cast<int>(i % 23) - 11) * 0.05f;
    }

    auto mag = ds::magnitude_spectrogram(sig, 128u, 64u, ds::WindowType::Hann);
    auto pwr = ds::power_spectrogram(sig, 128u, 64u, ds::WindowType::Hann);
    ISO_CHECK_EQ_UINT(mag.size(), pwr.size());
    for (std::size_t i = 0; i < mag.size(); ++i) {
        ISO_CHECK(mag[i] >= 0.0f && pwr[i] >= 0.0f);
        ISO_CHECK(approx_eq(mag[i] * mag[i], pwr[i], 1e-4f));
    }

    auto logp = ds::log_spectrogram(zeros, 128u, 64u, ds::WindowType::Hann);
    for (float v : logp) {
        ISO_CHECK(approx_eq(v, -23.02585f, 1e-3f));
    }

    for (float freq : {0.0f, 100.0f, 440.0f, 1000.0f, 8000.0f}) {
        ISO_CHECK(approx_eq(ds::mel_to_hz(ds::hz_to_mel(freq)), freq, 1e-4f));
    }

    auto fb = ds::mel_filterbank(40u, 1024u, 22050.0f);
    ISO_CHECK_EQ_UINT(fb.size(), 40u * 513u);
    for (std::size_t m = 0; m < 40u; ++m) {
        double rsum = 0.0;
        for (std::size_t k = 0; k < 513u; ++k) {
            float w = fb[m * 513u + k];
            ISO_CHECK(w >= 0.0f);
            rsum += w;
        }
        ISO_CHECK(approx_eq(static_cast<float>(rsum), 1.0f, 1e-4f));
    }

    auto mel = ds::mel_spectrogram(sig, 256u, 128u, 40u, 16000.0f, ds::WindowType::Hann);
    ISO_CHECK_EQ_UINT(mel.size(), 7u * 40u);
    auto cc = ds::mfcc(sig, 256u, 128u, 40u, 13u, 16000.0f, ds::WindowType::Hann);
    ISO_CHECK_EQ_UINT(cc.size(), 7u * 13u);
    auto cc_z = ds::mfcc(zeros, 256u, 128u, 40u, 13u, 16000.0f, ds::WindowType::Hann);
    for (float v : cc_z) ISO_CHECK(v == v);
}

void test_hostile_and_cwe() {
    float nan_v = std::numeric_limits<float>::quiet_NaN();
    float inf_v = std::numeric_limits<float>::infinity();
    float ninf_v = -std::numeric_limits<float>::infinity();
    float sub_v = std::numeric_limits<float>::denorm_min();

    ISO_CHECK(throws_err(ds::Error::EmptySignal, [] { ds::stft({}, 64u, 32u); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::stft({1, 2, 3, 4}, 0u, 1u); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::stft({1, 2, 3, 4}, 2u, 0u); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] {
        ds::stft({1, 2, 3, 4}, 2u, 1u, static_cast<ds::WindowType>(99));
    }));
    ISO_CHECK(throws_err(ds::Error::SignalTooShort, [] { ds::stft({1, 2, 3, 4}, 64u, 32u); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] {
        ds::stft({1, 2, 3, 4}, static_cast<std::uint32_t>(ds::MAX_FFT_LEN + 1u), 32u);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidSpectrogram, [] {
        ds::istft({}, 64u, 32u, ds::WindowType::Hann, 64u);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidSpectrogram, [] {
        ds::istft({1, 2, 3}, 2u, 1u, ds::WindowType::Hann, 4u);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] {
        ds::istft({1, 0, 0, 0}, 2u, 1u, ds::WindowType::Hann,
                  static_cast<std::uint32_t>(ds::MAX_SAMPLES + 1u));
    }));

    for (float bad : {nan_v, inf_v, ninf_v}) {
        ISO_CHECK(throws_err(ds::Error::InvalidParam, [=] { ds::stft({1.0f, bad}, 2u, 1u); }));
        ISO_CHECK(throws_err(ds::Error::InvalidSpectrogram, [=] {
            ds::istft({1.0f, bad, 0.0f, 0.0f}, 2u, 1u, ds::WindowType::Hann, 4u);
        }));
        ISO_CHECK(ds::mel_filterbank(40u, 512u, bad).empty());
        ISO_CHECK(throws_err(ds::Error::InvalidParam, [=] { ds::hz_to_mel(bad); }));
        ISO_CHECK(throws_err(ds::Error::InvalidParam, [=] { ds::mel_to_hz(bad); }));
    }

    ISO_CHECK(throws_err(ds::Error::EmptySignal, [] {
        ds::log_spectrogram({}, 64u, 32u, ds::WindowType::Hann, 0.0f);
    }));
    ISO_CHECK(throws_err(ds::Error::EmptySignal, [] {
        ds::mel_spectrogram({}, 1048576u, 0u, 0u, 0.0f, static_cast<ds::WindowType>(99));
    }));
    ISO_CHECK(throws_err(ds::Error::EmptySignal, [] {
        ds::mfcc({}, 1048576u, 0u, 0u, 0u, 0.0f, static_cast<ds::WindowType>(99));
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::hz_to_mel(-1.0f); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::mel_to_hz(-1.0f); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::mel_to_hz(1000000.0f); }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] { ds::mel_to_hz(800000.0f); }));
    ISO_CHECK(ds::mel_filterbank(40u, 512u, sub_v).empty());
    ISO_CHECK(ds::mel_filterbank(40u, 512u, 0.0f).empty());

    std::vector<float> hot(64, 1e20f);
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [&] {
        ds::power_spectrogram(hot, 64u, 32u, ds::WindowType::Rectangular);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] {
        ds::log_spectrogram({1, 2, 3, 4}, 2u, 1u, ds::WindowType::Hann, 0.0f);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] {
        ds::mfcc({1, 2, 3, 4}, 2u, 1u, 16u, 17u, 16000.0f);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidSpectrogram, [] {
        ds::griffin_lim({1.0f, -0.5f, 0.0f, 0.0f}, 2u, 1u, ds::WindowType::Hann, 4u, 4u);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidSpectrogram, [] {
        std::vector<float> mag33(33, 1.0f);
        ds::griffin_lim(mag33, 64u, 32u, ds::WindowType::Hann, 10u, 2000000u);
    }));
    ISO_CHECK(throws_err(ds::Error::InvalidParam, [] {
        ds::griffin_lim({1.0f, 0.0f, 0.0f, 0.0f}, 2u, 1u, ds::WindowType::Hann,
                        ds::MAX_GRIFFIN_LIM_ITERS + 1u, 4u);
    }));
}

void test_pbt_splitmix64_invariants() {
    SplitMix64 rng(UINT64_C(0x123456789ABCDEF0));
    for (int iter = 0; iter < 10; ++iter) {
        const std::size_t n = 256u;
        std::vector<float> s1(n), s2(n), comb(n);
        float alpha = rng.sample(-2.0f, 2.0f);
        float beta = rng.sample(-2.0f, 2.0f);
        for (std::size_t i = 0; i < n; ++i) {
            s1[i] = rng.sample(-1.0f, 1.0f);
            s2[i] = rng.sample(-1.0f, 1.0f);
            comb[i] = alpha * s1[i] + beta * s2[i];
        }
        auto sp1 = ds::stft(s1, 64u, 32u, ds::WindowType::Hann);
        auto sp2 = ds::stft(s2, 64u, 32u, ds::WindowType::Hann);
        auto spc = ds::stft(comb, 64u, 32u, ds::WindowType::Hann);
        ISO_CHECK_EQ_UINT(sp1.size(), spc.size());
        for (std::size_t i = 0; i < spc.size(); ++i) {
            ISO_CHECK(approx_eq(spc[i], alpha * sp1[i] + beta * sp2[i], 1e-4f));
        }

        auto rec = ds::istft(sp1, 64u, 32u, ds::WindowType::Hann, static_cast<std::uint32_t>(n));
        for (std::size_t i = 64u; i < n - 64u; ++i) {
            ISO_CHECK(approx_eq(rec[i], s1[i], 1e-3f));
        }
    }
}

} // namespace

int main() {
    test_windows_and_parsing();
    test_stft_istft_ola_and_griffin_lim();
    test_spectrograms_mel_and_mfcc();
    test_hostile_and_cwe();
    test_pbt_splitmix64_invariants();
    return ISO_TEST_RESULT();
}
