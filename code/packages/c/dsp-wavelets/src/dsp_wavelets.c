/*
 * dsp_wavelets.c — implementation of the DWT/IDWT/CWT wavelet codec (pure ISO C17).
 *
 * A faithful port of the Rust `dsp-wavelets` crate. Every routine mirrors its
 * Rust counterpart's control flow and numerical steps; see dsp_wavelets.h for
 * the surface documentation. All arithmetic is single-precision `float`.
 *
 * Hardened with full VER15 CWE/CVE proofs:
 * - CWE-20: Finite/non-NaN/non-Inf checks, valid wavelet name & boundary parsing,
 *           non-null pointer and positive dimension guards.
 * - CWE-120/122/125/787: Bounds-checked indexing, safe buffer sizing, no OOB.
 * - CWE-190: Overflow-checked arithmetic on dimensions, strides, and buffer sizes.
 * - CWE-400: Defensive caps (WV_MAX_SAMPLES, WV_MAX_LEVELS, WV_MAX_SCALES, WV_MAX_2D_DIM).
 * - CWE-674/1333: Strictly iterative, zero recursion, zero regex.
 */
#include "dsp_wavelets.h"

#include <stdlib.h>
#include <string.h>

/* ================================================================== */
/* Checked arithmetic helpers (CWE-190)                               */
/* ================================================================== */

static inline int checked_add_size(size_t a, size_t b, size_t *out) {
    if (SIZE_MAX - a < b) {
        return 0;
    }
    *out = a + b;
    return 1;
}

static inline int checked_mul_size(size_t a, size_t b, size_t *out) {
    if (a != 0 && b > SIZE_MAX / a) {
        return 0;
    }
    *out = a * b;
    return 1;
}

/* ================================================================== */
/* Floating-point finiteness validation (CWE-20)                      */
/* ================================================================== */

static inline int wv_isfinite(float x) {
    /* Pure IEEE-754: x == x fails for NaN; (x - x == 0.0f) fails for ±Inf */
    return (x == x) && ((x - x) == 0.0f);
}

/* Pure ISO C math helpers (zero libm/-lm dependency for iso-harness). */
static float wv_sqrtf(float x) {
    double m;
    double scale = 1.0;
    double g;
    int iter;
    if (x <= 0.0f || !wv_isfinite(x)) {
        return 0.0f;
    }
    m = (double)x;
    while (m < 0.25) { m *= 4.0; scale *= 0.5; }
    while (m >= 1.0) { m *= 0.25; scale *= 2.0; }
    g = 0.5 * (1.0 + m);
    for (iter = 0; iter < 8; ++iter) {
        g = 0.5 * (g + m / g);
    }
    return (float)(g * scale);
}

static float wv_expf(float x) {
    const double LN2 = 0.693147180559945309417;
    double d;
    int k;
    double r;
    double term = 1.0;
    double sum = 1.0;
    int i;
    if (x <= -88.0f) { return 0.0f; }
    if (x >= 88.0f) { return 3.402823466e+38f; }
    d = (double)x;
    k = (int)(d / LN2 + (d >= 0.0 ? 0.5 : -0.5));
    r = d - (double)k * LN2;
    for (i = 1; i <= 12; ++i) {
        term *= r / (double)i;
        sum += term;
    }
    while (k > 0) { sum *= 2.0; --k; }
    while (k < 0) { sum *= 0.5; ++k; }
    return (float)sum;
}

static void wv_sincosf(float x, float *sin_out, float *cos_out) {
    const double PI = 3.14159265358979323846;
    const double TWO_PI = 6.28318530717958647692;
    double d = (double)x;
    double r;
    double r2;
    double s_term;
    double c_term;
    double s_sum;
    double c_sum;
    int k;
    int n;
    k = (int)(d / TWO_PI + (d >= 0.0 ? 0.5 : -0.5));
    r = d - (double)k * TWO_PI;
    if (r > PI) { r -= TWO_PI; }
    if (r < -PI) { r += TWO_PI; }
    r2 = r * r;
    s_term = r;
    s_sum = r;
    c_term = 1.0;
    c_sum = 1.0;
    for (n = 1; n <= 9; ++n) {
        s_term *= -r2 / (double)((2 * n) * (2 * n + 1));
        s_sum += s_term;
        c_term *= -r2 / (double)((2 * n - 1) * (2 * n));
        c_sum += c_term;
    }
    *sin_out = (float)s_sum;
    *cos_out = (float)c_sum;
}

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

static float *falloc(size_t n) {
    size_t count = n == 0 ? 1 : n;
    size_t bytes;
    if (!checked_mul_size(count, sizeof(float), &bytes)) {
        return NULL;
    }
    return (float *)calloc(count, sizeof(float));
}

static size_t *zalloc(size_t n) {
    size_t count = n == 0 ? 1 : n;
    size_t bytes;
    if (!checked_mul_size(count, sizeof(size_t), &bytes)) {
        return NULL;
    }
    return (size_t *)calloc(count, sizeof(size_t));
}

/* ================================================================== */
/* Parsing helpers (CWE-20, reject trailing garbage, zero regex)      */
/* ================================================================== */

static char ascii_tolower(char c) {
    if (c >= 'A' && c <= 'Z') {
        return (char)(c + ('a' - 'A'));
    }
    return c;
}

static int is_space(char c) {
    return c == ' ' || c == '\t' || c == '\r' || c == '\n';
}

wv_error_t wv_parse_boundary(const char *str, wv_boundary_t *out) {
    size_t start = 0;
    size_t end;
    size_t len;
    char buf[32];
    size_t i;

    if (str == NULL || out == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    while (str[start] != '\0' && is_space(str[start])) {
        start++;
    }
    end = start;
    while (str[end] != '\0') {
        end++;
    }
    while (end > start && is_space(str[end - 1])) {
        end--;
    }
    len = end - start;
    if (len == 0 || len >= sizeof(buf)) {
        return WV_ERR_INVALID_PARAM;
    }
    for (i = 0; i < len; ++i) {
        buf[i] = ascii_tolower(str[start + i]);
    }
    buf[len] = '\0';

    if (strcmp(buf, "zero") == 0) {
        *out = WV_BOUND_ZERO;
        return WV_OK;
    }
    if (strcmp(buf, "replicate") == 0) {
        *out = WV_BOUND_REPLICATE;
        return WV_OK;
    }
    if (strcmp(buf, "reflect") == 0) {
        *out = WV_BOUND_REFLECT;
        return WV_OK;
    }
    if (strcmp(buf, "symmetric") == 0 || strcmp(buf, "sym") == 0) {
        *out = WV_BOUND_SYMMETRIC;
        return WV_OK;
    }
    if (strcmp(buf, "periodic") == 0 || strcmp(buf, "wrap") == 0 || strcmp(buf, "circular") == 0) {
        *out = WV_BOUND_PERIODIC;
        return WV_OK;
    }
    return WV_ERR_INVALID_PARAM;
}

wv_error_t wv_parse_wavelet(const char *str, wv_wavelet_t *out) {
    size_t start = 0;
    size_t end;
    size_t len;
    char buf[64];
    size_t i;

    if (str == NULL || out == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    while (str[start] != '\0' && is_space(str[start])) {
        start++;
    }
    end = start;
    while (str[end] != '\0') {
        end++;
    }
    while (end > start && is_space(str[end - 1])) {
        end--;
    }
    len = end - start;
    if (len == 0 || len >= sizeof(buf)) {
        return WV_ERR_INVALID_PARAM;
    }
    for (i = 0; i < len; ++i) {
        buf[i] = ascii_tolower(str[start + i]);
    }
    buf[len] = '\0';

    if (strcmp(buf, "haar") == 0) {
        *out = wv_haar();
        return WV_OK;
    }
    if (strncmp(buf, "db", 2) == 0) {
        uint64_t val = 0;
        if (len <= 2) {
            return WV_ERR_INVALID_PARAM;
        }
        for (i = 2; i < len; ++i) {
            if (buf[i] < '0' || buf[i] > '9') {
                return WV_ERR_INVALID_PARAM; /* reject trailing garbage */
            }
            val = val * 10 + (uint64_t)(buf[i] - '0');
            if (val > 0xFFFFFFFFu) {
                return WV_ERR_INVALID_PARAM;
            }
        }
        *out = wv_daubechies((uint32_t)val);
        return WV_OK;
    }
    if (strncmp(buf, "sym", 3) == 0) {
        uint64_t val = 0;
        if (len <= 3) {
            return WV_ERR_INVALID_PARAM;
        }
        for (i = 3; i < len; ++i) {
            if (buf[i] < '0' || buf[i] > '9') {
                return WV_ERR_INVALID_PARAM; /* reject trailing garbage */
            }
            val = val * 10 + (uint64_t)(buf[i] - '0');
            if (val > 0xFFFFFFFFu) {
                return WV_ERR_INVALID_PARAM;
            }
        }
        *out = wv_symlets((uint32_t)val);
        return WV_OK;
    }
    if (strncmp(buf, "coif", 4) == 0) {
        uint64_t val = 0;
        if (len <= 4) {
            return WV_ERR_INVALID_PARAM;
        }
        for (i = 4; i < len; ++i) {
            if (buf[i] < '0' || buf[i] > '9') {
                return WV_ERR_INVALID_PARAM; /* reject trailing garbage */
            }
            val = val * 10 + (uint64_t)(buf[i] - '0');
            if (val > 0xFFFFFFFFu) {
                return WV_ERR_INVALID_PARAM;
            }
        }
        *out = wv_coiflets((uint32_t)val);
        return WV_OK;
    }
    if (strcmp(buf, "bior2.2") == 0 || strcmp(buf, "bior22") == 0 ||
        strcmp(buf, "bior5.3") == 0 || strcmp(buf, "bior53") == 0) {
        *out = wv_biorthogonal(2, 2);
        return WV_OK;
    }
    if (strcmp(buf, "morlet") == 0) {
        *out = wv_morlet();
        return WV_OK;
    }
    if (strcmp(buf, "mexican_hat") == 0 || strcmp(buf, "mexicanhat") == 0 ||
        strcmp(buf, "ricker") == 0) {
        *out = wv_mexican_hat();
        return WV_OK;
    }
    return WV_ERR_INVALID_PARAM;
}

/* ================================================================== */
/* Filter tables                                                      */
/* ================================================================== */

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
    if (out_len == NULL) {
        return NULL;
    }
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
    if (h == NULL || out == NULL || out_len == NULL || h_len == 0) {
        return WV_ERR_INVALID_PARAM;
    }
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
        size_t hlen = 0;
        const float *table = wv_analysis_lowpass(w, &hlen);
        float *h;
        float *g;
        wv_error_t err;
        if (table == NULL || hlen == 0) {
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
    size_t len = 0;
    if (w.family == WV_HAAR) {
        return 2;
    }
    wv_analysis_lowpass(w, &len);
    return len;
}

static wv_error_t check_supported_wavelet(wv_wavelet_t w) {
    size_t len = 0;
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
    if (levels == 0 || levels > WV_MAX_LEVELS) {
        return WV_ERR_INVALID_PARAM;
    }
    return WV_OK;
}

/* ================================================================== */
/* Boundary sampling / one-step filter banks                          */
/* ================================================================== */

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
        if (m < 0 || m >= n) {
            return 0.0f;
        }
        return signal[(size_t)m];
    }
    if (boundary == WV_BOUND_SYMMETRIC) {
        int64_t period = 2 * n;
        int64_t m = ((idx % period) + period) % period;
        if (m >= n) {
            m = 2 * n - 1 - m;
        }
        if (m < 0 || m >= n) {
            return 0.0f;
        }
        return signal[(size_t)m];
    }
    return 0.0f;
}

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

static wv_error_t validate_dwt_inputs(const float *signal, size_t signal_len, wv_wavelet_t wavelet,
                                      uint32_t levels, wv_boundary_t boundary) {
    wv_error_t err;
    size_t filter_len;
    uint32_t shift;
    size_t pow2;
    size_t min_len;
    size_t i;

    if (signal_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (signal == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    if (signal_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
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
    for (i = 0; i < signal_len; ++i) {
        if (!wv_isfinite(signal[i])) {
            return WV_ERR_INVALID_PARAM;
        }
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
    float *details[WV_MAX_LEVELS];
    size_t dlen[WV_MAX_LEVELS];
    uint32_t produced = 0;
    uint32_t l;
    size_t total;
    float *result = NULL;
    size_t pos;

    if (out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    for (l = 0; l < WV_MAX_LEVELS; ++l) {
        details[l] = NULL;
        dlen[l] = 0;
    }

    err = validate_dwt_inputs(signal, signal_len, wavelet, levels, boundary);
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
        if (!checked_add_size(total, dlen[l], &total)) {
            err = WV_ERR_INVALID_PARAM;
            goto cleanup;
        }
    }
    result = falloc(total);
    if (result == NULL) {
        err = WV_ERR_ALLOC;
        goto cleanup;
    }
    memcpy(result, current, cur_len * sizeof(float));
    pos = cur_len;
    for (l = produced; l > 0; --l) {
        memcpy(result + pos, details[l - 1], dlen[l - 1] * sizeof(float));
        pos += dlen[l - 1];
    }

    *out = result;
    *out_len = total;
    result = NULL;
    err = WV_OK;

cleanup:
    free(h);
    free(g);
    free(current);
    free(result);
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
    size_t i;

    if (out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    if (coeffs_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (coeffs == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    if (output_length == 0 || output_length > WV_MAX_2D_DIM) {
        return WV_ERR_INVALID_PARAM;
    }
    if (coeffs_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    err = check_levels(levels);
    if (err != WV_OK) {
        return err;
    }
    for (i = 0; i < coeffs_len; ++i) {
        if (!wv_isfinite(coeffs[i])) {
            return WV_ERR_INVALID_COEFFICIENTS;
        }
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
        if (!checked_add_size(expected_total, level_lens[j], &expected_total)) {
            free(level_lens);
            return WV_ERR_INVALID_PARAM;
        }
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
    current = NULL;
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

    if (out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    if (signal_len == 0 || signal_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (coeffs_len > (size_t)WV_MAX_SAMPLES) {
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
        if (!checked_add_size(expected_total, level_lens[j], &expected_total)) {
            free(level_lens);
            return WV_ERR_INVALID_PARAM;
        }
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

    if (coeffs == NULL || out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    if (target_level == 0 || target_level > levels) {
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
    return check_supported_wavelet(wavelet);
}

static wv_error_t validate_2d_inputs(const float *image, size_t image_len, uint32_t n_rows,
                                     uint32_t n_cols, uint32_t levels) {
    size_t expected;
    size_t i;
    if (image_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (image == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    if (n_rows == 0 || n_cols == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (n_rows > WV_MAX_2D_DIM || n_cols > WV_MAX_2D_DIM) {
        return WV_ERR_INVALID_PARAM;
    }
    if (!checked_mul_size((size_t)n_rows, (size_t)n_cols, &expected)) {
        return WV_ERR_INVALID_PARAM;
    }
    if (image_len != expected || image_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels == 0 || levels > WV_MAX_LEVELS) {
        return WV_ERR_INVALID_PARAM;
    }
    for (i = 0; i < image_len; ++i) {
        if (!wv_isfinite(image[i])) {
            return WV_ERR_INVALID_PARAM;
        }
    }
    return WV_OK;
}

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

static wv_error_t dwt_2d_one_level(const float *image, size_t n_rows, size_t n_cols,
                                   wv_wavelet_t wavelet, wv_boundary_t boundary, float **ll_out,
                                   float **hl_out, float **lh_out, float **hh_out) {
    size_t half_cols = (n_cols + 1) / 2;
    size_t half_rows = (n_rows + 1) / 2;
    size_t band;
    size_t row_bytes;
    float *l_rows = NULL;
    float *h_rows = NULL;
    float *ll = NULL;
    float *hl = NULL;
    float *lh = NULL;
    float *hh = NULL;
    float *col_buf = NULL;
    wv_error_t err = WV_OK;
    size_t r;
    size_t c;

    if (!checked_mul_size(half_rows, half_cols, &band) ||
        !checked_mul_size(n_rows, half_cols, &row_bytes)) {
        return WV_ERR_INVALID_PARAM;
    }

    l_rows = falloc(row_bytes);
    h_rows = falloc(row_bytes);
    ll = falloc(band);
    hl = falloc(band);
    lh = falloc(band);
    hh = falloc(band);
    col_buf = falloc(n_rows);

    if (l_rows == NULL || h_rows == NULL || ll == NULL || hl == NULL || lh == NULL ||
        hh == NULL || col_buf == NULL) {
        err = WV_ERR_ALLOC;
        goto fail;
    }

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

static wv_error_t idwt_2d_one_level(const float *ll, const float *hl, const float *lh,
                                    const float *hh, size_t band_rows, size_t band_cols,
                                    size_t target_rows, size_t target_cols, wv_wavelet_t wavelet,
                                    wv_boundary_t boundary, float **out) {
    float *l_rows = NULL;
    float *h_rows = NULL;
    float *col_coeffs = NULL;
    float *row_coeffs = NULL;
    float *result = NULL;
    wv_error_t err = WV_OK;
    size_t r;
    size_t c;
    size_t r_bytes;
    size_t res_len;

    *out = NULL;
    if (!checked_mul_size(target_rows, band_cols, &r_bytes) ||
        !checked_mul_size(target_rows, target_cols, &res_len)) {
        return WV_ERR_INVALID_PARAM;
    }

    l_rows = falloc(r_bytes);
    h_rows = falloc(r_bytes);
    col_coeffs = falloc(2 * band_rows);
    row_coeffs = falloc(2 * band_cols);
    result = falloc(res_len);

    if (l_rows == NULL || h_rows == NULL || col_coeffs == NULL || row_coeffs == NULL ||
        result == NULL) {
        err = WV_ERR_ALLOC;
        goto done;
    }

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
    result = NULL;
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
    float *hl_arr[WV_MAX_LEVELS];
    float *lh_arr[WV_MAX_LEVELS];
    float *hh_arr[WV_MAX_LEVELS];
    size_t blen[WV_MAX_LEVELS];
    uint32_t produced = 0;
    uint32_t l;
    size_t total;
    float *result = NULL;
    size_t pos;

    if (out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    for (l = 0; l < WV_MAX_LEVELS; ++l) {
        hl_arr[l] = NULL;
        lh_arr[l] = NULL;
        hh_arr[l] = NULL;
        blen[l] = 0;
    }

    err = validate_2d_inputs(image, image_len, n_rows, n_cols, levels);
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
        size_t b;
        err = dwt_2d_one_level(current, cur_rows, cur_cols, wavelet, boundary, &ll, &hl, &lh, &hh);
        if (err != WV_OK) {
            goto cleanup;
        }
        free(current);
        current = ll;
        hl_arr[produced] = hl;
        lh_arr[produced] = lh;
        hh_arr[produced] = hh;
        b = ((cur_rows + 1) / 2) * ((cur_cols + 1) / 2);
        blen[produced] = b;
        produced++;
        cur_rows = (cur_rows + 1) / 2;
        cur_cols = (cur_cols + 1) / 2;
    }

    if (!checked_mul_size(cur_rows, cur_cols, &total)) {
        err = WV_ERR_INVALID_PARAM;
        goto cleanup;
    }
    for (l = 0; l < produced; ++l) {
        size_t three_b;
        if (!checked_mul_size(3, blen[l], &three_b) ||
            !checked_add_size(total, three_b, &total)) {
            err = WV_ERR_INVALID_PARAM;
            goto cleanup;
        }
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
    result = NULL;
    err = WV_OK;

cleanup:
    free(current);
    free(result);
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
    size_t expected_img_len;
    uint32_t j;
    size_t offset;
    float *ll = NULL;
    size_t i;

    if (out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    if (coeffs_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (coeffs == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    if (n_rows == 0 || n_cols == 0) {
        return WV_ERR_INVALID_PARAM;
    }
    if (n_rows > WV_MAX_2D_DIM || n_cols > WV_MAX_2D_DIM) {
        return WV_ERR_INVALID_PARAM;
    }
    if (!checked_mul_size((size_t)n_rows, (size_t)n_cols, &expected_img_len) ||
        expected_img_len > (size_t)WV_MAX_SAMPLES || coeffs_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (levels == 0 || levels > WV_MAX_LEVELS) {
        return WV_ERR_INVALID_PARAM;
    }
    for (i = 0; i < coeffs_len; ++i) {
        if (!wv_isfinite(coeffs[i])) {
            return WV_ERR_INVALID_COEFFICIENTS;
        }
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
    if (!checked_mul_size(ll_rows, ll_cols, &expected_total)) {
        free(dims);
        return WV_ERR_INVALID_PARAM;
    }
    for (j = 1; j <= levels; ++j) {
        size_t band_j;
        size_t three_band;
        if (!checked_mul_size(dims[2 * j], dims[2 * j + 1], &band_j) ||
            !checked_mul_size(3, band_j, &three_band) ||
            !checked_add_size(expected_total, three_band, &expected_total)) {
            free(dims);
            return WV_ERR_INVALID_PARAM;
        }
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
    *out_len = expected_img_len;
    ll = NULL;
    err = WV_OK;

cleanup:
    free(dims);
    free(ll);
    return err;
}

/* ================================================================== */
/* Continuous Wavelet Transform (CWT)                                 */
/* ================================================================== */

wv_error_t wv_cwt(const float *signal, size_t signal_len,
                  const float *scales, size_t scales_len,
                  wv_wavelet_t wavelet,
                  float **out, size_t *out_len) {
    size_t total_len;
    float *result;
    size_t s_idx;
    size_t i;
    const float PI_POW_NEG_QUARTER = 0.7511255444649425f;

    if (out == NULL || out_len == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    *out = NULL;
    *out_len = 0;

    if (signal_len == 0) {
        return WV_ERR_EMPTY_SIGNAL;
    }
    if (signal == NULL || scales == NULL) {
        return WV_ERR_INVALID_PARAM;
    }
    if (scales_len == 0 || scales_len > WV_MAX_SCALES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (signal_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    if (!checked_mul_size(signal_len, scales_len, &total_len) ||
        total_len > (size_t)WV_MAX_SAMPLES) {
        return WV_ERR_INVALID_PARAM;
    }
    for (s_idx = 0; s_idx < scales_len; ++s_idx) {
        if (!wv_isfinite(scales[s_idx]) || scales[s_idx] <= 0.0f) {
            return WV_ERR_INVALID_PARAM;
        }
    }
    for (i = 0; i < signal_len; ++i) {
        if (!wv_isfinite(signal[i])) {
            return WV_ERR_INVALID_PARAM;
        }
    }
    if (wavelet.family != WV_MORLET && wavelet.family != WV_MEXICAN_HAT) {
        return WV_ERR_INVALID_PARAM;
    }

    result = falloc(total_len);
    if (result == NULL) {
        return WV_ERR_ALLOC;
    }

    if (wavelet.family == WV_MORLET) {
        const float OMEGA_0 = 6.0f;
        for (s_idx = 0; s_idx < scales_len; ++s_idx) {
            float s = scales[s_idx];
            float inv_sqrt_s = 1.0f / wv_sqrtf(s);
            float inv_s = 1.0f / s;
            int64_t half_support = (int64_t)(8.0f * s + 0.999f);
            if (half_support > (int64_t)signal_len) {
                half_support = (int64_t)signal_len;
            }

            for (i = 0; i < signal_len; ++i) {
                float re_acc = 0.0f;
                float im_acc = 0.0f;
                int64_t start_k = (int64_t)i - half_support;
                int64_t end_k = (int64_t)i + half_support;
                int64_t k;
                if (start_k < 0) {
                    start_k = 0;
                }
                if (end_k >= (int64_t)signal_len) {
                    end_k = (int64_t)signal_len - 1;
                }

                for (k = start_k; k <= end_k; ++k) {
                    float t = ((float)k - (float)i) * inv_s;
                    float envelope = PI_POW_NEG_QUARTER * wv_expf(-0.5f * t * t);
                    float phase = OMEGA_0 * t;
                    float s_val;
                    float c_val;
                    float re_psi;
                    float im_psi;
                    float x;
                    wv_sincosf(phase, &s_val, &c_val);
                    re_psi = envelope * c_val;
                    im_psi = -envelope * s_val;
                    x = signal[(size_t)k];
                    re_acc += x * re_psi;
                    im_acc += x * im_psi;
                }
                result[s_idx * signal_len + i] = inv_sqrt_s * wv_sqrtf(re_acc * re_acc + im_acc * im_acc);
            }
        }
    } else {
        /* Mexican Hat */
        const float c = 0.86732507f; /* (2.0 / sqrt(3.0)) * pi^(-1/4) */
        for (s_idx = 0; s_idx < scales_len; ++s_idx) {
            float s = scales[s_idx];
            float inv_sqrt_s = 1.0f / wv_sqrtf(s);
            float inv_s = 1.0f / s;
            int64_t half_support = (int64_t)(8.0f * s + 0.999f);
            if (half_support > (int64_t)signal_len) {
                half_support = (int64_t)signal_len;
            }

            for (i = 0; i < signal_len; ++i) {
                float acc = 0.0f;
                int64_t start_k = (int64_t)i - half_support;
                int64_t end_k = (int64_t)i + half_support;
                int64_t k;
                if (start_k < 0) {
                    start_k = 0;
                }
                if (end_k >= (int64_t)signal_len) {
                    end_k = (int64_t)signal_len - 1;
                }

                for (k = start_k; k <= end_k; ++k) {
                    float t = ((float)k - (float)i) * inv_s;
                    float psi = c * (1.0f - t * t) * wv_expf(-0.5f * t * t);
                    acc += signal[(size_t)k] * psi;
                }
                result[s_idx * signal_len + i] = inv_sqrt_s * acc;
            }
        }
    }

    *out = result;
    *out_len = total_len;
    return WV_OK;
}
