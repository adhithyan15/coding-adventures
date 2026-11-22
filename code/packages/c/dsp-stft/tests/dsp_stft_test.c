/*
 * dsp_stft_test.c — unit, Family-1/4 invariant, hostile-input, and SplitMix64 PBT
 * tests for c/dsp-stft (pure ISO C17).
 */
#include "dsp_stft.h"
#include "iso_test.h"

#include <float.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

static float af(float x) { return x < 0.0f ? -x : x; }

static int approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) scale = af(b);
    if (scale < 1.0f) scale = 1.0f;
    return af(a - b) <= scale * tol;
}

static float make_nan(void) {
    union { uint32_t u; float f; } v;
    v.u = 0x7FC00000u;
    return v.f;
}

static float make_inf(void) {
    union { uint32_t u; float f; } v;
    v.u = 0x7F800000u;
    return v.f;
}

static float make_neg_inf(void) {
    union { uint32_t u; float f; } v;
    v.u = 0xFF800000u;
    return v.f;
}

static float make_min_subnormal(void) {
    union { uint32_t u; float f; } v;
    v.u = 0x00000001u;
    return v.f;
}

typedef struct {
    uint64_t state;
} splitmix64_t;

static uint64_t sm64_next(splitmix64_t *rng) {
    uint64_t z;
    rng->state += UINT64_C(0x9E3779B97F4A7C15);
    z = rng->state;
    z = (z ^ (z >> 30)) * UINT64_C(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)) * UINT64_C(0x94D049BB133111EB);
    return z ^ (z >> 31);
}

static float sm64_sample_f32(splitmix64_t *rng, float min_v, float max_v) {
    uint64_t u = sm64_next(rng);
    double unit = (double)(u >> 11) / 9007199254740992.0;
    return (float)((double)min_v + ((double)max_v - (double)min_v) * unit);
}

static void test_windows_and_parsing(void) {
    stft_window_t w = STFT_WIN_HANN;
    float *win = NULL;
    size_t wlen = 0;

    ISO_CHECK(stft_parse_window(" hann ", &w) == STFT_OK && w == STFT_WIN_HANN);
    ISO_CHECK(stft_parse_window("Hanning", &w) == STFT_OK && w == STFT_WIN_HANN);
    ISO_CHECK(stft_parse_window("HAMMING", &w) == STFT_OK && w == STFT_WIN_HAMMING);
    ISO_CHECK(stft_parse_window("blackman", &w) == STFT_OK && w == STFT_WIN_BLACKMAN);
    ISO_CHECK(stft_parse_window("boxcar", &w) == STFT_OK && w == STFT_WIN_RECTANGULAR);
    ISO_CHECK(stft_parse_window("hann extra", &w) == STFT_ERR_INVALID_PARAM);

    ISO_CHECK(stft_hann_window(16u, &win, &wlen) == STFT_OK && wlen == 16u);
    ISO_CHECK(approx_eq(win[0], 0.0f, 1e-6f));
    ISO_CHECK(approx_eq(win[15], 0.0f, 1e-6f));
    stft_free_buf(&win, wlen);

    ISO_CHECK(stft_hamming_window(1u, &win, &wlen) == STFT_OK && wlen == 1u);
    ISO_CHECK(approx_eq(win[0], 1.0f, 1e-6f));
    stft_free_buf(&win, wlen);

    ISO_CHECK(stft_blackman_window(9u, &win, &wlen) == STFT_OK && wlen == 9u);
    ISO_CHECK(approx_eq(win[4], 1.0f, 1e-5f));
    stft_free_buf(&win, wlen);

    ISO_CHECK(stft_rectangular_window(8u, &win, &wlen) == STFT_OK && wlen == 8u);
    ISO_CHECK(approx_eq(win[3], 1.0f, 1e-6f));
    stft_free_buf(&win, wlen);
}

static void test_stft_istft_cola_and_griffin_lim(void) {
    const size_t n = 1024u;
    float *sig = (float *)malloc(n * sizeof(float));
    float *spec = NULL;
    float *recon = NULL;
    float *mag = NULL;
    float *gl_recon = NULL;
    size_t slen = 0, rlen = 0, mlen = 0, glen = 0;
    int is_nola = 0, is_cola = 0;
    float min_norm = 0.0f, max_norm = 0.0f;
    size_t i;

    for (i = 0; i < n; ++i) {
        /* Deterministic multi-tone signal */
        double t = (double)i * 0.05;
        sig[i] = (float)(0.7 * ((i % 17) / 17.0 - 0.5) + 0.3 * (((i * 3) % 29) / 29.0 - 0.5) + 0.0001 * t);
    }

    /* Power-of-two STFT + ISTFT COLA round-trip */
    ISO_CHECK(stft_forward(sig, n, 128u, 64u, STFT_WIN_HANN, &spec, &slen) == STFT_OK);
    ISO_CHECK_EQ_UINT(slen, (1u + (n - 128u) / 64u) * (128u / 2u + 1u) * 2u);
    ISO_CHECK(stft_istft(spec, slen, 128u, 64u, STFT_WIN_HANN, (uint32_t)n, &recon, &rlen) == STFT_OK);
    ISO_CHECK_EQ_UINT(rlen, n);
    for (i = 128u; i < n - 128u; ++i) {
        ISO_CHECK(approx_eq(recon[i], sig[i], 1e-3f));
    }
    stft_free_buf(&spec, slen);
    stft_free_buf(&recon, rlen);

    /* Non-power-of-two Bluestein STFT + ISTFT round-trip (n_fft = 60, hop = 30) */
    ISO_CHECK(stft_forward(sig, 480u, 60u, 30u, STFT_WIN_HANN, &spec, &slen) == STFT_OK);
    ISO_CHECK_EQ_UINT(slen, (1u + (480u - 60u) / 30u) * (60u / 2u + 1u) * 2u);
    ISO_CHECK(stft_istft(spec, slen, 60u, 30u, STFT_WIN_HANN, 480u, &recon, &rlen) == STFT_OK);
    for (i = 60u; i < 420u; ++i) {
        ISO_CHECK(approx_eq(recon[i], sig[i], 1e-3f));
    }
    stft_free_buf(&spec, slen);
    stft_free_buf(&recon, rlen);

    /* OLA / COLA checks */
    ISO_CHECK(stft_check_ola(128u, 64u, STFT_WIN_HANN, &is_nola, &is_cola, &min_norm, &max_norm) == STFT_OK);
    ISO_CHECK(is_nola == 1 && min_norm > 0.1f);
    ISO_CHECK(stft_check_ola(128u, 128u, STFT_WIN_RECTANGULAR, &is_nola, &is_cola, &min_norm, &max_norm) == STFT_OK);
    ISO_CHECK(is_nola == 1 && is_cola == 1);
    ISO_CHECK(stft_check_ola(64u, 128u, STFT_WIN_HANN, &is_nola, &is_cola, &min_norm, &max_norm) == STFT_OK);
    ISO_CHECK(is_nola == 0 && is_cola == 0);

    /* Griffin-Lim phase reconstruction from magnitude spectrogram */
    ISO_CHECK(stft_magnitude_spectrogram(sig, 512u, 64u, 16u, STFT_WIN_HANN, &mag, &mlen) == STFT_OK);
    ISO_CHECK(stft_griffin_lim(mag, mlen, 64u, 16u, STFT_WIN_HANN, 12u, 512u, &gl_recon, &glen) == STFT_OK);
    ISO_CHECK_EQ_UINT(glen, 512u);
    {
        float *gl_mag = NULL;
        size_t gl_mlen = 0;
        double num = 0.0, den = 0.0;
        ISO_CHECK(stft_magnitude_spectrogram(gl_recon, glen, 64u, 16u, STFT_WIN_HANN, &gl_mag, &gl_mlen) == STFT_OK);
        ISO_CHECK_EQ_UINT(gl_mlen, mlen);
        for (i = 0; i < mlen; ++i) {
            double d = (double)gl_mag[i] - (double)mag[i];
            num += d * d;
            den += (double)mag[i] * (double)mag[i];
        }
        ISO_CHECK(den > 0.0 && (num / den) < 0.05);
        stft_free_buf(&gl_mag, gl_mlen);
    }

    stft_free_buf(&mag, mlen);
    stft_free_buf(&gl_recon, glen);
    stft_free_buf(&sig, n);
}

static void test_spectrograms_mel_and_mfcc(void) {
    const size_t n = 1024u;
    float *sig = (float *)malloc(n * sizeof(float));
    float *zeros = (float *)calloc(n, sizeof(float));
    float *mag = NULL, *pwr = NULL, *logp = NULL, *fb = NULL, *mel = NULL, *cc = NULL;
    size_t ml = 0, pl = 0, ll = 0, fbl = 0, mell = 0, ccl = 0;
    float mel_v = 0.0f, hz_v = 0.0f;
    size_t i;

    for (i = 0; i < n; ++i) {
        sig[i] = (float)((int)(i % 23u) - 11) * 0.05f;
    }

    ISO_CHECK(stft_magnitude_spectrogram(sig, n, 128u, 64u, STFT_WIN_HANN, &mag, &ml) == STFT_OK);
    ISO_CHECK(stft_power_spectrogram(sig, n, 128u, 64u, STFT_WIN_HANN, &pwr, &pl) == STFT_OK);
    ISO_CHECK(mag != NULL && pwr != NULL);
    ISO_CHECK_EQ_UINT(ml, pl);
    if (mag != NULL && pwr != NULL) {
        for (i = 0; i < ml; ++i) {
            ISO_CHECK(mag[i] >= 0.0f && pwr[i] >= 0.0f);
            ISO_CHECK(approx_eq(mag[i] * mag[i], pwr[i], 1e-4f));
        }
    }
    stft_free_buf(&mag, ml);
    stft_free_buf(&pwr, pl);

    ISO_CHECK(stft_log_spectrogram(zeros, n, 128u, 64u, STFT_WIN_HANN, &logp, &ll) == STFT_OK);
    for (i = 0; i < ll; ++i) {
        ISO_CHECK(approx_eq(logp[i], -23.02585f, 1e-3f));
    }
    stft_free_buf(&logp, ll);

    /* Mel Hz round-trip */
    {
        const float freqs[5] = {0.0f, 100.0f, 440.0f, 1000.0f, 8000.0f};
        for (i = 0; i < 5u; ++i) {
            ISO_CHECK(stft_hz_to_mel(freqs[i], &mel_v) == STFT_OK);
            ISO_CHECK(stft_mel_to_hz(mel_v, &hz_v) == STFT_OK);
            ISO_CHECK(approx_eq(hz_v, freqs[i], 1e-4f));
        }
    }

    /* Mel filterbank rows sum to 1.0 */
    ISO_CHECK(stft_mel_filterbank(40u, 1024u, 22050.0f, &fb, &fbl) == STFT_OK);
    ISO_CHECK_EQ_UINT(fbl, 40u * 513u);
    for (i = 0; i < 40u; ++i) {
        double rsum = 0.0;
        size_t k;
        for (k = 0; k < 513u; ++k) {
            float w = fb[i * 513u + k];
            ISO_CHECK(w >= 0.0f);
            rsum += w;
        }
        ISO_CHECK(approx_eq((float)rsum, 1.0f, 1e-4f));
    }
    stft_free_buf(&fb, fbl);

    /* Mel spectrogram & MFCC (exercises Bluestein O(N log N) DCT-II for n_mels=40) */
    ISO_CHECK(stft_mel_spectrogram(sig, n, 256u, 128u, 40u, 16000.0f, STFT_WIN_HANN, &mel, &mell) == STFT_OK);
    ISO_CHECK_EQ_UINT(mell, 7u * 40u);
    for (i = 0; i < mell; ++i) {
        ISO_CHECK(mel[i] >= 0.0f);
    }
    stft_free_buf(&mel, mell);

    ISO_CHECK(stft_mfcc(sig, n, 256u, 128u, 40u, 13u, 16000.0f, STFT_WIN_HANN, &cc, &ccl) == STFT_OK);
    ISO_CHECK_EQ_UINT(ccl, 7u * 13u);
    stft_free_buf(&cc, ccl);

    /* Zero signal MFCC finite */
    ISO_CHECK(stft_mfcc(zeros, n, 256u, 128u, 40u, 13u, 16000.0f, STFT_WIN_HANN, &cc, &ccl) == STFT_OK);
    ISO_CHECK_EQ_UINT(ccl, 7u * 13u);
    for (i = 0; i < ccl; ++i) {
        ISO_CHECK(cc[i] == cc[i]);
    }
    stft_free_buf(&cc, ccl);

    stft_free_buf(&sig, n);
    stft_free_buf(&zeros, n);
}

static void test_hostile_and_cwe(void) {
    float dummy[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    float hot[64];
    float *out = NULL;
    size_t olen = 0;
    float bad_vals[3];
    float f_out = 0.0f;
    int i;

    for (i = 0; i < 64; ++i) hot[i] = 1e20f;
    bad_vals[0] = make_nan();
    bad_vals[1] = make_inf();
    bad_vals[2] = make_neg_inf();

    /* Empty / NULL / domain checks */
    ISO_CHECK(stft_forward(dummy, 0, 64u, 32u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_EMPTY_SIGNAL);
    ISO_CHECK(stft_forward(NULL, 4, 2u, 1u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_forward(dummy, 4, 0u, 1u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_forward(dummy, 4, 2u, 0u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_forward(dummy, 4, 2u, 1u, (stft_window_t)99, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_istft(dummy, 0, 64u, 32u, STFT_WIN_HANN, 64u, &out, &olen) == STFT_ERR_INVALID_SPECTROGRAM);
    ISO_CHECK(stft_istft(dummy, 3, 2u, 1u, STFT_WIN_HANN, 4u, &out, &olen) == STFT_ERR_INVALID_SPECTROGRAM);

    /* CWE-125 / CWE-400: pre-flight bounds BEFORE scanning elements */
    ISO_CHECK(stft_forward(dummy, (size_t)STFT_MAX_SAMPLES + 1u, 64u, 32u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_forward(dummy, 4, 64u, 32u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_SIGNAL_TOO_SHORT);
    ISO_CHECK(stft_forward(dummy, 4, STFT_MAX_FFT_LEN + 1u, 32u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_istft(dummy, (size_t)STFT_MAX_SAMPLES + 1u, 64u, 32u, STFT_WIN_HANN, 64u, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_istft(dummy, 4, 2u, 1u, STFT_WIN_HANN, STFT_MAX_SAMPLES + 1u, &out, &olen) == STFT_ERR_INVALID_PARAM);

    /* Non-finite inputs */
    for (i = 0; i < 3; ++i) {
        float bad2[2] = {1.0f, 0.0f};
        float bad4[4] = {1.0f, 0.0f, 0.0f, 0.0f};
        bad2[1] = bad_vals[i];
        bad4[1] = bad_vals[i];
        ISO_CHECK(stft_forward(bad2, 2, 2u, 1u, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
        ISO_CHECK(stft_istft(bad4, 4, 2u, 1u, STFT_WIN_HANN, 4u, &out, &olen) == STFT_ERR_INVALID_SPECTROGRAM);
        ISO_CHECK(stft_mel_filterbank(40u, 512u, bad_vals[i], &out, &olen) == STFT_ERR_INVALID_PARAM);
        ISO_CHECK(stft_hz_to_mel(bad_vals[i], &f_out) == STFT_ERR_INVALID_PARAM);
        ISO_CHECK(stft_mel_to_hz(bad_vals[i], &f_out) == STFT_ERR_INVALID_PARAM);
    }

    /* Mel / MFCC / Log / Griffin-Lim bounds, precedence & overflows */
    ISO_CHECK(stft_log_spectrogram_eps(NULL, 0, 64u, 32u, STFT_WIN_HANN, 0.0f, &out, &olen) == STFT_ERR_EMPTY_SIGNAL);
    ISO_CHECK(stft_mel_spectrogram(NULL, 0, 1048576u, 0u, 0u, 0.0f, (stft_window_t)99, &out, &olen) == STFT_ERR_EMPTY_SIGNAL);
    ISO_CHECK(stft_mfcc(NULL, 0, 1048576u, 0u, 0u, 0u, 0.0f, (stft_window_t)99, &out, &olen) == STFT_ERR_EMPTY_SIGNAL);
    ISO_CHECK(stft_hz_to_mel(-1.0f, &f_out) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mel_to_hz(-1.0f, &f_out) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mel_to_hz(1000000.0f, &f_out) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mel_to_hz(800000.0f, &f_out) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mel_filterbank(40u, 512u, make_min_subnormal(), &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mel_filterbank(40u, 512u, 0.0f, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_power_spectrogram(hot, 64, 64u, 32u, STFT_WIN_RECTANGULAR, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_log_spectrogram_eps(dummy, 4, 2u, 1u, STFT_WIN_HANN, 0.0f, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mfcc(dummy, 4, 2u, 1u, 16u, 17u, 16000.0f, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);
    ISO_CHECK(stft_mfcc(dummy, 4, 2u, 1u, 16u, 0u, 16000.0f, STFT_WIN_HANN, &out, &olen) == STFT_ERR_INVALID_PARAM);

    {
        float neg_mag[4] = {1.0f, -0.5f, 0.0f, 0.0f};
        float mag33[33];
        size_t k;
        for (k = 0; k < 33u; ++k) mag33[k] = 1.0f;
        ISO_CHECK(stft_griffin_lim(neg_mag, 4, 2u, 1u, STFT_WIN_HANN, 4u, 4u, &out, &olen) == STFT_ERR_INVALID_SPECTROGRAM);
        ISO_CHECK(stft_griffin_lim(mag33, 33u, 64u, 32u, STFT_WIN_HANN, 10u, 2000000u, &out, &olen) == STFT_ERR_INVALID_SPECTROGRAM);
        ISO_CHECK(stft_griffin_lim(dummy, 4, 2u, 1u, STFT_WIN_HANN, 0u, 4u, &out, &olen) == STFT_ERR_INVALID_PARAM);
        ISO_CHECK(stft_griffin_lim(dummy, 4, 2u, 1u, STFT_WIN_HANN, STFT_MAX_GRIFFIN_LIM_ITERS + 1u, 4u, &out, &olen) == STFT_ERR_INVALID_PARAM);
    }

    /* Idempotent zero-on-free */
    ISO_CHECK(stft_hann_window(8u, &out, &olen) == STFT_OK);
    stft_free_buf(&out, olen);
    ISO_CHECK(out == NULL);
    stft_free_buf(&out, olen);
    ISO_CHECK(out == NULL);
}

static void test_pbt_splitmix64_invariants(void) {
    splitmix64_t rng;
    int iter;
    rng.state = UINT64_C(0x123456789ABCDEF0);

    for (iter = 0; iter < 10; ++iter) {
        const size_t n = 256u;
        float *s1 = (float *)malloc(n * sizeof(float));
        float *s2 = (float *)malloc(n * sizeof(float));
        float *comb = (float *)malloc(n * sizeof(float));
        float alpha = sm64_sample_f32(&rng, -2.0f, 2.0f);
        float beta = sm64_sample_f32(&rng, -2.0f, 2.0f);
        float *sp1 = NULL, *sp2 = NULL, *spc = NULL, *rec = NULL;
        size_t l1 = 0, l2 = 0, lc = 0, lr = 0;
        size_t i;

        for (i = 0; i < n; ++i) {
            s1[i] = sm64_sample_f32(&rng, -1.0f, 1.0f);
            s2[i] = sm64_sample_f32(&rng, -1.0f, 1.0f);
            comb[i] = alpha * s1[i] + beta * s2[i];
        }

        /* Linearity: stft(alpha*s1 + beta*s2) == alpha*stft(s1) + beta*stft(s2) */
        ISO_CHECK(stft_forward(s1, n, 64u, 32u, STFT_WIN_HANN, &sp1, &l1) == STFT_OK);
        ISO_CHECK(stft_forward(s2, n, 64u, 32u, STFT_WIN_HANN, &sp2, &l2) == STFT_OK);
        ISO_CHECK(stft_forward(comb, n, 64u, 32u, STFT_WIN_HANN, &spc, &lc) == STFT_OK);
        ISO_CHECK_EQ_UINT(l1, l2);
        ISO_CHECK_EQ_UINT(l1, lc);
        for (i = 0; i < lc; ++i) {
            ISO_CHECK(approx_eq(spc[i], alpha * sp1[i] + beta * sp2[i], 1e-4f));
        }

        /* COLA roundtrip */
        ISO_CHECK(stft_istft(sp1, l1, 64u, 32u, STFT_WIN_HANN, (uint32_t)n, &rec, &lr) == STFT_OK);
        ISO_CHECK_EQ_UINT(lr, n);
        for (i = 64u; i < n - 64u; ++i) {
            ISO_CHECK(approx_eq(rec[i], s1[i], 1e-3f));
        }

        stft_free_buf(&sp1, l1);
        stft_free_buf(&sp2, l2);
        stft_free_buf(&spc, lc);
        stft_free_buf(&rec, lr);
        stft_free_buf(&s1, n);
        stft_free_buf(&s2, n);
        stft_free_buf(&comb, n);
    }

    /* Parseval energy theorem on single-frame Rectangular STFT (N=64) */
    {
        float frame[64];
        float *spec = NULL;
        size_t slen = 0;
        double time_e = 0.0, freq_e = 0.0;
        size_t k;
        for (k = 0; k < 64u; ++k) {
            frame[k] = sm64_sample_f32(&rng, -1.0f, 1.0f);
            time_e += (double)frame[k] * (double)frame[k];
        }
        ISO_CHECK(stft_forward(frame, 64u, 64u, 64u, STFT_WIN_RECTANGULAR, &spec, &slen) == STFT_OK);
        /* bins = 33: DC + 31 interior (counted twice) + Nyquist */
        freq_e += (double)spec[0] * (double)spec[0] + (double)spec[1] * (double)spec[1];
        for (k = 1u; k < 32u; ++k) {
            double re = spec[2u * k], im = spec[2u * k + 1u];
            freq_e += 2.0 * (re * re + im * im);
        }
        freq_e += (double)spec[64u] * (double)spec[64u] + (double)spec[65u] * (double)spec[65u];
        freq_e /= 64.0;
        ISO_CHECK(approx_eq((float)time_e, (float)freq_e, 1e-4f));
        stft_free_buf(&spec, slen);
    }
}

int main(void) {
    test_windows_and_parsing();
    test_stft_istft_cola_and_griffin_lim();
    test_spectrograms_mel_and_mfcc();
    test_hostile_and_cwe();
    test_pbt_splitmix64_invariants();
    return ISO_TEST_RESULT();
}
