/*
 * dsp_filters_test.c — unit, hostile-input, and SplitMix64 PBT tests for c/dsp-filters.
 */
#include "dsp_filters.h"
#include "iso_test.h"

#include <float.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

#define TEST_PI 3.14159265358979323846

static float make_nan_f(void) {
    volatile float z = 0.0f;
    return z / z;
}

static float make_inf_f(void) {
    volatile float z = 0.0f;
    return 1.0f / z;
}

static double abs_d(double x) {
    return x < 0.0 ? -x : x;
}

static void check_close_slice(const float *a, const float *b, size_t n, double tol) {
    size_t i;
    for (i = 0; i < n; ++i) {
        double da = (double)a[i];
        double db = (double)b[i];
        double aa = abs_d(da);
        double ab = abs_d(db);
        double scale = aa > ab ? aa : ab;
        if (scale < 1.0) {
            scale = 1.0;
        }
        ISO_CHECK_EQ_DBL(da, db, scale * tol);
    }
}

static double test_sin(double x) {
    double two_pi = 2.0 * TEST_PI;
    int64_t k = (int64_t)(x / two_pi + (x >= 0.0 ? 0.5 : -0.5));
    double r = x - (double)k * two_pi;
    double z = r * r;
    double term = r;
    double sum = r;
    int n;
    for (n = 1; n <= 12; ++n) {
        term *= -z / (double)((2 * n) * (2 * n + 1));
        sum += term;
    }
    return sum;
}

typedef struct {
    uint64_t state;
} splitmix64_t;

static uint64_t splitmix64_next(splitmix64_t *rng) {
    uint64_t z;
    rng->state += 0x9e3779b97f4a7c15ULL;
    z = rng->state;
    z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9ULL;
    z = (z ^ (z >> 27)) * 0x94d049bb133111ebULL;
    return z ^ (z >> 31);
}

static float sample_f32(splitmix64_t *rng) {
    uint64_t u = splitmix64_next(rng);
    float m = (float)(u & 0x00ffffffULL) / (float)0x01000000ULL;
    return (m * 2.0f) - 1.0f;
}

/* ------------------------------------------------------------------ */
/* FIR & Windowed-Sinc Design Tests                                   */
/* ------------------------------------------------------------------ */

static void test_fir_and_fir_fft(void) {
    float sig[8] = {1.0f, 2.0f, 3.0f, 4.0f, 5.0f, 6.0f, 7.0f, 8.0f};
    float ker[3] = {0.25f, 0.5f, 0.25f};
    float *d = NULL;
    float *f = NULL;
    size_t dlen = 0;
    size_t flen = 0;

    ISO_CHECK_EQ_INT(dsp_fir(sig, 8, ker, 3, &d, &dlen), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT(dsp_fir_fft(sig, 8, ker, 3, &f, &flen), DSP_FILTER_OK);
    ISO_CHECK_EQ_UINT(dlen, 10u);
    ISO_CHECK_EQ_UINT(flen, 10u);
    check_close_slice(d, f, 10, 1e-4);
    dsp_filter_free(d);
    dsp_filter_free(f);

    /* Identity kernel [1.0] */
    {
        float id_k[1] = {1.0f};
        ISO_CHECK_EQ_INT(dsp_fir(sig, 8, id_k, 1, &d, &dlen), DSP_FILTER_OK);
        check_close_slice(d, sig, 8, 1e-6);
        dsp_filter_free(d);
    }
}

static void test_fir_design_helpers(void) {
    float *lp = NULL, *hp = NULL, *bp = NULL, *bs = NULL;
    size_t len = 0;
    double sum = 0.0;
    size_t i;

    ISO_CHECK_EQ_INT(dsp_design_low_pass(0.2f, 33u, DSP_WINDOW_HAMMING, &lp, &len), DSP_FILTER_OK);
    ISO_CHECK_EQ_UINT(len, 33u);
    for (i = 0; i < len; ++i) {
        sum += (double)lp[i];
        ISO_CHECK_EQ_DBL(lp[i], lp[len - 1 - i], 1e-6);
    }
    ISO_CHECK_EQ_DBL(sum, 1.0, 1e-5);
    dsp_filter_free(lp);

    ISO_CHECK_EQ_INT(dsp_design_high_pass(0.2f, 33u, DSP_WINDOW_HANN, &hp, &len), DSP_FILTER_OK);
    sum = 0.0;
    for (i = 0; i < len; ++i) {
        sum += (double)hp[i];
    }
    ISO_CHECK_EQ_DBL(sum, 0.0, 1e-5);
    dsp_filter_free(hp);

    ISO_CHECK_EQ_INT(dsp_design_band_pass(0.15f, 0.35f, 65u, DSP_WINDOW_BLACKMAN, &bp, &len), DSP_FILTER_OK);
    sum = 0.0;
    for (i = 0; i < len; ++i) {
        sum += (double)bp[i];
    }
    ISO_CHECK(abs_d(sum) < 0.05);
    dsp_filter_free(bp);

    ISO_CHECK_EQ_INT(dsp_design_band_stop(0.15f, 0.35f, 65u, DSP_WINDOW_RECTANGULAR, &bs, &len), DSP_FILTER_OK);
    sum = 0.0;
    for (i = 0; i < len; ++i) {
        sum += (double)bs[i];
    }
    ISO_CHECK_EQ_DBL(sum, 1.0, 0.05);
    dsp_filter_free(bs);
}

/* ------------------------------------------------------------------ */
/* IIR, Biquad, SOS & Butterworth Tests                               */
/* ------------------------------------------------------------------ */

static void test_iir_and_biquad(void) {
    float step[200];
    float b1[1] = {1.0f};
    float a1[2] = {1.0f, -0.9f};
    float *out = NULL;
    size_t out_len = 0;
    size_t i;

    for (i = 0; i < 200; ++i) {
        step[i] = 1.0f;
    }
    ISO_CHECK_EQ_INT(dsp_iir(step, 200, b1, 1, a1, 2, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK_EQ_DBL(out[0], 1.0, 1e-5);
    ISO_CHECK_EQ_DBL(out[1], 1.9, 1e-5);
    ISO_CHECK_EQ_DBL(out[199], 10.0, 1e-3);
    dsp_filter_free(out);

    /* Failure-atomic Biquad: mid-block NaN must NOT mutate z1/z2 */
    {
        dsp_biquad_t bq;
        float ok_blk[3] = {1.0f, 0.5f, -0.25f};
        float bad_blk[3] = {1.0f, 0.0f, 0.5f};
        float z1_saved, z2_saved;
        float s_ok = 0.0f;
        bad_blk[1] = make_nan_f();

        ISO_CHECK_EQ_INT(dsp_biquad_lowpass(&bq, 0.2f, 0.70710678f), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_biquad_process(&bq, ok_blk, 3, &out, &out_len), DSP_FILTER_OK);
        dsp_filter_free(out);
        z1_saved = bq.z1;
        z2_saved = bq.z2;

        ISO_CHECK_EQ_INT(dsp_biquad_process(&bq, bad_blk, 3, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK(out == NULL);
        ISO_CHECK_EQ_DBL(bq.z1, z1_saved, 0.0);
        ISO_CHECK_EQ_DBL(bq.z2, z2_saved, 0.0);

        ISO_CHECK_EQ_INT(dsp_biquad_process_sample(&bq, make_inf_f(), &s_ok), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_DBL(bq.z1, z1_saved, 0.0);
        ISO_CHECK_EQ_DBL(bq.z2, z2_saved, 0.0);

        dsp_biquad_reset(&bq);
        ISO_CHECK_EQ_DBL(bq.z1, 0.0, 0.0);
        ISO_CHECK_EQ_DBL(bq.z2, 0.0, 0.0);
    }

    /* Cookbook constructors smoke check */
    {
        dsp_biquad_t bq;
        ISO_CHECK_EQ_INT(dsp_biquad_highpass(&bq, 0.1f, 0.707f), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_biquad_bandpass(&bq, 0.2f, 1.0f), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_biquad_notch(&bq, 0.25f, 2.0f), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_biquad_allpass(&bq, 0.25f, 1.0f), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_biquad_peak(&bq, 0.25f, 6.0f, 1.0f), DSP_FILTER_OK);
    }
}

static double steady_rms(const float *x, size_t start, size_t end) {
    double acc = 0.0;
    size_t i;
    for (i = start; i < end; ++i) {
        acc += (double)x[i] * (double)x[i];
    }
    return acc / (double)(end - start);
}

static void test_butterworth_all_types_and_zero_phase(void) {
    dsp_sos_section_t *sos = NULL;
    size_t n_sec = 0;
    float sig_lo[256];
    float sig_hi[256];
    float sig_mid[256];
    float *out = NULL;
    size_t out_len = 0;
    size_t i;

    for (i = 0; i < 256; ++i) {
        sig_lo[i] = (float)test_sin(2.0 * TEST_PI * 0.03 * (double)i);
        sig_hi[i] = (float)test_sin(2.0 * TEST_PI * 0.42 * (double)i);
        sig_mid[i] = (float)test_sin(2.0 * TEST_PI * 0.25 * (double)i);
    }

    /* 4th-order Butterworth Lowpass at 0.15 */
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_LOWPASS, 4u, 0.15f, 0.0f, &sos, &n_sec), DSP_FILTER_OK);
    ISO_CHECK_EQ_UINT(n_sec, 2u);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_lo, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) > 0.4);
    dsp_filter_free(out);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_hi, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) < 0.01);
    dsp_filter_free(out);
    dsp_filter_free(sos);

    /* 4th-order Butterworth Highpass at 0.30 */
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_HIGHPASS, 4u, 0.30f, 0.0f, &sos, &n_sec), DSP_FILTER_OK);
    ISO_CHECK_EQ_UINT(n_sec, 2u);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_hi, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) > 0.4);
    dsp_filter_free(out);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_lo, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) < 0.01);
    dsp_filter_free(out);
    dsp_filter_free(sos);

    /* 3rd-order Butterworth Bandpass [0.18, 0.32] (odd order exercises real + complex pair) */
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_BANDPASS, 3u, 0.18f, 0.32f, &sos, &n_sec), DSP_FILTER_OK);
    ISO_CHECK_EQ_UINT(n_sec, 3u);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_mid, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) > 0.4);
    dsp_filter_free(out);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_lo, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) < 0.01);
    dsp_filter_free(out);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_hi, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) < 0.01);
    dsp_filter_free(out);
    dsp_filter_free(sos);

    /* 2nd-order Butterworth Bandstop [0.18, 0.32] */
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_BANDSTOP, 2u, 0.18f, 0.32f, &sos, &n_sec), DSP_FILTER_OK);
    ISO_CHECK_EQ_UINT(n_sec, 2u);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_mid, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) < 0.02);
    dsp_filter_free(out);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_lo, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) > 0.4);
    dsp_filter_free(out);
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_hi, 256, &out, &out_len), DSP_FILTER_OK);
    ISO_CHECK(steady_rms(out, 64, 256) > 0.4);
    dsp_filter_free(out);

    /* Zero-phase sosfiltfilt preserves phase of passband sinusoid */
    {
        float *zp = NULL;
        size_t zp_len = 0;
        double dot = 0.0;
        ISO_CHECK_EQ_INT(dsp_sosfiltfilt(sos, n_sec, sig_lo, 256, &zp, &zp_len), DSP_FILTER_OK);
        for (i = 32; i < 224; ++i) {
            dot += (double)zp[i] * (double)sig_lo[i];
        }
        ISO_CHECK(dot > 80.0);
        dsp_filter_free(zp);
    }
    dsp_filter_free(sos);

    /* (b, a) Butterworth parity vs SOS + filtfilt */
    {
        float *b = NULL, *a = NULL;
        size_t blen = 0, alen = 0;
        float *y_ba = NULL, *y_sos = NULL;
        size_t yblen = 0, yslen = 0;
        ISO_CHECK_EQ_INT(dsp_butterworth_lowpass(2u, 0.2f, &b, &blen, &a, &alen), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_LOWPASS, 2u, 0.2f, 0.0f, &sos, &n_sec), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_iir(sig_lo, 256, b, blen, a, alen, &y_ba, &yblen), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig_lo, 256, &y_sos, &yslen), DSP_FILTER_OK);
        check_close_slice(y_ba, y_sos, 256, 1e-4);
        dsp_filter_free(y_ba);
        dsp_filter_free(y_sos);
        ISO_CHECK_EQ_INT(dsp_filtfilt(sig_lo, 256, b, blen, a, alen, &y_ba, &yblen), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_sosfiltfilt(sos, n_sec, sig_lo, 256, &y_sos, &yslen), DSP_FILTER_OK);
        check_close_slice(y_ba, y_sos, 256, 1e-4);
        dsp_filter_free(y_ba);
        dsp_filter_free(y_sos);
        dsp_filter_free(b);
        dsp_filter_free(a);
        dsp_filter_free(sos);

        ISO_CHECK_EQ_INT(dsp_butterworth_highpass(1u, 0.2f, &b, &blen, &a, &alen), DSP_FILTER_OK);
        dsp_filter_free(b);
        dsp_filter_free(a);
        ISO_CHECK_EQ_INT(dsp_butterworth_bandpass(2u, 0.15f, 0.35f, &b, &blen, &a, &alen), DSP_FILTER_OK);
        ISO_CHECK_EQ_UINT(blen, 5u);
        dsp_filter_free(b);
        dsp_filter_free(a);
        ISO_CHECK_EQ_INT(dsp_butterworth_bandstop(1u, 0.15f, 0.35f, &b, &blen, &a, &alen), DSP_FILTER_OK);
        ISO_CHECK_EQ_UINT(blen, 3u);
        dsp_filter_free(b);
        dsp_filter_free(a);
    }

    /* Empty SOS pass-through */
    ISO_CHECK_EQ_INT(dsp_sosfilt(NULL, 0, sig_lo, 16, &out, &out_len), DSP_FILTER_OK);
    check_close_slice(out, sig_lo, 16, 0.0);
    dsp_filter_free(out);
}

static void test_inplace_and_parsers(void) {
    float sig[32];
    float buf[32];
    float b[2] = {0.5f, 0.5f};
    float a[2] = {1.0f, -0.2f};
    float *ref = NULL;
    size_t rlen = 0;
    dsp_sos_section_t *sos = NULL;
    size_t n_sec = 0;
    dsp_biquad_t bq;
    dsp_window_type_t win;
    dsp_butter_type_t bt;
    size_t i;

    for (i = 0; i < 32; ++i) {
        sig[i] = (float)i * 0.1f;
    }

    memcpy(buf, sig, sizeof(sig));
    ISO_CHECK_EQ_INT(dsp_iir(sig, 32, b, 2, a, 2, &ref, &rlen), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT(dsp_iir_into(buf, 32, b, 2, a, 2, buf), DSP_FILTER_OK);
    check_close_slice(buf, ref, 32, 1e-6);
    dsp_filter_free(ref);

    memcpy(buf, sig, sizeof(sig));
    ISO_CHECK_EQ_INT(dsp_filtfilt(sig, 32, b, 2, a, 2, &ref, &rlen), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT(dsp_filtfilt_into(buf, 32, b, 2, a, 2, buf), DSP_FILTER_OK);
    check_close_slice(buf, ref, 32, 1e-6);
    dsp_filter_free(ref);

    ISO_CHECK_EQ_INT(dsp_biquad_lowpass(&bq, 0.2f, 0.707f), DSP_FILTER_OK);
    memcpy(buf, sig, sizeof(sig));
    ISO_CHECK_EQ_INT(dsp_biquad_process_into(&bq, buf, 32, buf), DSP_FILTER_OK);

    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_LOWPASS, 2u, 0.2f, 0.0f, &sos, &n_sec), DSP_FILTER_OK);
    memcpy(buf, sig, sizeof(sig));
    ISO_CHECK_EQ_INT(dsp_sosfilt(sos, n_sec, sig, 32, &ref, &rlen), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT(dsp_sosfilt_into(sos, n_sec, buf, 32, buf), DSP_FILTER_OK);
    check_close_slice(buf, ref, 32, 1e-6);
    dsp_filter_free(ref);

    memcpy(buf, sig, sizeof(sig));
    ISO_CHECK_EQ_INT(dsp_sosfiltfilt(sos, n_sec, sig, 32, &ref, &rlen), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT(dsp_sosfiltfilt_into(sos, n_sec, buf, 32, buf), DSP_FILTER_OK);
    check_close_slice(buf, ref, 32, 1e-6);
    dsp_filter_free(ref);
    dsp_filter_free(sos);

    /* Parsers */
    ISO_CHECK_EQ_INT(dsp_window_parse(" Hamming ", &win), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT((int)win, (int)DSP_WINDOW_HAMMING);
    ISO_CHECK_EQ_INT(dsp_window_parse("bad", &win), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butter_parse_type("bandstop", &bt), DSP_FILTER_OK);
    ISO_CHECK_EQ_INT((int)bt, (int)DSP_BUTTER_BANDSTOP);
    ISO_CHECK_EQ_INT(dsp_butter_parse_type("notch!", &bt), DSP_FILTER_ERR_INVALID_COEFFICIENT);
}

static void test_hostile_inputs(void) {
    float valid[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    float zero_a[2] = {0.0f, 0.5f};
    float *out = NULL, *b = NULL, *a = NULL;
    size_t out_len = 0, blen = 0, alen = 0;
    dsp_sos_section_t *sos = NULL;
    size_t n_sec = 0;
    dsp_biquad_t bq;
    float nan_v = make_nan_f();
    float pinf = make_inf_f();

    /* Empty & NULL */
    ISO_CHECK_EQ_INT(dsp_fir(valid, 0, valid, 2, &out, &out_len), DSP_FILTER_ERR_EMPTY_SIGNAL);
    ISO_CHECK_EQ_INT(dsp_fir(valid, 2, valid, 0, &out, &out_len), DSP_FILTER_ERR_EMPTY_KERNEL);
    ISO_CHECK_EQ_INT(dsp_fir_fft(valid, 0, valid, 2, &out, &out_len), DSP_FILTER_ERR_EMPTY_SIGNAL);
    ISO_CHECK_EQ_INT(dsp_iir(valid, 0, valid, 1, valid, 1, &out, &out_len), DSP_FILTER_ERR_EMPTY_SIGNAL);
    ISO_CHECK_EQ_INT(dsp_iir(valid, 2, valid, 0, valid, 1, &out, &out_len), DSP_FILTER_ERR_EMPTY_KERNEL);
    ISO_CHECK_EQ_INT(dsp_iir(valid, 2, valid, 1, valid, 0, &out, &out_len), DSP_FILTER_ERR_EMPTY_KERNEL);
    ISO_CHECK_EQ_INT(dsp_iir(valid, 2, valid, 1, zero_a, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_fir(NULL, 2, valid, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_sosfilt(NULL, 1, valid, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);

    /* Enum & parameter validation */
    ISO_CHECK_EQ_INT(dsp_design_low_pass(0.2f, 5u, (dsp_window_type_t)99, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_design_low_pass(0.2f, 4u, DSP_WINDOW_HAMMING, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_design_low_pass(0.6f, 5u, DSP_WINDOW_HAMMING, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_design_band_pass(0.3f, 0.2f, 5u, DSP_WINDOW_HAMMING, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butterworth_sos((dsp_butter_type_t)99, 2u, 0.2f, 0.0f, &sos, &n_sec), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_LOWPASS, 0u, 0.2f, 0.0f, &sos, &n_sec), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_LOWPASS, 65u, 0.2f, 0.0f, &sos, &n_sec), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_LOWPASS, 2u, 0.5f, 0.0f, &sos, &n_sec), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butterworth_sos(DSP_BUTTER_BANDPASS, 2u, 0.3f, 0.2f, &sos, &n_sec), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_butterworth_lowpass(3u, 0.2f, &b, &blen, &a, &alen), DSP_FILTER_ERR_INVALID_COEFFICIENT);

    /* Non-finite & FLT_MAX overflow */
    {
        float bad_sig[2];
        float huge[2] = {FLT_MAX, FLT_MAX};
        float one[1] = {1.0f};
        bad_sig[0] = 1.0f;
        bad_sig[1] = nan_v;
        ISO_CHECK_EQ_INT(dsp_fir(bad_sig, 2, valid, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_fir(huge, 2, huge, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_fir_fft(huge, 2, huge, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_biquad_init(&bq, 1.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_biquad_init(&bq, FLT_MAX, 0.0f, 0.0f, 1e-30f, 0.0f, 0.0f), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_biquad_peak(&bq, 0.2f, 1e6f, 1.0f), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_iir(huge, 2, one, 1, one, 1, &out, &out_len), DSP_FILTER_OK);
        dsp_filter_free(out);
        bad_sig[1] = pinf;
        ISO_CHECK_EQ_INT(dsp_sosfiltfilt(NULL, 0, bad_sig, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    }

    /* Separately rounded f32 DFII-T overflow & Biquad validation order (DSP03 AF-03) */
    {
        dsp_sos_section_t two_sec[2] = {
            {2.0f, 0.0f, 0.0f, 1.0f, 0.0f, 0.0f},
            {0.5f, 0.0f, 0.0f, 1.0f, 0.0f, 0.0f}
        };
        dsp_sos_section_t b1_sec[1] = {{0.0f, 2.0f, 0.0f, 1.0f, 0.0f, 0.0f}};
        dsp_sos_section_t fma_sec[1] = {{FLT_MAX, 0.0f, 0.0f, 1.0f, 1.0f, 0.0f}};
        float fmax_one[1] = {FLT_MAX};
        float seq_two[2] = {1.0f, 2.0f};
        float y = 0.0f;
        float scratch = 0.0f;

        ISO_CHECK_EQ_INT(dsp_sosfilt(two_sec, 2, fmax_one, 1, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK(out == NULL);
        ISO_CHECK_EQ_INT(dsp_sosfilt(b1_sec, 1, fmax_one, 1, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK(out == NULL);
        ISO_CHECK_EQ_INT(dsp_sosfilt(fma_sec, 1, seq_two, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK(out == NULL);

        ISO_CHECK_EQ_INT(dsp_biquad_init(&bq, FLT_MAX, 0.0f, 0.0f, 1.0f, 1.0f, 0.0f), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_biquad_process(&bq, seq_two, 2, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_DBL(bq.z1, 0.0, 0.0);
        ISO_CHECK_EQ_DBL(bq.z2, 0.0, 0.0);

        ISO_CHECK_EQ_INT(dsp_biquad_process_sample(&bq, 1.0f, &y), DSP_FILTER_OK);
        ISO_CHECK_EQ_DBL(y, (double)FLT_MAX, 0.0);
        ISO_CHECK_EQ_DBL(bq.z1, -(double)FLT_MAX, 0.0);
        ISO_CHECK_EQ_INT(dsp_biquad_process_sample(&bq, 2.0f, &y), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_DBL(bq.z1, -(double)FLT_MAX, 0.0);

        bq.b0 = nan_v;
        ISO_CHECK_EQ_INT(dsp_biquad_process_sample(&bq, 1.0f, &y), DSP_FILTER_ERR_INVALID_COEFFICIENT);
        ISO_CHECK_EQ_INT(dsp_biquad_process_into(&bq, valid, 0, &scratch), DSP_FILTER_ERR_EMPTY_SIGNAL);
    }

    /* Cap rejection */
    ISO_CHECK_EQ_INT(dsp_fir(valid, DSP_FILTER_MAX_SAMPLES + 1, valid, 1, &out, &out_len), DSP_FILTER_ERR_INVALID_COEFFICIENT);
    ISO_CHECK_EQ_INT(dsp_design_low_pass(0.2f, (uint32_t)(DSP_FILTER_MAX_TAPS + 1), DSP_WINDOW_RECTANGULAR, &out, &out_len),
                     DSP_FILTER_ERR_INVALID_COEFFICIENT);
}

static void test_pbt_splitmix64(void) {
    splitmix64_t rng;
    static const size_t pairs[5][2] = {{8, 3}, {16, 5}, {32, 8}, {64, 17}, {100, 20}};
    size_t pi;
    rng.state = 0x123456789ABCDEF0ULL;

    for (pi = 0; pi < 5; ++pi) {
        size_t n = pairs[pi][0];
        size_t k = pairs[pi][1];
        float *sig = (float *)malloc(n * sizeof(float));
        float *ker = (float *)malloc(k * sizeof(float));
        float *d = NULL, *f = NULL;
        size_t dlen = 0, flen = 0;
        size_t i;
        ISO_CHECK(sig != NULL && ker != NULL);
        for (i = 0; i < n; ++i) sig[i] = sample_f32(&rng);
        for (i = 0; i < k; ++i) ker[i] = sample_f32(&rng);
        ISO_CHECK_EQ_INT(dsp_fir(sig, n, ker, k, &d, &dlen), DSP_FILTER_OK);
        ISO_CHECK_EQ_INT(dsp_fir_fft(sig, n, ker, k, &f, &flen), DSP_FILTER_OK);
        check_close_slice(d, f, dlen, 1e-4);
        dsp_filter_free(d);
        dsp_filter_free(f);
        free(sig);
        free(ker);
    }
}

int main(void) {
    test_fir_and_fir_fft();
    test_fir_design_helpers();
    test_iir_and_biquad();
    test_butterworth_all_types_and_zero_phase();
    test_inplace_and_parsers();
    test_hostile_inputs();
    test_pbt_splitmix64();
    return ISO_TEST_RESULT();
}
