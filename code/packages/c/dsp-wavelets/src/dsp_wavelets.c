/*
 * dsp_wavelets.c — implementation of the DWT/IDWT wavelet codec.
 *
 * A faithful port of the Rust `dsp-wavelets` crate.  Every routine mirrors its
 * Rust counterpart's control flow and numerical steps; see dsp_wavelets.h for
 * the surface documentation.  All arithmetic is single-precision `float`,
 * matching the crate's f32 dtype.
 *
 * Because the transforms are allocation-free in Rust only in the sense of
 * using Vec<f32>, this port uses malloc'd float buffers.  Every allocation is
 * checked for size_t overflow and for failure; on any error path every
 * intermediate buffer is freed before returning.
 */
#include "dsp_wavelets.h"

#include <stdlib.h>
#include <string.h>

/* ================================================================== */
/* Wavelet constructors                                               */
/* ================================================================== */

wv_wavelet_t wv_haar(void) { wv_wavelet_t w; w.family = WV_HAAR; w.n = 0; w.n2 = 0; return w; }
wv_wavelet_t wv_daubechies(uint32_t n) { wv_wavelet_t w; w.family = WV_DAUBECHIES; w.n = n; w.n2 = 0; return w; }
wv_wavelet_t wv_symlets(uint32_t n) { wv_wavelet_t w; w.family = WV_SYMLETS; w.n = n; w.n2 = 0; return w; }
wv_wavelet_t wv_coiflets(uint32_t n) { wv_wavelet_t w; w.family = WV_COIFLETS; w.n = n; w.n2 = 0; return w; }
wv_wavelet_t wv_biorthogonal(uint32_t d, uint32_t r) { wv_wavelet_t w; w.family = WV_BIORTHOGONAL; w.n = d; w.n2 = r; return w; }
wv_wavelet_t wv_morlet(void) { wv_wavelet_t w; w.family = WV_MORLET; w.n = 0; w.n2 = 0; return w; }
wv_wavelet_t wv_mexican_hat(void) { wv_wavelet_t w; w.family = WV_MEXICAN_HAT; w.n = 0; w.n2 = 0; return w; }

void wv_free(void *p) { free(p); }

/* ================================================================== */
/* Checked allocation helpers                                         */
/* ================================================================== */

/* Allocate `n` floats, all zero-initialised (calloc's multiply is the checked
 * multiply guarding against size_t overflow).  n == 0 yields a 1-element
 * allocation so the returned pointer is always non-NULL and freeable. */
static float *falloc(size_t n) {
    return (float *)calloc(n == 0 ? 1 : n, sizeof(float));
}

static size_t *zalloc(size_t n) {
    return (size_t *)calloc(n == 0 ? 1 : n, sizeof(size_t));
}

/* ================================================================== */
/* Filter tables                                                      */
/* ================================================================== */

/* PyWavelets `dec_lo` analysis lowpass coefficients, stored as float. */
static const float DB2[4] = {-0.12940952f, 0.22414386f, 0.8365163f, 0.4829629f};
static const float DB4[8] = {-0.010597402f, 0.03288301f, 0.030841382f, -0.18703482f,
                             -0.02798377f, 0.6308808f, 0.71484655f, 0.23037781f};
static const float DB6[12] = {0.11154074f, 0.4946239f, 0.7511339f, 0.31525034f,
                              -0.2262647f, -0.12976687f, 0.097501606f, 0.027522866f,
                              -0.03158204f, 0.00055384222f, 0.0047772573f, -0.0010773011f};
static const float DB8[16] = {0.05441584f, 0.3128716f, 0.67563075f, 0.5853547f,
                              -0.015829105f, -0.28401554f, 0.00047248456f, 0.12874743f,
                              -0.0173693f, -0.044088256f, 0.0139810275f, 0.008746094f,
                              -0.004870353f, -0.00039174038f, 0.00067544994f, -0.00011747678f};
static const float SYM4[8] = {-0.075765714f, -0.029635528f, 0.49761868f, 0.8037388f,
                              0.2978578f, -0.099219546f, -0.012603967f, 0.0322231f};
static const float COIF1[6] = {-0.015655728f, -0.07273262f, 0.38486484f, 0.852572f,
                               0.33789766f, -0.07273262f};

const float *wv_analysis_lowpass(wv_wavelet_t w, size_t *out_len) {
    switch (w.family) {
    case WV_DAUBECHIES:
        if (w.n == 2) { *out_len = 4; return DB2; }
        if (w.n == 4) { *out_len = 8; return DB4; }
        if (w.n == 6) { *out_len = 12; return DB6; }
        if (w.n == 8) { *out_len = 16; return DB8; }
        break;
    case WV_SYMLETS:
        if (w.n == 4) { *out_len = 8; return SYM4; }
        break;
    case WV_COIFLETS:
        if (w.n == 1) { *out_len = 6; return COIF1; }
        break;
    default:
        break;
    }
    *out_len = 0;
    return NULL;
}

wv_error_t wv_qmf_highpass(const float *h, size_t h_len, float **out, size_t *out_len) {
    float *g;
    size_t i;
    *out = NULL;
    g = falloc(h_len);
    if (g == NULL) {
        return WV_ERR_ALLOC;
    }
    for (i = 0; i < h_len; ++i) {
        float reversed = h[h_len - 1 - i];
        g[i] = (i & 1) == 0 ? reversed : -reversed;
    }
    *out = g;
    *out_len = h_len;
    return WV_OK;
}

/* Analysis filter pair (h, g).  Both are freshly allocated (free with
 * wv_free).  Returns WV_ERR_INVALID_PARAM for unsupported wavelets. */
static wv_error_t analysis_filters(wv_wavelet_t w, float **h_out, float **g_out, size_t *len_out) {
    *h_out = NULL;
    *g_out = NULL;
    if (w.family == WV_HAAR) {
        float *h = falloc(2);
        float *g = falloc(2);
        if (h == NULL || g == NULL) {
            free(h);
            free(g);
            return WV_ERR_ALLOC;
        }
        h[0] = WV_FRAC_1_SQRT_2;
        h[1] = WV_FRAC_1_SQRT_2;
        g[0] = WV_FRAC_1_SQRT_2;
        g[1] = -WV_FRAC_1_SQRT_2;
        *h_out = h;
        *g_out = g;
        *len_out = 2;
        return WV_OK;
    }
    if (w.family == WV_DAUBECHIES || w.family == WV_SYMLETS || w.family == WV_COIFLETS) {
        size_t hlen;
        const float *table = wv_analysis_lowpass(w, &hlen);
        float *h;
        float *g;
        wv_error_t err;
        if (table == NULL) {
            return WV_ERR_INVALID_PARAM;
        }
        h = falloc(hlen);
        if (h == NULL) {
            return WV_ERR_ALLOC;
        }
        memcpy(h, table, hlen * sizeof(float));
        err = wv_qmf_highpass(h, hlen, &g, &hlen);
        if (err != WV_OK) {
            free(h);
            return err;
        }
        *h_out = h;
        *g_out = g;
        *len_out = hlen;
        return WV_OK;
    }
    return WV_ERR_INVALID_PARAM;
}

static size_t filter_length_for(wv_wavelet_t w) {
    size_t len;
    if (w.family == WV_HAAR) {
        return 2;
    }
    wv_analysis_lowpass(w, &len);
    return len;
}

static wv_error_t check_supported_wavelet(wv_wavelet_t w) {
    size_t len;
    if (w.family == WV_HAAR) {
        return WV_OK;
    }
    if (w.family == WV_DAUBECHIES || w.family == WV_SYMLETS || w.family == WV_COIFLETS) {
        wv_analysis_lowpass(w, &len);
        return len == 0 ? WV_ERR_INVALID_PARAM : WV_OK;
    }
    return WV_ERR_INVALID_PARAM;
}

static wv_error_t check_supported_boundary(wv_boundary_t b) {
    if (b == WV_BOUND_SYMMETRIC || b == WV_BOUND_PERIODIC) {
        return WV_OK;
    }
    return WV_ERR_INVALID_PARAM;
}

static wv_error_t check_levels(uint32_t levels) {
    if (levels == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels > WV_MAX_LEVELS) {
        return WV_ERR_INVALID_PARAM;
    }
    return WV_OK;
}

/* ================================================================== */
/* Boundary sampling / one-step filter banks                          */
/* ================================================================== */

/* Sample signal[idx] with the requested boundary extension.  idx may be
 * negative or >= signal_len; the boundary rule maps it into [0, signal_len). */
static float sample_with_boundary(const float *signal, size_t signal_len, int64_t idx,
                                  wv_boundary_t boundary) {
    int64_t n = (int64_t)signal_len;
    if (n == 0) {
        return 0.0f;
    }
    if (idx >= 0 && idx < n) {
        return signal[(size_t)idx];
    }
    if (boundary == WV_BOUND_PERIODIC) {
        int64_t m = ((idx % n) + n) % n;
        return signal[(size_t)m];
    }
    if (boundary == WV_BOUND_SYMMETRIC) {
        int64_t period = 2 * n;
        int64_t m = ((idx % period) + period) % period;
        if (m >= n) {
            m = 2 * n - 1 - m;
        }
        return signal[(size_t)m];
    }
    return 0.0f; /* Zero / Replicate / Reflect pre-rejected upstream */
}

/* One Mallat step: filter with h and g, downsample by 2 (keep odd indices).
 * Allocates ca and cd, each of length ceil(n/2). */
static wv_error_t filter_and_downsample(const float *signal, size_t n, const float *h,
                                        const float *g, size_t filter_len,
                                        wv_boundary_t boundary, float **ca_out, float **cd_out,
                                        size_t *half_out) {
    size_t out_len = (n + 1) / 2;
    size_t out_idx;
    float *ca = falloc(out_len);
    float *cd = falloc(out_len);
    if (ca == NULL || cd == NULL) {
        free(ca);
        free(cd);
        return WV_ERR_ALLOC;
    }
    for (out_idx = 0; out_idx < out_len; ++out_idx) {
        int64_t k = (int64_t)(2 * out_idx + 1);
        float acc_h = 0.0f;
        float acc_g = 0.0f;
        size_t i;
        for (i = 0; i < filter_len; ++i) {
            int64_t src = k - (int64_t)i;
            float sample = sample_with_boundary(signal, n, src, boundary);
            acc_h += h[i] * sample;
            acc_g += g[i] * sample;
        }
        ca[out_idx] = acc_h;
        cd[out_idx] = acc_g;
    }
    *ca_out = ca;
    *cd_out = cd;
    *half_out = out_len;
    return WV_OK;
}

/* Generic one-step synthesis for any orthogonal filter pair.  Allocates the
 * result of length target_len. */
static wv_error_t synthesize_one_level(const float *ca, size_t ca_len, const float *cd,
                                       size_t cd_len, const float *h, const float *g,
                                       size_t filter_len, size_t target_len,
                                       wv_boundary_t boundary, float **out) {
    size_t nn;
    float *res = falloc(target_len);
    *out = NULL;
    if (res == NULL) {
        return WV_ERR_ALLOC;
    }
    for (nn = 0; nn < target_len; ++nn) {
        float acc = 0.0f;
        size_t i;
        for (i = 0; i < filter_len; ++i) {
            int64_t numerator = (int64_t)nn + (int64_t)i - 1;
            if ((numerator & 1) == 0) {
                int64_t m = numerator / 2;
                float ca_val = sample_with_boundary(ca, ca_len, m, boundary);
                float cd_val = sample_with_boundary(cd, cd_len, m, boundary);
                acc += h[i] * ca_val + g[i] * cd_val;
            }
        }
        res[nn] = acc;
    }
    *out = res;
    return WV_OK;
}

/* Per-level signal lengths [L_0, L_1, ..., L_J] under ceil(/2) halving.
 * Allocates a size_t array of length levels+1. */
static wv_error_t forward_level_lengths(size_t signal_len, uint32_t levels, size_t **out) {
    size_t *lens = zalloc((size_t)levels + 1);
    size_t cur = signal_len;
    uint32_t i;
    *out = NULL;
    if (lens == NULL) {
        return WV_ERR_ALLOC;
    }
    lens[0] = signal_len;
    for (i = 0; i < levels; ++i) {
        cur = (cur + 1) / 2;
        lens[i + 1] = cur;
    }
    *out = lens;
    return WV_OK;
}

static wv_error_t validate_dwt_inputs(size_t signal_len, wv_wavelet_t wavelet, uint32_t levels,
                                      wv_boundary_t boundary) {
    wv_error_t err;
    size_t filter_len;
    uint32_t shift;
    size_t pow2;
    size_t min_len;
    if (signal_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    err = check_levels(levels);
    if (err != WV_OK) {
        return err;
    }
    err = check_supported_wavelet(wavelet);
    if (err != WV_OK) {
        return err;
    }
    err = check_supported_boundary(boundary);
    if (err != WV_OK) {
        return err;
    }
    if (signal_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    filter_len = filter_length_for(wavelet);
    shift = levels - 1;
    if (shift > 31) {
        shift = 31;
    }
    pow2 = (size_t)1 << shift;
    min_len = filter_len > pow2 ? filter_len : pow2;
    if (signal_len < min_len) {
        return WV_ERR_SIGNAL_TOO_SHORT;
    }
    return WV_OK;
}

/* ================================================================== */
/* 1-D forward / inverse                                              */
/* ================================================================== */

wv_error_t wv_dwt_1d(const float *signal, size_t signal_len, wv_wavelet_t wavelet,
                     uint32_t levels, wv_boundary_t boundary, float **out, size_t *out_len) {
    wv_error_t err;
    float *h = NULL;
    float *g = NULL;
    size_t flen = 0;
    float *current = NULL;
    size_t cur_len;
    /* levels is bounded by WV_MAX_LEVELS (31); fixed arrays hold the details. */
    float *details[WV_MAX_LEVELS];
    size_t dlen[WV_MAX_LEVELS];
    uint32_t produced = 0;
    uint32_t l;
    size_t total;
    float *result = NULL;
    size_t pos;

    *out = NULL;

    err = validate_dwt_inputs(signal_len, wavelet, levels, boundary);
    if (err != WV_OK) {
        return err;
    }
    err = analysis_filters(wavelet, &h, &g, &flen);
    if (err != WV_OK) {
        return err;
    }

    current = falloc(signal_len);
    if (current == NULL) {
        err = WV_ERR_ALLOC;
        goto cleanup;
    }
    memcpy(current, signal, signal_len * sizeof(float));
    cur_len = signal_len;

    for (l = 0; l < levels; ++l) {
        float *ca = NULL;
        float *cd = NULL;
        size_t half = 0;
        err = filter_and_downsample(current, cur_len, h, g, flen, boundary, &ca, &cd, &half);
        if (err != WV_OK) {
            goto cleanup;
        }
        free(current);
        current = ca;
        cur_len = half;
        details[produced] = cd;
        dlen[produced] = half;
        produced++;
    }

    total = cur_len;
    for (l = 0; l < produced; ++l) {
        total += dlen[l];
    }
    result = falloc(total);
    if (result == NULL) {
        err = WV_ERR_ALLOC;
        goto cleanup;
    }
    memcpy(result, current, cur_len * sizeof(float));
    pos = cur_len;
    /* details[produced-1] is the coarsest (cD_J); emit coarsest first. */
    for (l = produced; l > 0; --l) {
        memcpy(result + pos, details[l - 1], dlen[l - 1] * sizeof(float));
        pos += dlen[l - 1];
    }

    *out = result;
    *out_len = total;
    err = WV_OK;

cleanup:
    free(h);
    free(g);
    free(current);
    for (l = 0; l < produced; ++l) {
        free(details[l]);
    }
    return err;
}

wv_error_t wv_idwt_1d(const float *coeffs, size_t coeffs_len, wv_wavelet_t wavelet,
                      uint32_t levels, wv_boundary_t boundary, uint32_t output_length,
                      float **out, size_t *out_len) {
    wv_error_t err;
    size_t *level_lens = NULL;
    size_t coarsest_ca_len;
    size_t expected_total;
    uint32_t j;
    size_t offset;
    float *current = NULL;
    size_t cur_len;
    float *h = NULL;
    float *g = NULL;
    size_t flen = 0;

    *out = NULL;

    if (coeffs_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (output_length == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (output_length > WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    err = check_levels(levels);
    if (err != WV_OK) {
        return err;
    }
    if (coeffs_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    err = check_supported_wavelet(wavelet);
    if (err != WV_OK) {
        return err;
    }
    err = check_supported_boundary(boundary);
    if (err != WV_OK) {
        return err;
    }

    err = forward_level_lengths(output_length, levels, &level_lens);
    if (err != WV_OK) {
        return err;
    }
    coarsest_ca_len = level_lens[levels];
    expected_total = coarsest_ca_len;
    for (j = 1; j <= levels; ++j) {
        expected_total += level_lens[j];
    }
    if (coeffs_len != expected_total) {
        free(level_lens);
        return WV_ERR_INVALID_COEFFICIENTS;
    }

    err = analysis_filters(wavelet, &h, &g, &flen);
    if (err != WV_OK) {
        free(level_lens);
        return err;
    }

    current = falloc(coarsest_ca_len);
    if (current == NULL) {
        err = WV_ERR_ALLOC;
        goto cleanup;
    }
    memcpy(current, coeffs, coarsest_ca_len * sizeof(float));
    cur_len = coarsest_ca_len;
    offset = coarsest_ca_len;

    for (j = levels; j >= 1; --j) {
        size_t cd_len = level_lens[j];
        size_t target_len = level_lens[j - 1];
        float *next = NULL;
        err = synthesize_one_level(current, cur_len, coeffs + offset, cd_len, h, g, flen,
                                   target_len, boundary, &next);
        if (err != WV_OK) {
            goto cleanup;
        }
        offset += cd_len;
        free(current);
        current = next;
        cur_len = target_len;
    }

    *out = current;
    *out_len = cur_len;
    current = NULL; /* ownership transferred */
    err = WV_OK;

cleanup:
    free(level_lens);
    free(h);
    free(g);
    free(current);
    return err;
}

wv_error_t wv_split_levels(size_t coeffs_len, size_t signal_len, uint32_t levels, size_t **out,
                           size_t *out_len) {
    wv_error_t err;
    size_t *level_lens = NULL;
    size_t coarsest_ca;
    size_t expected_total;
    uint32_t j;
    size_t *offsets;
    size_t off;
    size_t idx;

    *out = NULL;

    if (signal_len == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (signal_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    err = check_levels(levels);
    if (err != WV_OK) {
        return err;
    }
    err = forward_level_lengths(signal_len, levels, &level_lens);
    if (err != WV_OK) {
        return err;
    }
    coarsest_ca = level_lens[levels];
    expected_total = coarsest_ca;
    for (j = 1; j <= levels; ++j) {
        expected_total += level_lens[j];
    }
    if (coeffs_len != expected_total) {
        free(level_lens);
        return WV_ERR_INVALID_COEFFICIENTS;
    }

    offsets = zalloc((size_t)levels + 2);
    if (offsets == NULL) {
        free(level_lens);
        return WV_ERR_ALLOC;
    }
    idx = 0;
    offsets[idx++] = 0; /* cA_J */
    off = coarsest_ca;
    for (j = levels; j >= 1; --j) {
        offsets[idx++] = off;
        off += level_lens[j];
    }
    offsets[idx++] = off; /* total_len sentinel */

    free(level_lens);
    *out = offsets;
    *out_len = (size_t)levels + 2;
    return WV_OK;
}

wv_error_t wv_slice_level(const float *coeffs, size_t coeffs_len, size_t signal_len,
                          uint32_t levels, uint32_t target_level, wv_band_t band,
                          const float **out, size_t *out_len) {
    wv_error_t err;
    size_t *offsets = NULL;
    size_t offsets_len;
    size_t start;
    size_t end;

    *out = NULL;

    if (target_level == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (target_level > levels) {
        return WV_ERR_INVALID_PARAM;
    }
    if (band == WV_BAND_APPROXIMATION && target_level != levels) {
        return WV_ERR_INVALID_PARAM;
    }
    err = wv_split_levels(coeffs_len, signal_len, levels, &offsets, &offsets_len);
    if (err != WV_OK) {
        return err;
    }
    if (band == WV_BAND_APPROXIMATION) {
        start = offsets[0];
        end = offsets[1];
    } else {
        size_t idx = (size_t)(levels - target_level + 1);
        start = offsets[idx];
        end = offsets[idx + 1];
    }
    free(offsets);
    *out = coeffs + start;
    *out_len = end - start;
    return WV_OK;
}

/* ================================================================== */
/* 2-D forward / inverse                                              */
/* ================================================================== */

static wv_error_t check_2d_supported(wv_wavelet_t wavelet, wv_boundary_t boundary) {
    if (boundary != WV_BOUND_PERIODIC) {
        return WV_ERR_INVALID_PARAM;
    }
    switch (wavelet.family) {
    case WV_HAAR:
    case WV_DAUBECHIES:
    case WV_SYMLETS:
    case WV_COIFLETS:
        return WV_OK;
    default:
        return WV_ERR_INVALID_PARAM;
    }
}

static wv_error_t validate_2d_inputs(size_t image_len, uint32_t n_rows, uint32_t n_cols,
                                     uint32_t levels) {
    if (image_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (n_rows == 0 || n_cols == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if ((size_t)n_rows > (size_t)WV_MAX_SAMPLES || (size_t)n_cols > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (image_len != (size_t)n_rows * (size_t)n_cols) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels > WV_MAX_LEVELS) {
        return WV_ERR_INVALID_PARAM;
    }
    return WV_OK;
}

/* Per-level (rows, cols) dims: dims[0]=(n_rows,n_cols), then ceil(/2) halving.
 * Allocates a flat size_t array of length 2*(levels+1): [r0,c0,r1,c1,...]. */
static wv_error_t forward_level_dims(size_t n_rows, size_t n_cols, uint32_t levels,
                                     size_t **out) {
    size_t *dims = zalloc((size_t)2 * ((size_t)levels + 1));
    size_t r = n_rows;
    size_t c = n_cols;
    uint32_t i;
    *out = NULL;
    if (dims == NULL) {
        return WV_ERR_ALLOC;
    }
    dims[0] = n_rows;
    dims[1] = n_cols;
    for (i = 0; i < levels; ++i) {
        r = (r + 1) / 2;
        c = (c + 1) / 2;
        dims[2 * (i + 1)] = r;
        dims[2 * (i + 1) + 1] = c;
    }
    *out = dims;
    return WV_OK;
}

/* One level of 2-D DWT: row pass then column pass → (ll, hl, lh, hh), each of
 * size half_rows*half_cols.  All four are allocated on success. */
static wv_error_t dwt_2d_one_level(const float *image, size_t n_rows, size_t n_cols,
                                   wv_wavelet_t wavelet, wv_boundary_t boundary, float **ll_out,
                                   float **hl_out, float **lh_out, float **hh_out) {
    size_t half_cols = (n_cols + 1) / 2;
    size_t half_rows = (n_rows + 1) / 2;
    size_t band = half_rows * half_cols;
    float *l_rows = falloc(n_rows * half_cols);
    float *h_rows = falloc(n_rows * half_cols);
    float *ll = falloc(band);
    float *hl = falloc(band);
    float *lh = falloc(band);
    float *hh = falloc(band);
    float *col_buf = falloc(n_rows);
    wv_error_t err = WV_OK;
    size_t r;
    size_t c;

    if (l_rows == NULL || h_rows == NULL || ll == NULL || hl == NULL || lh == NULL ||
        hh == NULL || col_buf == NULL) {
        err = WV_ERR_ALLOC;
        goto fail;
    }

    /* Row pass: 1-level 1-D DWT of each row, splitting into L and H halves. */
    for (r = 0; r < n_rows; ++r) {
        float *row_coeffs = NULL;
        size_t rc_len = 0;
        err = wv_dwt_1d(image + r * n_cols, n_cols, wavelet, 1, boundary, &row_coeffs, &rc_len);
        if (err != WV_OK) {
            goto fail;
        }
        memcpy(l_rows + r * half_cols, row_coeffs, half_cols * sizeof(float));
        memcpy(h_rows + r * half_cols, row_coeffs + half_cols, half_cols * sizeof(float));
        free(row_coeffs);
    }

    /* Column pass over each of L and H. */
    for (c = 0; c < half_cols; ++c) {
        float *col_coeffs = NULL;
        size_t cc_len = 0;
        for (r = 0; r < n_rows; ++r) {
            col_buf[r] = l_rows[r * half_cols + c];
        }
        err = wv_dwt_1d(col_buf, n_rows, wavelet, 1, boundary, &col_coeffs, &cc_len);
        if (err != WV_OK) {
            goto fail;
        }
        for (r = 0; r < half_rows; ++r) {
            ll[r * half_cols + c] = col_coeffs[r];
            lh[r * half_cols + c] = col_coeffs[half_rows + r];
        }
        free(col_coeffs);

        for (r = 0; r < n_rows; ++r) {
            col_buf[r] = h_rows[r * half_cols + c];
        }
        err = wv_dwt_1d(col_buf, n_rows, wavelet, 1, boundary, &col_coeffs, &cc_len);
        if (err != WV_OK) {
            goto fail;
        }
        for (r = 0; r < half_rows; ++r) {
            hl[r * half_cols + c] = col_coeffs[r];
            hh[r * half_cols + c] = col_coeffs[half_rows + r];
        }
        free(col_coeffs);
    }

    free(l_rows);
    free(h_rows);
    free(col_buf);
    *ll_out = ll;
    *hl_out = hl;
    *lh_out = lh;
    *hh_out = hh;
    return WV_OK;

fail:
    free(l_rows);
    free(h_rows);
    free(col_buf);
    free(ll);
    free(hl);
    free(lh);
    free(hh);
    return err;
}

/* Inverse of dwt_2d_one_level.  Allocates the target_rows*target_cols result. */
static wv_error_t idwt_2d_one_level(const float *ll, const float *hl, const float *lh,
                                    const float *hh, size_t band_rows, size_t band_cols,
                                    size_t target_rows, size_t target_cols, wv_wavelet_t wavelet,
                                    wv_boundary_t boundary, float **out) {
    float *l_rows = falloc(target_rows * band_cols);
    float *h_rows = falloc(target_rows * band_cols);
    float *col_coeffs = falloc(2 * band_rows);
    float *row_coeffs = falloc(2 * band_cols);
    float *result = falloc(target_rows * target_cols);
    wv_error_t err = WV_OK;
    size_t r;
    size_t c;

    *out = NULL;
    if (l_rows == NULL || h_rows == NULL || col_coeffs == NULL || row_coeffs == NULL ||
        result == NULL) {
        err = WV_ERR_ALLOC;
        goto done;
    }

    /* Inverse column pass. */
    for (c = 0; c < band_cols; ++c) {
        float *col = NULL;
        size_t col_len = 0;
        for (r = 0; r < band_rows; ++r) {
            col_coeffs[r] = ll[r * band_cols + c];
            col_coeffs[band_rows + r] = lh[r * band_cols + c];
        }
        err = wv_idwt_1d(col_coeffs, 2 * band_rows, wavelet, 1, boundary, (uint32_t)target_rows,
                         &col, &col_len);
        if (err != WV_OK) {
            goto done;
        }
        for (r = 0; r < target_rows; ++r) {
            l_rows[r * band_cols + c] = col[r];
        }
        free(col);

        for (r = 0; r < band_rows; ++r) {
            col_coeffs[r] = hl[r * band_cols + c];
            col_coeffs[band_rows + r] = hh[r * band_cols + c];
        }
        err = wv_idwt_1d(col_coeffs, 2 * band_rows, wavelet, 1, boundary, (uint32_t)target_rows,
                         &col, &col_len);
        if (err != WV_OK) {
            goto done;
        }
        for (r = 0; r < target_rows; ++r) {
            h_rows[r * band_cols + c] = col[r];
        }
        free(col);
    }

    /* Inverse row pass. */
    for (r = 0; r < target_rows; ++r) {
        float *row = NULL;
        size_t row_len = 0;
        for (c = 0; c < band_cols; ++c) {
            row_coeffs[c] = l_rows[r * band_cols + c];
            row_coeffs[band_cols + c] = h_rows[r * band_cols + c];
        }
        err = wv_idwt_1d(row_coeffs, 2 * band_cols, wavelet, 1, boundary, (uint32_t)target_cols,
                         &row, &row_len);
        if (err != WV_OK) {
            goto done;
        }
        memcpy(result + r * target_cols, row, target_cols * sizeof(float));
        free(row);
    }

    *out = result;
    result = NULL; /* ownership transferred */
    err = WV_OK;

done:
    free(l_rows);
    free(h_rows);
    free(col_coeffs);
    free(row_coeffs);
    free(result);
    return err;
}

wv_error_t wv_dwt_2d(const float *image, size_t image_len, uint32_t n_rows, uint32_t n_cols,
                     wv_wavelet_t wavelet, uint32_t levels, wv_boundary_t boundary, float **out,
                     size_t *out_len) {
    wv_error_t err;
    float *current = NULL;
    size_t cur_rows;
    size_t cur_cols;
    /* Three detail bands (hl, lh, hh) per level. */
    float *hl_arr[WV_MAX_LEVELS];
    float *lh_arr[WV_MAX_LEVELS];
    float *hh_arr[WV_MAX_LEVELS];
    size_t blen[WV_MAX_LEVELS];
    uint32_t produced = 0;
    uint32_t l;
    size_t total;
    float *result = NULL;
    size_t pos;

    *out = NULL;

    err = validate_2d_inputs(image_len, n_rows, n_cols, levels);
    if (err != WV_OK) {
        return err;
    }
    err = check_2d_supported(wavelet, boundary);
    if (err != WV_OK) {
        return err;
    }

    current = falloc(image_len);
    if (current == NULL) {
        return WV_ERR_ALLOC;
    }
    memcpy(current, image, image_len * sizeof(float));
    cur_rows = n_rows;
    cur_cols = n_cols;

    for (l = 0; l < levels; ++l) {
        float *ll = NULL;
        float *hl = NULL;
        float *lh = NULL;
        float *hh = NULL;
        err = dwt_2d_one_level(current, cur_rows, cur_cols, wavelet, boundary, &ll, &hl, &lh, &hh);
        if (err != WV_OK) {
            goto cleanup;
        }
        free(current);
        current = ll;
        hl_arr[produced] = hl;
        lh_arr[produced] = lh;
        hh_arr[produced] = hh;
        blen[produced] = ((cur_rows + 1) / 2) * ((cur_cols + 1) / 2);
        produced++;
        cur_rows = (cur_rows + 1) / 2;
        cur_cols = (cur_cols + 1) / 2;
    }

    total = cur_rows * cur_cols;
    for (l = 0; l < produced; ++l) {
        total += 3 * blen[l];
    }
    result = falloc(total);
    if (result == NULL) {
        err = WV_ERR_ALLOC;
        goto cleanup;
    }
    memcpy(result, current, cur_rows * cur_cols * sizeof(float));
    pos = cur_rows * cur_cols;
    for (l = produced; l > 0; --l) {
        size_t bl = blen[l - 1];
        memcpy(result + pos, hl_arr[l - 1], bl * sizeof(float));
        pos += bl;
        memcpy(result + pos, lh_arr[l - 1], bl * sizeof(float));
        pos += bl;
        memcpy(result + pos, hh_arr[l - 1], bl * sizeof(float));
        pos += bl;
    }

    *out = result;
    *out_len = total;
    err = WV_OK;

cleanup:
    free(current);
    for (l = 0; l < produced; ++l) {
        free(hl_arr[l]);
        free(lh_arr[l]);
        free(hh_arr[l]);
    }
    return err;
}

wv_error_t wv_idwt_2d(const float *coeffs, size_t coeffs_len, uint32_t n_rows, uint32_t n_cols,
                      wv_wavelet_t wavelet, uint32_t levels, wv_boundary_t boundary, float **out,
                      size_t *out_len) {
    wv_error_t err;
    size_t *dims = NULL;
    size_t ll_rows;
    size_t ll_cols;
    size_t expected_total;
    uint32_t j;
    size_t offset;
    float *ll = NULL;

    *out = NULL;

    if (coeffs_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (n_rows == 0 || n_cols == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if ((size_t)n_rows > (size_t)WV_MAX_SAMPLES || (size_t)n_cols > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels > WV_MAX_LEVELS) {
        return WV_ERR_INVALID_PARAM;
    }
    err = check_2d_supported(wavelet, boundary);
    if (err != WV_OK) {
        return err;
    }

    err = forward_level_dims(n_rows, n_cols, levels, &dims);
    if (err != WV_OK) {
        return err;
    }
    ll_rows = dims[2 * levels];
    ll_cols = dims[2 * levels + 1];
    expected_total = ll_rows * ll_cols;
    for (j = 1; j <= levels; ++j) {
        expected_total += 3 * dims[2 * j] * dims[2 * j + 1];
    }
    if (coeffs_len != expected_total) {
        free(dims);
        return WV_ERR_INVALID_COEFFICIENTS;
    }

    ll = falloc(ll_rows * ll_cols);
    if (ll == NULL) {
        free(dims);
        return WV_ERR_ALLOC;
    }
    memcpy(ll, coeffs, ll_rows * ll_cols * sizeof(float));
    offset = ll_rows * ll_cols;

    for (j = levels; j >= 1; --j) {
        size_t rj = dims[2 * j];
        size_t cj = dims[2 * j + 1];
        size_t band = rj * cj;
        const float *hl = coeffs + offset;
        const float *lh = coeffs + offset + band;
        const float *hh = coeffs + offset + 2 * band;
        size_t target_rows = dims[2 * (j - 1)];
        size_t target_cols = dims[2 * (j - 1) + 1];
        float *next = NULL;
        err = idwt_2d_one_level(ll, hl, lh, hh, rj, cj, target_rows, target_cols, wavelet,
                                boundary, &next);
        if (err != WV_OK) {
            goto cleanup;
        }
        offset += 3 * band;
        free(ll);
        ll = next;
    }

    *out = ll;
    *out_len = ll_rows == 0 ? 0 : (size_t)n_rows * (size_t)n_cols;
    ll = NULL; /* ownership transferred */
    err = WV_OK;

cleanup:
    free(dims);
    free(ll);
    return err;
}
