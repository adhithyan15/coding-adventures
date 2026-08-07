/*
 * dsp_wavelets_test.c — unit tests for the C wavelet transform port.
 *
 * Mirrors the Rust crate's suite: error paths, output-length contract, the
 * hand-worked Haar reference vector, constant/dirac structural properties,
 * perfect-reconstruction round-trips (Haar + Daubechies/Symlets/Coiflets, 1-D
 * and 2-D), split_levels/slice_level, and the filter-bank invariants.
 *
 * Test signals use simple ramps rather than sin/cos so the lane stays free of
 * <math.h>; perfect reconstruction holds for any input signal.
 */
#include "dsp_wavelets.h"
#include "iso_test.h"

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
    size_t hlen;
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
        size_t len;
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
    /* Shape contract. */
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

    /* Haar / rect / Db4 round trips. */
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
    /* Constant image → zero detail. */
    {
        float *image = constant(16 * 16, 2.5f);
        float *coeffs = NULL;
        size_t clen = 0;
        size_t i;
        ISO_CHECK(wv_dwt_2d(image, 256, 16, 16, wv_haar(), 2, WV_BOUND_PERIODIC, &coeffs, &clen) ==
                  WV_OK);
        for (i = 16; i < clen; ++i) { /* LL_2 = 4*4 = 16 */
            ISO_CHECK(af(coeffs[i]) <= 1e-6f);
        }
        free(image);
        wv_free(coeffs);
    }
    /* 2-D error paths. */
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
    return ISO_TEST_RESULT();
}
