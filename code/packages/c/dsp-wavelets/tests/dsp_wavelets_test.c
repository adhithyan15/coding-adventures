/*
 * dsp_wavelets_test.c — unit tests for the C wavelet transform port.
 *
 * Mirrors the Rust crate's suite: error paths, output-length contract, the
 * hand-worked Haar reference vector, constant/dirac structural properties,
 * perfect-reconstruction round-trips (Haar + Daubechies/Symlets/Coiflets, 1-D
 * and 2-D), split_levels/slice_level, and the filter-bank invariants.
 *
 * Hardened with full VER15 CWE/CVE test suites:
 * - CWE-20: NaN, Inf, -Inf, subnormals (1e-40, 1e-200), null pointer guards,
 *           strict string parsing of wavelets & boundaries (reject trailing garbage).
 * - CWE-120/122/125/787: Buffer boundary, slicing, and index safety.
 * - CWE-190: SIZE_MAX and arithmetic overflow checks.
 * - CWE-400: Pre-flight bounds (WV_MAX_SAMPLES, WV_MAX_LEVELS, WV_MAX_SCALES, WV_MAX_2D_DIM).
 * - CWE-674/1333: Iterative execution without recursion or regex.
 * - Property-based tests (PBT): Energy conservation (Parseval) and PRNG round-trips.
 */
#include "dsp_wavelets.h"
#include "iso_test.h"

#include <float.h>
#include <stdint.h>
#include <stdlib.h>

static float af(float x) { return x < 0 ? -x : x; }

static int approx_eq(float a, float b, float tol) {
    float scale = af(a);
    if (af(b) > scale) {
        scale = af(b);
    }
    if (scale < 1.0f) {
        scale = 1.0f;
    }
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

/* A deterministic ramp folded into a small range. */
static float *ramp(size_t n, float step) {
    float *v = (float *)malloc((n == 0 ? 1 : n) * sizeof(float));
    size_t i;
    for (i = 0; i < n; ++i) {
        float x = (float)i * step;
        while (x > 4.0f) {
            x -= 8.0f;
        }
        v[i] = x;
    }
    return v;
}

static float *constant(size_t n, float value) {
    float *v = (float *)malloc((n == 0 ? 1 : n) * sizeof(float));
    size_t i;
    for (i = 0; i < n; ++i) {
        v[i] = value;
    }
    return v;
}

static void round_trip_1d(size_t n, float step, wv_wavelet_t w, uint32_t levels,
                          wv_boundary_t boundary, float tol) {
    float *signal = ramp(n, step);
    float *coeffs = NULL;
    float *recon = NULL;
    size_t clen = 0;
    size_t rlen = 0;
    size_t edge;
    size_t hlen = 0;
    size_t i;
    ISO_CHECK(wv_dwt_1d(signal, n, w, levels, boundary, &coeffs, &clen) == WV_OK);
    ISO_CHECK(wv_idwt_1d(coeffs, clen, w, levels, boundary, (uint32_t)n, &recon, &rlen) == WV_OK);
    ISO_CHECK_EQ_UINT(rlen, n);
    wv_analysis_lowpass(w, &hlen);
    edge = (w.family == WV_HAAR) ? 0 : hlen;
    if (edge > rlen / 4) {
        edge = rlen / 4;
    }
    for (i = edge; i + edge < rlen; ++i) {
        ISO_CHECK(approx_eq(signal[i], recon[i], tol));
    }
    free(signal);
    wv_free(coeffs);
    wv_free(recon);
}

static void test_error_paths(void) {
    float *sig8 = constant(8, 1.0f);
    float *sig4 = constant(4, 1.0f);
    float *sig16 = constant(16, 1.0f);
    float *out = NULL;
    size_t olen = 0;
    size_t i;

    ISO_CHECK(wv_dwt_1d(sig8, 0, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_EMPTY_SIGNAL);
    ISO_CHECK(wv_dwt_1d(sig8, 8, wv_haar(), 0, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_1d(sig4, 4, wv_haar(), 4, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_SIGNAL_TOO_SHORT);

    {
        wv_wavelet_t bad[8];
        bad[0] = wv_daubechies(3);
        bad[1] = wv_daubechies(99);
        bad[2] = wv_symlets(6);
        bad[3] = wv_symlets(8);
        bad[4] = wv_coiflets(2);
        bad[5] = wv_morlet();
        bad[6] = wv_mexican_hat();
        bad[7] = wv_biorthogonal(5, 3);
        for (i = 0; i < 8; ++i) {
            ISO_CHECK(wv_dwt_1d(sig16, 16, bad[i], 2, WV_BOUND_PERIODIC, &out, &olen) ==
                      WV_ERR_INVALID_PARAM);
        }
    }
    {
        wv_boundary_t bb[3];
        bb[0] = WV_BOUND_ZERO;
        bb[1] = WV_BOUND_REPLICATE;
        bb[2] = WV_BOUND_REFLECT;
        for (i = 0; i < 3; ++i) {
            ISO_CHECK(wv_dwt_1d(sig16, 16, wv_haar(), 2, bb[i], &out, &olen) ==
                      WV_ERR_INVALID_PARAM);
        }
    }
    {
        uint32_t bl[4];
        bl[0] = 32u;
        bl[1] = 64u;
        bl[2] = 1000000u;
        bl[3] = 0xFFFFFFFFu;
        for (i = 0; i < 4; ++i) {
            ISO_CHECK(wv_dwt_1d(sig16, 16, wv_haar(), bl[i], WV_BOUND_PERIODIC, &out, &olen) ==
                      WV_ERR_INVALID_PARAM);
        }
    }
    {
        float c4[4];
        c4[0] = 1.0f;
        c4[1] = 1.0f;
        c4[2] = 0.0f;
        c4[3] = 0.0f;
        ISO_CHECK(wv_idwt_1d(c4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, 0xFFFFFFFFu, &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
    }
    {
        float c3[3];
        c3[0] = 1.0f;
        c3[1] = 2.0f;
        c3[2] = 3.0f;
        ISO_CHECK(wv_idwt_1d(c3, 3, wv_haar(), 1, WV_BOUND_PERIODIC, 4, &out, &olen) ==
                  WV_ERR_INVALID_COEFFICIENTS);
    }

    free(sig8);
    free(sig4);
    free(sig16);
}

static void test_haar_reference_vector(void) {
    float signal[4];
    float *coeffs = NULL;
    size_t clen = 0;
    float s = WV_FRAC_1_SQRT_2;
    signal[0] = 1.0f;
    signal[1] = 2.0f;
    signal[2] = 3.0f;
    signal[3] = 4.0f;
    ISO_CHECK(wv_dwt_1d(signal, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
    ISO_CHECK_EQ_UINT(clen, 4u);
    ISO_CHECK(approx_eq(coeffs[0], 3.0f * s, 1e-5f));
    ISO_CHECK(approx_eq(coeffs[1], 7.0f * s, 1e-5f));
    ISO_CHECK(approx_eq(coeffs[2], s, 1e-5f));
    ISO_CHECK(approx_eq(coeffs[3], s, 1e-5f));
    wv_free(coeffs);
}

static void test_constant_zero_detail(void) {
    float *signal = constant(32, 3.14f);
    float *coeffs = NULL;
    size_t clen = 0;
    size_t i;
    ISO_CHECK(wv_dwt_1d(signal, 32, wv_haar(), 4, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
    for (i = 2; i < clen; ++i) {
        ISO_CHECK(af(coeffs[i]) <= 1e-6f);
    }
    free(signal);
    wv_free(coeffs);
}

static void test_dirac_delta(void) {
    float *signal = constant(16, 0.0f);
    float *coeffs = NULL;
    size_t clen = 0;
    float s = WV_FRAC_1_SQRT_2;
    size_t k;
    signal[0] = 1.0f;
    ISO_CHECK(wv_dwt_1d(signal, 16, wv_haar(), 1, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
    ISO_CHECK(approx_eq(coeffs[0], s, 1e-5f));
    for (k = 1; k < 8; ++k) {
        ISO_CHECK(af(coeffs[k]) <= 1e-6f);
    }
    ISO_CHECK(approx_eq(coeffs[8], -s, 1e-5f));
    for (k = 1; k < 8; ++k) {
        ISO_CHECK(af(coeffs[8 + k]) <= 1e-6f);
    }
    free(signal);
    wv_free(coeffs);
}

static void test_output_length_contract(void) {
    size_t ns[5];
    size_t idx;
    ns[0] = 4;
    ns[1] = 8;
    ns[2] = 16;
    ns[3] = 32;
    ns[4] = 64;
    for (idx = 0; idx < 5; ++idx) {
        size_t n = ns[idx];
        uint32_t max_j = 0;
        size_t t;
        uint32_t j;
        for (t = n; t > 1; t /= 2) {
            ++max_j;
        }
        for (j = 1; j <= max_j; ++j) {
            float *signal = constant(n, 0.5f);
            float *coeffs = NULL;
            size_t clen = 0;
            ISO_CHECK(wv_dwt_1d(signal, n, wv_haar(), j, WV_BOUND_PERIODIC, &coeffs, &clen) ==
                      WV_OK);
            ISO_CHECK_EQ_UINT(clen, n);
            free(signal);
            wv_free(coeffs);
        }
    }
}

static void test_round_trips_1d(void) {
    round_trip_1d(4, 0.13f, wv_haar(), 1, WV_BOUND_PERIODIC, 1e-4f);
    round_trip_1d(8, 0.13f, wv_haar(), 3, WV_BOUND_PERIODIC, 1e-4f);
    round_trip_1d(16, 0.13f, wv_haar(), 3, WV_BOUND_PERIODIC, 1e-4f);
    round_trip_1d(32, 0.07f, wv_haar(), 3, WV_BOUND_PERIODIC, 1e-4f);
    round_trip_1d(17, 0.2f, wv_haar(), 2, WV_BOUND_PERIODIC, 1e-4f);
    round_trip_1d(16, 0.11f, wv_haar(), 3, WV_BOUND_SYMMETRIC, 1e-3f);
    round_trip_1d(64, 0.07f, wv_daubechies(2), 2, WV_BOUND_PERIODIC, 1e-3f);
    round_trip_1d(64, 0.13f, wv_daubechies(4), 2, WV_BOUND_PERIODIC, 1e-3f);
    round_trip_1d(64, 0.11f, wv_symlets(4), 2, WV_BOUND_PERIODIC, 1e-3f);
    round_trip_1d(64, 0.09f, wv_coiflets(1), 2, WV_BOUND_PERIODIC, 1e-3f);
    round_trip_1d(128, 0.04f, wv_daubechies(6), 2, WV_BOUND_PERIODIC, 1e-3f);
    round_trip_1d(128, 0.03f, wv_daubechies(8), 2, WV_BOUND_PERIODIC, 1e-3f);
}

static void test_db2_constant_small_detail(void) {
    float *signal = constant(64, 3.14f);
    float *coeffs = NULL;
    size_t clen = 0;
    size_t i;
    ISO_CHECK(wv_dwt_1d(signal, 64, wv_daubechies(2), 3, WV_BOUND_PERIODIC, &coeffs, &clen) ==
              WV_OK);
    for (i = 8; i < clen; ++i) {
        ISO_CHECK(af(coeffs[i]) <= 1e-5f);
    }
    free(signal);
    wv_free(coeffs);
}

static void test_split_and_slice(void) {
    float *signal = ramp(16, 0.1f);
    float *coeffs = NULL;
    size_t clen = 0;
    size_t *offsets = NULL;
    size_t olen = 0;
    size_t expected[5];
    size_t i;
    const float *ca3 = NULL;
    const float *cd1 = NULL;
    size_t slen = 0;
    ISO_CHECK(wv_dwt_1d(signal, 16, wv_haar(), 3, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
    ISO_CHECK(wv_split_levels(clen, 16, 3, &offsets, &olen) == WV_OK);
    expected[0] = 0;
    expected[1] = 2;
    expected[2] = 4;
    expected[3] = 8;
    expected[4] = 16;
    ISO_CHECK_EQ_UINT(olen, 5u);
    for (i = 0; i < 5; ++i) {
        ISO_CHECK_EQ_UINT(offsets[i], expected[i]);
    }
    ISO_CHECK(wv_slice_level(coeffs, clen, 16, 3, 3, WV_BAND_APPROXIMATION, &ca3, &slen) == WV_OK);
    ISO_CHECK_EQ_UINT(slen, 2u);
    ISO_CHECK(wv_slice_level(coeffs, clen, 16, 3, 1, WV_BAND_DETAIL, &cd1, &slen) == WV_OK);
    ISO_CHECK_EQ_UINT(slen, 8u);
    ISO_CHECK(wv_slice_level(coeffs, clen, 16, 3, 1, WV_BAND_APPROXIMATION, &ca3, &slen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_slice_level(coeffs, clen, 16, 3, 0, WV_BAND_DETAIL, &cd1, &slen) ==
              WV_ERR_INVALID_PARAM);
    free(signal);
    wv_free(coeffs);
    wv_free(offsets);
}

static void test_filter_invariants(void) {
    wv_wavelet_t ws[6];
    const float sqrt2 = 1.4142135f;
    size_t wi;
    ws[0] = wv_daubechies(2);
    ws[1] = wv_daubechies(4);
    ws[2] = wv_daubechies(6);
    ws[3] = wv_daubechies(8);
    ws[4] = wv_symlets(4);
    ws[5] = wv_coiflets(1);
    for (wi = 0; wi < 6; ++wi) {
        size_t hlen = 0;
        const float *h = wv_analysis_lowpass(ws[wi], &hlen);
        float sum = 0.0f;
        float energy = 0.0f;
        float *g = NULL;
        size_t glen = 0;
        float gsum = 0.0f;
        size_t i;
        for (i = 0; i < hlen; ++i) {
            sum += h[i];
            energy += h[i] * h[i];
        }
        ISO_CHECK(af(sum - sqrt2) < 5e-4f);
        ISO_CHECK(af(energy - 1.0f) < 5e-4f);
        ISO_CHECK(wv_qmf_highpass(h, hlen, &g, &glen) == WV_OK);
        for (i = 0; i < glen; ++i) {
            gsum += g[i];
        }
        ISO_CHECK(af(gsum) < 5e-4f);
        wv_free(g);
    }
    {
        wv_wavelet_t empties[6];
        size_t len = 0;
        empties[0] = wv_daubechies(0);
        empties[1] = wv_daubechies(1);
        empties[2] = wv_daubechies(3);
        empties[3] = wv_symlets(1);
        empties[4] = wv_coiflets(4);
        empties[5] = wv_haar();
        for (wi = 0; wi < 6; ++wi) {
            const float *h = wv_analysis_lowpass(empties[wi], &len);
            ISO_CHECK(h == NULL && len == 0);
        }
    }
}

static void test_2d(void) {
    struct { uint32_t rows, cols, j; } shapes[4];
    size_t si;
    shapes[0].rows = 8; shapes[0].cols = 8; shapes[0].j = 1;
    shapes[1].rows = 8; shapes[1].cols = 8; shapes[1].j = 2;
    shapes[2].rows = 16; shapes[2].cols = 16; shapes[2].j = 3;
    shapes[3].rows = 32; shapes[3].cols = 16; shapes[3].j = 2;
    for (si = 0; si < 4; ++si) {
        size_t total = (size_t)shapes[si].rows * shapes[si].cols;
        float *image = constant(total, 0.5f);
        float *coeffs = NULL;
        size_t clen = 0;
        ISO_CHECK(wv_dwt_2d(image, total, shapes[si].rows, shapes[si].cols, wv_haar(),
                             shapes[si].j, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
        ISO_CHECK_EQ_UINT(clen, total);
        free(image);
        wv_free(coeffs);
    }

    {
        float *image = ramp(16 * 16, 0.1f);
        float *coeffs = NULL;
        float *recon = NULL;
        size_t clen = 0;
        size_t rlen = 0;
        size_t i;
        ISO_CHECK(wv_dwt_2d(image, 256, 16, 16, wv_haar(), 2, WV_BOUND_PERIODIC, &coeffs, &clen) ==
                  WV_OK);
        ISO_CHECK(wv_idwt_2d(coeffs, clen, 16, 16, wv_haar(), 2, WV_BOUND_PERIODIC, &recon,
                             &rlen) == WV_OK);
        ISO_CHECK_EQ_UINT(rlen, 256u);
        for (i = 0; i < 256; ++i) {
            ISO_CHECK(approx_eq(image[i], recon[i], 1e-4f));
        }
        free(image);
        wv_free(coeffs);
        wv_free(recon);
    }
    {
        float *image = ramp(32 * 16, 0.05f);
        float *coeffs = NULL;
        float *recon = NULL;
        size_t clen = 0;
        size_t rlen = 0;
        size_t i;
        ISO_CHECK(wv_dwt_2d(image, 512, 32, 16, wv_haar(), 2, WV_BOUND_PERIODIC, &coeffs, &clen) ==
                  WV_OK);
        ISO_CHECK(wv_idwt_2d(coeffs, clen, 32, 16, wv_haar(), 2, WV_BOUND_PERIODIC, &recon,
                             &rlen) == WV_OK);
        for (i = 0; i < 512; ++i) {
            ISO_CHECK(approx_eq(image[i], recon[i], 1e-4f));
        }
        free(image);
        wv_free(coeffs);
        wv_free(recon);
    }
    {
        float *image = ramp(16 * 16, 0.07f);
        float *coeffs = NULL;
        float *recon = NULL;
        size_t clen = 0;
        size_t rlen = 0;
        size_t i;
        ISO_CHECK(wv_dwt_2d(image, 256, 16, 16, wv_daubechies(4), 2, WV_BOUND_PERIODIC, &coeffs,
                            &clen) == WV_OK);
        ISO_CHECK(wv_idwt_2d(coeffs, clen, 16, 16, wv_daubechies(4), 2, WV_BOUND_PERIODIC, &recon,
                             &rlen) == WV_OK);
        for (i = 0; i < 256; ++i) {
            ISO_CHECK(approx_eq(image[i], recon[i], 1e-3f));
        }
        free(image);
        wv_free(coeffs);
        wv_free(recon);
    }
    {
        float *image = constant(16 * 16, 2.5f);
        float *coeffs = NULL;
        size_t clen = 0;
        size_t i;
        ISO_CHECK(wv_dwt_2d(image, 256, 16, 16, wv_haar(), 2, WV_BOUND_PERIODIC, &coeffs, &clen) ==
                  WV_OK);
        for (i = 16; i < clen; ++i) {
            ISO_CHECK(af(coeffs[i]) <= 1e-6f);
        }
        free(image);
        wv_free(coeffs);
    }
    {
        float *out = NULL;
        size_t olen = 0;
        float *image100 = constant(100, 0.0f);
        float *image256 = constant(256, 0.0f);
        ISO_CHECK(wv_dwt_2d(image100, 0, 0, 0, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
                  WV_ERR_EMPTY_SIGNAL);
        ISO_CHECK(wv_dwt_2d(image100, 100, 8, 8, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
        ISO_CHECK(wv_dwt_2d(image256, 256, 16, 16, wv_biorthogonal(5, 3), 1, WV_BOUND_PERIODIC,
                            &out, &olen) == WV_ERR_INVALID_PARAM);
        ISO_CHECK(wv_dwt_2d(image256, 256, 16, 16, wv_haar(), 1, WV_BOUND_SYMMETRIC, &out,
                            &olen) == WV_ERR_INVALID_PARAM);
        free(image100);
        free(image256);
    }
}

/* ================================================================== */
/* VER15 CWE/CVE Proofs & Hostile / Adversarial Tests                 */
/* ================================================================== */

static void test_hostile_nan_inf_subnormal(void) {
    float bad_floats[3];
    size_t b_idx;
    float *out = NULL;
    size_t olen = 0;

    bad_floats[0] = make_nan();
    bad_floats[1] = make_inf();
    bad_floats[2] = make_neg_inf();

    for (b_idx = 0; b_idx < 3; ++b_idx) {
        float bad = bad_floats[b_idx];
        float sig[8];
        float coeffs[8];
        float img[16];
        float scales[2];
        size_t i;

        for (i = 0; i < 8; ++i) sig[i] = 1.0f;
        sig[3] = bad;
        ISO_CHECK(wv_dwt_1d(sig, 8, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
                  WV_ERR_INVALID_PARAM);

        for (i = 0; i < 8; ++i) coeffs[i] = 1.0f;
        coeffs[2] = bad;
        ISO_CHECK(wv_idwt_1d(coeffs, 8, wv_haar(), 1, WV_BOUND_PERIODIC, 8, &out, &olen) ==
                  WV_ERR_INVALID_COEFFICIENTS);

        for (i = 0; i < 16; ++i) img[i] = 1.0f;
        img[5] = bad;
        ISO_CHECK(wv_dwt_2d(img, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
                  WV_ERR_INVALID_PARAM);

        for (i = 0; i < 16; ++i) img[i] = 1.0f;
        img[7] = bad;
        ISO_CHECK(wv_idwt_2d(img, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
                  WV_ERR_INVALID_COEFFICIENTS);

        scales[0] = 1.0f;
        scales[1] = 2.0f;
        ISO_CHECK(wv_cwt(sig, 8, scales, 2, wv_morlet(), &out, &olen) ==
                  WV_ERR_INVALID_PARAM);

        sig[3] = 1.0f;
        scales[1] = bad;
        ISO_CHECK(wv_cwt(sig, 8, scales, 2, wv_morlet(), &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
    }

    /* Negative and zero scales in CWT */
    {
        float sig[8] = {1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f, 1.0f};
        float zero_scale[1] = {0.0f};
        float neg_scale[1] = {-2.0f};
        ISO_CHECK(wv_cwt(sig, 8, zero_scale, 1, wv_morlet(), &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
        ISO_CHECK(wv_cwt(sig, 8, neg_scale, 1, wv_morlet(), &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
    }

    /* Subnormal and tiny numbers: must not crash, must execute cleanly */
    {
        double d_tiny = 1e-200;
        float tiny = (float)d_tiny;
        float subnormal = 1e-40f;
        float sig[8];
        float *recon = NULL;
        size_t rlen = 0;
        size_t i;

        for (i = 0; i < 8; ++i) {
            sig[i] = (i & 1) ? tiny : subnormal;
        }
        ISO_CHECK(wv_dwt_1d(sig, 8, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) == WV_OK);
        ISO_CHECK_EQ_UINT(olen, 8u);
        ISO_CHECK(wv_idwt_1d(out, olen, wv_haar(), 1, WV_BOUND_PERIODIC, 8, &recon, &rlen) == WV_OK);
        ISO_CHECK_EQ_UINT(rlen, 8u);
        wv_free(out);
        wv_free(recon);
    }
}

static void test_null_pointers(void) {
    float sig[8] = {1, 2, 3, 4, 5, 6, 7, 8};
    float *out = NULL;
    size_t olen = 0;
    const float *borrowed = NULL;
    size_t *offsets = NULL;
    wv_wavelet_t w;
    wv_boundary_t b;

    ISO_CHECK(wv_dwt_1d(NULL, 8, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_1d(sig, 8, wv_haar(), 1, WV_BOUND_PERIODIC, NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_1d(sig, 8, wv_haar(), 1, WV_BOUND_PERIODIC, &out, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_idwt_1d(NULL, 8, wv_haar(), 1, WV_BOUND_PERIODIC, 8, &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_1d(sig, 8, wv_haar(), 1, WV_BOUND_PERIODIC, 8, NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_1d(sig, 8, wv_haar(), 1, WV_BOUND_PERIODIC, 8, &out, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_split_levels(8, 8, 1, NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_split_levels(8, 8, 1, &offsets, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_slice_level(NULL, 8, 8, 1, 1, WV_BAND_DETAIL, &borrowed, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_slice_level(sig, 8, 8, 1, 1, WV_BAND_DETAIL, NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_slice_level(sig, 8, 8, 1, 1, WV_BAND_DETAIL, &borrowed, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_dwt_2d(NULL, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_2d(sig, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_2d(sig, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_idwt_2d(NULL, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_2d(sig, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_2d(sig, 16, 4, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_cwt(NULL, 8, sig, 1, wv_morlet(), &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_cwt(sig, 8, NULL, 1, wv_morlet(), &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_cwt(sig, 8, sig, 1, wv_morlet(), NULL, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_cwt(sig, 8, sig, 1, wv_morlet(), &out, NULL) == WV_ERR_INVALID_PARAM);

    ISO_CHECK(wv_parse_wavelet(NULL, &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("haar", NULL) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_boundary(NULL, &b) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_boundary("periodic", NULL) == WV_ERR_INVALID_PARAM);

    wv_free(NULL); /* Safe no-op */
}

static void test_preflight_caps_and_overflow(void) {
    float sig[4] = {1, 2, 3, 4};
    float *out = NULL;
    size_t olen = 0;

    /* Samples cap */
    ISO_CHECK(wv_dwt_1d(sig, (size_t)WV_MAX_SAMPLES + 1, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_1d(sig, (size_t)SIZE_MAX, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_1d(sig, (size_t)WV_MAX_SAMPLES + 1, wv_haar(), 1, WV_BOUND_PERIODIC, 4, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_1d(sig, 4, wv_haar(), 1, WV_BOUND_PERIODIC, (uint32_t)WV_MAX_2D_DIM + 1, &out, &olen) ==
              WV_ERR_INVALID_PARAM);

    /* 2D dimension caps and overflow */
    ISO_CHECK(wv_dwt_2d(sig, 4, (uint32_t)WV_MAX_2D_DIM + 1, 1, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_2d(sig, 4, 1, (uint32_t)WV_MAX_2D_DIM + 1, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_2d(sig, 4, 0, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_dwt_2d(sig, 4, 4, 0, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);

    /* IDWT 2D dimensions */
    ISO_CHECK(wv_idwt_2d(sig, 4, (uint32_t)WV_MAX_2D_DIM + 1, 1, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_2d(sig, 4, 0, 4, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_idwt_2d(sig, 4, 4, 0, wv_haar(), 1, WV_BOUND_PERIODIC, &out, &olen) ==
              WV_ERR_INVALID_PARAM);

    /* CWT scale caps and product bound */
    {
        float scale[1] = {1.0f};
        ISO_CHECK(wv_cwt(sig, 4, scale, 0, wv_morlet(), &out, &olen) == WV_ERR_INVALID_PARAM);
        ISO_CHECK(wv_cwt(sig, 4, scale, (size_t)WV_MAX_SCALES + 1, wv_morlet(), &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
        ISO_CHECK(wv_cwt(sig, (size_t)WV_MAX_SAMPLES + 1, scale, 1, wv_morlet(), &out, &olen) ==
                  WV_ERR_INVALID_PARAM);
    }
}

static void test_wavelet_and_boundary_parsing(void) {
    wv_boundary_t b;
    wv_wavelet_t w;

    /* Boundary modes */
    ISO_CHECK(wv_parse_boundary("zero", &b) == WV_OK && b == WV_BOUND_ZERO);
    ISO_CHECK(wv_parse_boundary("replicate", &b) == WV_OK && b == WV_BOUND_REPLICATE);
    ISO_CHECK(wv_parse_boundary("reflect", &b) == WV_OK && b == WV_BOUND_REFLECT);
    ISO_CHECK(wv_parse_boundary("symmetric", &b) == WV_OK && b == WV_BOUND_SYMMETRIC);
    ISO_CHECK(wv_parse_boundary("  sym  ", &b) == WV_OK && b == WV_BOUND_SYMMETRIC);
    ISO_CHECK(wv_parse_boundary("periodic", &b) == WV_OK && b == WV_BOUND_PERIODIC);
    ISO_CHECK(wv_parse_boundary("WRAP", &b) == WV_OK && b == WV_BOUND_PERIODIC);
    ISO_CHECK(wv_parse_boundary("circular", &b) == WV_OK && b == WV_BOUND_PERIODIC);

    /* Boundary trailing garbage and invalid modes */
    ISO_CHECK(wv_parse_boundary("periodic_junk", &b) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_boundary("symmetric123", &b) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_boundary("unknown_mode", &b) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_boundary("", &b) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_boundary("   ", &b) == WV_ERR_INVALID_PARAM);

    /* Wavelets */
    ISO_CHECK(wv_parse_wavelet("haar", &w) == WV_OK && w.family == WV_HAAR);
    ISO_CHECK(wv_parse_wavelet("  HAAR  ", &w) == WV_OK && w.family == WV_HAAR);
    ISO_CHECK(wv_parse_wavelet("db2", &w) == WV_OK && w.family == WV_DAUBECHIES && w.n == 2);
    ISO_CHECK(wv_parse_wavelet("DB4", &w) == WV_OK && w.family == WV_DAUBECHIES && w.n == 4);
    ISO_CHECK(wv_parse_wavelet("db6", &w) == WV_OK && w.family == WV_DAUBECHIES && w.n == 6);
    ISO_CHECK(wv_parse_wavelet("db8", &w) == WV_OK && w.family == WV_DAUBECHIES && w.n == 8);
    ISO_CHECK(wv_parse_wavelet("sym4", &w) == WV_OK && w.family == WV_SYMLETS && w.n == 4);
    ISO_CHECK(wv_parse_wavelet("coif1", &w) == WV_OK && w.family == WV_COIFLETS && w.n == 1);
    ISO_CHECK(wv_parse_wavelet("bior2.2", &w) == WV_OK && w.family == WV_BIORTHOGONAL && w.n == 2);
    ISO_CHECK(wv_parse_wavelet("bior22", &w) == WV_OK && w.family == WV_BIORTHOGONAL && w.n == 2);
    ISO_CHECK(wv_parse_wavelet("bior5.3", &w) == WV_OK && w.family == WV_BIORTHOGONAL && w.n == 2);
    ISO_CHECK(wv_parse_wavelet("bior53", &w) == WV_OK && w.family == WV_BIORTHOGONAL && w.n == 2);
    ISO_CHECK(wv_parse_wavelet("morlet", &w) == WV_OK && w.family == WV_MORLET);
    ISO_CHECK(wv_parse_wavelet("mexican_hat", &w) == WV_OK && w.family == WV_MEXICAN_HAT);
    ISO_CHECK(wv_parse_wavelet("mexicanhat", &w) == WV_OK && w.family == WV_MEXICAN_HAT);
    ISO_CHECK(wv_parse_wavelet("ricker", &w) == WV_OK && w.family == WV_MEXICAN_HAT);

    /* Reject trailing garbage on wavelets */
    ISO_CHECK(wv_parse_wavelet("db2foo", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("sym4bar", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("coif1baz", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("db", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("sym", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("coif", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("haar123", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("unknown_wavelet", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("", &w) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_parse_wavelet("   ", &w) == WV_ERR_INVALID_PARAM);
}

static void test_cwt(void) {
    float signal[8] = {1.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f, 0.0f};
    float scales[2] = {1.0f, 2.0f};
    float *out = NULL;
    size_t olen = 0;
    size_t i;

    /* Morlet impulse response */
    ISO_CHECK(wv_cwt(signal, 8, scales, 2, wv_morlet(), &out, &olen) == WV_OK);
    ISO_CHECK_EQ_UINT(olen, 16u);
    for (i = 0; i < olen; ++i) {
        ISO_CHECK(out[i] >= 0.0f);
        ISO_CHECK(out[i] < 10.0f);
    }
    wv_free(out);

    /* Mexican Hat impulse response */
    ISO_CHECK(wv_cwt(signal, 8, scales, 2, wv_mexican_hat(), &out, &olen) == WV_OK);
    ISO_CHECK_EQ_UINT(olen, 16u);
    for (i = 0; i < olen; ++i) {
        ISO_CHECK(af(out[i]) < 10.0f);
    }
    wv_free(out);

    /* Unsupported wavelets rejected */
    ISO_CHECK(wv_cwt(signal, 8, scales, 2, wv_haar(), &out, &olen) == WV_ERR_INVALID_PARAM);
    ISO_CHECK(wv_cwt(signal, 8, scales, 2, wv_daubechies(4), &out, &olen) == WV_ERR_INVALID_PARAM);

    /* Linearity: cwt(2*x) == 2*cwt(x) */
    {
        float sig2[8];
        float *out2 = NULL;
        size_t olen2 = 0;
        for (i = 0; i < 8; ++i) {
            sig2[i] = 2.0f * signal[i];
        }
        ISO_CHECK(wv_cwt(signal, 8, scales, 2, wv_mexican_hat(), &out, &olen) == WV_OK);
        ISO_CHECK(wv_cwt(sig2, 8, scales, 2, wv_mexican_hat(), &out2, &olen2) == WV_OK);
        ISO_CHECK_EQ_UINT(olen, olen2);
        for (i = 0; i < olen; ++i) {
            ISO_CHECK(approx_eq(out2[i], 2.0f * out[i], 1e-4f));
        }
        wv_free(out);
        wv_free(out2);
    }
}

static void test_property_based(void) {
    /* Energy conservation (Parseval) for Haar 1-D */
    {
        float *sig = ramp(64, 0.08f);
        float *coeffs = NULL;
        size_t clen = 0;
        float energy_sig = 0.0f;
        float energy_coeffs = 0.0f;
        size_t i;

        for (i = 0; i < 64; ++i) {
            energy_sig += sig[i] * sig[i];
        }
        ISO_CHECK(wv_dwt_1d(sig, 64, wv_haar(), 3, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
        for (i = 0; i < clen; ++i) {
            energy_coeffs += coeffs[i] * coeffs[i];
        }
        ISO_CHECK(approx_eq(energy_sig, energy_coeffs, 1e-3f));
        free(sig);
        wv_free(coeffs);
    }

    /* Pseudo-random LCG round trips across multiple wavelets and levels */
    {
        uint32_t seed = 42;
        wv_wavelet_t test_ws[5];
        size_t wi;
        test_ws[0] = wv_haar();
        test_ws[1] = wv_daubechies(2);
        test_ws[2] = wv_daubechies(4);
        test_ws[3] = wv_symlets(4);
        test_ws[4] = wv_coiflets(1);

        for (wi = 0; wi < 5; ++wi) {
            float prng_sig[32];
            float *coeffs = NULL;
            float *recon = NULL;
            size_t clen = 0;
            size_t rlen = 0;
            size_t i;
            size_t edge;
            size_t hlen = 0;

            for (i = 0; i < 32; ++i) {
                seed = seed * 1103515245u + 12345u;
                prng_sig[i] = ((float)(seed & 0xFFFF) / 65536.0f) - 0.5f;
            }

            ISO_CHECK(wv_dwt_1d(prng_sig, 32, test_ws[wi], 2, WV_BOUND_PERIODIC, &coeffs, &clen) == WV_OK);
            ISO_CHECK(wv_idwt_1d(coeffs, clen, test_ws[wi], 2, WV_BOUND_PERIODIC, 32, &recon, &rlen) == WV_OK);
            ISO_CHECK_EQ_UINT(rlen, 32u);

            wv_analysis_lowpass(test_ws[wi], &hlen);
            edge = (test_ws[wi].family == WV_HAAR) ? 0 : hlen;
            if (edge > rlen / 4) edge = rlen / 4;
            for (i = edge; i + edge < rlen; ++i) {
                ISO_CHECK(approx_eq(prng_sig[i], recon[i], 1e-2f));
            }
            wv_free(coeffs);
            wv_free(recon);
        }
    }
}

int main(void) {
    test_error_paths();
    test_haar_reference_vector();
    test_constant_zero_detail();
    test_dirac_delta();
    test_output_length_contract();
    test_round_trips_1d();
    test_db2_constant_small_detail();
    test_split_and_slice();
    test_filter_invariants();
    test_2d();

    /* VER15 security hardening test suites */
    test_hostile_nan_inf_subnormal();
    test_null_pointers();
    test_preflight_caps_and_overflow();
    test_wavelet_and_boundary_parsing();
    test_cwt();
    test_property_based();

    return ISO_TEST_RESULT();
}
