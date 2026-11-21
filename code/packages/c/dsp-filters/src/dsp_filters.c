/*
 * dsp_filters.c — FIR, IIR, Biquad, SOS, and Butterworth Filters (pure ISO C17).
 * ===========================================================================
 *
 * Pure ISO C17 implementation with zero <math.h> or -lm dependencies.
 * All intermediate trigonometry, square roots, exponentials, and filter design
 * polynomials run in IEEE-754 double precision and check |x| <= FLT_MAX before
 * narrowing to float.
 */
#include "dsp_filters.h"

#include <float.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ================================================================== */
/* Constants & Checked Arithmetic (CWE-190)                           */
/* ================================================================== */

#define DF_PI       3.14159265358979323846264338327950288
#define DF_TWO_PI   6.28318530717958647692528676655900577
#define DF_INV_PIO2 0.63661977236758134307553505349005745
#define DF_LN2      0.69314718055994530941723212145817657
#define DF_LN10     2.30258509299404568401799145468436421
#define DF_SQRT2    1.41421356237309504880168872420969808

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

static double *alloc_doubles(size_t count) {
    size_t bytes = 0;
    if (count == 0 || !checked_mul_size(count, sizeof(double), &bytes)) {
        return NULL;
    }
    return (double *)calloc(count, sizeof(double));
}

static float *alloc_floats(size_t count) {
    size_t bytes = 0;
    if (count == 0 || !checked_mul_size(count, sizeof(float), &bytes)) {
        return NULL;
    }
    return (float *)calloc(count, sizeof(float));
}

static void zero_and_free_doubles(double *p, size_t count) {
    if (p != NULL) {
        if (count > 0) {
            volatile double *vp = (volatile double *)p;
            size_t i;
            for (i = 0; i < count; ++i) {
                vp[i] = 0.0;
            }
        }
        free(p);
    }
}

static void zero_and_free_floats(float *p, size_t count) {
    if (p != NULL) {
        if (count > 0) {
            volatile float *vp = (volatile float *)p;
            size_t i;
            for (i = 0; i < count; ++i) {
                vp[i] = 0.0f;
            }
        }
        free(p);
    }
}

void dsp_filter_free(void *p) {
    free(p);
}

/* ================================================================== */
/* Pure ISO C17 Math Helpers (Zero <math.h> / -lm)                    */
/* ================================================================== */

static int df_isfinite_f(float x) {
    return (x == x) && ((x - x) == 0.0f);
}

static int df_isfinite_d(double x) {
    return (x == x) && ((x - x) == 0.0);
}

static double df_abs_d(double x) {
    return x < 0.0 ? -x : x;
}

static int df_narrow_float(double x, float *out) {
    if (!df_isfinite_d(x) || df_abs_d(x) > (double)FLT_MAX) {
        return 0;
    }
    *out = (float)x;
    return df_isfinite_f(*out);
}

static int df_checked_mul_f(float a, float b, float *out) {
    return df_narrow_float((double)a * (double)b, out);
}

static int df_checked_add_f(float a, float b, float *out) {
    return df_narrow_float((double)a + (double)b, out);
}

static int df_checked_sub_f(float a, float b, float *out) {
    return df_narrow_float((double)a - (double)b, out);
}

static int df_narrow_array(const double *src, size_t n, float *dst) {
    size_t i;
    for (i = 0; i < n; ++i) {
        if (!df_narrow_float(src[i], &dst[i])) {
            return 0;
        }
    }
    return 1;
}

static double df_sqrt(double x) {
    double scaled;
    double factor;
    double g;
    int i;
    if (x <= 0.0 || !df_isfinite_d(x)) {
        return 0.0;
    }
    scaled = x;
    factor = 1.0;
    while (scaled < 0.25) {
        scaled *= 4.0;
        factor *= 0.5;
    }
    while (scaled >= 4.0) {
        scaled *= 0.25;
        factor *= 2.0;
    }
    g = scaled >= 1.0 ? scaled : 1.0;
    for (i = 0; i < 16; ++i) {
        g = 0.5 * (g + scaled / g);
    }
    return g * factor;
}

static void df_sincos(double x, double *sin_out, double *cos_out) {
    static const double PIO2_HI = 1.57079632673412561417;
    static const double PIO2_LO = 6.07710050650619224932e-11;
    double q_raw;
    int64_t q;
    double r;
    double z;
    double s;
    double c;
    int quad;

    if (!df_isfinite_d(x)) {
        *sin_out = 0.0;
        *cos_out = 1.0;
        return;
    }
    /* Reduce large multiples by exact 2*PI quotient first if needed. */
    if (df_abs_d(x) > 1e6) {
        int64_t turns = (int64_t)(x / DF_TWO_PI);
        x -= (double)turns * DF_TWO_PI;
    }

    q_raw = x * DF_INV_PIO2;
    q = (int64_t)(q_raw >= 0.0 ? (q_raw + 0.5) : (q_raw - 0.5));
    r = (x - (double)q * PIO2_HI) - (double)q * PIO2_LO;
    z = r * r;

    s = 1.0 / 355687428096000.0;
    s = -1.0 / 1307674368000.0 + z * s;
    s = 1.0 / 6227020800.0 + z * s;
    s = -1.0 / 39916800.0 + z * s;
    s = 1.0 / 362880.0 + z * s;
    s = -1.0 / 5040.0 + z * s;
    s = 1.0 / 120.0 + z * s;
    s = -1.0 / 6.0 + z * s;
    s = r + r * z * s;

    c = -1.0 / 6402373705728000.0;
    c = 1.0 / 20922789888000.0 + z * c;
    c = -1.0 / 87178291200.0 + z * c;
    c = 1.0 / 479001600.0 + z * c;
    c = -1.0 / 3628800.0 + z * c;
    c = 1.0 / 40320.0 + z * c;
    c = -1.0 / 720.0 + z * c;
    c = 1.0 / 24.0 + z * c;
    c = 1.0 - 0.5 * z + z * z * c;

    quad = (int)(q & 3);
    if (quad < 0) {
        quad += 4;
    }
    switch (quad) {
        case 0: *sin_out = s;  *cos_out = c;  break;
        case 1: *sin_out = c;  *cos_out = -s; break;
        case 2: *sin_out = -s; *cos_out = -c; break;
        default: *sin_out = -c; *cos_out = s; break;
    }
}

static double df_tan(double x) {
    double s;
    double c;
    df_sincos(x, &s, &c);
    if (df_abs_d(c) < 1e-15) {
        return s >= 0.0 ? 1e308 : -1e308;
    }
    return s / c;
}

static int df_exp_safe(double x, double *out) {
    int k;
    double r;
    double term = 1.0;
    double sum = 1.0;
    int i;
    if (!df_isfinite_d(x)) {
        return 0;
    }
    if (x < -745.0) {
        *out = 0.0;
        return 1;
    }
    if (x > 709.0) {
        return 0;
    }
    k = (int)(x / DF_LN2 + (x >= 0.0 ? 0.5 : -0.5));
    r = x - (double)k * DF_LN2;
    for (i = 1; i <= 20; ++i) {
        term *= r / (double)i;
        sum += term;
    }
    while (k > 0) {
        sum *= 2.0;
        --k;
    }
    while (k < 0) {
        sum *= 0.5;
        ++k;
    }
    if (!df_isfinite_d(sum)) {
        return 0;
    }
    *out = sum;
    return 1;
}

/* ================================================================== */
/* Enum & Signal Validation                                           */
/* ================================================================== */

static int is_valid_window(dsp_window_type_t w) {
    return w == DSP_WINDOW_RECTANGULAR || w == DSP_WINDOW_HAMMING ||
           w == DSP_WINDOW_HANN || w == DSP_WINDOW_BLACKMAN;
}

static int is_valid_butter_type(dsp_butter_type_t t) {
    return t == DSP_BUTTER_LOWPASS || t == DSP_BUTTER_HIGHPASS ||
           t == DSP_BUTTER_BANDPASS || t == DSP_BUTTER_BANDSTOP;
}

static dsp_filter_error_t validate_float_array(const float *arr, size_t len,
                                               size_t max_len,
                                               dsp_filter_error_t empty_err) {
    size_t i;
    if (len == 0) {
        return empty_err;
    }
    if (arr == NULL || len > max_len) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    for (i = 0; i < len; ++i) {
        if (!df_isfinite_f(arr[i])) {
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    }
    return DSP_FILTER_OK;
}

/* ================================================================== */
/* FIR Filtering (Direct & FFT)                                       */
/* ================================================================== */

dsp_filter_error_t dsp_fir(const float *signal, size_t sig_len,
                           const float *kernel, size_t ker_len,
                           float **out, size_t *out_len) {
    dsp_filter_error_t err;
    size_t sum_len = 0;
    size_t total = 0;
    double *work = NULL;
    float *tmp = NULL;
    size_t i;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_float_array(kernel, ker_len, DSP_FILTER_MAX_TAPS, DSP_FILTER_ERR_EMPTY_KERNEL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    if (!checked_add_size(sig_len, ker_len, &sum_len) || sum_len == 0) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    total = sum_len - 1;
    if (total > DSP_FILTER_MAX_SAMPLES) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    work = alloc_doubles(total);
    tmp = alloc_floats(total);
    if (work == NULL || tmp == NULL) {
        zero_and_free_doubles(work, total);
        zero_and_free_floats(tmp, total);
        return DSP_FILTER_ERR_ALLOC;
    }

    for (i = 0; i < total; ++i) {
        size_t j_lo = (i + 1 > sig_len) ? (i + 1 - sig_len) : 0;
        size_t j_hi = (i + 1 < ker_len) ? (i + 1) : ker_len;
        size_t j;
        double acc = 0.0;
        for (j = j_lo; j < j_hi; ++j) {
            acc += (double)kernel[j] * (double)signal[i - j];
        }
        work[i] = acc;
    }

    if (!df_narrow_array(work, total, tmp)) {
        zero_and_free_doubles(work, total);
        zero_and_free_floats(tmp, total);
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    zero_and_free_doubles(work, total);
    *out = tmp;
    *out_len = total;
    return DSP_FILTER_OK;
}

static void fft_radix2_inplace(double *buf, size_t n, int inverse) {
    size_t i;
    size_t j = 0;
    size_t len;
    double sign = inverse ? 1.0 : -1.0;

    for (i = 1; i < n; ++i) {
        size_t bit = n >> 1;
        while (j & bit) {
            j ^= bit;
            bit >>= 1;
        }
        j ^= bit;
        if (i < j) {
            double tr = buf[2 * i];
            double ti = buf[2 * i + 1];
            buf[2 * i] = buf[2 * j];
            buf[2 * i + 1] = buf[2 * j + 1];
            buf[2 * j] = tr;
            buf[2 * j + 1] = ti;
        }
    }

    for (len = 2; len <= n; len <<= 1) {
        size_t half = len >> 1;
        double angle_step = sign * DF_TWO_PI / (double)len;
        double wr_step;
        double wi_step;
        df_sincos(angle_step, &wi_step, &wr_step);

        for (i = 0; i < n; i += len) {
            double wr = 1.0;
            double wi = 0.0;
            size_t k;
            for (k = 0; k < half; ++k) {
                size_t idx1 = 2 * (i + k);
                size_t idx2 = 2 * (i + k + half);
                double ur = buf[idx1];
                double ui = buf[idx1 + 1];
                double vr = buf[idx2] * wr - buf[idx2 + 1] * wi;
                double vi = buf[idx2] * wi + buf[idx2 + 1] * wr;
                double next_wr;

                buf[idx1] = ur + vr;
                buf[idx1 + 1] = ui + vi;
                buf[idx2] = ur - vr;
                buf[idx2 + 1] = ui - vi;

                next_wr = wr * wr_step - wi * wi_step;
                wi = wr * wi_step + wi * wr_step;
                wr = next_wr;
            }
        }
    }

    if (inverse) {
        double inv_n = 1.0 / (double)n;
        for (i = 0; i < 2 * n; ++i) {
            buf[i] *= inv_n;
        }
    }
}

dsp_filter_error_t dsp_fir_fft(const float *signal, size_t sig_len,
                               const float *kernel, size_t ker_len,
                               float **out, size_t *out_len) {
    dsp_filter_error_t err;
    size_t sum_len = 0;
    size_t total = 0;
    size_t fft_len = 2;
    size_t two_fft = 0;
    double *a = NULL;
    double *b = NULL;
    float *tmp = NULL;
    size_t i;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_float_array(kernel, ker_len, DSP_FILTER_MAX_TAPS, DSP_FILTER_ERR_EMPTY_KERNEL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    if (!checked_add_size(sig_len, ker_len, &sum_len) || sum_len == 0) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    total = sum_len - 1;
    if (total > DSP_FILTER_MAX_SAMPLES) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    while (fft_len < total) {
        if (fft_len > SIZE_MAX / 2) {
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
        fft_len <<= 1;
    }
    if (fft_len > DSP_FILTER_MAX_SAMPLES || !checked_mul_size(fft_len, 2, &two_fft)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    a = alloc_doubles(two_fft);
    b = alloc_doubles(two_fft);
    tmp = alloc_floats(total);
    if (a == NULL || b == NULL || tmp == NULL) {
        zero_and_free_doubles(a, two_fft);
        zero_and_free_doubles(b, two_fft);
        zero_and_free_floats(tmp, total);
        return DSP_FILTER_ERR_ALLOC;
    }

    for (i = 0; i < sig_len; ++i) {
        a[2 * i] = (double)signal[i];
    }
    for (i = 0; i < ker_len; ++i) {
        b[2 * i] = (double)kernel[i];
    }

    fft_radix2_inplace(a, fft_len, 0);
    fft_radix2_inplace(b, fft_len, 0);
    for (i = 0; i < fft_len; ++i) {
        double ar = a[2 * i];
        double ai = a[2 * i + 1];
        double br = b[2 * i];
        double bi = b[2 * i + 1];
        a[2 * i] = ar * br - ai * bi;
        a[2 * i + 1] = ar * bi + ai * br;
    }
    fft_radix2_inplace(a, fft_len, 1);

    for (i = 0; i < total; ++i) {
        if (!df_narrow_float(a[2 * i], &tmp[i])) {
            zero_and_free_doubles(a, two_fft);
            zero_and_free_doubles(b, two_fft);
            zero_and_free_floats(tmp, total);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    }

    zero_and_free_doubles(a, two_fft);
    zero_and_free_doubles(b, two_fft);
    *out = tmp;
    *out_len = total;
    return DSP_FILTER_OK;
}

/* ================================================================== */
/* Windowed-Sinc FIR Design                                           */
/* ================================================================== */

static double eval_window(dsp_window_type_t window, size_t k, double m) {
    double n_f = (double)k;
    double s1;
    double c1;
    double s2;
    double c2;
    if (m == 0.0) {
        return 1.0;
    }
    switch (window) {
        case DSP_WINDOW_RECTANGULAR:
            return 1.0;
        case DSP_WINDOW_HAMMING:
            df_sincos(DF_TWO_PI * n_f / m, &s1, &c1);
            return 0.54 - 0.46 * c1;
        case DSP_WINDOW_HANN:
            df_sincos(DF_TWO_PI * n_f / m, &s1, &c1);
            return 0.5 * (1.0 - c1);
        case DSP_WINDOW_BLACKMAN:
            df_sincos(DF_TWO_PI * n_f / m, &s1, &c1);
            df_sincos(2.0 * DF_TWO_PI * n_f / m, &s2, &c2);
            return 0.42 - 0.5 * c1 + 0.08 * c2;
        default:
            return 1.0;
    }
}

static dsp_filter_error_t design_low_pass_double(double fc, size_t n,
                                                 dsp_window_type_t window,
                                                 double *h) {
    double m = (double)(n - 1);
    double centre = m * 0.5;
    double sum = 0.0;
    size_t k;

    for (k = 0; k < n; ++k) {
        double x = (double)k - centre;
        double val;
        if (x == 0.0) {
            val = 2.0 * fc;
        } else {
            double s;
            double c;
            df_sincos(DF_TWO_PI * fc * x, &s, &c);
            val = s / (DF_PI * x);
        }
        h[k] = val * eval_window(window, k, m);
        sum += h[k];
    }

    if (df_abs_d(sum) > 0.0) {
        double inv = 1.0 / sum;
        for (k = 0; k < n; ++k) {
            h[k] *= inv;
        }
    }
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_design_low_pass(float cutoff_norm, uint32_t num_taps,
                                       dsp_window_type_t window,
                                       float **out, size_t *out_len) {
    size_t n;
    double *work = NULL;
    float *tmp = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    if (!is_valid_window(window)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (num_taps < 1u || (num_taps % 2u) == 0u || (size_t)num_taps > DSP_FILTER_MAX_TAPS) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_isfinite_f(cutoff_norm) || cutoff_norm < 0.0f || cutoff_norm > 0.5f) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    n = (size_t)num_taps;
    work = alloc_doubles(n);
    tmp = alloc_floats(n);
    if (work == NULL || tmp == NULL) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_ALLOC;
    }

    design_low_pass_double((double)cutoff_norm, n, window, work);
    if (!df_narrow_array(work, n, tmp)) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    zero_and_free_doubles(work, n);
    *out = tmp;
    *out_len = n;
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_design_high_pass(float cutoff_norm, uint32_t num_taps,
                                        dsp_window_type_t window,
                                        float **out, size_t *out_len) {
    size_t n;
    size_t k;
    double *work = NULL;
    float *tmp = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    if (!is_valid_window(window)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (num_taps < 1u || (num_taps % 2u) == 0u || (size_t)num_taps > DSP_FILTER_MAX_TAPS) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_isfinite_f(cutoff_norm) || cutoff_norm < 0.0f || cutoff_norm > 0.5f) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    n = (size_t)num_taps;
    work = alloc_doubles(n);
    tmp = alloc_floats(n);
    if (work == NULL || tmp == NULL) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_ALLOC;
    }

    design_low_pass_double((double)cutoff_norm, n, window, work);
    for (k = 0; k < n; ++k) {
        work[k] = -work[k];
    }
    work[(n - 1) / 2] += 1.0;

    if (!df_narrow_array(work, n, tmp)) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    zero_and_free_doubles(work, n);
    *out = tmp;
    *out_len = n;
    return DSP_FILTER_OK;
}

static dsp_filter_error_t design_band_pass_double(double f1, double f2, size_t n,
                                                  dsp_window_type_t window,
                                                  double *h) {
    double m = (double)(n - 1);
    double centre = m * 0.5;
    double f0 = 0.5 * (f1 + f2);
    double gain = 0.0;
    double gain_abs;
    size_t k;

    for (k = 0; k < n; ++k) {
        double x = (double)k - centre;
        double val;
        if (x == 0.0) {
            val = 2.0 * (f2 - f1);
        } else {
            double s1, c1, s2, c2;
            df_sincos(DF_TWO_PI * f2 * x, &s2, &c2);
            df_sincos(DF_TWO_PI * f1 * x, &s1, &c1);
            val = (s2 - s1) / (DF_PI * x);
        }
        h[k] = val * eval_window(window, k, m);
    }

    for (k = 0; k < n; ++k) {
        double x = (double)k - centre;
        double s, c;
        df_sincos(DF_TWO_PI * f0 * x, &s, &c);
        gain += h[k] * c;
    }
    gain_abs = df_abs_d(gain);
    if (gain_abs > 0.0) {
        double inv = 1.0 / gain_abs;
        for (k = 0; k < n; ++k) {
            h[k] *= inv;
        }
    }
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_design_band_pass(float low_norm, float high_norm,
                                        uint32_t num_taps, dsp_window_type_t window,
                                        float **out, size_t *out_len) {
    size_t n;
    double *work = NULL;
    float *tmp = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    if (!is_valid_window(window)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (num_taps < 1u || (num_taps % 2u) == 0u || (size_t)num_taps > DSP_FILTER_MAX_TAPS) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_isfinite_f(low_norm) || !df_isfinite_f(high_norm)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!(low_norm >= 0.0f && low_norm < high_norm && high_norm <= 0.5f)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    n = (size_t)num_taps;
    work = alloc_doubles(n);
    tmp = alloc_floats(n);
    if (work == NULL || tmp == NULL) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_ALLOC;
    }

    design_band_pass_double((double)low_norm, (double)high_norm, n, window, work);
    if (!df_narrow_array(work, n, tmp)) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    zero_and_free_doubles(work, n);
    *out = tmp;
    *out_len = n;
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_design_band_stop(float low_norm, float high_norm,
                                        uint32_t num_taps, dsp_window_type_t window,
                                        float **out, size_t *out_len) {
    size_t n;
    size_t k;
    double *work = NULL;
    float *tmp = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    if (!is_valid_window(window)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (num_taps < 1u || (num_taps % 2u) == 0u || (size_t)num_taps > DSP_FILTER_MAX_TAPS) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_isfinite_f(low_norm) || !df_isfinite_f(high_norm)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!(low_norm >= 0.0f && low_norm < high_norm && high_norm <= 0.5f)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    n = (size_t)num_taps;
    work = alloc_doubles(n);
    tmp = alloc_floats(n);
    if (work == NULL || tmp == NULL) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_ALLOC;
    }

    design_band_pass_double((double)low_norm, (double)high_norm, n, window, work);
    for (k = 0; k < n; ++k) {
        work[k] = -work[k];
    }
    work[(n - 1) / 2] += 1.0;

    if (!df_narrow_array(work, n, tmp)) {
        zero_and_free_doubles(work, n);
        zero_and_free_floats(tmp, n);
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    zero_and_free_doubles(work, n);
    *out = tmp;
    *out_len = n;
    return DSP_FILTER_OK;
}

/* ================================================================== */
/* Direct & Zero-Phase IIR Filtering                                  */
/* ================================================================== */

static dsp_filter_error_t iir_validate_coeffs(const float *b, size_t b_len,
                                              const float *a, size_t a_len,
                                              size_t *order_out) {
    dsp_filter_error_t err;
    size_t max_len;
    err = validate_float_array(b, b_len, DSP_FILTER_MAX_TAPS, DSP_FILTER_ERR_EMPTY_KERNEL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_float_array(a, a_len, DSP_FILTER_MAX_TAPS, DSP_FILTER_ERR_EMPTY_KERNEL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    if (a[0] == 0.0f) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    max_len = (b_len > a_len) ? b_len : a_len;
    *order_out = max_len - 1;
    if (*order_out > DSP_FILTER_MAX_ORDER) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    return DSP_FILTER_OK;
}

static dsp_filter_error_t iir_run_double(const double *sig, size_t sig_len,
                                         const float *b, size_t b_len,
                                         const float *a, size_t a_len,
                                         size_t order, double *out) {
    double inv_a0 = 1.0 / (double)a[0];
    double *b_scaled = NULL;
    double *a_scaled = NULL;
    double *state = NULL;
    size_t k;
    size_t i;

    b_scaled = alloc_doubles(order + 1);
    a_scaled = alloc_doubles(order + 1);
    if (b_scaled == NULL || a_scaled == NULL) {
        zero_and_free_doubles(b_scaled, order + 1);
        zero_and_free_doubles(a_scaled, order + 1);
        return DSP_FILTER_ERR_ALLOC;
    }
    if (order > 0) {
        state = alloc_doubles(order);
        if (state == NULL) {
            zero_and_free_doubles(b_scaled, order + 1);
            zero_and_free_doubles(a_scaled, order + 1);
            return DSP_FILTER_ERR_ALLOC;
        }
    }

    for (k = 0; k < b_len; ++k) {
        b_scaled[k] = (double)b[k] * inv_a0;
        if (!df_isfinite_d(b_scaled[k])) {
            zero_and_free_doubles(b_scaled, order + 1);
            zero_and_free_doubles(a_scaled, order + 1);
            zero_and_free_doubles(state, order);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    }
    for (k = 0; k < a_len; ++k) {
        a_scaled[k] = (double)a[k] * inv_a0;
        if (!df_isfinite_d(a_scaled[k])) {
            zero_and_free_doubles(b_scaled, order + 1);
            zero_and_free_doubles(a_scaled, order + 1);
            zero_and_free_doubles(state, order);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    }

    for (i = 0; i < sig_len; ++i) {
        double x = sig[i];
        double y = b_scaled[0] * x + (order > 0 ? state[0] : 0.0);
        if (!df_isfinite_d(y)) {
            zero_and_free_doubles(b_scaled, order + 1);
            zero_and_free_doubles(a_scaled, order + 1);
            zero_and_free_doubles(state, order);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
        out[i] = y;
        if (order > 0) {
            for (k = 0; k + 1 < order; ++k) {
                state[k] = b_scaled[k + 1] * x - a_scaled[k + 1] * y + state[k + 1];
            }
            state[order - 1] = b_scaled[order] * x - a_scaled[order] * y;
        }
    }

    zero_and_free_doubles(b_scaled, order + 1);
    zero_and_free_doubles(a_scaled, order + 1);
    zero_and_free_doubles(state, order);
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_iir_into(const float *signal, size_t sig_len,
                                const float *b, size_t b_len,
                                const float *a, size_t a_len,
                                float *out) {
    dsp_filter_error_t err;
    size_t order = 0;
    double *din = NULL;
    double *dout = NULL;
    float *tmp = NULL;
    size_t i;

    if (out == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = iir_validate_coeffs(b, b_len, a, a_len, &order);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    din = alloc_doubles(sig_len);
    dout = alloc_doubles(sig_len);
    tmp = alloc_floats(sig_len);
    if (din == NULL || dout == NULL || tmp == NULL) {
        zero_and_free_doubles(din, sig_len);
        zero_and_free_doubles(dout, sig_len);
        zero_and_free_floats(tmp, sig_len);
        return DSP_FILTER_ERR_ALLOC;
    }
    for (i = 0; i < sig_len; ++i) {
        din[i] = (double)signal[i];
    }

    err = iir_run_double(din, sig_len, b, b_len, a, a_len, order, dout);
    if (err == DSP_FILTER_OK && !df_narrow_array(dout, sig_len, tmp)) {
        err = DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (err == DSP_FILTER_OK) {
        memcpy(out, tmp, sig_len * sizeof(float));
    }

    zero_and_free_doubles(din, sig_len);
    zero_and_free_doubles(dout, sig_len);
    zero_and_free_floats(tmp, sig_len);
    return err;
}

dsp_filter_error_t dsp_iir(const float *signal, size_t sig_len,
                           const float *b, size_t b_len,
                           const float *a, size_t a_len,
                           float **out, size_t *out_len) {
    float *tmp = NULL;
    dsp_filter_error_t err;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    {
        size_t order = 0;
        err = iir_validate_coeffs(b, b_len, a, a_len, &order);
        if (err != DSP_FILTER_OK) {
            return err;
        }
    }

    tmp = alloc_floats(sig_len);
    if (tmp == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    err = dsp_iir_into(signal, sig_len, b, b_len, a, a_len, tmp);
    if (err != DSP_FILTER_OK) {
        zero_and_free_floats(tmp, sig_len);
        return err;
    }
    *out = tmp;
    *out_len = sig_len;
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_filtfilt_into(const float *signal, size_t sig_len,
                                     const float *b, size_t b_len,
                                     const float *a, size_t a_len,
                                     float *out) {
    dsp_filter_error_t err;
    size_t order = 0;
    size_t max_coeff_len;
    size_t pad_len;
    int use_padding;
    size_t work_len = 0;
    double *padded = NULL;
    double *fwd = NULL;
    double *rev = NULL;
    double *bwd = NULL;
    float *tmp = NULL;
    size_t i;

    if (out == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = iir_validate_coeffs(b, b_len, a, a_len, &order);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    max_coeff_len = (b_len > a_len) ? b_len : a_len;
    pad_len = 3u * max_coeff_len;
    use_padding = (sig_len > pad_len);
    if (use_padding) {
        size_t two_pad = 0;
        if (!checked_mul_size(pad_len, 2, &two_pad) ||
            !checked_add_size(sig_len, two_pad, &work_len) ||
            work_len > DSP_FILTER_MAX_SAMPLES) {
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    } else {
        work_len = sig_len;
    }

    padded = alloc_doubles(work_len);
    fwd = alloc_doubles(work_len);
    rev = alloc_doubles(work_len);
    bwd = alloc_doubles(work_len);
    tmp = alloc_floats(sig_len);
    if (padded == NULL || fwd == NULL || rev == NULL || bwd == NULL || tmp == NULL) {
        err = DSP_FILTER_ERR_ALLOC;
        goto cleanup;
    }

    if (use_padding) {
        double x0 = (double)signal[0];
        double x_end = (double)signal[sig_len - 1];
        for (i = 0; i < pad_len; ++i) {
            padded[i] = 2.0 * x0 - (double)signal[pad_len - i];
        }
        for (i = 0; i < sig_len; ++i) {
            padded[pad_len + i] = (double)signal[i];
        }
        for (i = 0; i < pad_len; ++i) {
            padded[pad_len + sig_len + i] = 2.0 * x_end - (double)signal[sig_len - 2 - i];
        }
    } else {
        for (i = 0; i < sig_len; ++i) {
            padded[i] = (double)signal[i];
        }
    }

    err = iir_run_double(padded, work_len, b, b_len, a, a_len, order, fwd);
    if (err != DSP_FILTER_OK) {
        goto cleanup;
    }
    for (i = 0; i < work_len; ++i) {
        rev[i] = fwd[work_len - 1 - i];
    }
    err = iir_run_double(rev, work_len, b, b_len, a, a_len, order, bwd);
    if (err != DSP_FILTER_OK) {
        goto cleanup;
    }

    for (i = 0; i < sig_len; ++i) {
        size_t idx = use_padding ? (work_len - 1 - (pad_len + i)) : (sig_len - 1 - i);
        if (!df_narrow_float(bwd[idx], &tmp[i])) {
            err = DSP_FILTER_ERR_INVALID_COEFFICIENT;
            goto cleanup;
        }
    }
    memcpy(out, tmp, sig_len * sizeof(float));

cleanup:
    zero_and_free_doubles(padded, work_len);
    zero_and_free_doubles(fwd, work_len);
    zero_and_free_doubles(rev, work_len);
    zero_and_free_doubles(bwd, work_len);
    zero_and_free_floats(tmp, sig_len);
    return err;
}

dsp_filter_error_t dsp_filtfilt(const float *signal, size_t sig_len,
                                const float *b, size_t b_len,
                                const float *a, size_t a_len,
                                float **out, size_t *out_len) {
    float *tmp = NULL;
    dsp_filter_error_t err;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    {
        size_t order = 0;
        err = iir_validate_coeffs(b, b_len, a, a_len, &order);
        if (err != DSP_FILTER_OK) {
            return err;
        }
    }

    tmp = alloc_floats(sig_len);
    if (tmp == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    err = dsp_filtfilt_into(signal, sig_len, b, b_len, a, a_len, tmp);
    if (err != DSP_FILTER_OK) {
        zero_and_free_floats(tmp, sig_len);
        return err;
    }
    *out = tmp;
    *out_len = sig_len;
    return DSP_FILTER_OK;
}

/* ================================================================== */
/* Biquad (Checked DFII-T)                                            */
/* ================================================================== */

dsp_filter_error_t dsp_biquad_init(dsp_biquad_t *bq,
                                   float b0, float b1, float b2,
                                   float a0, float a1, float a2) {
    float nb0, nb1, nb2, na1, na2;
    double inv_a0;
    if (bq == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_isfinite_f(b0) || !df_isfinite_f(b1) || !df_isfinite_f(b2) ||
        !df_isfinite_f(a0) || !df_isfinite_f(a1) || !df_isfinite_f(a2)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (a0 == 0.0f) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    inv_a0 = 1.0 / (double)a0;
    if (!df_narrow_float((double)b0 * inv_a0, &nb0) ||
        !df_narrow_float((double)b1 * inv_a0, &nb1) ||
        !df_narrow_float((double)b2 * inv_a0, &nb2) ||
        !df_narrow_float((double)a1 * inv_a0, &na1) ||
        !df_narrow_float((double)a2 * inv_a0, &na2)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    bq->b0 = nb0;
    bq->b1 = nb1;
    bq->b2 = nb2;
    bq->a1 = na1;
    bq->a2 = na2;
    bq->z1 = 0.0f;
    bq->z2 = 0.0f;
    return DSP_FILTER_OK;
}

void dsp_biquad_reset(dsp_biquad_t *bq) {
    if (bq != NULL) {
        bq->z1 = 0.0f;
        bq->z2 = 0.0f;
    }
}

static int biquad_step_checked(const dsp_biquad_t *bq, float input,
                               float z1_in, float z2_in,
                               float *out_y, float *out_z1, float *out_z2) {
    float b0x = 0.0f;
    float y = 0.0f;
    float b1x = 0.0f;
    float a1y = 0.0f;
    float z1_diff = 0.0f;
    float nz1 = 0.0f;
    float b2x = 0.0f;
    float a2y = 0.0f;
    float nz2 = 0.0f;
    if (!df_isfinite_f(input)) {
        return 0;
    }
    /* Separately rounded binary32 DFII-T steps (no FMA/reassociation, DSP03). */
    if (!df_checked_mul_f(bq->b0, input, &b0x) ||
        !df_checked_add_f(b0x, z1_in, &y) ||
        !df_checked_mul_f(bq->b1, input, &b1x) ||
        !df_checked_mul_f(bq->a1, y, &a1y) ||
        !df_checked_sub_f(b1x, a1y, &z1_diff) ||
        !df_checked_add_f(z1_diff, z2_in, &nz1) ||
        !df_checked_mul_f(bq->b2, input, &b2x) ||
        !df_checked_mul_f(bq->a2, y, &a2y) ||
        !df_checked_sub_f(b2x, a2y, &nz2)) {
        return 0;
    }
    *out_y = y;
    *out_z1 = nz1;
    *out_z2 = nz2;
    return 1;
}

dsp_filter_error_t dsp_biquad_process_sample(dsp_biquad_t *bq, float input, float *out) {
    float y = 0.0f;
    float nz1 = 0.0f;
    float nz2 = 0.0f;
    if (bq == NULL || out == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_isfinite_f(bq->b0) || !df_isfinite_f(bq->b1) || !df_isfinite_f(bq->b2) ||
        !df_isfinite_f(bq->a1) || !df_isfinite_f(bq->a2) ||
        !df_isfinite_f(bq->z1) || !df_isfinite_f(bq->z2)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!biquad_step_checked(bq, input, bq->z1, bq->z2, &y, &nz1, &nz2)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    bq->z1 = nz1;
    bq->z2 = nz2;
    *out = y;
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_biquad_process_into(dsp_biquad_t *bq,
                                           const float *signal, size_t sig_len,
                                           float *out) {
    dsp_filter_error_t err;
    float *tmp = NULL;
    float z1;
    float z2;
    size_t i;

    if (bq == NULL || out == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    if (!df_isfinite_f(bq->b0) || !df_isfinite_f(bq->b1) || !df_isfinite_f(bq->b2) ||
        !df_isfinite_f(bq->a1) || !df_isfinite_f(bq->a2) ||
        !df_isfinite_f(bq->z1) || !df_isfinite_f(bq->z2)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    tmp = alloc_floats(sig_len);
    if (tmp == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    z1 = bq->z1;
    z2 = bq->z2;
    for (i = 0; i < sig_len; ++i) {
        if (!biquad_step_checked(bq, signal[i], z1, z2, &tmp[i], &z1, &z2)) {
            zero_and_free_floats(tmp, sig_len);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    }

    memcpy(out, tmp, sig_len * sizeof(float));
    bq->z1 = z1;
    bq->z2 = z2;
    zero_and_free_floats(tmp, sig_len);
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_biquad_process(dsp_biquad_t *bq,
                                      const float *signal, size_t sig_len,
                                      float **out, size_t *out_len) {
    float *tmp = NULL;
    dsp_filter_error_t err;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    if (bq == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    tmp = alloc_floats(sig_len);
    if (tmp == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    err = dsp_biquad_process_into(bq, signal, sig_len, tmp);
    if (err != DSP_FILTER_OK) {
        zero_and_free_floats(tmp, sig_len);
        return err;
    }
    *out = tmp;
    *out_len = sig_len;
    return DSP_FILTER_OK;
}

static dsp_filter_error_t biquad_init_from_doubles(dsp_biquad_t *bq,
                                                   double b0, double b1, double b2,
                                                   double a0, double a1, double a2) {
    float fb0, fb1, fb2, fa0, fa1, fa2;
    if (bq == NULL || a0 == 0.0 || !df_isfinite_d(a0)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_narrow_float(b0 / a0, &fb0) ||
        !df_narrow_float(b1 / a0, &fb1) ||
        !df_narrow_float(b2 / a0, &fb2) ||
        !df_narrow_float(a1 / a0, &fa1) ||
        !df_narrow_float(a2 / a0, &fa2)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    (void)fa0;
    bq->b0 = fb0;
    bq->b1 = fb1;
    bq->b2 = fb2;
    bq->a1 = fa1;
    bq->a2 = fa2;
    bq->z1 = 0.0f;
    bq->z2 = 0.0f;
    return DSP_FILTER_OK;
}

static int validate_cookbook_args(float freq_norm, float q) {
    if (!df_isfinite_f(freq_norm) || freq_norm <= 0.0f || freq_norm >= 0.5f) {
        return 0;
    }
    if (!df_isfinite_f(q) || q <= 0.0f) {
        return 0;
    }
    return 1;
}

dsp_filter_error_t dsp_biquad_lowpass(dsp_biquad_t *bq, float cutoff_norm, float q) {
    double omega, sin_w, cos_w, alpha;
    if (bq == NULL || !validate_cookbook_args(cutoff_norm, q)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    omega = DF_TWO_PI * (double)cutoff_norm;
    df_sincos(omega, &sin_w, &cos_w);
    alpha = sin_w / (2.0 * (double)q);
    return biquad_init_from_doubles(bq, 0.5 * (1.0 - cos_w), 1.0 - cos_w, 0.5 * (1.0 - cos_w),
                                    1.0 + alpha, -2.0 * cos_w, 1.0 - alpha);
}

dsp_filter_error_t dsp_biquad_highpass(dsp_biquad_t *bq, float cutoff_norm, float q) {
    double omega, sin_w, cos_w, alpha;
    if (bq == NULL || !validate_cookbook_args(cutoff_norm, q)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    omega = DF_TWO_PI * (double)cutoff_norm;
    df_sincos(omega, &sin_w, &cos_w);
    alpha = sin_w / (2.0 * (double)q);
    return biquad_init_from_doubles(bq, 0.5 * (1.0 + cos_w), -(1.0 + cos_w), 0.5 * (1.0 + cos_w),
                                    1.0 + alpha, -2.0 * cos_w, 1.0 - alpha);
}

dsp_filter_error_t dsp_biquad_bandpass(dsp_biquad_t *bq, float center_norm, float q) {
    double omega, sin_w, cos_w, alpha;
    if (bq == NULL || !validate_cookbook_args(center_norm, q)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    omega = DF_TWO_PI * (double)center_norm;
    df_sincos(omega, &sin_w, &cos_w);
    alpha = sin_w / (2.0 * (double)q);
    return biquad_init_from_doubles(bq, 0.5 * sin_w, 0.0, -0.5 * sin_w,
                                    1.0 + alpha, -2.0 * cos_w, 1.0 - alpha);
}

dsp_filter_error_t dsp_biquad_notch(dsp_biquad_t *bq, float center_norm, float q) {
    double omega, sin_w, cos_w, alpha;
    if (bq == NULL || !validate_cookbook_args(center_norm, q)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    omega = DF_TWO_PI * (double)center_norm;
    df_sincos(omega, &sin_w, &cos_w);
    alpha = sin_w / (2.0 * (double)q);
    return biquad_init_from_doubles(bq, 1.0, -2.0 * cos_w, 1.0,
                                    1.0 + alpha, -2.0 * cos_w, 1.0 - alpha);
}

dsp_filter_error_t dsp_biquad_allpass(dsp_biquad_t *bq, float center_norm, float q) {
    double omega, sin_w, cos_w, alpha;
    if (bq == NULL || !validate_cookbook_args(center_norm, q)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    omega = DF_TWO_PI * (double)center_norm;
    df_sincos(omega, &sin_w, &cos_w);
    alpha = sin_w / (2.0 * (double)q);
    return biquad_init_from_doubles(bq, 1.0 - alpha, -2.0 * cos_w, 1.0 + alpha,
                                    1.0 + alpha, -2.0 * cos_w, 1.0 - alpha);
}

dsp_filter_error_t dsp_biquad_peak(dsp_biquad_t *bq, float center_norm, float gain_db, float q) {
    double a_gain = 0.0;
    double omega, sin_w, cos_w, alpha;
    if (bq == NULL || !validate_cookbook_args(center_norm, q) || !df_isfinite_f(gain_db)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (!df_exp_safe(((double)gain_db / 40.0) * DF_LN10, &a_gain) || a_gain <= 0.0) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    omega = DF_TWO_PI * (double)center_norm;
    df_sincos(omega, &sin_w, &cos_w);
    alpha = sin_w / (2.0 * (double)q);
    return biquad_init_from_doubles(bq,
                                    1.0 + alpha * a_gain, -2.0 * cos_w, 1.0 - alpha * a_gain,
                                    1.0 + alpha / a_gain, -2.0 * cos_w, 1.0 - alpha / a_gain);
}

/* ================================================================== */
/* Second-Order Sections (SOS) Direct & Zero-Phase Filtering          */
/* ================================================================== */

static dsp_filter_error_t validate_sos_sections(const dsp_sos_section_t *sections,
                                                size_t num_sections,
                                                dsp_biquad_t **filters_out) {
    dsp_biquad_t *filters = NULL;
    size_t s;
    if (num_sections > DSP_FILTER_MAX_ORDER) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (num_sections == 0) {
        *filters_out = NULL;
        return DSP_FILTER_OK;
    }
    if (sections == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    filters = (dsp_biquad_t *)calloc(num_sections, sizeof(dsp_biquad_t));
    if (filters == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    for (s = 0; s < num_sections; ++s) {
        dsp_filter_error_t err = dsp_biquad_init(
            &filters[s],
            sections[s].b0, sections[s].b1, sections[s].b2,
            sections[s].a0, sections[s].a1, sections[s].a2);
        if (err != DSP_FILTER_OK) {
            free(filters);
            return err;
        }
    }
    *filters_out = filters;
    return DSP_FILTER_OK;
}

static dsp_filter_error_t sos_run_doubles(dsp_biquad_t *filters, size_t num_sections,
                                          const double *in, size_t n, double *out) {
    size_t i;
    size_t s;
    float *z1 = NULL;
    float *z2 = NULL;
    if (num_sections == 0) {
        memcpy(out, in, n * sizeof(double));
        return DSP_FILTER_OK;
    }
    z1 = alloc_floats(num_sections);
    z2 = alloc_floats(num_sections);
    if (z1 == NULL || z2 == NULL) {
        zero_and_free_floats(z1, num_sections);
        zero_and_free_floats(z2, num_sections);
        return DSP_FILTER_ERR_ALLOC;
    }
    for (i = 0; i < n; ++i) {
        float v = 0.0f;
        if (!df_narrow_float(in[i], &v)) {
            zero_and_free_floats(z1, num_sections);
            zero_and_free_floats(z2, num_sections);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
        for (s = 0; s < num_sections; ++s) {
            float y = 0.0f;
            float nz1 = 0.0f;
            float nz2 = 0.0f;
            if (!biquad_step_checked(&filters[s], v, z1[s], z2[s], &y, &nz1, &nz2)) {
                zero_and_free_floats(z1, num_sections);
                zero_and_free_floats(z2, num_sections);
                return DSP_FILTER_ERR_INVALID_COEFFICIENT;
            }
            z1[s] = nz1;
            z2[s] = nz2;
            v = y;
        }
        out[i] = (double)v;
    }
    zero_and_free_floats(z1, num_sections);
    zero_and_free_floats(z2, num_sections);
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_sosfilt_into(const dsp_sos_section_t *sections, size_t num_sections,
                                    const float *signal, size_t sig_len,
                                    float *out) {
    dsp_filter_error_t err;
    dsp_biquad_t *filters = NULL;
    double *din = NULL;
    double *dout = NULL;
    float *tmp = NULL;
    size_t i;

    if (out == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_sos_sections(sections, num_sections, &filters);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    din = alloc_doubles(sig_len);
    dout = alloc_doubles(sig_len);
    tmp = alloc_floats(sig_len);
    if (din == NULL || dout == NULL || tmp == NULL) {
        err = DSP_FILTER_ERR_ALLOC;
        goto cleanup;
    }
    for (i = 0; i < sig_len; ++i) {
        din[i] = (double)signal[i];
    }

    err = sos_run_doubles(filters, num_sections, din, sig_len, dout);
    if (err == DSP_FILTER_OK && !df_narrow_array(dout, sig_len, tmp)) {
        err = DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (err == DSP_FILTER_OK) {
        memcpy(out, tmp, sig_len * sizeof(float));
    }

cleanup:
    free(filters);
    zero_and_free_doubles(din, sig_len);
    zero_and_free_doubles(dout, sig_len);
    zero_and_free_floats(tmp, sig_len);
    return err;
}

dsp_filter_error_t dsp_sosfilt(const dsp_sos_section_t *sections, size_t num_sections,
                               const float *signal, size_t sig_len,
                               float **out, size_t *out_len) {
    dsp_filter_error_t err;
    dsp_biquad_t *filters = NULL;
    float *tmp = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_sos_sections(sections, num_sections, &filters);
    free(filters);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    tmp = alloc_floats(sig_len);
    if (tmp == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    err = dsp_sosfilt_into(sections, num_sections, signal, sig_len, tmp);
    if (err != DSP_FILTER_OK) {
        zero_and_free_floats(tmp, sig_len);
        return err;
    }
    *out = tmp;
    *out_len = sig_len;
    return DSP_FILTER_OK;
}

dsp_filter_error_t dsp_sosfiltfilt_into(const dsp_sos_section_t *sections, size_t num_sections,
                                        const float *signal, size_t sig_len,
                                        float *out) {
    dsp_filter_error_t err;
    dsp_biquad_t *filters = NULL;
    size_t pad_len;
    int use_padding;
    size_t work_len = 0;
    double *padded = NULL;
    double *fwd = NULL;
    double *rev = NULL;
    double *bwd = NULL;
    float *tmp = NULL;
    size_t i;

    if (out == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_sos_sections(sections, num_sections, &filters);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    if (num_sections == 0) {
        free(filters);
        memmove(out, signal, sig_len * sizeof(float));
        return DSP_FILTER_OK;
    }

    pad_len = 3u * (2u * num_sections + 1u);
    use_padding = (sig_len > pad_len);
    if (use_padding) {
        size_t two_pad = 0;
        if (!checked_mul_size(pad_len, 2, &two_pad) ||
            !checked_add_size(sig_len, two_pad, &work_len) ||
            work_len > DSP_FILTER_MAX_SAMPLES) {
            free(filters);
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
    } else {
        work_len = sig_len;
    }

    padded = alloc_doubles(work_len);
    fwd = alloc_doubles(work_len);
    rev = alloc_doubles(work_len);
    bwd = alloc_doubles(work_len);
    tmp = alloc_floats(sig_len);
    if (padded == NULL || fwd == NULL || rev == NULL || bwd == NULL || tmp == NULL) {
        err = DSP_FILTER_ERR_ALLOC;
        goto cleanup;
    }

    if (use_padding) {
        double x0 = (double)signal[0];
        double x_end = (double)signal[sig_len - 1];
        for (i = 0; i < pad_len; ++i) {
            padded[i] = 2.0 * x0 - (double)signal[pad_len - i];
        }
        for (i = 0; i < sig_len; ++i) {
            padded[pad_len + i] = (double)signal[i];
        }
        for (i = 0; i < pad_len; ++i) {
            padded[pad_len + sig_len + i] = 2.0 * x_end - (double)signal[sig_len - 2 - i];
        }
    } else {
        for (i = 0; i < sig_len; ++i) {
            padded[i] = (double)signal[i];
        }
    }

    err = sos_run_doubles(filters, num_sections, padded, work_len, fwd);
    if (err != DSP_FILTER_OK) {
        goto cleanup;
    }
    for (i = 0; i < work_len; ++i) {
        rev[i] = fwd[work_len - 1 - i];
    }
    err = sos_run_doubles(filters, num_sections, rev, work_len, bwd);
    if (err != DSP_FILTER_OK) {
        goto cleanup;
    }

    for (i = 0; i < sig_len; ++i) {
        size_t idx = use_padding ? (work_len - 1 - (pad_len + i)) : (sig_len - 1 - i);
        if (!df_narrow_float(bwd[idx], &tmp[i])) {
            err = DSP_FILTER_ERR_INVALID_COEFFICIENT;
            goto cleanup;
        }
    }
    memcpy(out, tmp, sig_len * sizeof(float));

cleanup:
    free(filters);
    zero_and_free_doubles(padded, work_len);
    zero_and_free_doubles(fwd, work_len);
    zero_and_free_doubles(rev, work_len);
    zero_and_free_doubles(bwd, work_len);
    zero_and_free_floats(tmp, sig_len);
    return err;
}

dsp_filter_error_t dsp_sosfiltfilt(const dsp_sos_section_t *sections, size_t num_sections,
                                   const float *signal, size_t sig_len,
                                   float **out, size_t *out_len) {
    dsp_filter_error_t err;
    dsp_biquad_t *filters = NULL;
    float *tmp = NULL;

    if (out == NULL || out_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out = NULL;
    *out_len = 0;

    err = validate_float_array(signal, sig_len, DSP_FILTER_MAX_SAMPLES, DSP_FILTER_ERR_EMPTY_SIGNAL);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    err = validate_sos_sections(sections, num_sections, &filters);
    free(filters);
    if (err != DSP_FILTER_OK) {
        return err;
    }

    tmp = alloc_floats(sig_len);
    if (tmp == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }
    err = dsp_sosfiltfilt_into(sections, num_sections, signal, sig_len, tmp);
    if (err != DSP_FILTER_OK) {
        zero_and_free_floats(tmp, sig_len);
        return err;
    }
    *out = tmp;
    *out_len = sig_len;
    return DSP_FILTER_OK;
}

/* ================================================================== */
/* Butterworth Filter Design (LP / HP / BP / BS)                      */
/* ================================================================== */

static int make_sos_from_doubles(double b0, double b1, double b2,
                                 double a0, double a1, double a2,
                                 dsp_sos_section_t *sec) {
    float fb0, fb1, fb2, fa1, fa2;
    double inv_a0;
    if (a0 == 0.0 || !df_isfinite_d(a0)) {
        return 0;
    }
    inv_a0 = 1.0 / a0;
    if (!df_narrow_float(b0 * inv_a0, &fb0) ||
        !df_narrow_float(b1 * inv_a0, &fb1) ||
        !df_narrow_float(b2 * inv_a0, &fb2) ||
        !df_narrow_float(a1 * inv_a0, &fa1) ||
        !df_narrow_float(a2 * inv_a0, &fa2)) {
        return 0;
    }
    sec->b0 = fb0;
    sec->b1 = fb1;
    sec->b2 = fb2;
    sec->a0 = 1.0f;
    sec->a1 = fa1;
    sec->a2 = fa2;
    return 1;
}

/* Complex square root: sqrt(re + i*im) returning principal root with sr >= 0. */
static void cplx_sqrt(double re, double im, double *sr, double *si) {
    double r = df_sqrt(re * re + im * im);
    if (r == 0.0) {
        *sr = 0.0;
        *si = 0.0;
        return;
    }
    if (re >= 0.0) {
        double u = df_sqrt(0.5 * (r + re));
        *sr = u;
        *si = (u > 0.0) ? (0.5 * im / u) : 0.0;
    } else {
        double v = df_sqrt(0.5 * (r - re));
        if (im < 0.0) {
            v = -v;
        }
        *si = v;
        *sr = (v != 0.0) ? (0.5 * im / v) : 0.0;
    }
}

/* Factor quadratic s^2 - (p * B) * s + w0^2 = 0 for a stable analog pole p = pr + i*pi
 * (pr < 0) into two real conjugate quadratics s^2 + c1*s + c0, and emit the two
 * bilinear-transformed biquad sections. */
static int emit_bp_bs_pair(double pr, double pi, double b_band, double w0_sq,
                           int is_bandstop,
                           dsp_sos_section_t *s1, dsp_sos_section_t *s2) {
    /* Half-coefficient h = (p * B) / 2 */
    double hr = 0.5 * pr * b_band;
    double hi = 0.5 * pi * b_band;
    /* Discriminant d = h^2 - w0_sq */
    double dr = (hr * hr - hi * hi) - w0_sq;
    double di = 2.0 * hr * hi;
    double sr = 0.0, si = 0.0;
    double u1r, u1i, u2r, u2i;
    double c1_a, c0_a, c1_b, c0_b;

    cplx_sqrt(dr, di, &sr, &si);
    u1r = hr + sr;
    u1i = hi + si;
    u2r = hr - sr;
    u2i = hi - si;

    /* Conjugate quadratic: (s - u)(s - conj(u)) = s^2 - 2*Re(u)*s + |u|^2 */
    c1_a = -2.0 * u1r;
    c0_a = u1r * u1r + u1i * u1i;
    c1_b = -2.0 * u2r;
    c0_b = u2r * u2r + u2i * u2i;

    if (!is_bandstop) {
        return make_sos_from_doubles(b_band, 0.0, -b_band,
                                     1.0 + c1_a + c0_a, 2.0 * (c0_a - 1.0), 1.0 - c1_a + c0_a, s1) &&
               make_sos_from_doubles(b_band, 0.0, -b_band,
                                     1.0 + c1_b + c0_b, 2.0 * (c0_b - 1.0), 1.0 - c1_b + c0_b, s2);
    } else {
        double nb0 = 1.0 + w0_sq;
        double nb1 = 2.0 * (w0_sq - 1.0);
        double nb2 = 1.0 + w0_sq;
        return make_sos_from_doubles(nb0, nb1, nb2,
                                     1.0 + c1_a + c0_a, 2.0 * (c0_a - 1.0), 1.0 - c1_a + c0_a, s1) &&
               make_sos_from_doubles(nb0, nb1, nb2,
                                     1.0 + c1_b + c0_b, 2.0 * (c0_b - 1.0), 1.0 - c1_b + c0_b, s2);
    }
}

dsp_filter_error_t dsp_butterworth_sos(dsp_butter_type_t type, uint32_t order,
                                       float low_cutoff_norm, float high_cutoff_norm,
                                       dsp_sos_section_t **out_sections,
                                       size_t *out_num_sections) {
    size_t n_sec = 0;
    dsp_sos_section_t *sec = NULL;
    size_t pairs;
    size_t k;
    size_t idx = 0;

    if (out_sections == NULL || out_num_sections == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *out_sections = NULL;
    *out_num_sections = 0;

    if (!is_valid_butter_type(type)) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (order == 0u || order > DSP_FILTER_MAX_BUTTER_ORDER) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }

    if (type == DSP_BUTTER_LOWPASS || type == DSP_BUTTER_HIGHPASS) {
        if (!df_isfinite_f(low_cutoff_norm) || low_cutoff_norm <= 0.0f || low_cutoff_norm >= 0.5f) {
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
        n_sec = (size_t)((order + 1u) / 2u);
    } else {
        if (!df_isfinite_f(low_cutoff_norm) || !df_isfinite_f(high_cutoff_norm)) {
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
        if (!(low_cutoff_norm > 0.0f && low_cutoff_norm < high_cutoff_norm && high_cutoff_norm < 0.5f)) {
            return DSP_FILTER_ERR_INVALID_COEFFICIENT;
        }
        n_sec = (size_t)order;
    }

    sec = (dsp_sos_section_t *)calloc(n_sec, sizeof(dsp_sos_section_t));
    if (sec == NULL) {
        return DSP_FILTER_ERR_ALLOC;
    }

    pairs = (size_t)(order / 2u);

    if (type == DSP_BUTTER_LOWPASS || type == DSP_BUTTER_HIGHPASS) {
        double k_warp = df_tan(DF_PI * (double)low_cutoff_norm);
        double k2 = k_warp * k_warp;
        int is_hp = (type == DSP_BUTTER_HIGHPASS);

        for (k = 0; k < pairs; ++k) {
            double psi = DF_PI * (double)(2u * k + 1u) / (double)(2u * order);
            double sigma;
            double cos_psi;
            double a0, a1, a2;
            df_sincos(psi, &sigma, &cos_psi);
            a0 = 1.0 + 2.0 * sigma * k_warp + k2;
            a1 = 2.0 * (k2 - 1.0);
            a2 = 1.0 - 2.0 * sigma * k_warp + k2;
            if (!is_hp) {
                if (!make_sos_from_doubles(k2, 2.0 * k2, k2, a0, a1, a2, &sec[idx++])) {
                    free(sec);
                    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
                }
            } else {
                if (!make_sos_from_doubles(1.0, -2.0, 1.0, a0, a1, a2, &sec[idx++])) {
                    free(sec);
                    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
                }
            }
        }
        if ((order & 1u) != 0u) {
            double a0 = 1.0 + k_warp;
            double a1 = k_warp - 1.0;
            if (!is_hp) {
                if (!make_sos_from_doubles(k_warp, k_warp, 0.0, a0, a1, 0.0, &sec[idx++])) {
                    free(sec);
                    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
                }
            } else {
                if (!make_sos_from_doubles(1.0, -1.0, 0.0, a0, a1, 0.0, &sec[idx++])) {
                    free(sec);
                    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
                }
            }
        }
    } else {
        double w1 = df_tan(DF_PI * (double)low_cutoff_norm);
        double w2 = df_tan(DF_PI * (double)high_cutoff_norm);
        double b_band = w2 - w1;
        double w0_sq = w1 * w2;
        int is_bs = (type == DSP_BUTTER_BANDSTOP);

        for (k = 0; k < pairs; ++k) {
            double psi = DF_PI * (double)(2u * k + 1u) / (double)(2u * order);
            double sin_psi;
            double cos_psi;
            double pr;
            double pi;
            df_sincos(psi, &sin_psi, &cos_psi);
            /* For Butterworth poles |p|=1, 1/p = conj(p) = -sin_psi - i*cos_psi. */
            pr = -sin_psi;
            pi = is_bs ? -cos_psi : cos_psi;
            if (!emit_bp_bs_pair(pr, pi, b_band, w0_sq, is_bs, &sec[idx], &sec[idx + 1])) {
                free(sec);
                return DSP_FILTER_ERR_INVALID_COEFFICIENT;
            }
            idx += 2;
        }
        if ((order & 1u) != 0u) {
            double a0 = 1.0 + b_band + w0_sq;
            double a1 = 2.0 * (w0_sq - 1.0);
            double a2 = 1.0 - b_band + w0_sq;
            if (!is_bs) {
                if (!make_sos_from_doubles(b_band, 0.0, -b_band, a0, a1, a2, &sec[idx++])) {
                    free(sec);
                    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
                }
            } else {
                double nb0 = 1.0 + w0_sq;
                double nb1 = 2.0 * (w0_sq - 1.0);
                double nb2 = 1.0 + w0_sq;
                if (!make_sos_from_doubles(nb0, nb1, nb2, a0, a1, a2, &sec[idx++])) {
                    free(sec);
                    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
                }
            }
        }
    }

    *out_sections = sec;
    *out_num_sections = n_sec;
    return DSP_FILTER_OK;
}

/* Convert 1 or 2 SOS sections into expanded (b, a) polynomials by convolution. */
static dsp_filter_error_t sos_to_ba(const dsp_sos_section_t *sec, size_t n_sec,
                                    size_t poly_len,
                                    float **b_out, size_t *b_len,
                                    float **a_out, size_t *a_len) {
    double b_acc[5] = {1.0, 0.0, 0.0, 0.0, 0.0};
    double a_acc[5] = {1.0, 0.0, 0.0, 0.0, 0.0};
    size_t cur_len = 1;
    size_t s;
    float *bf = NULL;
    float *af = NULL;

    for (s = 0; s < n_sec; ++s) {
        double sb[3];
        double sa[3];
        double nb[5] = {0.0, 0.0, 0.0, 0.0, 0.0};
        double na[5] = {0.0, 0.0, 0.0, 0.0, 0.0};
        size_t i;
        size_t j;
        sb[0] = (double)sec[s].b0;
        sb[1] = (double)sec[s].b1;
        sb[2] = (double)sec[s].b2;
        sa[0] = (double)sec[s].a0;
        sa[1] = (double)sec[s].a1;
        sa[2] = (double)sec[s].a2;
        for (i = 0; i < cur_len; ++i) {
            for (j = 0; j < 3; ++j) {
                nb[i + j] += b_acc[i] * sb[j];
                na[i + j] += a_acc[i] * sa[j];
            }
        }
        cur_len += 2;
        memcpy(b_acc, nb, sizeof(nb));
        memcpy(a_acc, na, sizeof(na));
    }

    bf = alloc_floats(poly_len);
    af = alloc_floats(poly_len);
    if (bf == NULL || af == NULL) {
        zero_and_free_floats(bf, poly_len);
        zero_and_free_floats(af, poly_len);
        return DSP_FILTER_ERR_ALLOC;
    }
    if (!df_narrow_array(b_acc, poly_len, bf) || !df_narrow_array(a_acc, poly_len, af)) {
        zero_and_free_floats(bf, poly_len);
        zero_and_free_floats(af, poly_len);
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *b_out = bf;
    *b_len = poly_len;
    *a_out = af;
    *a_len = poly_len;
    return DSP_FILTER_OK;
}

static dsp_filter_error_t butterworth_ba_impl(dsp_butter_type_t type, uint32_t order,
                                              float low_norm, float high_norm,
                                              float **b_out, size_t *b_len,
                                              float **a_out, size_t *a_len) {
    dsp_sos_section_t *sec = NULL;
    size_t n_sec = 0;
    size_t poly_len;
    dsp_filter_error_t err;

    if (b_out == NULL || b_len == NULL || a_out == NULL || a_len == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    *b_out = NULL;
    *b_len = 0;
    *a_out = NULL;
    *a_len = 0;

    if (order != 1u && order != 2u) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    err = dsp_butterworth_sos(type, order, low_norm, high_norm, &sec, &n_sec);
    if (err != DSP_FILTER_OK) {
        return err;
    }
    poly_len = (type == DSP_BUTTER_LOWPASS || type == DSP_BUTTER_HIGHPASS)
               ? (size_t)(order + 1u)
               : (size_t)(2u * order + 1u);
    err = sos_to_ba(sec, n_sec, poly_len, b_out, b_len, a_out, a_len);
    free(sec);
    return err;
}

dsp_filter_error_t dsp_butterworth_lowpass(uint32_t order, float cutoff_norm,
                                           float **b_out, size_t *b_len,
                                           float **a_out, size_t *a_len) {
    return butterworth_ba_impl(DSP_BUTTER_LOWPASS, order, cutoff_norm, 0.0f,
                               b_out, b_len, a_out, a_len);
}

dsp_filter_error_t dsp_butterworth_highpass(uint32_t order, float cutoff_norm,
                                            float **b_out, size_t *b_len,
                                            float **a_out, size_t *a_len) {
    return butterworth_ba_impl(DSP_BUTTER_HIGHPASS, order, cutoff_norm, 0.0f,
                               b_out, b_len, a_out, a_len);
}

dsp_filter_error_t dsp_butterworth_bandpass(uint32_t order, float low_norm, float high_norm,
                                            float **b_out, size_t *b_len,
                                            float **a_out, size_t *a_len) {
    return butterworth_ba_impl(DSP_BUTTER_BANDPASS, order, low_norm, high_norm,
                               b_out, b_len, a_out, a_len);
}

dsp_filter_error_t dsp_butterworth_bandstop(uint32_t order, float low_norm, float high_norm,
                                            float **b_out, size_t *b_len,
                                            float **a_out, size_t *a_len) {
    return butterworth_ba_impl(DSP_BUTTER_BANDSTOP, order, low_norm, high_norm,
                               b_out, b_len, a_out, a_len);
}

/* ================================================================== */
/* String Parsing Helpers (CWE-20 / CWE-1333 zero-regex)              */
/* ================================================================== */

static int eq_ci_trimmed(const char *s, const char *lit) {
    size_t start = 0;
    size_t end;
    size_t lit_len = strlen(lit);
    size_t i;
    if (s == NULL) {
        return 0;
    }
    while (s[start] == ' ' || s[start] == '\t' || s[start] == '\n' || s[start] == '\r') {
        start++;
    }
    end = strlen(s);
    while (end > start && (s[end - 1] == ' ' || s[end - 1] == '\t' ||
                           s[end - 1] == '\n' || s[end - 1] == '\r')) {
        end--;
    }
    if (end - start != lit_len) {
        return 0;
    }
    for (i = 0; i < lit_len; ++i) {
        char a = s[start + i];
        char b = lit[i];
        if (a >= 'A' && a <= 'Z') {
            a = (char)(a - 'A' + 'a');
        }
        if (a != b) {
            return 0;
        }
    }
    return 1;
}

dsp_filter_error_t dsp_window_parse(const char *str, dsp_window_type_t *out) {
    if (out == NULL || str == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (eq_ci_trimmed(str, "rectangular") || eq_ci_trimmed(str, "rect") || eq_ci_trimmed(str, "boxcar")) {
        *out = DSP_WINDOW_RECTANGULAR;
        return DSP_FILTER_OK;
    }
    if (eq_ci_trimmed(str, "hamming")) {
        *out = DSP_WINDOW_HAMMING;
        return DSP_FILTER_OK;
    }
    if (eq_ci_trimmed(str, "hann") || eq_ci_trimmed(str, "hanning")) {
        *out = DSP_WINDOW_HANN;
        return DSP_FILTER_OK;
    }
    if (eq_ci_trimmed(str, "blackman")) {
        *out = DSP_WINDOW_BLACKMAN;
        return DSP_FILTER_OK;
    }
    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
}

dsp_filter_error_t dsp_butter_parse_type(const char *str, dsp_butter_type_t *out) {
    if (out == NULL || str == NULL) {
        return DSP_FILTER_ERR_INVALID_COEFFICIENT;
    }
    if (eq_ci_trimmed(str, "lowpass") || eq_ci_trimmed(str, "low") || eq_ci_trimmed(str, "lp")) {
        *out = DSP_BUTTER_LOWPASS;
        return DSP_FILTER_OK;
    }
    if (eq_ci_trimmed(str, "highpass") || eq_ci_trimmed(str, "high") || eq_ci_trimmed(str, "hp")) {
        *out = DSP_BUTTER_HIGHPASS;
        return DSP_FILTER_OK;
    }
    if (eq_ci_trimmed(str, "bandpass") || eq_ci_trimmed(str, "band") || eq_ci_trimmed(str, "bp")) {
        *out = DSP_BUTTER_BANDPASS;
        return DSP_FILTER_OK;
    }
    if (eq_ci_trimmed(str, "bandstop") || eq_ci_trimmed(str, "notch") || eq_ci_trimmed(str, "bs")) {
        *out = DSP_BUTTER_BANDSTOP;
        return DSP_FILTER_OK;
    }
    return DSP_FILTER_ERR_INVALID_COEFFICIENT;
}
