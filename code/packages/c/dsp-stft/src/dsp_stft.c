/*
 * dsp_stft.c — Short-Time Fourier Transform, ISTFT, Spectrograms, Mel & MFCC
 *              (pure ISO C17).
 *
 * Hardened for VER15 & Zero-CVE:
 * - Zero <math.h> / -lm: pure IEEE-754 double-precision sqrt, sincos, exp, ln.
 * - Structural/length/cap bounds checked BEFORE element finiteness scans
 *   (prevents CWE-125 out-of-bounds reads).
 * - Double-precision intermediates with |x| <= (double)FLT_MAX guard prior
 *   to float narrowing (no ISO C17 float-cast-overflow UB).
 * - Upfront enum domain validation, non-finite input AND output rejection.
 * - Checked size_t math (CWE-190 / CWE-400), heap-allocated large buffers.
 * - Zero-on-free via volatile unsigned char * (CWE-415 / CWE-416).
 * - Internal O(N log N) radix-2 + Bluestein FFT and O(N log N) Makhoul DCT-II.
 */
#include "dsp_stft.h"

#include <float.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ================================================================== */
/* Checked arithmetic & safe memory helpers (CWE-190/400/415/416)     */
/* ================================================================== */

static int checked_add_size(size_t a, size_t b, size_t *out) {
    if (SIZE_MAX - a < b) {
        return 0;
    }
    *out = a + b;
    return 1;
}

static int checked_mul_size(size_t a, size_t b, size_t *out) {
    if (a != 0 && b > SIZE_MAX / a) {
        return 0;
    }
    *out = a * b;
    return 1;
}

static void zero_bytes_volatile(void *ptr, size_t byte_len) {
    volatile unsigned char *vp;
    size_t i;
    if (ptr == NULL || byte_len == 0) {
        return;
    }
    vp = (volatile unsigned char *)ptr;
    for (i = 0; i < byte_len; ++i) {
        vp[i] = 0;
    }
}

static float *alloc_floats(size_t n) {
    size_t count = n == 0 ? 1 : n;
    size_t bytes = 0;
    if (!checked_mul_size(count, sizeof(float), &bytes)) {
        return NULL;
    }
    return (float *)calloc(count, sizeof(float));
}

static double *alloc_doubles(size_t n) {
    size_t count = n == 0 ? 1 : n;
    size_t bytes = 0;
    if (!checked_mul_size(count, sizeof(double), &bytes)) {
        return NULL;
    }
    return (double *)calloc(count, sizeof(double));
}

static void zero_free_floats(float **p, size_t n) {
    size_t bytes = 0;
    if (p == NULL || *p == NULL) {
        return;
    }
    if (n > 0 && checked_mul_size(n, sizeof(float), &bytes)) {
        zero_bytes_volatile(*p, bytes);
    }
    free(*p);
    *p = NULL;
}

static void zero_free_doubles(double **p, size_t n) {
    size_t bytes = 0;
    if (p == NULL || *p == NULL) {
        return;
    }
    if (n > 0 && checked_mul_size(n, sizeof(double), &bytes)) {
        zero_bytes_volatile(*p, bytes);
    }
    free(*p);
    *p = NULL;
}

void stft_free(void *p) {
    free(p);
}

void stft_free_buf(float **p, size_t len) {
    zero_free_floats(p, len);
}

/* ================================================================== */
/* Pure ISO IEEE-754 float/double finiteness & math (zero <math.h>)   */
/* ================================================================== */

static int f_isfinite(float x) {
    return (x == x) && ((x - x) == 0.0f) && (x <= FLT_MAX) && (x >= -FLT_MAX);
}

static int d_isfinite(double x) {
    return (x == x) && ((x - x) == 0.0) && (x <= DBL_MAX) && (x >= -DBL_MAX);
}

static int narrow_f32(double x, float *out) {
    float f;
    if (!d_isfinite(x)) {
        return 0;
    }
    if (x > (double)FLT_MAX || x < -(double)FLT_MAX) {
        return 0;
    }
    f = (float)x;
    if (!f_isfinite(f)) {
        return 0;
    }
    *out = f;
    return 1;
}

static double d_sqrt(double x) {
    double m;
    double scale = 1.0;
    double g;
    int iter;
    if (!d_isfinite(x) || x <= 0.0) {
        return 0.0;
    }
    m = x;
    while (m < 0.25) {
        m *= 4.0;
        scale *= 0.5;
    }
    while (m >= 1.0) {
        m *= 0.25;
        scale *= 2.0;
    }
    g = 0.5 * (1.0 + m);
    for (iter = 0; iter < 10; ++iter) {
        g = 0.5 * (g + m / g);
    }
    return g * scale;
}

static double pow2i_d(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    unsigned int n = k < 0 ? (unsigned int)(-(k + 1)) + 1u : (unsigned int)k;
    while (n > 0u) {
        if (n & 1u) {
            result *= base;
        }
        n >>= 1;
        if (n > 0u) {
            base *= base;
        }
    }
    return result;
}

static double d_exp(double x) {
    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375;
    const double C2 = -2.1219444005469058277e-4;
    double kf;
    int k;
    double r;
    double term = 1.0;
    double sum = 1.0;
    int i;

    if (x != x) {
        return x;
    }
    if (x == 0.0) {
        return 1.0;
    }
    if (x > 709.782712893384) {
        return DBL_MAX;
    }
    if (x < -745.13321910194) {
        return 0.0;
    }
    kf = x * INV_LN2;
    k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    r = (x - (double)k * C1) - (double)k * C2;
    for (i = 1; i <= 17; ++i) {
        term *= r / (double)i;
        sum += term;
    }
    if (k >= 1024) {
        double v = (sum * pow2i_d(1023)) * 2.0;
        return d_isfinite(v) ? v : DBL_MAX;
    }
    return sum * pow2i_d(k);
}

static double d_ln(double x) {
    const double SQRT_HALF = 0.70710678118654752440;
    const double SQRT_TWO = 1.41421356237309504880;
    const double LN2 = 0.69314718055994530942;
    double m;
    int k = 0;
    double u;
    double u2;
    double term;
    double sum = 0.0;
    int i;

    if (!d_isfinite(x) || x <= 0.0) {
        return 0.0;
    }
    m = x;
    while (m < SQRT_HALF) {
        m *= 2.0;
        --k;
    }
    while (m >= SQRT_TWO) {
        m *= 0.5;
        ++k;
    }
    u = (m - 1.0) / (m + 1.0);
    u2 = u * u;
    term = u;
    for (i = 0; i < 14; ++i) {
        sum += term / (double)(2 * i + 1);
        term *= u2;
    }
    return 2.0 * sum + (double)k * LN2;
}

static double d_log10(double x) {
    const double INV_LN10 = 0.43429448190325182765;
    return d_ln(x) * INV_LN10;
}

static double d_pow10(double x) {
    const double LN10 = 2.30258509299404568402;
    if (x > 308.2547155599167) {
        return DBL_MAX;
    }
    return d_exp(x * LN10);
}

static void d_sincos(double x, double *sin_out, double *cos_out) {
    const double PI_HALF = 1.57079632679489661923;
    const double INV_PI_HALF = 0.63661977236758134308;
    const double P1 = 1.57079632673412561417;
    const double P2 = 6.07710050650619224932e-11;
    const double TWO_PI = 6.28318530717958647693;
    double qf;
    int64_t q;
    double r;
    double r2;
    double s_term, s_sum;
    double c_term, c_sum;
    double s_val, c_val;
    int n;

    if (!d_isfinite(x)) {
        *sin_out = 0.0;
        *cos_out = 1.0;
        return;
    }
    if (x > 9.0e15 || x < -9.0e15) {
        *sin_out = 0.0;
        *cos_out = 1.0;
        return;
    }
    if (x > 1e12 || x < -1e12) {
        double k_big = (double)((int64_t)(x / TWO_PI));
        x -= k_big * TWO_PI;
    }
    qf = x * INV_PI_HALF;
    q = (int64_t)(qf >= 0.0 ? qf + 0.5 : qf - 0.5);
    r = (x - (double)q * P1) - (double)q * P2;
    if (r > PI_HALF) {
        r -= PI_HALF;
        q += 1;
    } else if (r < -PI_HALF) {
        r += PI_HALF;
        q -= 1;
    }
    r2 = r * r;
    s_term = r;
    s_sum = r;
    c_term = 1.0;
    c_sum = 1.0;
    for (n = 1; n <= 11; ++n) {
        s_term *= -r2 / (double)((2 * n) * (2 * n + 1));
        s_sum += s_term;
        c_term *= -r2 / (double)((2 * n - 1) * (2 * n));
        c_sum += c_term;
    }
    switch ((unsigned int)(q & 3)) {
        case 0u: s_val = s_sum; c_val = c_sum; break;
        case 1u: s_val = c_sum; c_val = -s_sum; break;
        case 2u: s_val = -s_sum; c_val = -c_sum; break;
        default: s_val = -c_sum; c_val = s_sum; break;
    }
    *sin_out = s_val;
    *cos_out = c_val;
}

/* ================================================================== */
/* Validation & Out-Pointer Helpers                                   */
/* ================================================================== */

static void init_out(float **out, size_t *out_len) {
    if (out != NULL) {
        *out = NULL;
    }
    if (out_len != NULL) {
        *out_len = 0;
    }
}

static int is_valid_window(stft_window_t w) {
    return w == STFT_WIN_HANN || w == STFT_WIN_HAMMING ||
           w == STFT_WIN_BLACKMAN || w == STFT_WIN_RECTANGULAR;
}

static int validate_float_array(const float *buf, size_t len) {
    size_t i;
    if (buf == NULL && len > 0) {
        return 0;
    }
    for (i = 0; i < len; ++i) {
        if (!f_isfinite(buf[i])) {
            return 0;
        }
    }
    return 1;
}

/* ================================================================== */
/* Window Parsing & Generation                                        */
/* ================================================================== */

static char ascii_tolower(char c) {
    if (c >= 'A' && c <= 'Z') {
        return (char)(c + ('a' - 'A'));
    }
    return c;
}

static int is_ascii_space(char c) {
    return c == ' ' || c == '\t' || c == '\r' || c == '\n';
}

stft_error_t stft_parse_window(const char *str, stft_window_t *out) {
    size_t len = 0;
    size_t start = 0;
    size_t end;
    size_t tok_len;
    char tok[32];
    size_t i;

    if (str == NULL || out == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    while (len <= 64u && str[len] != '\0') {
        ++len;
    }
    if (len == 0 || len > 64u) {
        return STFT_ERR_INVALID_PARAM;
    }
    while (start < len && is_ascii_space(str[start])) {
        ++start;
    }
    end = len;
    while (end > start && is_ascii_space(str[end - 1u])) {
        --end;
    }
    tok_len = end - start;
    if (tok_len == 0 || tok_len >= sizeof(tok)) {
        return STFT_ERR_INVALID_PARAM;
    }
    for (i = 0; i < tok_len; ++i) {
        tok[i] = ascii_tolower(str[start + i]);
    }
    tok[tok_len] = '\0';

    if (strcmp(tok, "hann") == 0 || strcmp(tok, "hanning") == 0) {
        *out = STFT_WIN_HANN;
        return STFT_OK;
    }
    if (strcmp(tok, "hamming") == 0) {
        *out = STFT_WIN_HAMMING;
        return STFT_OK;
    }
    if (strcmp(tok, "blackman") == 0) {
        *out = STFT_WIN_BLACKMAN;
        return STFT_OK;
    }
    if (strcmp(tok, "rectangular") == 0 || strcmp(tok, "rect") == 0 ||
        strcmp(tok, "boxcar") == 0) {
        *out = STFT_WIN_RECTANGULAR;
        return STFT_OK;
    }
    return STFT_ERR_INVALID_PARAM;
}

static stft_error_t fill_window_d(stft_window_t window, size_t n_fft, double *win) {
    const double TWO_PI = 6.28318530717958647693;
    const double FOUR_PI = 12.56637061435917295385;
    size_t n;
    double m;

    if (!is_valid_window(window) || n_fft == 0 || n_fft > STFT_MAX_FFT_LEN) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (n_fft == 1u) {
        win[0] = 1.0;
        return STFT_OK;
    }
    m = (double)(n_fft - 1u);
    for (n = 0; n < n_fft; ++n) {
        double nn = (double)n;
        if (window == STFT_WIN_RECTANGULAR) {
            win[n] = 1.0;
        } else {
            double s1 = 0.0, c1 = 1.0;
            d_sincos(TWO_PI * nn / m, &s1, &c1);
            if (window == STFT_WIN_HANN) {
                win[n] = 0.5 * (1.0 - c1);
            } else if (window == STFT_WIN_HAMMING) {
                win[n] = 0.54 - 0.46 * c1;
            } else {
                double s2 = 0.0, c2 = 1.0;
                d_sincos(FOUR_PI * nn / m, &s2, &c2);
                win[n] = 0.42 - 0.5 * c1 + 0.08 * c2;
            }
        }
    }
    return STFT_OK;
}

stft_error_t stft_build_window(stft_window_t window, uint32_t n_fft,
                               float **out, size_t *out_len) {
    size_t nf = (size_t)n_fft;
    double *win_d = NULL;
    float *res = NULL;
    size_t i;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (n_fft == 0 || nf > STFT_MAX_FFT_LEN || !is_valid_window(window)) {
        return STFT_ERR_INVALID_PARAM;
    }
    win_d = alloc_doubles(nf);
    res = alloc_floats(nf);
    if (win_d == NULL || res == NULL) {
        zero_free_doubles(&win_d, nf);
        zero_free_floats(&res, nf);
        return STFT_ERR_ALLOC;
    }
    err = fill_window_d(window, nf, win_d);
    if (err != STFT_OK) {
        zero_free_doubles(&win_d, nf);
        zero_free_floats(&res, nf);
        return err;
    }
    for (i = 0; i < nf; ++i) {
        float f = 0.0f;
        if (!narrow_f32(win_d[i], &f)) {
            zero_free_doubles(&win_d, nf);
            zero_free_floats(&res, nf);
            return STFT_ERR_INVALID_PARAM;
        }
        res[i] = f;
    }
    zero_free_doubles(&win_d, nf);
    *out = res;
    *out_len = nf;
    return STFT_OK;
}

stft_error_t stft_hann_window(uint32_t length, float **out, size_t *out_len) {
    return stft_build_window(STFT_WIN_HANN, length, out, out_len);
}
stft_error_t stft_hamming_window(uint32_t length, float **out, size_t *out_len) {
    return stft_build_window(STFT_WIN_HAMMING, length, out, out_len);
}
stft_error_t stft_blackman_window(uint32_t length, float **out, size_t *out_len) {
    return stft_build_window(STFT_WIN_BLACKMAN, length, out, out_len);
}
stft_error_t stft_rectangular_window(uint32_t length, float **out, size_t *out_len) {
    return stft_build_window(STFT_WIN_RECTANGULAR, length, out, out_len);
}

/* ================================================================== */
/* Internal O(N log N) Radix-2 + Bluestein FFT / RFFT / IRFFT         */
/* ================================================================== */

static int is_pow2(size_t n) {
    return n >= 1u && (n & (n - 1u)) == 0u;
}

static size_t next_pow2_checked(size_t n, int *ok) {
    size_t p = 1u;
    *ok = 1;
    if (n <= 1u) {
        return 1u;
    }
    while (p < n) {
        if (p > (SIZE_MAX >> 1)) {
            *ok = 0;
            return 0;
        }
        p <<= 1;
    }
    return p;
}

static void fft_bit_reverse(double *buf, size_t n) {
    size_t log_n = 0;
    size_t tmp = n;
    size_t i;
    while (tmp > 1u) {
        tmp >>= 1;
        ++log_n;
    }
    for (i = 0; i < n; ++i) {
        size_t x = i;
        size_t r = 0;
        size_t b;
        for (b = 0; b < log_n; ++b) {
            r = (r << 1) | (x & 1u);
            x >>= 1;
        }
        if (r > i) {
            double tr = buf[2u * i];
            double ti = buf[2u * i + 1u];
            buf[2u * i] = buf[2u * r];
            buf[2u * i + 1u] = buf[2u * r + 1u];
            buf[2u * r] = tr;
            buf[2u * r + 1u] = ti;
        }
    }
}

static void fft_radix2_d(double *buf, size_t n, int inverse) {
    const double PI = 3.14159265358979323846;
    double sign = inverse ? 1.0 : -1.0;
    size_t full = 2u;

    fft_bit_reverse(buf, n);
    while (full <= n) {
        size_t half = full >> 1;
        size_t block_start;
        for (block_start = 0; block_start < n; block_start += full) {
            size_t j;
            for (j = 0; j < half; ++j) {
                double theta = sign * 2.0 * PI * (double)j / (double)full;
                double w_im = 0.0, w_re = 1.0;
                size_t x_idx = 2u * (block_start + j);
                size_t y_idx = 2u * (block_start + j + half);
                double x_re = buf[x_idx], x_im = buf[x_idx + 1u];
                double y_re = buf[y_idx], y_im = buf[y_idx + 1u];
                double t_re, t_im;
                d_sincos(theta, &w_im, &w_re);
                t_re = w_re * y_re - w_im * y_im;
                t_im = w_re * y_im + w_im * y_re;
                buf[x_idx] = x_re + t_re;
                buf[x_idx + 1u] = x_im + t_im;
                buf[y_idx] = x_re - t_re;
                buf[y_idx + 1u] = x_im - t_im;
            }
        }
        full <<= 1;
    }
    if (inverse) {
        double inv_n = 1.0 / (double)n;
        size_t k;
        for (k = 0; k < 2u * n; ++k) {
            buf[k] *= inv_n;
        }
    }
}

static stft_error_t check_bluestein_m(size_t n, size_t *out_m) {
    size_t two_n = 0;
    size_t conv_len = 0;
    int ok = 0;
    size_t m = 0;
    if (!checked_mul_size(n, 2u, &two_n) || two_n == 0u) {
        return STFT_ERR_FFT;
    }
    conv_len = two_n - 1u;
    m = next_pow2_checked(conv_len, &ok);
    if (!ok || m > STFT_MAX_FFT_LEN) {
        return STFT_ERR_FFT;
    }
    *out_m = m;
    return STFT_OK;
}

static stft_error_t fft_complex_d(double *buf, size_t n, int inverse) {
    const double PI = 3.14159265358979323846;
    size_t m = 0;
    double *a = NULL;
    double *b = NULL;
    double sign = inverse ? 1.0 : -1.0;
    uint64_t mod;
    size_t k;
    stft_error_t err;

    if (n == 0 || n > STFT_MAX_FFT_LEN) {
        return STFT_ERR_FFT;
    }
    if (n == 1u) {
        return STFT_OK;
    }
    if (is_pow2(n)) {
        fft_radix2_d(buf, n, inverse);
        return STFT_OK;
    }

    err = check_bluestein_m(n, &m);
    if (err != STFT_OK) {
        return err;
    }
    a = alloc_doubles(2u * m);
    b = alloc_doubles(2u * m);
    if (a == NULL || b == NULL) {
        zero_free_doubles(&a, 2u * m);
        zero_free_doubles(&b, 2u * m);
        return STFT_ERR_ALLOC;
    }

    mod = 2u * (uint64_t)n;
    for (k = 0; k < n; ++k) {
        uint64_t sq = ((uint64_t)k * (uint64_t)k) % mod;
        double theta = sign * PI * (double)sq / (double)n;
        double sw = 0.0, cw = 1.0;
        double xr = buf[2u * k];
        double xi = buf[2u * k + 1u];
        d_sincos(theta, &sw, &cw);
        a[2u * k] = xr * cw - xi * sw;
        a[2u * k + 1u] = xr * sw + xi * cw;
        b[2u * k] = cw;
        b[2u * k + 1u] = -sw;
        if (k > 0u) {
            b[2u * (m - k)] = cw;
            b[2u * (m - k) + 1u] = -sw;
        }
    }

    fft_radix2_d(a, m, 0);
    fft_radix2_d(b, m, 0);

    for (k = 0; k < m; ++k) {
        double ar = a[2u * k], ai = a[2u * k + 1u];
        double br = b[2u * k], bi = b[2u * k + 1u];
        a[2u * k] = ar * br - ai * bi;
        a[2u * k + 1u] = ar * bi + ai * br;
    }
    fft_radix2_d(a, m, 1);

    for (k = 0; k < n; ++k) {
        uint64_t sq = ((uint64_t)k * (uint64_t)k) % mod;
        double theta = sign * PI * (double)sq / (double)n;
        double sw = 0.0, cw = 1.0;
        double cr = a[2u * k], ci = a[2u * k + 1u];
        double xr, xi;
        d_sincos(theta, &sw, &cw);
        xr = cr * cw - ci * sw;
        xi = cr * sw + ci * cw;
        if (inverse) {
            xr /= (double)n;
            xi /= (double)n;
        }
        buf[2u * k] = xr;
        buf[2u * k + 1u] = xi;
    }

    zero_free_doubles(&a, 2u * m);
    zero_free_doubles(&b, 2u * m);
    return STFT_OK;
}

/* ================================================================== */
/* Internal O(N log N) Makhoul DCT-II (Ortho)                         */
/* ================================================================== */

static stft_error_t dct_ii_ortho_d(const double *in, size_t n, double *out) {
    const double PI = 3.14159265358979323846;
    double *buf = NULL;
    size_t half_even, half_odd;
    size_t m, k;
    double s0, sk, two_n;
    stft_error_t err;

    if (n == 0 || n > STFT_MAX_FFT_LEN) {
        return STFT_ERR_INVALID_PARAM;
    }
    buf = alloc_doubles(2u * n);
    if (buf == NULL) {
        return STFT_ERR_ALLOC;
    }
    half_even = (n + 1u) / 2u;
    half_odd = n / 2u;
    for (m = 0; m < half_even; ++m) {
        buf[2u * m] = in[2u * m];
    }
    for (m = 0; m < half_odd; ++m) {
        buf[2u * (n - 1u - m)] = in[2u * m + 1u];
    }

    err = fft_complex_d(buf, n, 0);
    if (err != STFT_OK) {
        zero_free_doubles(&buf, 2u * n);
        return err;
    }

    s0 = 1.0 / d_sqrt((double)n);
    sk = d_sqrt(2.0 / (double)n);
    two_n = 2.0 * (double)n;
    for (k = 0; k < n; ++k) {
        double theta = PI * (double)k / two_n;
        double sin_t = 0.0, cos_t = 1.0;
        double scale = (k == 0u) ? s0 : sk;
        double val;
        d_sincos(theta, &sin_t, &cos_t);
        val = scale * (buf[2u * k] * cos_t + buf[2u * k + 1u] * sin_t);
        if (!d_isfinite(val)) {
            zero_free_doubles(&buf, 2u * n);
            return STFT_ERR_INVALID_PARAM;
        }
        out[k] = val;
    }

    zero_free_doubles(&buf, 2u * n);
    return STFT_OK;
}

/* ================================================================== */
/* STFT / ISTFT / COLA / Griffin-Lim                                  */
/* ================================================================== */

stft_error_t stft_forward(const float *signal, size_t signal_len,
                          uint32_t n_fft, uint32_t hop_length,
                          stft_window_t window,
                          float **out, size_t *out_len) {
    size_t nf = (size_t)n_fft;
    size_t hop = (size_t)hop_length;
    size_t num_frames = 0;
    size_t bins = 0;
    size_t frame_floats = 0;
    size_t total_out = 0;
    double *win_d = NULL;
    double *fft_buf = NULL;
    float *res = NULL;
    size_t m;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (signal_len == 0) {
        return STFT_ERR_EMPTY_SIGNAL;
    }
    if (signal == NULL || n_fft == 0 || hop_length == 0 ||
        !is_valid_window(window) || nf > STFT_MAX_FFT_LEN ||
        signal_len > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (signal_len < nf) {
        return STFT_ERR_SIGNAL_TOO_SHORT;
    }
    num_frames = 1u + (signal_len - nf) / hop;
    if (num_frames > STFT_MAX_FRAMES) {
        return STFT_ERR_INVALID_PARAM;
    }
    bins = nf / 2u + 1u;
    if (!checked_mul_size(bins, 2u, &frame_floats) ||
        !checked_mul_size(num_frames, frame_floats, &total_out) ||
        total_out > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (!is_pow2(nf)) {
        size_t dummy_m = 0;
        if (check_bluestein_m(nf, &dummy_m) != STFT_OK) {
            return STFT_ERR_FFT;
        }
    }
    if (!validate_float_array(signal, signal_len)) {
        return STFT_ERR_INVALID_PARAM;
    }

    win_d = alloc_doubles(nf);
    fft_buf = alloc_doubles(2u * nf);
    res = alloc_floats(total_out);
    if (win_d == NULL || fft_buf == NULL || res == NULL) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&fft_buf, 2u * nf);
        zero_free_floats(&res, total_out);
        return STFT_ERR_ALLOC;
    }

    err = fill_window_d(window, nf, win_d);
    if (err != STFT_OK) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&fft_buf, 2u * nf);
        zero_free_floats(&res, total_out);
        return err;
    }

    for (m = 0; m < num_frames; ++m) {
        size_t frame_start = m * hop;
        size_t out_offset = m * frame_floats;
        size_t k;
        for (k = 0; k < nf; ++k) {
            fft_buf[2u * k] = (double)signal[frame_start + k] * win_d[k];
            fft_buf[2u * k + 1u] = 0.0;
        }
        err = fft_complex_d(fft_buf, nf, 0);
        if (err != STFT_OK) {
            zero_free_doubles(&win_d, nf);
            zero_free_doubles(&fft_buf, 2u * nf);
            zero_free_floats(&res, total_out);
            return err;
        }
        for (k = 0; k < frame_floats; ++k) {
            float narrowed = 0.0f;
            if (!narrow_f32(fft_buf[k], &narrowed)) {
                zero_free_doubles(&win_d, nf);
                zero_free_doubles(&fft_buf, 2u * nf);
                zero_free_floats(&res, total_out);
                return STFT_ERR_FFT;
            }
            res[out_offset + k] = narrowed;
        }
    }

    zero_free_doubles(&win_d, nf);
    zero_free_doubles(&fft_buf, 2u * nf);
    *out = res;
    *out_len = total_out;
    return STFT_OK;
}

stft_error_t stft_istft(const float *spectrogram, size_t spec_len,
                        uint32_t n_fft, uint32_t hop_length,
                        stft_window_t window, uint32_t output_length,
                        float **out, size_t *out_len) {
    size_t nf = (size_t)n_fft;
    size_t hop = (size_t)hop_length;
    size_t olen = (size_t)output_length;
    size_t bins = 0;
    size_t frame_floats = 0;
    size_t num_frames = 0;
    size_t reflect_end = 0;
    double *win_d = NULL;
    double *fft_buf = NULL;
    double *acc = NULL;
    double *norm = NULL;
    float *res = NULL;
    size_t m, n;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (spec_len == 0) {
        return STFT_ERR_INVALID_SPECTROGRAM;
    }
    if (n_fft == 0 || hop_length == 0 || output_length == 0 ||
        !is_valid_window(window) || nf > STFT_MAX_FFT_LEN ||
        olen > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (spectrogram == NULL || spec_len > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    bins = nf / 2u + 1u;
    if (!checked_mul_size(bins, 2u, &frame_floats)) {
        return STFT_ERR_INVALID_PARAM;
    }
    if ((spec_len % frame_floats) != 0u) {
        return STFT_ERR_INVALID_SPECTROGRAM;
    }
    num_frames = spec_len / frame_floats;
    if (num_frames > STFT_MAX_FRAMES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (!is_pow2(nf)) {
        size_t dummy_m = 0;
        if (check_bluestein_m(nf, &dummy_m) != STFT_OK) {
            return STFT_ERR_FFT;
        }
    }
    if (!validate_float_array(spectrogram, spec_len)) {
        return STFT_ERR_INVALID_SPECTROGRAM;
    }

    win_d = alloc_doubles(nf);
    fft_buf = alloc_doubles(2u * nf);
    acc = alloc_doubles(olen);
    norm = alloc_doubles(olen);
    res = alloc_floats(olen);
    if (win_d == NULL || fft_buf == NULL || acc == NULL || norm == NULL || res == NULL) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&fft_buf, 2u * nf);
        zero_free_doubles(&acc, olen);
        zero_free_doubles(&norm, olen);
        zero_free_floats(&res, olen);
        return STFT_ERR_ALLOC;
    }

    err = fill_window_d(window, nf, win_d);
    if (err != STFT_OK) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&fft_buf, 2u * nf);
        zero_free_doubles(&acc, olen);
        zero_free_doubles(&norm, olen);
        zero_free_floats(&res, olen);
        return err;
    }

    reflect_end = (nf + 1u) / 2u;
    for (m = 0; m < num_frames; ++m) {
        size_t frame_start_global = 0;
        size_t frame_start_floats = m * frame_floats;
        size_t k;

        if (!checked_mul_size(m, hop, &frame_start_global)) {
            frame_start_global = olen;
        }
        zero_bytes_volatile(fft_buf, 2u * nf * sizeof(double));
        for (k = 0; k < frame_floats; ++k) {
            fft_buf[k] = (double)spectrogram[frame_start_floats + k];
        }
        for (k = 1u; k < reflect_end; ++k) {
            size_t dst = nf - k;
            fft_buf[2u * dst] = fft_buf[2u * k];
            fft_buf[2u * dst + 1u] = -fft_buf[2u * k + 1u];
        }
        err = fft_complex_d(fft_buf, nf, 1);
        if (err != STFT_OK) {
            zero_free_doubles(&win_d, nf);
            zero_free_doubles(&fft_buf, 2u * nf);
            zero_free_doubles(&acc, olen);
            zero_free_doubles(&norm, olen);
            zero_free_floats(&res, olen);
            return err;
        }
        for (k = 0; k < nf; ++k) {
            size_t idx = 0;
            double w;
            if (!checked_add_size(frame_start_global, k, &idx) || idx >= olen) {
                break;
            }
            w = win_d[k];
            acc[idx] += fft_buf[2u * k] * w;
            norm[idx] += w * w;
        }
    }

    for (n = 0; n < olen; ++n) {
        double val = norm[n] > 1e-10 ? (acc[n] / norm[n]) : 0.0;
        float narrowed = 0.0f;
        if (!narrow_f32(val, &narrowed)) {
            zero_free_doubles(&win_d, nf);
            zero_free_doubles(&fft_buf, 2u * nf);
            zero_free_doubles(&acc, olen);
            zero_free_doubles(&norm, olen);
            zero_free_floats(&res, olen);
            return STFT_ERR_INVALID_SPECTROGRAM;
        }
        res[n] = narrowed;
    }

    zero_free_doubles(&win_d, nf);
    zero_free_doubles(&fft_buf, 2u * nf);
    zero_free_doubles(&acc, olen);
    zero_free_doubles(&norm, olen);
    *out = res;
    *out_len = olen;
    return STFT_OK;
}

stft_error_t stft_check_ola(uint32_t n_fft, uint32_t hop_length,
                            stft_window_t window,
                            int *out_is_nola, int *out_is_cola,
                            float *out_min_norm, float *out_max_norm) {
    size_t nf = (size_t)n_fft;
    size_t hop = (size_t)hop_length;
    size_t sim_len = 0;
    size_t num_frames = 0;
    double *win_d = NULL;
    double *norm = NULL;
    size_t m, k, i;
    size_t start_check, end_check;
    double min_v = DBL_MAX;
    double max_v = 0.0;
    float f_min = 0.0f, f_max = 0.0f;
    stft_error_t err;

    if (out_is_nola != NULL) *out_is_nola = 0;
    if (out_is_cola != NULL) *out_is_cola = 0;
    if (out_min_norm != NULL) *out_min_norm = 0.0f;
    if (out_max_norm != NULL) *out_max_norm = 0.0f;

    if (n_fft == 0 || hop_length == 0 || nf > STFT_MAX_FFT_LEN ||
        !is_valid_window(window)) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (!checked_mul_size(nf, 4u, &sim_len) || sim_len > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    num_frames = 1u + (sim_len - nf) / hop;

    win_d = alloc_doubles(nf);
    norm = alloc_doubles(sim_len);
    if (win_d == NULL || norm == NULL) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&norm, sim_len);
        return STFT_ERR_ALLOC;
    }
    err = fill_window_d(window, nf, win_d);
    if (err != STFT_OK) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&norm, sim_len);
        return err;
    }

    for (m = 0; m < num_frames; ++m) {
        size_t fs = m * hop;
        for (k = 0; k < nf && (fs + k) < sim_len; ++k) {
            norm[fs + k] += win_d[k] * win_d[k];
        }
    }

    start_check = nf;
    end_check = sim_len - nf;
    if (end_check <= start_check) {
        start_check = 0u;
        end_check = sim_len;
    }
    for (i = start_check; i < end_check; ++i) {
        if (norm[i] < min_v) min_v = norm[i];
        if (norm[i] > max_v) max_v = norm[i];
    }

    if (!narrow_f32(min_v, &f_min) || !narrow_f32(max_v, &f_max)) {
        zero_free_doubles(&win_d, nf);
        zero_free_doubles(&norm, sim_len);
        return STFT_ERR_INVALID_PARAM;
    }

    if (out_min_norm != NULL) *out_min_norm = f_min;
    if (out_max_norm != NULL) *out_max_norm = f_max;
    if (out_is_nola != NULL) *out_is_nola = (min_v > 1e-10) ? 1 : 0;
    if (out_is_cola != NULL) {
        *out_is_cola = (min_v > 1e-10 && (max_v - min_v) <= 1e-3 * max_v) ? 1 : 0;
    }

    zero_free_doubles(&win_d, nf);
    zero_free_doubles(&norm, sim_len);
    return STFT_OK;
}

stft_error_t stft_griffin_lim(const float *mag_spec, size_t mag_len,
                              uint32_t n_fft, uint32_t hop_length,
                              stft_window_t window, uint32_t n_iter,
                              uint32_t output_length,
                              float **out, size_t *out_len) {
    size_t nf = (size_t)n_fft;
    size_t olen = (size_t)output_length;
    size_t bins = 0;
    size_t num_frames = 0;
    size_t spec_floats = 0;
    float *complex_spec = NULL;
    float *recon = NULL;
    size_t recon_len = 0;
    uint32_t iter;
    size_t i;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (mag_len == 0) {
        return STFT_ERR_INVALID_SPECTROGRAM;
    }
    if (n_fft == 0 || hop_length == 0 || output_length == 0 ||
        n_iter == 0 || n_iter > STFT_MAX_GRIFFIN_LIM_ITERS ||
        !is_valid_window(window) || nf > STFT_MAX_FFT_LEN ||
        olen > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (mag_spec == NULL || mag_len > STFT_MAX_SAMPLES || olen < nf) {
        return STFT_ERR_INVALID_PARAM;
    }
    bins = nf / 2u + 1u;
    if ((mag_len % bins) != 0u) {
        return STFT_ERR_INVALID_SPECTROGRAM;
    }
    num_frames = mag_len / bins;
    if (num_frames > STFT_MAX_FRAMES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (num_frames != (1u + (olen - nf) / (size_t)hop_length)) {
        return STFT_ERR_INVALID_SPECTROGRAM;
    }
    if (!checked_mul_size(mag_len, 2u, &spec_floats) ||
        spec_floats > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    for (i = 0; i < mag_len; ++i) {
        if (!f_isfinite(mag_spec[i]) || mag_spec[i] < 0.0f) {
            return STFT_ERR_INVALID_SPECTROGRAM;
        }
    }

    complex_spec = alloc_floats(spec_floats);
    if (complex_spec == NULL) {
        return STFT_ERR_ALLOC;
    }
    for (i = 0; i < mag_len; ++i) {
        complex_spec[2u * i] = mag_spec[i];
        complex_spec[2u * i + 1u] = 0.0f;
    }

    for (iter = 0; iter < n_iter; ++iter) {
        float *fwd_spec = NULL;
        size_t fwd_len = 0;
        err = stft_istft(complex_spec, spec_floats, n_fft, hop_length,
                         window, output_length, &recon, &recon_len);
        if (err != STFT_OK) {
            zero_free_floats(&complex_spec, spec_floats);
            return err;
        }
        err = stft_forward(recon, recon_len, n_fft, hop_length,
                           window, &fwd_spec, &fwd_len);
        zero_free_floats(&recon, recon_len);
        if (err != STFT_OK) {
            zero_free_floats(&complex_spec, spec_floats);
            return err;
        }
        if (fwd_len != spec_floats) {
            zero_free_floats(&fwd_spec, fwd_len);
            zero_free_floats(&complex_spec, spec_floats);
            return STFT_ERR_INVALID_SPECTROGRAM;
        }
        for (i = 0; i < mag_len; ++i) {
            double re = (double)fwd_spec[2u * i];
            double im = (double)fwd_spec[2u * i + 1u];
            double cur = d_sqrt(re * re + im * im);
            double target = (double)mag_spec[i];
            float out_re = 0.0f, out_im = 0.0f;
            if (cur > 1e-12) {
                if (!narrow_f32(target * (re / cur), &out_re) ||
                    !narrow_f32(target * (im / cur), &out_im)) {
                    zero_free_floats(&fwd_spec, fwd_len);
                    zero_free_floats(&complex_spec, spec_floats);
                    return STFT_ERR_INVALID_SPECTROGRAM;
                }
            } else {
                out_re = mag_spec[i];
                out_im = 0.0f;
            }
            complex_spec[2u * i] = out_re;
            complex_spec[2u * i + 1u] = out_im;
        }
        zero_free_floats(&fwd_spec, fwd_len);
    }

    err = stft_istft(complex_spec, spec_floats, n_fft, hop_length,
                     window, output_length, &recon, &recon_len);
    zero_free_floats(&complex_spec, spec_floats);
    if (err != STFT_OK) {
        return err;
    }
    *out = recon;
    *out_len = recon_len;
    return STFT_OK;
}

/* ================================================================== */
/* Magnitude, Power, and Log Spectrograms                             */
/* ================================================================== */

static stft_error_t spectrogram_impl(const float *signal, size_t signal_len,
                                     uint32_t n_fft, uint32_t hop_length,
                                     stft_window_t window, int take_sqrt,
                                     float **out, size_t *out_len) {
    float *complex_spec = NULL;
    size_t spec_len = 0;
    size_t total_bins = 0;
    float *res = NULL;
    size_t i;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    err = stft_forward(signal, signal_len, n_fft, hop_length, window,
                       &complex_spec, &spec_len);
    if (err != STFT_OK) {
        return err;
    }
    total_bins = spec_len / 2u;
    res = alloc_floats(total_bins);
    if (res == NULL) {
        zero_free_floats(&complex_spec, spec_len);
        return STFT_ERR_ALLOC;
    }
    for (i = 0; i < total_bins; ++i) {
        double re = (double)complex_spec[2u * i];
        double im = (double)complex_spec[2u * i + 1u];
        double val = re * re + im * im;
        float narrowed = 0.0f;
        if (take_sqrt) {
            val = d_sqrt(val);
        }
        if (!narrow_f32(val, &narrowed)) {
            zero_free_floats(&complex_spec, spec_len);
            zero_free_floats(&res, total_bins);
            return STFT_ERR_INVALID_PARAM;
        }
        res[i] = narrowed;
    }
    zero_free_floats(&complex_spec, spec_len);
    *out = res;
    *out_len = total_bins;
    return STFT_OK;
}

stft_error_t stft_magnitude_spectrogram(const float *signal, size_t signal_len,
                                        uint32_t n_fft, uint32_t hop_length,
                                        stft_window_t window,
                                        float **out, size_t *out_len) {
    return spectrogram_impl(signal, signal_len, n_fft, hop_length, window, 1, out, out_len);
}

stft_error_t stft_power_spectrogram(const float *signal, size_t signal_len,
                                    uint32_t n_fft, uint32_t hop_length,
                                    stft_window_t window,
                                    float **out, size_t *out_len) {
    return spectrogram_impl(signal, signal_len, n_fft, hop_length, window, 0, out, out_len);
}

stft_error_t stft_spectrogram(const float *signal, size_t signal_len,
                              uint32_t n_fft, uint32_t hop_length,
                              stft_window_t window,
                              float **out, size_t *out_len) {
    return stft_power_spectrogram(signal, signal_len, n_fft, hop_length, window, out, out_len);
}

stft_error_t stft_log_spectrogram_eps(const float *signal, size_t signal_len,
                                      uint32_t n_fft, uint32_t hop_length,
                                      stft_window_t window, float eps,
                                      float **out, size_t *out_len) {
    float *pwr = NULL;
    size_t pwr_len = 0;
    size_t i;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (signal_len == 0) {
        return STFT_ERR_EMPTY_SIGNAL;
    }
    if (!f_isfinite(eps) || eps <= 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    err = stft_power_spectrogram(signal, signal_len, n_fft, hop_length, window,
                                 &pwr, &pwr_len);
    if (err != STFT_OK) {
        return err;
    }
    for (i = 0; i < pwr_len; ++i) {
        double lp = d_ln((double)pwr[i] + (double)eps);
        float narrowed = 0.0f;
        if (!narrow_f32(lp, &narrowed)) {
            zero_free_floats(&pwr, pwr_len);
            return STFT_ERR_INVALID_PARAM;
        }
        pwr[i] = narrowed;
    }
    *out = pwr;
    *out_len = pwr_len;
    return STFT_OK;
}

stft_error_t stft_log_spectrogram(const float *signal, size_t signal_len,
                                  uint32_t n_fft, uint32_t hop_length,
                                  stft_window_t window,
                                  float **out, size_t *out_len) {
    return stft_log_spectrogram_eps(signal, signal_len, n_fft, hop_length,
                                    window, 1e-10f, out, out_len);
}

/* ================================================================== */
/* HTK Mel Scale, Mel Filterbank, Mel Spectrogram, and MFCC           */
/* ================================================================== */

stft_error_t stft_hz_to_mel(float f_hz, float *out_mel) {
    double m;
    float narrowed = 0.0f;
    if (out_mel == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    *out_mel = 0.0f;
    if (!f_isfinite(f_hz) || f_hz < 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    m = 2595.0 * d_log10(1.0 + (double)f_hz / 700.0);
    if (!narrow_f32(m, &narrowed)) {
        return STFT_ERR_INVALID_PARAM;
    }
    *out_mel = narrowed;
    return STFT_OK;
}

stft_error_t stft_mel_to_hz(float mel, float *out_hz) {
    double hz;
    float narrowed = 0.0f;
    if (out_hz == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    *out_hz = 0.0f;
    if (!f_isfinite(mel) || mel < 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    hz = 700.0 * (d_pow10((double)mel / 2595.0) - 1.0);
    if (!narrow_f32(hz, &narrowed)) {
        return STFT_ERR_INVALID_PARAM;
    }
    *out_hz = narrowed;
    return STFT_OK;
}

static stft_error_t build_mel_filterbank_d(uint32_t n_mels, uint32_t n_fft,
                                           float sample_rate,
                                           double **out_fb, size_t *out_len) {
    size_t nm = (size_t)n_mels;
    size_t nf = (size_t)n_fft;
    size_t bins = 0;
    size_t total_len = 0;
    float fmax_f = 0.0f;
    float bin_per_hz_f = 0.0f;
    double bin_per_hz;
    double mel_min = 0.0;
    double mel_max;
    double step;
    double *bin_pts = NULL;
    double *fb = NULL;
    size_t i, m, k;

    *out_fb = NULL;
    *out_len = 0;

    if (n_mels == 0 || n_fft == 0 || nm > STFT_MAX_MELS || nf > STFT_MAX_FFT_LEN) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (!f_isfinite(sample_rate) || sample_rate <= 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    /* Match Rust/TS subnormal sample_rate guard */
    fmax_f = sample_rate * 0.5f;
    bin_per_hz_f = (float)n_fft / sample_rate;
    if (!f_isfinite(fmax_f) || fmax_f <= 0.0f ||
        !f_isfinite(bin_per_hz_f) || bin_per_hz_f <= 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    bins = nf / 2u + 1u;
    if (!checked_mul_size(nm, bins, &total_len) || total_len > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }

    bin_per_hz = (double)nf / (double)sample_rate;
    mel_max = 2595.0 * d_log10(1.0 + ((double)sample_rate * 0.5) / 700.0);
    if (!d_isfinite(mel_max)) {
        return STFT_ERR_INVALID_PARAM;
    }
    step = (mel_max - mel_min) / (double)(nm + 1u);
    if (!d_isfinite(step)) {
        return STFT_ERR_INVALID_PARAM;
    }

    bin_pts = alloc_doubles(nm + 2u);
    fb = alloc_doubles(total_len);
    if (bin_pts == NULL || fb == NULL) {
        zero_free_doubles(&bin_pts, nm + 2u);
        zero_free_doubles(&fb, total_len);
        return STFT_ERR_ALLOC;
    }

    for (i = 0; i < nm + 2u; ++i) {
        double mel = mel_min + step * (double)i;
        double hz = 700.0 * (d_pow10(mel / 2595.0) - 1.0);
        double pt = hz * bin_per_hz;
        if (!d_isfinite(hz) || !d_isfinite(pt)) {
            zero_free_doubles(&bin_pts, nm + 2u);
            zero_free_doubles(&fb, total_len);
            return STFT_ERR_INVALID_PARAM;
        }
        bin_pts[i] = pt;
    }

    for (m = 0; m < nm; ++m) {
        double left = bin_pts[m];
        double center = bin_pts[m + 1u];
        double right = bin_pts[m + 2u];
        double row_sum = 0.0;
        for (k = 0; k < bins; ++k) {
            double kf = (double)k;
            double w = 0.0;
            if (kf >= left && kf <= right) {
                if (kf <= center) {
                    if (center > left) {
                        w = (kf - left) / (center - left);
                    }
                } else if (right > center) {
                    w = (right - kf) / (right - center);
                }
            }
            fb[m * bins + k] = w;
            row_sum += w;
        }
        if (row_sum > 0.0) {
            for (k = 0; k < bins; ++k) {
                fb[m * bins + k] /= row_sum;
            }
        }
    }

    zero_free_doubles(&bin_pts, nm + 2u);
    *out_fb = fb;
    *out_len = total_len;
    return STFT_OK;
}

stft_error_t stft_mel_filterbank(uint32_t n_mels, uint32_t n_fft,
                                 float sample_rate,
                                 float **out, size_t *out_len) {
    double *fb_d = NULL;
    size_t fb_len = 0;
    float *res = NULL;
    size_t i;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    err = build_mel_filterbank_d(n_mels, n_fft, sample_rate, &fb_d, &fb_len);
    if (err != STFT_OK) {
        return err;
    }
    res = alloc_floats(fb_len);
    if (res == NULL) {
        zero_free_doubles(&fb_d, fb_len);
        return STFT_ERR_ALLOC;
    }
    for (i = 0; i < fb_len; ++i) {
        float narrowed = 0.0f;
        if (!narrow_f32(fb_d[i], &narrowed)) {
            zero_free_doubles(&fb_d, fb_len);
            zero_free_floats(&res, fb_len);
            return STFT_ERR_INVALID_PARAM;
        }
        res[i] = narrowed;
    }
    zero_free_doubles(&fb_d, fb_len);
    *out = res;
    *out_len = fb_len;
    return STFT_OK;
}

stft_error_t stft_mel_spectrogram(const float *signal, size_t signal_len,
                                  uint32_t n_fft, uint32_t hop_length,
                                  uint32_t n_mels, float sample_rate,
                                  stft_window_t window,
                                  float **out, size_t *out_len) {
    size_t nm = (size_t)n_mels;
    size_t nf = (size_t)n_fft;
    size_t hop = (size_t)hop_length;
    size_t bins = 0;
    size_t fb_len = 0;
    float fmax_f = 0.0f;
    float bin_per_hz_f = 0.0f;
    double *fb_d = NULL;
    float *pwr = NULL;
    size_t pwr_len = 0;
    size_t num_frames = 0;
    size_t total_out = 0;
    float *res = NULL;
    size_t t, m, k;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (signal_len == 0) {
        return STFT_ERR_EMPTY_SIGNAL;
    }
    if (signal == NULL || n_fft == 0 || hop_length == 0 || n_mels == 0 ||
        !is_valid_window(window) || nf > STFT_MAX_FFT_LEN ||
        signal_len > STFT_MAX_SAMPLES || nm > STFT_MAX_MELS) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (!f_isfinite(sample_rate) || sample_rate <= 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    fmax_f = sample_rate * 0.5f;
    bin_per_hz_f = (float)n_fft / sample_rate;
    if (!f_isfinite(fmax_f) || fmax_f <= 0.0f ||
        !f_isfinite(bin_per_hz_f) || bin_per_hz_f <= 0.0f) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (signal_len < nf) {
        return STFT_ERR_SIGNAL_TOO_SHORT;
    }
    num_frames = 1u + (signal_len - nf) / hop;
    if (num_frames > STFT_MAX_FRAMES) {
        return STFT_ERR_INVALID_PARAM;
    }
    bins = nf / 2u + 1u;
    if (!checked_mul_size(nm, bins, &fb_len) || fb_len > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (!checked_mul_size(num_frames, nm, &total_out) ||
        total_out > STFT_MAX_SAMPLES) {
        return STFT_ERR_INVALID_PARAM;
    }

    /* Compute power spectrogram first so complex_spec is freed before fb_d. */
    err = stft_power_spectrogram(signal, signal_len, n_fft, hop_length, window,
                                 &pwr, &pwr_len);
    if (err != STFT_OK) {
        return err;
    }
    err = build_mel_filterbank_d(n_mels, n_fft, sample_rate, &fb_d, &fb_len);
    if (err != STFT_OK) {
        zero_free_floats(&pwr, pwr_len);
        return err;
    }

    res = alloc_floats(total_out);
    if (res == NULL) {
        zero_free_doubles(&fb_d, fb_len);
        zero_free_floats(&pwr, pwr_len);
        return STFT_ERR_ALLOC;
    }

    for (t = 0; t < num_frames; ++t) {
        for (m = 0; m < nm; ++m) {
            double acc = 0.0;
            float narrowed = 0.0f;
            for (k = 0; k < bins; ++k) {
                acc += fb_d[m * bins + k] * (double)pwr[t * bins + k];
            }
            if (!narrow_f32(acc, &narrowed)) {
                zero_free_doubles(&fb_d, fb_len);
                zero_free_floats(&pwr, pwr_len);
                zero_free_floats(&res, total_out);
                return STFT_ERR_INVALID_PARAM;
            }
            res[t * nm + m] = narrowed;
        }
    }

    zero_free_doubles(&fb_d, fb_len);
    zero_free_floats(&pwr, pwr_len);
    *out = res;
    *out_len = total_out;
    return STFT_OK;
}

stft_error_t stft_mfcc(const float *signal, size_t signal_len,
                       uint32_t n_fft, uint32_t hop_length,
                       uint32_t n_mels, uint32_t n_mfcc,
                       float sample_rate, stft_window_t window,
                       float **out, size_t *out_len) {
    const double EPS = 1e-10;
    size_t nm = (size_t)n_mels;
    size_t nc = (size_t)n_mfcc;
    float *mel = NULL;
    size_t mel_len = 0;
    size_t num_frames = 0;
    size_t total_out = 0;
    double *row_d = NULL;
    double *dct_d = NULL;
    float *res = NULL;
    size_t t, m, c;
    stft_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return STFT_ERR_INVALID_PARAM;
    }
    if (signal_len == 0) {
        return STFT_ERR_EMPTY_SIGNAL;
    }
    if (n_mfcc == 0 || n_mfcc > n_mels) {
        return STFT_ERR_INVALID_PARAM;
    }

    err = stft_mel_spectrogram(signal, signal_len, n_fft, hop_length,
                               n_mels, sample_rate, window, &mel, &mel_len);
    if (err != STFT_OK) {
        return err;
    }
    num_frames = mel_len / nm;
    if (!checked_mul_size(num_frames, nc, &total_out) ||
        total_out > STFT_MAX_SAMPLES) {
        zero_free_floats(&mel, mel_len);
        return STFT_ERR_INVALID_PARAM;
    }

    row_d = alloc_doubles(nm);
    dct_d = alloc_doubles(nm);
    res = alloc_floats(total_out);
    if (row_d == NULL || dct_d == NULL || res == NULL) {
        zero_free_floats(&mel, mel_len);
        zero_free_doubles(&row_d, nm);
        zero_free_doubles(&dct_d, nm);
        zero_free_floats(&res, total_out);
        return STFT_ERR_ALLOC;
    }

    for (t = 0; t < num_frames; ++t) {
        for (m = 0; m < nm; ++m) {
            double lm = d_ln((double)mel[t * nm + m] + EPS);
            if (!d_isfinite(lm)) {
                zero_free_floats(&mel, mel_len);
                zero_free_doubles(&row_d, nm);
                zero_free_doubles(&dct_d, nm);
                zero_free_floats(&res, total_out);
                return STFT_ERR_INVALID_PARAM;
            }
            row_d[m] = lm;
        }
        err = dct_ii_ortho_d(row_d, nm, dct_d);
        if (err != STFT_OK) {
            zero_free_floats(&mel, mel_len);
            zero_free_doubles(&row_d, nm);
            zero_free_doubles(&dct_d, nm);
            zero_free_floats(&res, total_out);
            return err;
        }
        for (c = 0; c < nc; ++c) {
            float narrowed = 0.0f;
            if (!narrow_f32(dct_d[c], &narrowed)) {
                zero_free_floats(&mel, mel_len);
                zero_free_doubles(&row_d, nm);
                zero_free_doubles(&dct_d, nm);
                zero_free_floats(&res, total_out);
                return STFT_ERR_INVALID_PARAM;
            }
            res[t * nc + c] = narrowed;
        }
    }

    zero_free_floats(&mel, mel_len);
    zero_free_doubles(&row_d, nm);
    zero_free_doubles(&dct_d, nm);
    *out = res;
    *out_len = total_out;
    return STFT_OK;
}
