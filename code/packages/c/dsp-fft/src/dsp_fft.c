/*
 * dsp_fft.c — Iterative 1-D FFT / IFFT / RFFT / IRFFT (pure ISO C17).
 * ============================================================================
 *
 * Implements radix-2 Cooley-Tukey, Bluestein chirp-Z convolution, and
 * conjugate-symmetric real transforms without `<math.h>` or recursion.
 */
#include "dsp_fft.h"

#include <stdlib.h>
#include <string.h>

/* ========================================================================== */
/* § 1. Secure Wipe, Checked Arithmetic, and Pure-ISO Trig                    */
/* ========================================================================== */

static void secure_zero_bytes(void *ptr, size_t byte_len) {
    volatile unsigned char *p = (volatile unsigned char *)ptr;
    size_t i;
    if (p == NULL) {
        return;
    }
    for (i = 0; i < byte_len; ++i) {
        p[i] = 0u;
    }
}

void dsp_fft_free_buffer(float *buf, size_t float_len) {
    size_t byte_len = 0;
    if (buf == NULL) {
        return;
    }
    if (dsp_complex_checked_mul(float_len, sizeof(float), &byte_len)) {
        secure_zero_bytes(buf, byte_len);
    }
    free(buf);
}

void dsp_fft_free(void *p) {
    free(p);
}

const char *dsp_fft_error_str(dsp_fft_error_t err) {
    switch (err) {
    case DSP_FFT_OK:
        return "ok";
    case DSP_FFT_ERR_INVALID_INPUT:
        return "invalid input";
    case DSP_FFT_ERR_NOT_POWER_OF_TWO:
        return "FFT length is not a power of two";
    case DSP_FFT_ERR_ALLOC:
        return "allocation failed";
    }
    return "unknown error";
}

int dsp_fft_is_power_of_two(size_t n) {
    return (n != 0u) && ((n & (n - 1u)) == 0u);
}

/* Pure ISO C sin/cos on double using quadrant reduction mod pi/2 + 10-term
 * Maclaurin series on |r| <= pi/4 (error < 1e-17, well below IEEE-754 double). */
static void fft_sincos_d(double x, double *sin_out, double *cos_out) {
    static const double PI = 3.14159265358979323846;
    static const double HALF_PI = 1.57079632679489661923;
    static const double TWO_OVER_PI = 0.63661977236758134308;
    double kf;
    long long k;
    double r;
    double r2;
    double s_term;
    double s_sum;
    double c_term;
    double c_sum;
    int q;
    int n;

    /* Reduce x = k * (pi/2) + r with |r| <= pi/4. */
    kf = x * TWO_OVER_PI;
    k = (long long)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    r = x - (double)k * HALF_PI;
    while (r > HALF_PI * 0.5) {
        r -= HALF_PI;
        k += 1;
    }
    while (r < -HALF_PI * 0.5) {
        r += HALF_PI;
        k -= 1;
    }
    (void)PI;

    r2 = r * r;
    s_term = r;
    s_sum = r;
    c_term = 1.0;
    c_sum = 1.0;
    for (n = 1; n <= 10; ++n) {
        s_term *= -r2 / (double)((2 * n) * (2 * n + 1));
        s_sum += s_term;
        c_term *= -r2 / (double)((2 * n - 1) * (2 * n));
        c_sum += c_term;
    }

    q = (int)(k & 3LL);
    switch (q) {
    case 0:
        *sin_out = s_sum;
        *cos_out = c_sum;
        break;
    case 1:
        *sin_out = c_sum;
        *cos_out = -s_sum;
        break;
    case 2:
        *sin_out = -s_sum;
        *cos_out = -c_sum;
        break;
    default: /* 3 */
        *sin_out = -c_sum;
        *cos_out = s_sum;
        break;
    }
}

static dsp_fft_error_t alloc_float_buf(size_t count, float **out) {
    size_t byte_len = 0;
    *out = NULL;
    if (!dsp_complex_checked_mul(count, sizeof(float), &byte_len)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = (float *)calloc(count, sizeof(float));
    if (*out == NULL) {
        return DSP_FFT_ERR_ALLOC;
    }
    return DSP_FFT_OK;
}

static int all_finite_f(const float *buf, size_t count) {
    size_t i;
    if (buf == NULL && count > 0u) {
        return 0;
    }
    for (i = 0u; i < count; ++i) {
        if (!dsp_complex_is_finite(buf[i])) {
            return 0;
        }
    }
    return 1;
}

static int is_valid_dir(dsp_fft_direction_t dir) {
    return (dir == DSP_FFT_FORWARD) || (dir == DSP_FFT_INVERSE);
}

static int validate_interleaved_bounds(const float *buf, size_t float_len,
                                       int require_pow2, size_t *out_n,
                                       dsp_fft_error_t *out_err) {
    size_t n;
    if (buf == NULL) {
        *out_err = DSP_FFT_ERR_INVALID_INPUT;
        return 0;
    }
    if ((float_len % 2u) != 0u) {
        *out_err = DSP_FFT_ERR_INVALID_INPUT;
        return 0;
    }
    if (float_len > DSP_FFT_MAX_SAMPLES) {
        *out_err = DSP_FFT_ERR_INVALID_INPUT;
        return 0;
    }
    n = float_len / 2u;
    if (n == 0u || n > DSP_FFT_MAX_LEN) {
        *out_err = DSP_FFT_ERR_INVALID_INPUT;
        return 0;
    }
    if (require_pow2 && !dsp_fft_is_power_of_two(n)) {
        *out_err = DSP_FFT_ERR_NOT_POWER_OF_TWO;
        return 0;
    }
    *out_n = n;
    *out_err = DSP_FFT_OK;
    return 1;
}

static int validate_interleaved_buf(const float *buf, size_t float_len,
                                    int require_pow2, size_t *out_n,
                                    dsp_fft_error_t *out_err) {
    if (!validate_interleaved_bounds(buf, float_len, require_pow2, out_n,
                                     out_err)) {
        return 0;
    }
    if (!all_finite_f(buf, float_len)) {
        *out_err = DSP_FFT_ERR_INVALID_INPUT;
        return 0;
    }
    return 1;
}

static int check_bluestein_bounds(size_t n, size_t *out_m, size_t *out_two_m) {
    size_t two_n = 0u;
    size_t conv_len = 0u;
    size_t m = 1u;
    size_t two_m = 0u;

    if (n == 0u || n > DSP_FFT_MAX_LEN) {
        return 0;
    }
    if (n == 1u) {
        if (out_m != NULL) {
            *out_m = 1u;
        }
        if (out_two_m != NULL) {
            *out_two_m = 2u;
        }
        return 1;
    }
    if (!dsp_complex_checked_mul(n, 2u, &two_n) || two_n < 1u) {
        return 0;
    }
    conv_len = two_n - 1u;
    while (m < conv_len) {
        if (m > DSP_FFT_MAX_LEN / 2u) {
            return 0;
        }
        m <<= 1u;
    }
    if (m > DSP_FFT_MAX_LEN || !dsp_complex_checked_mul(m, 2u, &two_m) ||
        two_m > DSP_FFT_MAX_SAMPLES) {
        return 0;
    }
    if (out_m != NULL) {
        *out_m = m;
    }
    if (out_two_m != NULL) {
        *out_two_m = two_m;
    }
    return 1;
}

/* ========================================================================== */
/* § 2. Iterative Radix-2 Cooley-Tukey Core                                   */
/* ========================================================================== */

static size_t bit_reverse(size_t x, unsigned int bits) {
    size_t r = 0u;
    unsigned int b;
    for (b = 0u; b < bits; ++b) {
        r = (r << 1u) | (x & 1u);
        x >>= 1u;
    }
    return r;
}

static void bit_reversal_permute(float *buf, size_t n) {
    unsigned int log_n = 0u;
    size_t tmp = n;
    size_t i;
    while (tmp > 1u) {
        tmp >>= 1u;
        log_n += 1u;
    }
    for (i = 0u; i < n; ++i) {
        size_t j = bit_reverse(i, log_n);
        if (j > i) {
            size_t ii = 2u * i;
            size_t jj = 2u * j;
            float tr = buf[ii];
            float ti = buf[ii + 1u];
            buf[ii] = buf[jj];
            buf[ii + 1u] = buf[jj + 1u];
            buf[jj] = tr;
            buf[jj + 1u] = ti;
        }
    }
}

static void butterflies(float *buf, size_t n, dsp_fft_direction_t dir) {
    static const double TWO_PI = 6.28318530717958647692;
    double sign = (dir == DSP_FFT_FORWARD) ? -1.0 : 1.0;
    size_t full = 2u;

    while (full <= n) {
        size_t half = full / 2u;
        size_t block_start;
        for (block_start = 0u; block_start < n; block_start += full) {
            size_t j;
            for (j = 0u; j < half; ++j) {
                double theta = sign * TWO_PI * (double)j / (double)full;
                double w_im_d = 0.0;
                double w_re_d = 1.0;
                float w_re;
                float w_im;
                size_t x_idx = 2u * (block_start + j);
                size_t y_idx = 2u * (block_start + j + half);
                float x_re = buf[x_idx];
                float x_im = buf[x_idx + 1u];
                float y_re = buf[y_idx];
                float y_im = buf[y_idx + 1u];
                float t_re;
                float t_im;

                fft_sincos_d(theta, &w_im_d, &w_re_d);
                w_re = (float)w_re_d;
                w_im = (float)w_im_d;

                t_re = w_re * y_re - w_im * y_im;
                t_im = w_re * y_im + w_im * y_re;

                buf[x_idx] = x_re + t_re;
                buf[x_idx + 1u] = x_im + t_im;
                buf[y_idx] = x_re - t_re;
                buf[y_idx + 1u] = x_im - t_im;
            }
        }
        full *= 2u;
    }
}

dsp_fft_error_t dsp_fft_scalar(const float *signal, size_t float_len,
                               float **out, size_t *out_len) {
    size_t n = 0u;
    dsp_fft_error_t err = DSP_FFT_OK;
    float *buf = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0u;

    if (!validate_interleaved_buf(signal, float_len, 1, &n, &err)) {
        return err;
    }
    err = alloc_float_buf(float_len, &buf);
    if (err != DSP_FFT_OK) {
        return err;
    }
    memcpy(buf, signal, float_len * sizeof(float));
    bit_reversal_permute(buf, n);
    butterflies(buf, n, DSP_FFT_FORWARD);
    if (!all_finite_f(buf, float_len)) {
        dsp_fft_free_buffer(buf, float_len);
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    *out = buf;
    *out_len = float_len;
    return DSP_FFT_OK;
}

dsp_fft_error_t dsp_ifft_scalar(const float *spectrum, size_t float_len,
                                float **out, size_t *out_len) {
    size_t n = 0u;
    size_t i;
    dsp_fft_error_t err = DSP_FFT_OK;
    float *buf = NULL;
    float inv_n;

    if (out == NULL || out_len == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0u;

    if (!validate_interleaved_buf(spectrum, float_len, 1, &n, &err)) {
        return err;
    }
    err = alloc_float_buf(float_len, &buf);
    if (err != DSP_FFT_OK) {
        return err;
    }
    memcpy(buf, spectrum, float_len * sizeof(float));
    bit_reversal_permute(buf, n);
    butterflies(buf, n, DSP_FFT_INVERSE);

    inv_n = 1.0f / (float)n;
    for (i = 0u; i < float_len; ++i) {
        buf[i] *= inv_n;
    }
    if (!all_finite_f(buf, float_len)) {
        dsp_fft_free_buffer(buf, float_len);
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    *out = buf;
    *out_len = float_len;
    return DSP_FFT_OK;
}

/* ========================================================================== */
/* § 3. Bluestein's Algorithm for Arbitrary N >= 1                            */
/* ========================================================================== */

static dsp_fft_error_t build_chirp(size_t n, double sign, float **out_chirp) {
    static const double PI = 3.14159265358979323846;
    size_t two_n_sz = 0u;
    uint64_t two_n;
    float *chirp = NULL;
    size_t k;
    dsp_fft_error_t err;

    if (!dsp_complex_checked_mul(n, 2u, &two_n_sz)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    err = alloc_float_buf(two_n_sz, &chirp);
    if (err != DSP_FFT_OK) {
        return err;
    }
    two_n = (uint64_t)n * 2ULL;
    for (k = 0u; k < n; ++k) {
        uint64_t k_sq = (uint64_t)k * (uint64_t)k;
        uint64_t residue = k_sq % two_n;
        double theta = sign * PI * (double)residue / (double)n;
        double s = 0.0;
        double c = 1.0;
        fft_sincos_d(theta, &s, &c);
        chirp[2u * k] = (float)c;
        chirp[2u * k + 1u] = (float)s;
    }
    *out_chirp = chirp;
    return DSP_FFT_OK;
}

dsp_fft_error_t dsp_bluestein_scalar(const float *signal, size_t float_len,
                                     dsp_fft_direction_t dir,
                                     float **out, size_t *out_len) {
    size_t n = 0u;
    size_t m = 1u;
    size_t two_n = 0u;
    size_t two_m = 0u;
    size_t k;
    double sign;
    float inv_n;
    dsp_fft_error_t err = DSP_FFT_OK;
    float *chirp = NULL;
    float *a = NULL;
    float *b = NULL;
    float *a_spec = NULL;
    float *b_spec = NULL;
    float *c_spec = NULL;
    float *conv = NULL;
    float *res = NULL;
    size_t dummy_len = 0u;

    if (out == NULL || out_len == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0u;

    if (!is_valid_dir(dir)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!validate_interleaved_bounds(signal, float_len, 0, &n, &err)) {
        return err;
    }
    if (!check_bluestein_bounds(n, &m, &two_m)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!all_finite_f(signal, float_len)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (n == 1u) {
        err = alloc_float_buf(2u, &res);
        if (err != DSP_FFT_OK) {
            return err;
        }
        res[0] = signal[0];
        res[1] = signal[1];
        *out = res;
        *out_len = 2u;
        return DSP_FFT_OK;
    }

    two_n = float_len;

    sign = (dir == DSP_FFT_FORWARD) ? -1.0 : 1.0;
    err = build_chirp(n, sign, &chirp);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }

    err = alloc_float_buf(two_m, &a);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }
    err = alloc_float_buf(two_m, &b);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }
    err = alloc_float_buf(two_m, &c_spec);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }
    err = alloc_float_buf(two_n, &res);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }

    for (k = 0u; k < n; ++k) {
        float xr = signal[2u * k];
        float xi = signal[2u * k + 1u];
        float cr = chirp[2u * k];
        float ci = chirp[2u * k + 1u];
        a[2u * k] = xr * cr - xi * ci;
        a[2u * k + 1u] = xr * ci + xi * cr;
    }

    for (k = 0u; k < n; ++k) {
        b[2u * k] = chirp[2u * k];
        b[2u * k + 1u] = -chirp[2u * k + 1u];
    }
    for (k = 1u; k < n; ++k) {
        size_t idx = m - k;
        b[2u * idx] = chirp[2u * k];
        b[2u * idx + 1u] = -chirp[2u * k + 1u];
    }

    err = dsp_fft_scalar(a, two_m, &a_spec, &dummy_len);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }
    err = dsp_fft_scalar(b, two_m, &b_spec, &dummy_len);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }

    for (k = 0u; k < m; ++k) {
        float ar = a_spec[2u * k];
        float ai = a_spec[2u * k + 1u];
        float br = b_spec[2u * k];
        float bi = b_spec[2u * k + 1u];
        c_spec[2u * k] = ar * br - ai * bi;
        c_spec[2u * k + 1u] = ar * bi + ai * br;
    }

    err = dsp_ifft_scalar(c_spec, two_m, &conv, &dummy_len);
    if (err != DSP_FFT_OK) {
        goto cleanup;
    }

    inv_n = (dir == DSP_FFT_INVERSE) ? (1.0f / (float)n) : 1.0f;
    for (k = 0u; k < n; ++k) {
        float cr = conv[2u * k];
        float ci = conv[2u * k + 1u];
        float wr = chirp[2u * k];
        float wi = chirp[2u * k + 1u];
        float re = wr * cr - wi * ci;
        float im = wr * ci + wi * cr;
        res[2u * k] = re * inv_n;
        res[2u * k + 1u] = im * inv_n;
    }
    if (!all_finite_f(res, two_n)) {
        err = DSP_FFT_ERR_INVALID_INPUT;
        goto cleanup;
    }

    *out = res;
    *out_len = two_n;
    res = NULL;
    err = DSP_FFT_OK;

cleanup:
    dsp_fft_free_buffer(chirp, two_n);
    dsp_fft_free_buffer(a, two_m);
    dsp_fft_free_buffer(b, two_m);
    dsp_fft_free_buffer(a_spec, two_m);
    dsp_fft_free_buffer(b_spec, two_m);
    dsp_fft_free_buffer(c_spec, two_m);
    dsp_fft_free_buffer(conv, two_m);
    if (res != NULL) {
        dsp_fft_free_buffer(res, two_n);
    }
    return err;
}

/* ========================================================================== */
/* § 4. Real-Input FFT (RFFT / IRFFT)                                         */
/* ========================================================================== */

static dsp_fft_error_t run_complex_fft_auto(const float *interleaved, size_t n,
                                            dsp_fft_direction_t dir,
                                            float **out, size_t *out_len) {
    size_t float_len = 0u;
    if (!dsp_complex_checked_mul(n, 2u, &float_len)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (dsp_fft_is_power_of_two(n)) {
        return (dir == DSP_FFT_FORWARD)
                   ? dsp_fft_scalar(interleaved, float_len, out, out_len)
                   : dsp_ifft_scalar(interleaved, float_len, out, out_len);
    }
    return dsp_bluestein_scalar(interleaved, float_len, dir, out, out_len);
}

dsp_fft_error_t dsp_rfft_scalar(const float *signal, size_t signal_len,
                                float **out, size_t *out_len) {
    size_t i;
    size_t half_bins;
    size_t out_floats = 0u;
    size_t two_n = 0u;
    float *interleaved = NULL;
    float *full = NULL;
    size_t full_len = 0u;
    float *res = NULL;
    dsp_fft_error_t err = DSP_FFT_OK;

    if (out == NULL || out_len == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0u;

    if (signal == NULL || signal_len == 0u || signal_len > DSP_FFT_MAX_LEN ||
        signal_len > DSP_FFT_MAX_SAMPLES) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!dsp_complex_checked_mul(signal_len, 2u, &two_n) ||
        two_n > DSP_FFT_MAX_SAMPLES) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!dsp_fft_is_power_of_two(signal_len) &&
        !check_bluestein_bounds(signal_len, NULL, NULL)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    half_bins = signal_len / 2u + 1u;
    if (!dsp_complex_checked_mul(half_bins, 2u, &out_floats)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!all_finite_f(signal, signal_len)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    err = alloc_float_buf(two_n, &interleaved);
    if (err != DSP_FFT_OK) {
        return err;
    }
    for (i = 0u; i < signal_len; ++i) {
        interleaved[2u * i] = signal[i];
        interleaved[2u * i + 1u] = 0.0f;
    }

    err = run_complex_fft_auto(interleaved, signal_len, DSP_FFT_FORWARD,
                               &full, &full_len);
    dsp_fft_free_buffer(interleaved, two_n);
    if (err != DSP_FFT_OK) {
        return err;
    }

    err = alloc_float_buf(out_floats, &res);
    if (err != DSP_FFT_OK) {
        dsp_fft_free_buffer(full, full_len);
        return err;
    }
    memcpy(res, full, out_floats * sizeof(float));
    dsp_fft_free_buffer(full, full_len);

    *out = res;
    *out_len = out_floats;
    return DSP_FFT_OK;
}

dsp_fft_error_t dsp_irfft_scalar(const float *half_spectrum, size_t float_len,
                                 uint32_t output_length,
                                 float **out, size_t *out_len) {
    size_t n = (size_t)output_length;
    size_t half_bins;
    size_t expected_half_bins;
    size_t two_n = 0u;
    size_t reflect_end;
    size_t k;
    float *full = NULL;
    float *inverted = NULL;
    size_t inv_len = 0u;
    float *res = NULL;
    dsp_fft_error_t err = DSP_FFT_OK;

    if (out == NULL || out_len == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0u;

    if (half_spectrum == NULL || n == 0u || n > DSP_FFT_MAX_LEN ||
        float_len > DSP_FFT_MAX_SAMPLES || (float_len % 2u) != 0u) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    half_bins = float_len / 2u;
    expected_half_bins = n / 2u + 1u;
    if (half_bins != expected_half_bins) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!dsp_complex_checked_mul(n, 2u, &two_n) || two_n > DSP_FFT_MAX_SAMPLES) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!dsp_fft_is_power_of_two(n) &&
        !check_bluestein_bounds(n, NULL, NULL)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!all_finite_f(half_spectrum, float_len)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    err = alloc_float_buf(two_n, &full);
    if (err != DSP_FFT_OK) {
        return err;
    }
    memcpy(full, half_spectrum, float_len * sizeof(float));

    reflect_end = (n + 1u) / 2u;
    for (k = 1u; k < reflect_end; ++k) {
        size_t dst = n - k;
        full[2u * dst] = full[2u * k];
        full[2u * dst + 1u] = -full[2u * k + 1u];
    }

    err = run_complex_fft_auto(full, n, DSP_FFT_INVERSE, &inverted, &inv_len);
    dsp_fft_free_buffer(full, two_n);
    if (err != DSP_FFT_OK) {
        return err;
    }

    err = alloc_float_buf(n, &res);
    if (err != DSP_FFT_OK) {
        dsp_fft_free_buffer(inverted, inv_len);
        return err;
    }
    for (k = 0u; k < n; ++k) {
        res[k] = inverted[2u * k];
    }
    dsp_fft_free_buffer(inverted, inv_len);
    if (!all_finite_f(res, n)) {
        dsp_fft_free_buffer(res, n);
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    *out = res;
    *out_len = n;
    return DSP_FFT_OK;
}

/* ========================================================================== */
/* § 5. High-Level ComplexTensor API                                          */
/* ========================================================================== */

static dsp_fft_error_t map_complex_err(dsp_complex_error_t ce) {
    if (ce == DSP_COMPLEX_OK) {
        return DSP_FFT_OK;
    }
    if (ce == DSP_COMPLEX_ERR_ALLOC) {
        return DSP_FFT_ERR_ALLOC;
    }
    return DSP_FFT_ERR_INVALID_INPUT;
}

dsp_fft_error_t dsp_fft(const float *signal, size_t len, int is_complex,
                        dsp_complex_tensor_t *out) {
    float *buf = NULL;
    size_t buf_len = 0u;
    dsp_fft_error_t err = DSP_FFT_OK;
    dsp_complex_error_t ce;

    if (out == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    out->data = NULL;
    out->len = 0u;

    if (signal == NULL || len == 0u) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    if (is_complex) {
        size_t n = 0u;
        if (!validate_interleaved_bounds(signal, len, 0, &n, &err)) {
            return err;
        }
        if (!dsp_fft_is_power_of_two(n) &&
            !check_bluestein_bounds(n, NULL, NULL)) {
            return DSP_FFT_ERR_INVALID_INPUT;
        }
        if (!all_finite_f(signal, len)) {
            return DSP_FFT_ERR_INVALID_INPUT;
        }
        err = run_complex_fft_auto(signal, n, DSP_FFT_FORWARD, &buf, &buf_len);
        if (err != DSP_FFT_OK) {
            return err;
        }
    } else {
        size_t two_n = 0u;
        size_t i;
        float *interleaved = NULL;
        if (len > DSP_FFT_MAX_LEN || len > DSP_FFT_MAX_SAMPLES) {
            return DSP_FFT_ERR_INVALID_INPUT;
        }
        if (!dsp_complex_checked_mul(len, 2u, &two_n) ||
            two_n > DSP_FFT_MAX_SAMPLES) {
            return DSP_FFT_ERR_INVALID_INPUT;
        }
        if (!dsp_fft_is_power_of_two(len) &&
            !check_bluestein_bounds(len, NULL, NULL)) {
            return DSP_FFT_ERR_INVALID_INPUT;
        }
        if (!all_finite_f(signal, len)) {
            return DSP_FFT_ERR_INVALID_INPUT;
        }
        err = alloc_float_buf(two_n, &interleaved);
        if (err != DSP_FFT_OK) {
            return err;
        }
        for (i = 0u; i < len; ++i) {
            interleaved[2u * i] = signal[i];
            interleaved[2u * i + 1u] = 0.0f;
        }
        err = run_complex_fft_auto(interleaved, len, DSP_FFT_FORWARD,
                                   &buf, &buf_len);
        dsp_fft_free_buffer(interleaved, two_n);
        if (err != DSP_FFT_OK) {
            return err;
        }
    }

    ce = dsp_complex_tensor_from_interleaved(buf, buf_len, out);
    dsp_fft_free_buffer(buf, buf_len);
    return map_complex_err(ce);
}

dsp_fft_error_t dsp_ifft(const dsp_complex_tensor_t *spectrum,
                         dsp_complex_tensor_t *out) {
    size_t n;
    size_t float_len = 0u;
    const float *data;
    float *buf = NULL;
    size_t buf_len = 0u;
    dsp_fft_error_t err;
    dsp_complex_error_t ce;
    dsp_complex_tensor_t tmp = {NULL, 0u};

    if (out == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    if (spectrum == NULL || spectrum->len == 0u ||
        spectrum->len > DSP_FFT_MAX_LEN || spectrum->data == NULL) {
        if (out != spectrum) {
            out->data = NULL;
            out->len = 0u;
        }
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    n = spectrum->len;
    if (!dsp_complex_checked_mul(n, 2u, &float_len) ||
        float_len > DSP_FFT_MAX_SAMPLES ||
        (!dsp_fft_is_power_of_two(n) &&
         !check_bluestein_bounds(n, NULL, NULL))) {
        if (out != spectrum) {
            out->data = NULL;
            out->len = 0u;
        }
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    data = spectrum->data;
    if (!all_finite_f(data, float_len)) {
        if (out != spectrum) {
            out->data = NULL;
            out->len = 0u;
        }
        return DSP_FFT_ERR_INVALID_INPUT;
    }

    err = run_complex_fft_auto(data, n, DSP_FFT_INVERSE, &buf, &buf_len);
    if (err != DSP_FFT_OK) {
        if (out != spectrum) {
            out->data = NULL;
            out->len = 0u;
        }
        return err;
    }
    ce = dsp_complex_tensor_from_interleaved(buf, buf_len, &tmp);
    dsp_fft_free_buffer(buf, buf_len);
    if (ce != DSP_COMPLEX_OK) {
        if (out != spectrum) {
            out->data = NULL;
            out->len = 0u;
        }
        return map_complex_err(ce);
    }
    if (out == spectrum) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_FFT_OK;
}

dsp_fft_error_t dsp_rfft(const float *signal, size_t len,
                         dsp_complex_tensor_t *out) {
    float *buf = NULL;
    size_t buf_len = 0u;
    dsp_fft_error_t err;
    dsp_complex_error_t ce;

    if (out == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    out->data = NULL;
    out->len = 0u;

    err = dsp_rfft_scalar(signal, len, &buf, &buf_len);
    if (err != DSP_FFT_OK) {
        return err;
    }
    ce = dsp_complex_tensor_from_interleaved(buf, buf_len, out);
    dsp_fft_free_buffer(buf, buf_len);
    return map_complex_err(ce);
}

dsp_fft_error_t dsp_irfft(const dsp_complex_tensor_t *half_spectrum,
                          uint32_t output_length,
                          float **out, size_t *out_len) {
    size_t float_len = 0u;
    if (out == NULL || out_len == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0u;
    if (half_spectrum == NULL || half_spectrum->len == 0u ||
        half_spectrum->data == NULL) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    if (!dsp_complex_checked_mul(half_spectrum->len, 2u, &float_len)) {
        return DSP_FFT_ERR_INVALID_INPUT;
    }
    return dsp_irfft_scalar(half_spectrum->data, float_len, output_length,
                            out, out_len);
}
