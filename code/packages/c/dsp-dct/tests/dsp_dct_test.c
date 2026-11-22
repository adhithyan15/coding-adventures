/*
 * dsp_dct_test.c — unit, hostile-input, and SplitMix64 PBT tests for c/dsp-dct.
 */
#include "dsp_dct.h"
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

static int all_finite_f(const float *a, size_t n) {
    size_t i;
    for (i = 0; i < n; ++i) {
        if (!((a[i] == a[i]) && ((a[i] - a[i]) == 0.0f))) {
            return 0;
        }
    }
    return 1;
}

/* Deterministic SplitMix64 PRNG (matching coding_adventures_test_bootstrap). */
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

/* Pure ISO test-local trig/sqrt for closed-form oracles. */
static double test_cos(double x) {
    double two_pi = 2.0 * TEST_PI;
    int64_t k = (int64_t)(x / two_pi + (x >= 0.0 ? 0.5 : -0.5));
    double r = x - (double)k * two_pi;
    double z = r * r;
    double term = 1.0;
    double sum = 1.0;
    int n;
    for (n = 1; n <= 12; ++n) {
        term *= -z / (double)((2 * n - 1) * (2 * n));
        sum += term;
    }
    return sum;
}

static double test_sqrt(double x) {
    double g = x > 1.0 ? x : 1.0;
    int i;
    for (i = 0; i < 32; ++i) {
        g = 0.5 * (g + x / g);
    }
    return g;
}

/* ------------------------------------------------------------------ */
/* Unit Tests: DCT-I..IV & DST-I..IV                                  */
/* ------------------------------------------------------------------ */

static void test_dct_closed_form(void) {
    /* Impulse x = [1, 0, ..., 0] -> DCT-II(None) X[k] = 2 * cos(pi*k/(2N)) */
    float sig[8] = {1.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f};
    float *out = NULL;
    size_t out_len = 0;
    size_t k;
    ISO_CHECK_EQ_INT(dsp_dct_1d(sig, 8, DSP_DCT_II, DSP_DCT_NORM_NONE, &out, &out_len), DSP_DCT_OK);
    ISO_CHECK_EQ_UINT(out_len, 8u);
    for (k = 0; k < 8; ++k) {
        double expected = 2.0 * test_cos(TEST_PI * (double)k / 16.0);
        ISO_CHECK_EQ_DBL(out[k], expected, 1e-5);
    }
    dsp_dct_free(out);

    /* Constant DC signal -> DCT-II(Ortho) concentrates at bin 0: X[0] = sqrt(N) */
    {
        float dc[16];
        for (k = 0; k < 16; ++k) {
            dc[k] = 1.0f;
        }
        ISO_CHECK_EQ_INT(dsp_dct_1d(dc, 16, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_OK);
        ISO_CHECK_EQ_DBL(out[0], test_sqrt(16.0), 1e-5);
        for (k = 1; k < 16; ++k) {
            ISO_CHECK_EQ_DBL(out[k], 0.0, 1e-5);
        }
        dsp_dct_free(out);
    }
}

static void test_dct_ii_iii_matches_naive_oracle(void) {
    static const size_t sizes[6] = {2, 3, 4, 5, 8, 16};
    size_t si;
    for (si = 0; si < 6; ++si) {
        size_t n = sizes[si];
        float sig[16];
        float naive_ii[16];
        float naive_iii[16];
        float *fft_ii = NULL;
        float *fft_iii = NULL;
        size_t out_len = 0;
        size_t k;
        size_t m;

        for (m = 0; m < n; ++m) {
            sig[m] = (float)test_cos(0.3 * (double)m - 0.7);
        }
        for (k = 0; k < n; ++k) {
            double acc2 = 0.0;
            double acc3 = (double)sig[0];
            for (m = 0; m < n; ++m) {
                acc2 += (double)sig[m] * test_cos(TEST_PI * (double)k * (2.0 * (double)m + 1.0) / (2.0 * (double)n));
            }
            for (m = 1; m < n; ++m) {
                acc3 += 2.0 * (double)sig[m] * test_cos(TEST_PI * (double)m * (2.0 * (double)k + 1.0) / (2.0 * (double)n));
            }
            naive_ii[k] = (float)(2.0 * acc2);
            naive_iii[k] = (float)acc3;
        }

        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, n, DSP_DCT_II, DSP_DCT_NORM_NONE, &fft_ii, &out_len), DSP_DCT_OK);
        check_close_slice(fft_ii, naive_ii, n, 1e-4);
        dsp_dct_free(fft_ii);

        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, n, DSP_DCT_III, DSP_DCT_NORM_NONE, &fft_iii, &out_len), DSP_DCT_OK);
        check_close_slice(fft_iii, naive_iii, n, 1e-4);
        dsp_dct_free(fft_iii);
    }
}

static void test_all_dct_and_dst_roundtrips(void) {
    static const dsp_dct_type_t dct_fwd[4] = {DSP_DCT_I, DSP_DCT_II, DSP_DCT_III, DSP_DCT_IV};
    static const dsp_dct_type_t dct_inv[4] = {DSP_DCT_I, DSP_DCT_III, DSP_DCT_II, DSP_DCT_IV};
    static const dsp_dst_type_t dst_fwd[4] = {DSP_DST_I, DSP_DST_II, DSP_DST_III, DSP_DST_IV};
    static const dsp_dst_type_t dst_inv[4] = {DSP_DST_I, DSP_DST_III, DSP_DST_II, DSP_DST_IV};
    static const dsp_dct_norm_t norms[2] = {DSP_DCT_NORM_NONE, DSP_DCT_NORM_ORTHO};
    float sig[7] = {1.5f, -2.25f, 3.0f, 0.5f, -1.0f, 4.25f, -0.75f};
    size_t ti;
    size_t ni;

    for (ti = 0; ti < 4; ++ti) {
        for (ni = 0; ni < 2; ++ni) {
            float *c = NULL;
            float *r = NULL;
            size_t clen = 0;
            size_t rlen = 0;
            ISO_CHECK_EQ_INT(dsp_dct_1d(sig, 7, dct_fwd[ti], norms[ni], &c, &clen), DSP_DCT_OK);
            ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, dct_inv[ti], norms[ni], &r, &rlen), DSP_DCT_OK);
            check_close_slice(r, sig, 7, 1e-4);
            dsp_dct_free(c);
            dsp_dct_free(r);

            ISO_CHECK_EQ_INT(dsp_dst_1d(sig, 7, dst_fwd[ti], norms[ni], &c, &clen), DSP_DCT_OK);
            ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, dst_inv[ti], norms[ni], &r, &rlen), DSP_DCT_OK);
            check_close_slice(r, sig, 7, 1e-4);
            dsp_dct_free(c);
            dsp_dct_free(r);
        }
    }
}

static void test_2d_transforms(void) {
    float dc[64];
    float *c = NULL;
    float *r = NULL;
    size_t clen = 0;
    size_t rlen = 0;
    size_t i;

    for (i = 0; i < 64; ++i) {
        dc[i] = 1.0f;
    }
    ISO_CHECK_EQ_INT(dsp_dct_2d(dc, 64, 8, 8, DSP_DCT_II, DSP_DCT_NORM_NONE, &c, &clen), DSP_DCT_OK);
    ISO_CHECK_EQ_UINT(clen, 64u);
    ISO_CHECK_EQ_DBL(c[0], 256.0, 1e-3);
    for (i = 1; i < 64; ++i) {
        ISO_CHECK_EQ_DBL(c[i], 0.0, 1e-3);
    }
    ISO_CHECK_EQ_INT(dsp_idct_2d(c, clen, 8, 8, DSP_DCT_III, DSP_DCT_NORM_NONE, &r, &rlen), DSP_DCT_OK);
    check_close_slice(r, dc, 64, 1e-4);
    dsp_dct_free(c);
    dsp_dct_free(r);

    /* Non-square 3x5 DCT-II/III & DST-II/III roundtrip */
    {
        float img[15];
        for (i = 0; i < 15; ++i) {
            img[i] = (float)i * 0.25f - 1.5f;
        }
        ISO_CHECK_EQ_INT(dsp_dct_2d(img, 15, 3, 5, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_2d(c, clen, 3, 5, DSP_DCT_III, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, img, 15, 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        ISO_CHECK_EQ_INT(dsp_dst_2d(img, 15, 3, 5, DSP_DST_II, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_2d(c, clen, 3, 5, DSP_DST_III, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, img, 15, 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);
    }
}

static void test_inplace_aliasing_and_parsers(void) {
    float orig[6] = {1.0f, -2.0f, 3.0f, -4.0f, 5.0f, -6.0f};
    float buf[6];
    float nan_buf[4] = {1.0f, 0.0f, 3.0f, 4.0f};
    dsp_dct_type_t dt;
    dsp_dst_type_t st;
    dsp_dct_norm_t nm;

    nan_buf[1] = make_nan_f();
    memcpy(buf, orig, sizeof(orig));

    /* In-place out == signal roundtrip */
    ISO_CHECK_EQ_INT(dsp_dct_1d_into(buf, 6, DSP_DCT_II, DSP_DCT_NORM_ORTHO, buf), DSP_DCT_OK);
    ISO_CHECK_EQ_INT(dsp_idct_1d_into(buf, 6, DSP_DCT_III, DSP_DCT_NORM_ORTHO, buf), DSP_DCT_OK);
    check_close_slice(buf, orig, 6, 1e-4);

    ISO_CHECK_EQ_INT(dsp_dst_1d_into(buf, 6, DSP_DST_II, DSP_DCT_NORM_ORTHO, buf), DSP_DCT_OK);
    ISO_CHECK_EQ_INT(dsp_idst_1d_into(buf, 6, DSP_DST_III, DSP_DCT_NORM_ORTHO, buf), DSP_DCT_OK);
    check_close_slice(buf, orig, 6, 1e-4);

    ISO_CHECK_EQ_INT(dsp_dct_2d_into(buf, 6, 2, 3, DSP_DCT_II, DSP_DCT_NORM_ORTHO, buf), DSP_DCT_OK);
    ISO_CHECK_EQ_INT(dsp_idct_2d_into(buf, 6, 2, 3, DSP_DCT_III, DSP_DCT_NORM_ORTHO, buf), DSP_DCT_OK);
    check_close_slice(buf, orig, 6, 1e-4);

    /* Aliased error must leave input buffer untouched */
    ISO_CHECK_EQ_INT(dsp_dct_1d_into(nan_buf, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, nan_buf), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_DBL(nan_buf[0], 1.0, 0.0);
    ISO_CHECK_EQ_DBL(nan_buf[2], 3.0, 0.0);

    /* Parsers */
    ISO_CHECK_EQ_INT(dsp_dct_parse_type("  ii ", &dt), DSP_DCT_OK);
    ISO_CHECK_EQ_INT((int)dt, (int)DSP_DCT_II);
    ISO_CHECK_EQ_INT(dsp_dct_parse_type("DCT-IV", &dt), DSP_DCT_OK);
    ISO_CHECK_EQ_INT((int)dt, (int)DSP_DCT_IV);
    ISO_CHECK_EQ_INT(dsp_dct_parse_type("dct-ii!", &dt), DSP_DCT_ERR_INVALID_INPUT);

    ISO_CHECK_EQ_INT(dsp_dst_parse_type("dst-iii", &st), DSP_DCT_OK);
    ISO_CHECK_EQ_INT((int)st, (int)DSP_DST_III);
    ISO_CHECK_EQ_INT(dsp_dst_parse_type("9", &st), DSP_DCT_ERR_INVALID_INPUT);

    ISO_CHECK_EQ_INT(dsp_dct_parse_norm(" ORTHO\n", &nm), DSP_DCT_OK);
    ISO_CHECK_EQ_INT((int)nm, (int)DSP_DCT_NORM_ORTHO);
    ISO_CHECK_EQ_INT(dsp_dct_parse_norm("none", &nm), DSP_DCT_OK);
    ISO_CHECK_EQ_INT((int)nm, (int)DSP_DCT_NORM_NONE);
    ISO_CHECK_EQ_INT(dsp_dct_parse_norm("bogus", &nm), DSP_DCT_ERR_INVALID_INPUT);
}

/* ------------------------------------------------------------------ */
/* Hostile Input Tests (CWE-20 / CWE-190 / CWE-400 / CWE-476)         */
/* ------------------------------------------------------------------ */

static void test_hostile_inputs(void) {
    float *out = NULL;
    size_t out_len = 0;
    float valid[4] = {1.0f, 2.0f, 3.0f, 4.0f};
    float nan_v = make_nan_f();
    float pinf = make_inf_f();
    float ninf = -pinf;

    /* NULL guards */
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, NULL, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, NULL), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d(NULL, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d_into(valid, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, NULL), DSP_DCT_ERR_INVALID_INPUT);

    /* Empty inputs (must precede enum domain checks per warrant) */
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, 0, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d(NULL, 0, (dsp_dct_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_idct_1d(valid, 0, DSP_DCT_III, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_idct_1d(NULL, 0, (dsp_dct_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dst_1d(valid, 0, DSP_DST_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dst_1d(NULL, 0, (dsp_dst_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_idst_1d(valid, 0, DSP_DST_III, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_idst_1d(NULL, 0, (dsp_dst_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d_into(NULL, 0, (dsp_dct_type_t)99, DSP_DCT_NORM_ORTHO, valid), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(valid, 0, 8, 8, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(NULL, 0, 4, 4, (dsp_dct_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_EMPTY_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(NULL, 0, 0, 4, (dsp_dct_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_idct_2d(NULL, 0, 4, 0, (dsp_dct_type_t)99, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);

    /* Enum domain validation */
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, 4, (dsp_dct_type_t)99, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, 4, DSP_DCT_II, (dsp_dct_norm_t)99, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dst_1d(valid, 4, (dsp_dst_type_t)0, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_idst_1d(valid, 4, (dsp_dst_type_t)5, DSP_DCT_NORM_NONE, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);

    /* DCT-I / DST-I require N >= 2 */
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, 1, DSP_DCT_I, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dst_1d(valid, 1, DSP_DST_I, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);

    /* Non-finite float rejection */
    {
        float bads[3];
        size_t bi;
        bads[0] = nan_v;
        bads[1] = pinf;
        bads[2] = ninf;
        for (bi = 0; bi < 3; ++bi) {
            float s[3];
            s[0] = 1.0f;
            s[1] = bads[bi];
            s[2] = 3.0f;
            ISO_CHECK_EQ_INT(dsp_dct_1d(s, 3, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
            ISO_CHECK_EQ_INT(dsp_dst_1d(s, 3, DSP_DST_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
            ISO_CHECK_EQ_INT(dsp_dct_2d(s, 3, 1, 3, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
        }
    }

    /* Near-FLT_MAX overflow rejection vs contractive N=1 Ortho survival */
    {
        float big1[1] = {2.0e38f};
        float max1[1] = {FLT_MAX};
        float max4[4] = {FLT_MAX, FLT_MAX, FLT_MAX, FLT_MAX};
        ISO_CHECK_EQ_INT(dsp_dct_1d(big1, 1, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_OK);
        ISO_CHECK_EQ_DBL(out[0], 2.0e38f, 1e32);
        dsp_dct_free(out);

        ISO_CHECK_EQ_INT(dsp_dct_1d(max1, 1, DSP_DCT_II, DSP_DCT_NORM_NONE, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
        ISO_CHECK(out == NULL);
        ISO_CHECK_EQ_INT(dsp_dct_1d(max4, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
        ISO_CHECK(out == NULL);
        ISO_CHECK_EQ_INT(dsp_dct_2d(max4, 4, 2, 2, DSP_DCT_II, DSP_DCT_NORM_NONE, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    }

    /* Subnormal stability */
    {
        float sub = FLT_MIN / 4.0f;
        float s[4];
        s[0] = sub;
        s[1] = -sub;
        s[2] = sub * 2.0f;
        s[3] = 0.0f;
        ISO_CHECK_EQ_INT(dsp_dct_1d(s, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_OK);
        ISO_CHECK(all_finite_f(out, out_len));
        dsp_dct_free(out);
    }

    /* 2-D dimension and CWE-190 / CWE-400 bounds */
    ISO_CHECK_EQ_INT(dsp_dct_2d(valid, 4, 0, 4, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(valid, 4, 4, 0, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(valid, 4, 2, 3, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(valid, 4, (uint32_t)(DSP_DCT_MAX_LEN + 1), 2, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_2d(valid, 4, UINT32_MAX, UINT32_MAX, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
    ISO_CHECK_EQ_INT(dsp_dct_1d(valid, DSP_DCT_MAX_LEN + 1, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &out, &out_len), DSP_DCT_ERR_INVALID_INPUT);
}

/* ------------------------------------------------------------------ */
/* Deterministic SplitMix64 PBT                                       */
/* ------------------------------------------------------------------ */

static void test_pbt_1d_and_2d(void) {
    splitmix64_t rng;
    static const size_t sizes[10] = {1, 2, 3, 4, 5, 7, 8, 15, 23, 32};
    static const uint32_t dims[6][2] = {{1, 1}, {2, 2}, {3, 4}, {5, 3}, {8, 8}, {7, 6}};
    size_t si;
    size_t di;

    rng.state = 0xA1B2C3D4E5F60718ULL;
    for (si = 0; si < 10; ++si) {
        size_t n = sizes[si];
        float *sig = (float *)malloc(n * sizeof(float));
        float *c = NULL;
        float *r = NULL;
        size_t clen = 0;
        size_t rlen = 0;
        double e_time = 0.0;
        double e_freq = 0.0;
        size_t i;

        ISO_CHECK(sig != NULL);
        for (i = 0; i < n; ++i) {
            sig[i] = sample_f32(&rng);
            e_time += (double)sig[i] * (double)sig[i];
        }

        /* DCT-II / III Ortho roundtrip + Parseval energy conservation */
        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, n, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, DSP_DCT_III, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, n, 1e-4);
        for (i = 0; i < n; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        /* DCT-III / II Ortho roundtrip + Parseval energy conservation */
        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, n, DSP_DCT_III, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, n, 1e-4);
        e_freq = 0.0;
        for (i = 0; i < n; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        /* DCT-IV Ortho self-inverse + energy conservation */
        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, n, DSP_DCT_IV, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, DSP_DCT_IV, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, n, 1e-4);
        e_freq = 0.0;
        for (i = 0; i < n; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        /* DST-II / III Ortho roundtrip + Parseval */
        ISO_CHECK_EQ_INT(dsp_dst_1d(sig, n, DSP_DST_II, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_III, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, n, 1e-4);
        e_freq = 0.0;
        for (i = 0; i < n; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        /* DST-III / II Ortho roundtrip + Parseval */
        ISO_CHECK_EQ_INT(dsp_dst_1d(sig, n, DSP_DST_III, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_II, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, n, 1e-4);
        e_freq = 0.0;
        for (i = 0; i < n; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        /* DST-IV Ortho self-inverse + Parseval */
        ISO_CHECK_EQ_INT(dsp_dst_1d(sig, n, DSP_DST_IV, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_IV, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, n, 1e-4);
        e_freq = 0.0;
        for (i = 0; i < n; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);

        if (n >= 2) {
            /* DCT-I & DST-I Ortho roundtrip + Parseval */
            ISO_CHECK_EQ_INT(dsp_dct_1d(sig, n, DSP_DCT_I, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
            ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, DSP_DCT_I, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
            check_close_slice(r, sig, n, 1e-4);
            e_freq = 0.0;
            for (i = 0; i < n; ++i) {
                e_freq += (double)c[i] * (double)c[i];
            }
            ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
            dsp_dct_free(c);
            dsp_dct_free(r);

            ISO_CHECK_EQ_INT(dsp_dst_1d(sig, n, DSP_DST_I, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
            ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_I, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
            check_close_slice(r, sig, n, 1e-4);
            e_freq = 0.0;
            for (i = 0; i < n; ++i) {
                e_freq += (double)c[i] * (double)c[i];
            }
            ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
            dsp_dct_free(c);
            dsp_dct_free(r);
        }

        free(sig);
    }

    rng.state = 0xCA11AB1ED00DFEEDULL;
    for (di = 0; di < 6; ++di) {
        uint32_t h = dims[di][0];
        uint32_t w = dims[di][1];
        size_t total = (size_t)h * (size_t)w;
        float *img = (float *)malloc(total * sizeof(float));
        float *c = NULL;
        float *r = NULL;
        size_t clen = 0;
        size_t rlen = 0;
        double e_time = 0.0;
        double e_freq = 0.0;
        size_t i;

        ISO_CHECK(img != NULL);
        for (i = 0; i < total; ++i) {
            img[i] = sample_f32(&rng);
            e_time += (double)img[i] * (double)img[i];
        }

        ISO_CHECK_EQ_INT(dsp_dct_2d(img, total, h, w, DSP_DCT_II, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_2d(c, clen, h, w, DSP_DCT_III, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, img, total, 1e-4);
        for (i = 0; i < total; ++i) {
            e_freq += (double)c[i] * (double)c[i];
        }
        ISO_CHECK_EQ_DBL(e_time, e_freq, (e_time > 1.0 ? e_time : 1.0) * 1e-4);
        dsp_dct_free(c);
        dsp_dct_free(r);
        free(img);
    }

    /* Large-N O(N log N) verification for all 8 transforms (N = 16384) */
    {
        size_t big_n = 16384;
        float *sig = (float *)malloc(big_n * sizeof(float));
        float *c = NULL;
        float *r = NULL;
        size_t clen = 0;
        size_t rlen = 0;
        size_t i;
        ISO_CHECK(sig != NULL);
        for (i = 0; i < big_n; ++i) {
            sig[i] = sample_f32(&rng);
        }
        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, big_n, DSP_DCT_I, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, DSP_DCT_I, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, 64, 1e-3);
        dsp_dct_free(c);
        dsp_dct_free(r);

        ISO_CHECK_EQ_INT(dsp_dct_1d(sig, big_n, DSP_DCT_IV, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idct_1d(c, clen, DSP_DCT_IV, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, 64, 1e-3);
        dsp_dct_free(c);
        dsp_dct_free(r);

        ISO_CHECK_EQ_INT(dsp_dst_1d(sig, big_n, DSP_DST_I, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_I, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, 64, 1e-3);
        dsp_dct_free(c);
        dsp_dct_free(r);

        ISO_CHECK_EQ_INT(dsp_dst_1d(sig, big_n, DSP_DST_II, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_III, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, 64, 1e-3);
        dsp_dct_free(c);
        dsp_dct_free(r);

        ISO_CHECK_EQ_INT(dsp_dst_1d(sig, big_n, DSP_DST_IV, DSP_DCT_NORM_ORTHO, &c, &clen), DSP_DCT_OK);
        ISO_CHECK_EQ_INT(dsp_idst_1d(c, clen, DSP_DST_IV, DSP_DCT_NORM_ORTHO, &r, &rlen), DSP_DCT_OK);
        check_close_slice(r, sig, 64, 1e-3);
        dsp_dct_free(c);
        dsp_dct_free(r);

        free(sig);
    }
}

int main(void) {
    test_dct_closed_form();
    test_dct_ii_iii_matches_naive_oracle();
    test_all_dct_and_dst_roundtrips();
    test_2d_transforms();
    test_inplace_aliasing_and_parsers();
    test_hostile_inputs();
    test_pbt_1d_and_2d();
    return ISO_TEST_RESULT();
}
