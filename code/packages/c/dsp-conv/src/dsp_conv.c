/*
 * dsp_conv.c — 1-D, FFT/Overlap-Add, and 2-D Convolution (pure ISO C17).
 *
 * Hardened for VER15 & Zero-CVE:
 * - Zero <math.h> / -lm: pure IEEE-754 double-precision exp, sqrt, sincos.
 * - Structural/length/cap bounds checked BEFORE element finiteness scans
 *   (prevents CWE-125 out-of-bounds reads).
 * - Double-precision intermediates with |x| <= (double)FLT_MAX guard prior
 *   to float narrowing (no ISO C17 float-cast-overflow UB).
 * - Upfront enum/mode domain validation, non-finite input AND output rejection.
 * - Checked size_t math (CWE-190 / CWE-400), heap-allocated large buffers.
 * - Zero-on-free via volatile unsigned char * (CWE-415 / CWE-416).
 */
#include "dsp_conv.h"

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

void conv_free(void *p) {
    free(p);
}

void conv_free_buf(float **p, size_t len) {
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
    double s_term;
    double s_sum;
    double c_term;
    double c_sum;
    double s_val;
    double c_val;
    int n;

    if (!d_isfinite(x)) {
        *sin_out = 0.0;
        *cos_out = 1.0;
        return;
    }
    /* Guard against float-to-int64 overflow UB before (int64_t)(x / TWO_PI). */
    if (x > 9.0e15 || x < -9.0e15) {
        *sin_out = 0.0;
        *cos_out = 1.0;
        return;
    }
    /* Keep angle within safe int64 range for quadrant reduction. */
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
        case 0u:
            s_val = s_sum;
            c_val = c_sum;
            break;
        case 1u:
            s_val = c_sum;
            c_val = -s_sum;
            break;
        case 2u:
            s_val = -s_sum;
            c_val = -c_sum;
            break;
        default:
            s_val = -c_sum;
            c_val = s_sum;
            break;
    }
    *sin_out = s_val;
    *cos_out = c_val;
}

/* ================================================================== */
/* Upfront domain and input validation                                */
/* ================================================================== */

static int is_valid_boundary(conv_boundary_t mode) {
    return mode == CONV_BOUND_ZERO || mode == CONV_BOUND_REPLICATE ||
           mode == CONV_BOUND_REFLECT || mode == CONV_BOUND_WRAP;
}

static int is_valid_mode(conv_mode_t mode) {
    return mode == CONV_MODE_FULL || mode == CONV_MODE_SAME ||
           mode == CONV_MODE_VALID;
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

static void init_out(float **out, size_t *out_len) {
    if (out != NULL) {
        *out = NULL;
    }
    if (out_len != NULL) {
        *out_len = 0;
    }
}

/* ================================================================== */
/* String parsing (bounded, zero regex, rejects trailing garbage)     */
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

static int extract_normalized_token(const char *str, char *buf, size_t buf_cap) {
    size_t len = 0;
    size_t start = 0;
    size_t end;
    size_t tok_len;
    size_t i;

    if (str == NULL || buf == NULL || buf_cap == 0) {
        return 0;
    }
    while (len <= 64u && str[len] != '\0') {
        ++len;
    }
    if (len == 0 || len > 64u) {
        return 0;
    }
    while (start < len && is_ascii_space(str[start])) {
        ++start;
    }
    end = len;
    while (end > start && is_ascii_space(str[end - 1])) {
        --end;
    }
    tok_len = end - start;
    if (tok_len == 0 || tok_len >= buf_cap) {
        return 0;
    }
    for (i = 0; i < tok_len; ++i) {
        buf[i] = ascii_tolower(str[start + i]);
    }
    buf[tok_len] = '\0';
    return 1;
}

conv_error_t conv_parse_boundary(const char *str, conv_boundary_t *out) {
    char tok[32];
    if (str == NULL || out == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!extract_normalized_token(str, tok, sizeof(tok))) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (strcmp(tok, "zero") == 0) {
        *out = CONV_BOUND_ZERO;
        return CONV_OK;
    }
    if (strcmp(tok, "replicate") == 0) {
        *out = CONV_BOUND_REPLICATE;
        return CONV_OK;
    }
    if (strcmp(tok, "reflect") == 0) {
        *out = CONV_BOUND_REFLECT;
        return CONV_OK;
    }
    if (strcmp(tok, "wrap") == 0) {
        *out = CONV_BOUND_WRAP;
        return CONV_OK;
    }
    return CONV_ERR_INVALID_INPUT;
}

conv_error_t conv_parse_mode(const char *str, conv_mode_t *out) {
    char tok[32];
    if (str == NULL || out == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!extract_normalized_token(str, tok, sizeof(tok))) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (strcmp(tok, "full") == 0) {
        *out = CONV_MODE_FULL;
        return CONV_OK;
    }
    if (strcmp(tok, "same") == 0) {
        *out = CONV_MODE_SAME;
        return CONV_OK;
    }
    if (strcmp(tok, "valid") == 0) {
        *out = CONV_MODE_VALID;
        return CONV_OK;
    }
    return CONV_ERR_INVALID_INPUT;
}

/* ================================================================== */
/* Boundary index extension & sampling                                */
/* ================================================================== */

static int extend_index_fast(int64_t idx, size_t n, conv_boundary_t mode,
                             size_t *out_idx) {
    if (idx >= 0 && (uint64_t)idx < (uint64_t)n) {
        *out_idx = (size_t)idx;
        return 1;
    }
    switch (mode) {
        case CONV_BOUND_ZERO:
            return 0;
        case CONV_BOUND_REPLICATE:
            if (idx < 0) {
                *out_idx = 0;
            } else {
                *out_idx = n - 1u;
            }
            return 1;
        case CONV_BOUND_REFLECT: {
            uint64_t period;
            uint64_t m;
            if (n == 1u) {
                *out_idx = 0;
                return 1;
            }
            period = 2u * ((uint64_t)n - 1u);
            if (idx >= 0) {
                m = (uint64_t)idx % period;
            } else {
                uint64_t abs_idx = (uint64_t)(-(idx + 1)) + 1u;
                uint64_t rem = abs_idx % period;
                m = (rem == 0u) ? 0u : (period - rem);
            }
            if (m >= (uint64_t)n) {
                m = period - m;
            }
            *out_idx = (size_t)m;
            return 1;
        }
        case CONV_BOUND_WRAP: {
            uint64_t m;
            if (idx >= 0) {
                m = (uint64_t)idx % (uint64_t)n;
            } else {
                uint64_t abs_idx = (uint64_t)(-(idx + 1)) + 1u;
                uint64_t rem = abs_idx % (uint64_t)n;
                m = (rem == 0u) ? 0u : ((uint64_t)n - rem);
            }
            *out_idx = (size_t)m;
            return 1;
        }
        default:
            return 0;
    }
}

conv_error_t conv_extend_index(int64_t idx, size_t n, conv_boundary_t mode,
                               int *has_value, size_t *out_idx) {
    size_t mapped = 0;
    int hv;
    if (has_value == NULL || out_idx == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    *has_value = 0;
    *out_idx = 0;
    if (n == 0 || n > (size_t)INT64_MAX) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!is_valid_boundary(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    hv = extend_index_fast(idx, n, mode, &mapped);
    *has_value = hv;
    *out_idx = hv ? mapped : 0u;
    return CONV_OK;
}

conv_error_t conv_sample(const float *signal, size_t signal_len, int64_t idx,
                         conv_boundary_t mode, float *out) {
    size_t mapped = 0;
    int has_val = 0;
    conv_error_t err;
    if (out == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    *out = 0.0f;
    if (signal_len == 0) {
        return CONV_ERR_EMPTY_SIGNAL;
    }
    if (signal == NULL || signal_len > CONV_MAX_SAMPLES || !is_valid_boundary(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    err = conv_extend_index(idx, signal_len, mode, &has_val, &mapped);
    if (err != CONV_OK) {
        return err;
    }
    if (!has_val) {
        *out = 0.0f;
        return CONV_OK;
    }
    if (!f_isfinite(signal[mapped])) {
        return CONV_ERR_INVALID_INPUT;
    }
    *out = signal[mapped];
    return CONV_OK;
}

conv_error_t conv_next_pow2(size_t n, size_t *out) {
    size_t p = 1u;
    if (out == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    *out = 0;
    if (n <= 1u) {
        *out = 1u;
        return CONV_OK;
    }
    while (p < n) {
        if (p > (SIZE_MAX >> 1)) {
            return CONV_ERR_INVALID_INPUT;
        }
        p <<= 1;
    }
    *out = p;
    return CONV_OK;
}

/* ================================================================== */
/* Internal double-precision radix-2 FFT / RFFT / IRFFT               */
/* ================================================================== */

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

static void fft_butterflies_d(double *buf, size_t n, int inverse) {
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
                double w_im = 0.0;
                double w_re = 1.0;
                size_t x_idx = 2u * (block_start + j);
                size_t y_idx = 2u * (block_start + j + half);
                double x_re = buf[x_idx];
                double x_im = buf[x_idx + 1u];
                double y_re = buf[y_idx];
                double y_im = buf[y_idx + 1u];
                double t_re;
                double t_im;
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

/* ================================================================== */
/* 1-D Direct Convolution & Cross-Correlation                         */
/* ================================================================== */

conv_error_t conv_conv1d(const float *signal, size_t signal_len,
                         const float *kernel, size_t kernel_len,
                         conv_boundary_t mode,
                         float **out, size_t *out_len) {
    float *res;
    size_t centre;
    size_t i;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    /* Structural / domain checks BEFORE element finiteness scan (CWE-125). */
    if (signal_len == 0) {
        return CONV_ERR_EMPTY_SIGNAL;
    }
    if (kernel_len == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (signal == NULL || kernel == NULL ||
        signal_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN ||
        !is_valid_boundary(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!validate_float_array(signal, signal_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }

    res = alloc_floats(signal_len);
    if (res == NULL) {
        return CONV_ERR_ALLOC;
    }

    centre = kernel_len / 2u;
    for (i = 0; i < signal_len; ++i) {
        double acc = 0.0;
        size_t j;
        float narrowed = 0.0f;
        for (j = 0; j < kernel_len; ++j) {
            int64_t src_idx = (int64_t)i + (int64_t)centre - (int64_t)j;
            size_t mapped = 0;
            if (extend_index_fast(src_idx, signal_len, mode, &mapped)) {
                acc += (double)kernel[j] * (double)signal[mapped];
            }
        }
        if (!narrow_f32(acc, &narrowed)) {
            zero_free_floats(&res, signal_len);
            return CONV_ERR_INVALID_INPUT;
        }
        res[i] = narrowed;
    }

    *out = res;
    *out_len = signal_len;
    return CONV_OK;
}

conv_error_t conv_convolve(const float *signal, size_t signal_len,
                           const float *kernel, size_t kernel_len,
                           conv_mode_t mode, conv_boundary_t boundary,
                           float **out, size_t *out_len) {
    size_t res_len = 0;
    float *res;
    size_t i;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (signal_len == 0) {
        return CONV_ERR_EMPTY_SIGNAL;
    }
    if (kernel_len == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (signal == NULL || kernel == NULL ||
        signal_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN ||
        !is_valid_mode(mode) || !is_valid_boundary(boundary)) {
        return CONV_ERR_INVALID_INPUT;
    }

    if (mode == CONV_MODE_SAME) {
        return conv_conv1d(signal, signal_len, kernel, kernel_len, boundary, out, out_len);
    }

    if (mode == CONV_MODE_VALID) {
        if (signal_len < kernel_len) {
            return CONV_ERR_KERNEL_TOO_LARGE;
        }
        res_len = signal_len - kernel_len + 1u;
    } else {
        size_t sum_len = 0;
        if (!checked_add_size(signal_len, kernel_len, &sum_len) || sum_len == 0) {
            return CONV_ERR_INVALID_INPUT;
        }
        res_len = sum_len - 1u;
        if (res_len > CONV_MAX_SAMPLES) {
            return CONV_ERR_INVALID_INPUT;
        }
    }

    if (!validate_float_array(signal, signal_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }

    res = alloc_floats(res_len);
    if (res == NULL) {
        return CONV_ERR_ALLOC;
    }

    if (mode == CONV_MODE_VALID) {
        for (i = 0; i < res_len; ++i) {
            double acc = 0.0;
            size_t j;
            float narrowed = 0.0f;
            for (j = 0; j < kernel_len; ++j) {
                acc += (double)kernel[j] * (double)signal[i + kernel_len - 1u - j];
            }
            if (!narrow_f32(acc, &narrowed)) {
                zero_free_floats(&res, res_len);
                return CONV_ERR_INVALID_INPUT;
            }
            res[i] = narrowed;
        }
    } else {
        /* CONV_MODE_FULL */
        for (i = 0; i < res_len; ++i) {
            double acc = 0.0;
            size_t j;
            float narrowed = 0.0f;
            for (j = 0; j < kernel_len; ++j) {
                int64_t src_idx = (int64_t)i - (int64_t)j;
                size_t mapped = 0;
                if (extend_index_fast(src_idx, signal_len, boundary, &mapped)) {
                    acc += (double)kernel[j] * (double)signal[mapped];
                }
            }
            if (!narrow_f32(acc, &narrowed)) {
                zero_free_floats(&res, res_len);
                return CONV_ERR_INVALID_INPUT;
            }
            res[i] = narrowed;
        }
    }

    *out = res;
    *out_len = res_len;
    return CONV_OK;
}

conv_error_t conv_correlate(const float *signal, size_t signal_len,
                            const float *kernel, size_t kernel_len,
                            conv_mode_t mode,
                            float **out, size_t *out_len) {
    float *rev_kernel;
    size_t i;
    conv_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (signal_len == 0) {
        return CONV_ERR_EMPTY_SIGNAL;
    }
    if (kernel_len == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (signal == NULL || kernel == NULL ||
        signal_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN ||
        !is_valid_mode(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (mode == CONV_MODE_VALID && signal_len < kernel_len) {
        return CONV_ERR_KERNEL_TOO_LARGE;
    }
    if (mode == CONV_MODE_FULL) {
        size_t sum_len = 0;
        if (!checked_add_size(signal_len, kernel_len, &sum_len) ||
            sum_len - 1u > CONV_MAX_SAMPLES) {
            return CONV_ERR_INVALID_INPUT;
        }
    }
    if (!validate_float_array(signal, signal_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }

    rev_kernel = alloc_floats(kernel_len);
    if (rev_kernel == NULL) {
        return CONV_ERR_ALLOC;
    }
    for (i = 0; i < kernel_len; ++i) {
        rev_kernel[i] = kernel[kernel_len - 1u - i];
    }

    err = conv_convolve(signal, signal_len, rev_kernel, kernel_len,
                        mode, CONV_BOUND_ZERO, out, out_len);
    zero_free_floats(&rev_kernel, kernel_len);
    return err;
}

/* ================================================================== */
/* FFT-based 1-D Convolution & Overlap-Add                            */
/* ================================================================== */

conv_error_t conv_convolve_fft(const float *signal, size_t signal_len,
                               const float *kernel, size_t kernel_len,
                               conv_mode_t mode,
                               float **out, size_t *out_len) {
    size_t sum_len = 0;
    size_t full_len = 0;
    size_t fft_len = 0;
    size_t buf_doubles = 0;
    double *sig_buf = NULL;
    double *ker_buf = NULL;
    float *res = NULL;
    size_t res_len = 0;
    size_t offset = 0;
    size_t i;
    conv_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (signal_len == 0) {
        return CONV_ERR_EMPTY_SIGNAL;
    }
    if (kernel_len == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (signal == NULL || kernel == NULL ||
        signal_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN ||
        !is_valid_mode(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (mode == CONV_MODE_VALID && signal_len < kernel_len) {
        return CONV_ERR_KERNEL_TOO_LARGE;
    }
    if (!checked_add_size(signal_len, kernel_len, &sum_len) || sum_len == 0) {
        return CONV_ERR_INVALID_INPUT;
    }
    full_len = sum_len - 1u;
    if (full_len > CONV_MAX_SAMPLES) {
        return CONV_ERR_INVALID_INPUT;
    }
    err = conv_next_pow2(full_len, &fft_len);
    if (err != CONV_OK || fft_len > CONV_MAX_FFT_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!checked_mul_size(fft_len, 2u, &buf_doubles)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!validate_float_array(signal, signal_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }

    sig_buf = alloc_doubles(buf_doubles);
    ker_buf = alloc_doubles(buf_doubles);
    if (sig_buf == NULL || ker_buf == NULL) {
        zero_free_doubles(&sig_buf, buf_doubles);
        zero_free_doubles(&ker_buf, buf_doubles);
        return CONV_ERR_ALLOC;
    }

    for (i = 0; i < signal_len; ++i) {
        sig_buf[2u * i] = (double)signal[i];
    }
    for (i = 0; i < kernel_len; ++i) {
        ker_buf[2u * i] = (double)kernel[i];
    }

    fft_butterflies_d(sig_buf, fft_len, 0);
    fft_butterflies_d(ker_buf, fft_len, 0);

    for (i = 0; i < fft_len; ++i) {
        double re_s = sig_buf[2u * i];
        double im_s = sig_buf[2u * i + 1u];
        double re_k = ker_buf[2u * i];
        double im_k = ker_buf[2u * i + 1u];
        double re = re_s * re_k - im_s * im_k;
        double im = re_s * im_k + im_s * re_k;
        if (!d_isfinite(re) || !d_isfinite(im)) {
            zero_free_doubles(&sig_buf, buf_doubles);
            zero_free_doubles(&ker_buf, buf_doubles);
            return CONV_ERR_INVALID_INPUT;
        }
        sig_buf[2u * i] = re;
        sig_buf[2u * i + 1u] = im;
    }

    zero_free_doubles(&ker_buf, buf_doubles);
    fft_butterflies_d(sig_buf, fft_len, 1);

    if (mode == CONV_MODE_FULL) {
        res_len = full_len;
        offset = 0;
    } else if (mode == CONV_MODE_SAME) {
        res_len = signal_len;
        offset = kernel_len / 2u;
    } else {
        res_len = signal_len - kernel_len + 1u;
        offset = kernel_len - 1u;
    }

    res = alloc_floats(res_len);
    if (res == NULL) {
        zero_free_doubles(&sig_buf, buf_doubles);
        return CONV_ERR_ALLOC;
    }

    for (i = 0; i < res_len; ++i) {
        float narrowed = 0.0f;
        if (!narrow_f32(sig_buf[2u * (offset + i)], &narrowed)) {
            zero_free_floats(&res, res_len);
            zero_free_doubles(&sig_buf, buf_doubles);
            return CONV_ERR_INVALID_INPUT;
        }
        res[i] = narrowed;
    }

    zero_free_doubles(&sig_buf, buf_doubles);
    *out = res;
    *out_len = res_len;
    return CONV_OK;
}

conv_error_t conv_overlap_add(const float *signal, size_t signal_len,
                              const float *kernel, size_t kernel_len,
                              const size_t *block_size, conv_mode_t mode,
                              float **out, size_t *out_len) {
    size_t block_len = 0;
    size_t sum_len = 0;
    size_t full_len = 0;
    size_t conv_block_max = 0;
    size_t fft_len = 0;
    size_t buf_doubles = 0;
    double *ker_buf = NULL;
    double *blk_buf = NULL;
    double *accum = NULL;
    float *res = NULL;
    size_t res_len = 0;
    size_t offset = 0;
    size_t start = 0;
    size_t i;
    conv_error_t err;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (signal_len == 0) {
        return CONV_ERR_EMPTY_SIGNAL;
    }
    if (kernel_len == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (signal == NULL || kernel == NULL ||
        signal_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN ||
        !is_valid_mode(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }

    if (block_size == NULL) {
        size_t np2 = 0;
        err = conv_next_pow2(kernel_len, &np2);
        if (err != CONV_OK) {
            return CONV_ERR_INVALID_INPUT;
        }
        block_len = np2 < 64u ? 64u : np2;
    } else {
        if (*block_size == 0 || *block_size > CONV_MAX_SAMPLES) {
            return CONV_ERR_INVALID_INPUT;
        }
        block_len = *block_size;
    }

    if (mode == CONV_MODE_VALID && signal_len < kernel_len) {
        return CONV_ERR_KERNEL_TOO_LARGE;
    }

    if (!checked_add_size(signal_len, kernel_len, &sum_len) || sum_len == 0) {
        return CONV_ERR_INVALID_INPUT;
    }
    full_len = sum_len - 1u;
    if (full_len > CONV_MAX_SAMPLES) {
        return CONV_ERR_INVALID_INPUT;
    }

    if (!checked_add_size(block_len, kernel_len, &conv_block_max) || conv_block_max == 0) {
        return CONV_ERR_INVALID_INPUT;
    }
    err = conv_next_pow2(conv_block_max - 1u, &fft_len);
    if (err != CONV_OK || fft_len > CONV_MAX_FFT_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!checked_mul_size(fft_len, 2u, &buf_doubles)) {
        return CONV_ERR_INVALID_INPUT;
    }

    if (!validate_float_array(signal, signal_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }

    ker_buf = alloc_doubles(buf_doubles);
    blk_buf = alloc_doubles(buf_doubles);
    accum = alloc_doubles(full_len);
    if (ker_buf == NULL || blk_buf == NULL || accum == NULL) {
        zero_free_doubles(&ker_buf, buf_doubles);
        zero_free_doubles(&blk_buf, buf_doubles);
        zero_free_doubles(&accum, full_len);
        return CONV_ERR_ALLOC;
    }

    for (i = 0; i < kernel_len; ++i) {
        ker_buf[2u * i] = (double)kernel[i];
    }
    fft_butterflies_d(ker_buf, fft_len, 0);

    while (start < signal_len) {
        size_t rem = signal_len - start;
        size_t actual = rem < block_len ? rem : block_len;
        size_t block_conv_len = actual + kernel_len - 1u;

        zero_bytes_volatile(blk_buf, buf_doubles * sizeof(double));
        for (i = 0; i < actual; ++i) {
            blk_buf[2u * i] = (double)signal[start + i];
        }
        fft_butterflies_d(blk_buf, fft_len, 0);
        for (i = 0; i < fft_len; ++i) {
            double re_s = blk_buf[2u * i];
            double im_s = blk_buf[2u * i + 1u];
            double re_k = ker_buf[2u * i];
            double im_k = ker_buf[2u * i + 1u];
            double re = re_s * re_k - im_s * im_k;
            double im = re_s * im_k + im_s * re_k;
            if (!d_isfinite(re) || !d_isfinite(im)) {
                zero_free_doubles(&ker_buf, buf_doubles);
                zero_free_doubles(&blk_buf, buf_doubles);
                zero_free_doubles(&accum, full_len);
                return CONV_ERR_INVALID_INPUT;
            }
            blk_buf[2u * i] = re;
            blk_buf[2u * i + 1u] = im;
        }
        fft_butterflies_d(blk_buf, fft_len, 1);

        for (i = 0; i < block_conv_len; ++i) {
            size_t out_pos = start + i;
            if (out_pos < full_len) {
                accum[out_pos] += blk_buf[2u * i];
            }
        }
        start += block_len;
    }

    if (mode == CONV_MODE_FULL) {
        res_len = full_len;
        offset = 0;
    } else if (mode == CONV_MODE_SAME) {
        res_len = signal_len;
        offset = kernel_len / 2u;
    } else {
        res_len = signal_len - kernel_len + 1u;
        offset = kernel_len - 1u;
    }

    res = alloc_floats(res_len);
    if (res == NULL) {
        zero_free_doubles(&ker_buf, buf_doubles);
        zero_free_doubles(&blk_buf, buf_doubles);
        zero_free_doubles(&accum, full_len);
        return CONV_ERR_ALLOC;
    }

    for (i = 0; i < res_len; ++i) {
        float narrowed = 0.0f;
        if (!narrow_f32(accum[offset + i], &narrowed)) {
            zero_free_floats(&res, res_len);
            zero_free_doubles(&ker_buf, buf_doubles);
            zero_free_doubles(&blk_buf, buf_doubles);
            zero_free_doubles(&accum, full_len);
            return CONV_ERR_INVALID_INPUT;
        }
        res[i] = narrowed;
    }

    zero_free_doubles(&ker_buf, buf_doubles);
    zero_free_doubles(&blk_buf, buf_doubles);
    zero_free_doubles(&accum, full_len);
    *out = res;
    *out_len = res_len;
    return CONV_OK;
}

/* ================================================================== */
/* 2-D and Separable 2-D Convolution                                  */
/* ================================================================== */

conv_error_t conv_conv2d(const float *image, size_t image_len,
                         const float *kernel, size_t kernel_len,
                         uint32_t image_height, uint32_t image_width,
                         uint32_t kernel_height, uint32_t kernel_width,
                         conv_boundary_t mode,
                         float **out, size_t *out_len) {
    size_t h = (size_t)image_height;
    size_t w = (size_t)image_width;
    size_t kh = (size_t)kernel_height;
    size_t kw = (size_t)kernel_width;
    size_t expected_img = 0;
    size_t expected_ker = 0;
    size_t ch, cw;
    float *res = NULL;
    size_t r, c;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_height == 0 || image_width == 0 || image_len == 0) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (kernel_len == 0 || kernel_height == 0 || kernel_width == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (image == NULL || kernel == NULL || !is_valid_boundary(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!checked_mul_size(h, w, &expected_img)) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (expected_img > CONV_MAX_SAMPLES) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_len != expected_img) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (!checked_mul_size(kh, kw, &expected_ker)) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (expected_ker > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (kernel_len != expected_ker) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (!validate_float_array(image, image_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (kh > h || kw > w) {
        return CONV_ERR_KERNEL_TOO_LARGE;
    }

    res = alloc_floats(expected_img);
    if (res == NULL) {
        return CONV_ERR_ALLOC;
    }

    ch = kh / 2u;
    cw = kw / 2u;
    for (r = 0; r < h; ++r) {
        for (c = 0; c < w; ++c) {
            double acc = 0.0;
            size_t kr;
            float narrowed = 0.0f;
            for (kr = 0; kr < kh; ++kr) {
                int64_t src_r = (int64_t)r + (int64_t)ch - (int64_t)kr;
                size_t row = 0;
                size_t kc;
                if (!extend_index_fast(src_r, h, mode, &row)) {
                    continue;
                }
                for (kc = 0; kc < kw; ++kc) {
                    int64_t src_c = (int64_t)c + (int64_t)cw - (int64_t)kc;
                    size_t col = 0;
                    if (!extend_index_fast(src_c, w, mode, &col)) {
                        continue;
                    }
                    acc += (double)kernel[kr * kw + kc] * (double)image[row * w + col];
                }
            }
            if (!narrow_f32(acc, &narrowed)) {
                zero_free_floats(&res, expected_img);
                return CONV_ERR_INVALID_INPUT;
            }
            res[r * w + c] = narrowed;
        }
    }

    *out = res;
    *out_len = expected_img;
    return CONV_OK;
}

conv_error_t conv_sep_conv2d(const float *image, size_t image_len,
                             const float *h_kernel, size_t h_len,
                             const float *v_kernel, size_t v_len,
                             uint32_t image_height, uint32_t image_width,
                             conv_boundary_t mode,
                             float **out, size_t *out_len) {
    size_t h = (size_t)image_height;
    size_t w = (size_t)image_width;
    size_t expected_img = 0;
    double *intermediate = NULL;
    float *res = NULL;
    size_t ch, cv;
    size_t r, c;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_height == 0 || image_width == 0 || image_len == 0) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (h_len == 0 || v_len == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (image == NULL || h_kernel == NULL || v_kernel == NULL ||
        !is_valid_boundary(mode)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_len > CONV_MAX_SAMPLES ||
        h_len > CONV_MAX_KERNEL_LEN || v_len > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!checked_mul_size(h, w, &expected_img)) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (expected_img > CONV_MAX_SAMPLES) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_len != expected_img) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (!validate_float_array(image, image_len) ||
        !validate_float_array(h_kernel, h_len) ||
        !validate_float_array(v_kernel, v_len)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (h_len > w || v_len > h) {
        return CONV_ERR_KERNEL_TOO_LARGE;
    }

    intermediate = alloc_doubles(expected_img);
    res = alloc_floats(expected_img);
    if (intermediate == NULL || res == NULL) {
        zero_free_doubles(&intermediate, expected_img);
        zero_free_floats(&res, expected_img);
        return CONV_ERR_ALLOC;
    }

    ch = h_len / 2u;
    cv = v_len / 2u;

    /* Row pass */
    for (r = 0; r < h; ++r) {
        for (c = 0; c < w; ++c) {
            double acc = 0.0;
            size_t j;
            for (j = 0; j < h_len; ++j) {
                int64_t src_c = (int64_t)c + (int64_t)ch - (int64_t)j;
                size_t col = 0;
                if (extend_index_fast(src_c, w, mode, &col)) {
                    acc += (double)h_kernel[j] * (double)image[r * w + col];
                }
            }
            if (!d_isfinite(acc)) {
                zero_free_doubles(&intermediate, expected_img);
                zero_free_floats(&res, expected_img);
                return CONV_ERR_INVALID_INPUT;
            }
            intermediate[r * w + c] = acc;
        }
    }

    /* Column pass */
    for (c = 0; c < w; ++c) {
        for (r = 0; r < h; ++r) {
            double acc = 0.0;
            size_t j;
            float narrowed = 0.0f;
            for (j = 0; j < v_len; ++j) {
                int64_t src_r = (int64_t)r + (int64_t)cv - (int64_t)j;
                size_t row = 0;
                if (extend_index_fast(src_r, h, mode, &row)) {
                    acc += (double)v_kernel[j] * intermediate[row * w + c];
                }
            }
            if (!narrow_f32(acc, &narrowed)) {
                zero_free_doubles(&intermediate, expected_img);
                zero_free_floats(&res, expected_img);
                return CONV_ERR_INVALID_INPUT;
            }
            res[r * w + c] = narrowed;
        }
    }

    zero_free_doubles(&intermediate, expected_img);
    *out = res;
    *out_len = expected_img;
    return CONV_OK;
}

conv_error_t conv_convolve_2d(const float *image, size_t image_len,
                              const float *kernel, size_t kernel_len,
                              uint32_t image_height, uint32_t image_width,
                              uint32_t kernel_height, uint32_t kernel_width,
                              conv_mode_t mode, conv_boundary_t boundary,
                              float **out, size_t *out_len) {
    size_t h = (size_t)image_height;
    size_t w = (size_t)image_width;
    size_t kh = (size_t)kernel_height;
    size_t kw = (size_t)kernel_width;
    size_t expected_img = 0;
    size_t expected_ker = 0;
    size_t out_h = 0;
    size_t out_w = 0;
    size_t res_len = 0;
    float *res = NULL;
    size_t r, c;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_height == 0 || image_width == 0 || image_len == 0) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (kernel_len == 0 || kernel_height == 0 || kernel_width == 0) {
        return CONV_ERR_EMPTY_KERNEL;
    }
    if (!is_valid_mode(mode) || !is_valid_boundary(boundary)) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (mode == CONV_MODE_SAME) {
        return conv_conv2d(image, image_len, kernel, kernel_len,
                           image_height, image_width,
                           kernel_height, kernel_width,
                           boundary, out, out_len);
    }
    if (image == NULL || kernel == NULL ||
        image_len > CONV_MAX_SAMPLES || kernel_len > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!checked_mul_size(h, w, &expected_img)) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (expected_img > CONV_MAX_SAMPLES) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (image_len != expected_img) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (!checked_mul_size(kh, kw, &expected_ker)) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }
    if (expected_ker > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (kernel_len != expected_ker) {
        return CONV_ERR_IMAGE_SIZE_MISMATCH;
    }

    if (mode == CONV_MODE_VALID) {
        if (h < kh || w < kw) {
            return CONV_ERR_KERNEL_TOO_LARGE;
        }
        out_h = h - kh + 1u;
        out_w = w - kw + 1u;
    } else {
        size_t sh = 0, sw = 0;
        if (!checked_add_size(h, kh, &sh) || sh == 0 ||
            !checked_add_size(w, kw, &sw) || sw == 0) {
            return CONV_ERR_INVALID_INPUT;
        }
        out_h = sh - 1u;
        out_w = sw - 1u;
    }

    if (!checked_mul_size(out_h, out_w, &res_len) || res_len > CONV_MAX_SAMPLES) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!validate_float_array(image, image_len) ||
        !validate_float_array(kernel, kernel_len)) {
        return CONV_ERR_INVALID_INPUT;
    }

    res = alloc_floats(res_len);
    if (res == NULL) {
        return CONV_ERR_ALLOC;
    }

    if (mode == CONV_MODE_VALID) {
        for (r = 0; r < out_h; ++r) {
            for (c = 0; c < out_w; ++c) {
                double acc = 0.0;
                size_t kr, kc;
                float narrowed = 0.0f;
                for (kr = 0; kr < kh; ++kr) {
                    size_t img_r = r + kh - 1u - kr;
                    for (kc = 0; kc < kw; ++kc) {
                        size_t img_c = c + kw - 1u - kc;
                        acc += (double)kernel[kr * kw + kc] *
                               (double)image[img_r * w + img_c];
                    }
                }
                if (!narrow_f32(acc, &narrowed)) {
                    zero_free_floats(&res, res_len);
                    return CONV_ERR_INVALID_INPUT;
                }
                res[r * out_w + c] = narrowed;
            }
        }
    } else {
        /* CONV_MODE_FULL */
        for (r = 0; r < out_h; ++r) {
            for (c = 0; c < out_w; ++c) {
                double acc = 0.0;
                size_t kr, kc;
                float narrowed = 0.0f;
                for (kr = 0; kr < kh; ++kr) {
                    int64_t src_r = (int64_t)r - (int64_t)kr;
                    size_t row = 0;
                    if (!extend_index_fast(src_r, h, boundary, &row)) {
                        continue;
                    }
                    for (kc = 0; kc < kw; ++kc) {
                        int64_t src_c = (int64_t)c - (int64_t)kc;
                        size_t col = 0;
                        if (!extend_index_fast(src_c, w, boundary, &col)) {
                            continue;
                        }
                        acc += (double)kernel[kr * kw + kc] *
                               (double)image[row * w + col];
                    }
                }
                if (!narrow_f32(acc, &narrowed)) {
                    zero_free_floats(&res, res_len);
                    return CONV_ERR_INVALID_INPUT;
                }
                res[r * out_w + c] = narrowed;
            }
        }
    }

    *out = res;
    *out_len = res_len;
    return CONV_OK;
}

/* ================================================================== */
/* Image Filter Kernel Generators                                     */
/* ================================================================== */

conv_error_t conv_gaussian_blur_kernel(float sigma, uint32_t size,
                                       float **out, size_t *out_len) {
    const double TWO_PI = 6.28318530717958647693;
    size_t n = (size_t)size;
    double d_sigma;
    double two_sigma_sq;
    double norm_pdf;
    double centre;
    double *tmp = NULL;
    double total = 0.0;
    float *res = NULL;
    size_t i;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (size == 0 || (size % 2u) == 0 || n > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!f_isfinite(sigma) || sigma <= 0.0f) {
        return CONV_ERR_INVALID_INPUT;
    }
    d_sigma = (double)sigma;
    /* Reject subnormal / tiny sigma that would underflow float 2*sigma^2 or
     * overflow 1/(sigma*sqrt(2*pi)) in f32, matching Rust/TS bounds. */
    if ((2.0f * sigma * sigma) <= 0.0f || !f_isfinite(2.0f * sigma * sigma)) {
        return CONV_ERR_INVALID_INPUT;
    }
    two_sigma_sq = 2.0 * d_sigma * d_sigma;
    if (!d_isfinite(two_sigma_sq) || two_sigma_sq <= 0.0) {
        return CONV_ERR_INVALID_INPUT;
    }
    norm_pdf = 1.0 / (d_sigma * d_sqrt(TWO_PI));
    if (!d_isfinite(norm_pdf) || norm_pdf > (double)FLT_MAX) {
        return CONV_ERR_INVALID_INPUT;
    }

    tmp = alloc_doubles(n);
    res = alloc_floats(n);
    if (tmp == NULL || res == NULL) {
        zero_free_doubles(&tmp, n);
        zero_free_floats(&res, n);
        return CONV_ERR_ALLOC;
    }

    centre = (double)(n - 1u) * 0.5;
    for (i = 0; i < n; ++i) {
        double x = (double)i - centre;
        double v = norm_pdf * d_exp(-(x * x) / two_sigma_sq);
        if (!d_isfinite(v)) {
            zero_free_doubles(&tmp, n);
            zero_free_floats(&res, n);
            return CONV_ERR_INVALID_INPUT;
        }
        tmp[i] = v;
        total += v;
    }
    if (!d_isfinite(total) || total <= 0.0) {
        zero_free_doubles(&tmp, n);
        zero_free_floats(&res, n);
        return CONV_ERR_INVALID_INPUT;
    }
    for (i = 0; i < n; ++i) {
        float narrowed = 0.0f;
        if (!narrow_f32(tmp[i] / total, &narrowed)) {
            zero_free_doubles(&tmp, n);
            zero_free_floats(&res, n);
            return CONV_ERR_INVALID_INPUT;
        }
        res[i] = narrowed;
    }

    zero_free_doubles(&tmp, n);
    *out = res;
    *out_len = n;
    return CONV_OK;
}

conv_error_t conv_box_blur_kernel(uint32_t size,
                                  float **out, size_t *out_len) {
    size_t n = (size_t)size;
    float *res = NULL;
    float val = 0.0f;
    size_t i;

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (size == 0 || n > CONV_MAX_KERNEL_LEN) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!narrow_f32(1.0 / (double)n, &val)) {
        return CONV_ERR_INVALID_INPUT;
    }
    res = alloc_floats(n);
    if (res == NULL) {
        return CONV_ERR_ALLOC;
    }
    for (i = 0; i < n; ++i) {
        res[i] = val;
    }
    *out = res;
    *out_len = n;
    return CONV_OK;
}

static conv_error_t alloc_3x3_kernel(const float taps[9],
                                     float **out, size_t *out_len) {
    float *res;
    size_t i;
    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    res = alloc_floats(9u);
    if (res == NULL) {
        return CONV_ERR_ALLOC;
    }
    for (i = 0; i < 9u; ++i) {
        res[i] = taps[i];
    }
    *out = res;
    *out_len = 9u;
    return CONV_OK;
}

conv_error_t conv_sobel_x_kernel(float **out, size_t *out_len) {
    static const float taps[9] = {
        -1.0f, 0.0f, 1.0f,
        -2.0f, 0.0f, 2.0f,
        -1.0f, 0.0f, 1.0f
    };
    return alloc_3x3_kernel(taps, out, out_len);
}

conv_error_t conv_sobel_y_kernel(float **out, size_t *out_len) {
    static const float taps[9] = {
        -1.0f, -2.0f, -1.0f,
         0.0f,  0.0f,  0.0f,
         1.0f,  2.0f,  1.0f
    };
    return alloc_3x3_kernel(taps, out, out_len);
}

conv_error_t conv_laplacian_kernel(float **out, size_t *out_len) {
    static const float taps[9] = {
        0.0f,  1.0f, 0.0f,
        1.0f, -4.0f, 1.0f,
        0.0f,  1.0f, 0.0f
    };
    return alloc_3x3_kernel(taps, out, out_len);
}

conv_error_t conv_sharpen_kernel(float amount, float **out, size_t *out_len) {
    double c_dbl;
    float center = 0.0f;
    float neg_amount = 0.0f;
    float taps[9];

    init_out(out, out_len);
    if (out == NULL || out_len == NULL) {
        return CONV_ERR_INVALID_INPUT;
    }
    if (!f_isfinite(amount)) {
        return CONV_ERR_INVALID_INPUT;
    }
    c_dbl = 1.0 + 4.0 * (double)amount;
    if (!narrow_f32(c_dbl, &center) || !narrow_f32(-(double)amount, &neg_amount)) {
        return CONV_ERR_INVALID_INPUT;
    }
    taps[0] = 0.0f;       taps[1] = neg_amount; taps[2] = 0.0f;
    taps[3] = neg_amount; taps[4] = center;     taps[5] = neg_amount;
    taps[6] = 0.0f;       taps[7] = neg_amount; taps[8] = 0.0f;
    return alloc_3x3_kernel(taps, out, out_len);
}
