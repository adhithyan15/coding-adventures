/*
 * dsp_dct.c — Discrete Cosine & Sine Transforms (pure ISO C17, zero <math.h>).
 * ===========================================================================
 *
 * MATHEMATICAL OVERVIEW
 * ---------------------
 * All eight classical discrete trigonometric transforms (DCT-I..IV, DST-I..IV)
 * project a finite real sequence x[0..N-1] onto an orthogonal basis of
 * sampled cosines or sines whose frequencies and phase shifts correspond to
 * different boundary symmetry conditions:
 *
 *   Type | Left boundary        | Right boundary       | Domain
 *   -----+----------------------+----------------------+--------
 *   DCT-I| even at n=0          | even at n=N-1        | N >= 2
 *   DCT-II| even at n=-1/2      | even at n=N-1/2      | N >= 1
 *   DCT-III| even at n=0        | odd at n=N           | N >= 1
 *   DCT-IV| even at n=-1/2      | odd at n=N-1/2       | N >= 1
 *   DST-I| odd at n=-1          | odd at n=N           | N >= 2
 *   DST-II| odd at n=-1/2       | odd at n=N-1/2       | N >= 1
 *   DST-III| odd at n=0         | even at n=N          | N >= 1
 *   DST-IV| odd at n=-1/2       | even at n=N-1/2      | N >= 1
 *
 * FAST ALGORITHM (DCT-II / DCT-III)
 * ---------------------------------
 * For DCT-II and DCT-III we use Makhoul's (1980) permutation reduction to a
 * length-N DFT, computed by an internal iterative radix-2 Cooley-Tukey FFT
 * (when N is a power of two) or Bluestein's chirp-z FFT (for arbitrary N).
 * Intermediate arithmetic runs in IEEE-754 double precision; outputs are
 * range-checked against (double)FLT_MAX before narrowing to float to avoid
 * ISO C17 §6.3.1.5p2 float-cast-overflow UB.
 */
#include "dsp_dct.h"

#include <float.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ================================================================== */
/* Constants & Checked Arithmetic (CWE-190)                           */
/* ================================================================== */

#define DD_PI       3.14159265358979323846264338327950288
#define DD_TWO_PI   6.28318530717958647692528676655900577
#define DD_HALF_PI  1.57079632679489661923132169163975144
#define DD_INV_PIO2 0.63661977236758134307553505349005745

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

void dsp_dct_free(void *p) {
    free(p);
}

/* ================================================================== */
/* Pure ISO C17 Floating-Point & Trig Helpers (Zero <math.h> / -lm)   */
/* ================================================================== */

static int dd_isfinite_f(float x) {
    return (x == x) && ((x - x) == 0.0f);
}

static int dd_isfinite_d(double x) {
    return (x == x) && ((x - x) == 0.0);
}

static double dd_abs_d(double x) {
    return x < 0.0 ? -x : x;
}

/* Safe double -> float narrowing: checks finiteness AND |x| <= FLT_MAX in
 * double before casting so float-cast-overflow UB is impossible. */
static int dd_narrow_float(double x, float *out) {
    if (!dd_isfinite_d(x) || dd_abs_d(x) > (double)FLT_MAX) {
        return 0;
    }
    *out = (float)x;
    return dd_isfinite_f(*out);
}

static int dd_narrow_array(const double *src, size_t n, float *dst) {
    size_t i;
    for (i = 0; i < n; ++i) {
        if (!dd_narrow_float(src[i], &dst[i])) {
            return 0;
        }
    }
    return 1;
}

/* Newton-Raphson square root in double precision (~1e-16 relative). */
static double dd_sqrt(double x) {
    double scaled;
    double factor;
    double g;
    int i;
    if (x <= 0.0 || !dd_isfinite_d(x)) {
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

/* High-accuracy sin/cos via quadrant reduction to [-pi/4, pi/4] and degree-19
 * Horner Maclaurin kernels. Angles in DCT/DST/Bluestein are pre-reduced to
 * [0, 2*pi) or small multiples of pi, so nearest-quarter-turn reduction is exact. */
static void dd_sincos(double x, double *sin_out, double *cos_out) {
    static const double PIO2_HI = 1.57079632673412561417;
    static const double PIO2_LO = 6.07710050650619224932e-11;
    double q_raw;
    int64_t q;
    double r;
    double z;
    double s;
    double c;
    int quad;

    if (!dd_isfinite_d(x)) {
        *sin_out = 0.0;
        *cos_out = 1.0;
        return;
    }

    q_raw = x * DD_INV_PIO2;
    q = (int64_t)(q_raw >= 0.0 ? (q_raw + 0.5) : (q_raw - 0.5));
    r = (x - (double)q * PIO2_HI) - (double)q * PIO2_LO;
    z = r * r;

    /* sin(r) = r * (1 + z*(-1/6 + z*(1/120 + ...))) up to r^17 */
    s = 1.0 / 355687428096000.0;             /* +1/17! */
    s = -1.0 / 1307674368000.0 + z * s;      /* -1/15! */
    s = 1.0 / 6227020800.0 + z * s;          /* +1/13! */
    s = -1.0 / 39916800.0 + z * s;           /* -1/11! */
    s = 1.0 / 362880.0 + z * s;              /* +1/9!  */
    s = -1.0 / 5040.0 + z * s;               /* -1/7!  */
    s = 1.0 / 120.0 + z * s;                 /* +1/5!  */
    s = -1.0 / 6.0 + z * s;                  /* -1/3!  */
    s = r + r * z * s;

    /* cos(r) = 1 - z/2 + z^2*(1/4! - z/6! + ... + z^7/18!) */
    c = -1.0 / 6402373705728000.0;           /* -1/18! */
    c = 1.0 / 20922789888000.0 + z * c;      /* +1/16! */
    c = -1.0 / 87178291200.0 + z * c;        /* -1/14! */
    c = 1.0 / 479001600.0 + z * c;           /* +1/12! */
    c = -1.0 / 3628800.0 + z * c;            /* -1/10! */
    c = 1.0 / 40320.0 + z * c;               /* +1/8!  */
    c = -1.0 / 720.0 + z * c;                /* -1/6!  */
    c = 1.0 / 24.0 + z * c;                  /* +1/4!  */
    c = 1.0 - 0.5 * z + z * z * c;

    quad = (int)(q & 3);
    if (quad < 0) {
        quad += 4;
    }
    switch (quad) {
        case 0:
            *sin_out = s;
            *cos_out = c;
            break;
        case 1:
            *sin_out = c;
            *cos_out = -s;
            break;
        case 2:
            *sin_out = -s;
            *cos_out = -c;
            break;
        default:
            *sin_out = -c;
            *cos_out = s;
            break;
    }
}

/* Compute cos/sin for rational multiples of pi: theta = PI * num / den,
 * reducing num modulo 2*den in uint64_t so large indices never lose precision. */
static void dd_sincos_pi_frac(uint64_t num, uint64_t den, double *sin_out, double *cos_out) {
    uint64_t mod = 2u * den;
    uint64_t r = num % mod;
    double theta = (DD_PI * (double)r) / (double)den;
    dd_sincos(theta, sin_out, cos_out);
}

/* ================================================================== */
/* Enum & Input Validation Helpers                                    */
/* ================================================================== */

static int is_valid_dct_type(dsp_dct_type_t t) {
    return t == DSP_DCT_I || t == DSP_DCT_II || t == DSP_DCT_III || t == DSP_DCT_IV;
}

static int is_valid_dst_type(dsp_dst_type_t t) {
    return t == DSP_DST_I || t == DSP_DST_II || t == DSP_DST_III || t == DSP_DST_IV;
}

static int is_valid_norm(dsp_dct_norm_t n) {
    return n == DSP_DCT_NORM_NONE || n == DSP_DCT_NORM_ORTHO;
}

static dsp_dct_error_t validate_1d_common(const float *buf, size_t len, int min_len) {
    size_t i;
    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (buf == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (len > DSP_DCT_MAX_LEN || len > DSP_DCT_MAX_SAMPLES) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (len < (size_t)min_len) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    for (i = 0; i < len; ++i) {
        if (!dd_isfinite_f(buf[i])) {
            return DSP_DCT_ERR_INVALID_INPUT;
        }
    }
    return DSP_DCT_OK;
}

/* ================================================================== */
/* Internal Iterative Radix-2 + Bluestein FFT (double precision)      */
/* ================================================================== */

static int is_pow2_size(size_t n) {
    return n >= 1 && (n & (n - 1)) == 0;
}

static int next_pow2_size(size_t n, size_t *out) {
    size_t p = 1;
    while (p < n) {
        if (p > SIZE_MAX / 2) {
            return 0;
        }
        p <<= 1;
    }
    *out = p;
    return 1;
}

/* In-place iterative Cooley-Tukey radix-2 FFT / IFFT on interleaved `buf` of
 * `2 * n` doubles (`n` must be a power of 2). `inverse != 0` scales by `1/n`. */
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
        double angle_step = sign * DD_TWO_PI / (double)len;
        double w_step_r;
        double w_step_i;
        dd_sincos(angle_step, &w_step_i, &w_step_r);

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

                next_wr = wr * w_step_r - wi * w_step_i;
                wi = wr * w_step_i + wi * w_step_r;
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

/* Arbitrary-N FFT/IFFT via Bluestein's chirp-z algorithm (iterative).
 * `in` and `out` are interleaved `2 * n` doubles (may alias). */
static dsp_dct_error_t fft_complex_double(const double *in, size_t n, int inverse, double *out) {
    size_t two_n = 0;
    size_t m_min = 0;
    size_t m = 0;
    size_t two_m = 0;
    double *a = NULL;
    double *b = NULL;
    double *chirp = NULL;
    size_t k;

    if (!checked_mul_size(n, 2, &two_n)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }

    if (n == 1) {
        out[0] = in[0];
        out[1] = in[1];
        return DSP_DCT_OK;
    }

    if (is_pow2_size(n)) {
        if (out != in) {
            memcpy(out, in, two_n * sizeof(double));
        }
        fft_radix2_inplace(out, n, inverse);
        return DSP_DCT_OK;
    }

    /* Bluestein convolution length m >= 2*n - 1, power of 2. */
    m_min = two_n - 1;
    if (!next_pow2_size(m_min, &m) || !checked_mul_size(m, 2, &two_m)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }

    a = alloc_doubles(two_m);
    b = alloc_doubles(two_m);
    chirp = alloc_doubles(two_n);
    if (a == NULL || b == NULL || chirp == NULL) {
        zero_and_free_doubles(a, two_m);
        zero_and_free_doubles(b, two_m);
        zero_and_free_doubles(chirp, two_n);
        return DSP_DCT_ERR_ALLOC;
    }

    /* chirp[k] = exp(-/+ i * pi * k^2 / n) for forward / inverse. */
    for (k = 0; k < n; ++k) {
        uint64_t k2 = (uint64_t)k * (uint64_t)k;
        double s;
        double c;
        dd_sincos_pi_frac(k2, (uint64_t)n, &s, &c);
        chirp[2 * k] = c;
        chirp[2 * k + 1] = inverse ? s : -s;
    }

    for (k = 0; k < n; ++k) {
        double xr = in[2 * k];
        double xi = in[2 * k + 1];
        double cr = chirp[2 * k];
        double ci = chirp[2 * k + 1];
        a[2 * k] = xr * cr - xi * ci;
        a[2 * k + 1] = xr * ci + xi * cr;
    }

    b[0] = chirp[0];
    b[1] = -chirp[1];
    for (k = 1; k < n; ++k) {
        double cr = chirp[2 * k];
        double ci = -chirp[2 * k + 1]; /* conjugate */
        b[2 * k] = cr;
        b[2 * k + 1] = ci;
        b[2 * (m - k)] = cr;
        b[2 * (m - k) + 1] = ci;
    }

    fft_radix2_inplace(a, m, 0);
    fft_radix2_inplace(b, m, 0);
    for (k = 0; k < m; ++k) {
        double ar = a[2 * k];
        double ai = a[2 * k + 1];
        double br = b[2 * k];
        double bi = b[2 * k + 1];
        a[2 * k] = ar * br - ai * bi;
        a[2 * k + 1] = ar * bi + ai * br;
    }
    fft_radix2_inplace(a, m, 1);

    for (k = 0; k < n; ++k) {
        double ar = a[2 * k];
        double ai = a[2 * k + 1];
        double cr = chirp[2 * k];
        double ci = chirp[2 * k + 1];
        double yr = ar * cr - ai * ci;
        double yi = ar * ci + ai * cr;
        if (inverse) {
            double inv_n = 1.0 / (double)n;
            yr *= inv_n;
            yi *= inv_n;
        }
        out[2 * k] = yr;
        out[2 * k + 1] = yi;
    }

    zero_and_free_doubles(a, two_m);
    zero_and_free_doubles(b, two_m);
    zero_and_free_doubles(chirp, two_n);
    return DSP_DCT_OK;
}

/* ================================================================== */
/* 1-D Transform Cores (operating on double arrays of length n)       */
/* ================================================================== */

/* DCT-II via Makhoul's reduction in O(N log N). */
static dsp_dct_error_t dct_ii_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    size_t two_n = 0;
    double *interleaved = NULL;
    size_t half_even = (n + 1) / 2;
    size_t half_odd = n / 2;
    size_t m;
    size_t k;
    double s0;
    double sk;
    dsp_dct_error_t err;

    if (!checked_mul_size(n, 2, &two_n)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    interleaved = alloc_doubles(two_n);
    if (interleaved == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }

    for (m = 0; m < half_even; ++m) {
        interleaved[2 * m] = x[2 * m];
    }
    for (m = 0; m < half_odd; ++m) {
        interleaved[2 * (n - 1 - m)] = x[2 * m + 1];
    }

    err = fft_complex_double(interleaved, n, 0, interleaved);
    if (err != DSP_DCT_OK) {
        zero_and_free_doubles(interleaved, two_n);
        return err;
    }

    if (norm == DSP_DCT_NORM_ORTHO) {
        s0 = 1.0 / dd_sqrt((double)n);
        sk = dd_sqrt(2.0 / (double)n);
    } else {
        s0 = 2.0;
        sk = 2.0;
    }

    for (k = 0; k < n; ++k) {
        double sin_t;
        double cos_t;
        double y_re = interleaved[2 * k];
        double y_im = interleaved[2 * k + 1];
        double scale = (k == 0) ? s0 : sk;
        dd_sincos_pi_frac((uint64_t)k, (uint64_t)(2 * n), &sin_t, &cos_t);
        out[k] = scale * (y_re * cos_t + y_im * sin_t);
    }

    zero_and_free_doubles(interleaved, two_n);
    return DSP_DCT_OK;
}

/* Un-normalised DCT-III via inverse Makhoul in O(N log N). */
static dsp_dct_error_t dct_iii_unnorm_core(const double *x, size_t n, double *out) {
    size_t two_n = 0;
    double *interleaved = NULL;
    size_t half_even = (n + 1) / 2;
    size_t half_odd = n / 2;
    size_t k;
    size_t m;
    double n_d = (double)n;
    dsp_dct_error_t err;

    if (n == 1) {
        out[0] = x[0];
        return DSP_DCT_OK;
    }
    if (!checked_mul_size(n, 2, &two_n)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    interleaved = alloc_doubles(two_n);
    if (interleaved == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }

    interleaved[0] = x[0];
    interleaved[1] = 0.0;
    for (k = 1; k < n; ++k) {
        double sin_t;
        double cos_t;
        double re_k = x[k];
        double im_k = -x[n - k];
        dd_sincos_pi_frac((uint64_t)k, (uint64_t)(2 * n), &sin_t, &cos_t);
        interleaved[2 * k] = re_k * cos_t - im_k * sin_t;
        interleaved[2 * k + 1] = re_k * sin_t + im_k * cos_t;
    }

    err = fft_complex_double(interleaved, n, 1, interleaved);
    if (err != DSP_DCT_OK) {
        zero_and_free_doubles(interleaved, two_n);
        return err;
    }

    for (m = 0; m < half_even; ++m) {
        out[2 * m] = interleaved[2 * m] * n_d;
    }
    for (m = 0; m < half_odd; ++m) {
        out[2 * m + 1] = interleaved[2 * (n - 1 - m)] * n_d;
    }

    zero_and_free_doubles(interleaved, two_n);
    return DSP_DCT_OK;
}

/* DCT-I (N >= 2) in O(N log N) via even symmetric length-2(N-1) FFT. */
static dsp_dct_error_t dct_i_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    size_t denom = n - 1;
    size_t m = 0;
    size_t two_m = 0;
    double *buf = NULL;
    size_t k;
    dsp_dct_error_t err;

    if (!checked_mul_size(denom, 2, &m) || !checked_mul_size(m, 2, &two_m)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    buf = alloc_doubles(two_m);
    if (buf == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }

    if (norm == DSP_DCT_NORM_ORTHO) {
        double inv_sqrt2 = 1.0 / dd_sqrt(2.0);
        double scale = dd_sqrt(2.0 / (double)denom);
        buf[0] = x[0] * inv_sqrt2;
        buf[2 * denom] = x[denom] * inv_sqrt2;
        for (k = 1; k < denom; ++k) {
            double half_x = 0.5 * x[k];
            buf[2 * k] = half_x;
            buf[2 * (m - k)] = half_x;
        }
        err = fft_complex_double(buf, m, 0, buf);
        if (err == DSP_DCT_OK) {
            for (k = 0; k < n; ++k) {
                double wk = (k == 0 || k == n - 1) ? inv_sqrt2 : 1.0;
                out[k] = scale * wk * buf[2 * k];
            }
        }
    } else {
        buf[0] = x[0];
        buf[2 * denom] = x[denom];
        for (k = 1; k < denom; ++k) {
            buf[2 * k] = x[k];
            buf[2 * (m - k)] = x[k];
        }
        err = fft_complex_double(buf, m, 0, buf);
        if (err == DSP_DCT_OK) {
            for (k = 0; k < n; ++k) {
                out[k] = buf[2 * k];
            }
        }
    }

    zero_and_free_doubles(buf, two_m);
    return err;
}

/* DCT-IV / DST-IV shared O(N log N) engine via length-2N pre/post-twiddled FFT. */
static dsp_dct_error_t dct_dst_iv_core(const double *x, size_t n, dsp_dct_norm_t norm,
                                       int is_sine, double *out) {
    size_t two_n = 0;
    size_t four_n_dbl = 0;
    double *buf = NULL;
    double scale = (norm == DSP_DCT_NORM_ORTHO) ? dd_sqrt(2.0 / (double)n) : 2.0;
    size_t k;
    dsp_dct_error_t err;

    if (!checked_mul_size(n, 2, &two_n) || !checked_mul_size(two_n, 2, &four_n_dbl)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    buf = alloc_doubles(four_n_dbl);
    if (buf == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }

    for (k = 0; k < n; ++k) {
        double s;
        double c;
        dd_sincos_pi_frac((uint64_t)k, (uint64_t)two_n, &s, &c);
        buf[2 * k] = x[k] * c;
        buf[2 * k + 1] = -x[k] * s;
    }

    err = fft_complex_double(buf, two_n, 0, buf);
    if (err == DSP_DCT_OK) {
        uint64_t four_n = 4u * (uint64_t)n;
        for (k = 0; k < n; ++k) {
            double s;
            double c;
            double yr = buf[2 * k];
            double yi = buf[2 * k + 1];
            /* Multiply (yr + i*yi) by exp(-i * pi * (2k+1) / (4N)) = (c - i*s). */
            dd_sincos_pi_frac(2u * (uint64_t)k + 1u, four_n, &s, &c);
            if (!is_sine) {
                out[k] = scale * (yr * c + yi * s);
            } else {
                out[k] = scale * (yr * s - yi * c);
            }
        }
    }

    zero_and_free_doubles(buf, four_n_dbl);
    return err;
}

static dsp_dct_error_t dct_iv_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    return dct_dst_iv_core(x, n, norm, 0, out);
}

/* DST-I (N >= 2) in O(N log N) via odd antisymmetric length-2(N+1) FFT. */
static dsp_dct_error_t dst_i_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    size_t n1 = n + 1;
    size_t m = 0;
    size_t two_m = 0;
    double *buf = NULL;
    double scale = (norm == DSP_DCT_NORM_ORTHO) ? (0.5 * dd_sqrt(2.0 / (double)n1)) : 1.0;
    size_t k;
    dsp_dct_error_t err;

    if (!checked_mul_size(n1, 2, &m) || !checked_mul_size(m, 2, &two_m)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    buf = alloc_doubles(two_m);
    if (buf == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }

    for (k = 0; k < n; ++k) {
        buf[2 * (k + 1)] = x[k];
        buf[2 * (m - (k + 1))] = -x[k];
    }

    err = fft_complex_double(buf, m, 0, buf);
    if (err == DSP_DCT_OK) {
        for (k = 0; k < n; ++k) {
            out[k] = -buf[2 * (k + 1) + 1] * scale;
        }
    }

    zero_and_free_doubles(buf, two_m);
    return err;
}

/* DST-II in O(N log N) via sign-flipped DCT-II and output reversal. */
static dsp_dct_error_t dst_ii_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    double *y = alloc_doubles(n);
    double *tmp = alloc_doubles(n);
    size_t k;
    dsp_dct_error_t err;
    if (y == NULL || tmp == NULL) {
        zero_and_free_doubles(y, n);
        zero_and_free_doubles(tmp, n);
        return DSP_DCT_ERR_ALLOC;
    }
    for (k = 0; k < n; ++k) {
        y[k] = (k & 1u) ? -x[k] : x[k];
    }
    err = dct_ii_core(y, n, norm, tmp);
    if (err == DSP_DCT_OK) {
        for (k = 0; k < n; ++k) {
            out[k] = tmp[n - 1 - k];
        }
    }
    zero_and_free_doubles(y, n);
    zero_and_free_doubles(tmp, n);
    return err;
}

/* DST-III in O(N log N) via reversed DCT-III and odd output sign flip.
 * Under ORTHO, computes S_II^T (orthonormal inverse of DST-II). */
static dsp_dct_error_t dst_iii_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    double *y = alloc_doubles(n);
    size_t k;
    dsp_dct_error_t err;
    if (y == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    if (norm == DSP_DCT_NORM_ORTHO) {
        double s0 = 1.0 / dd_sqrt((double)n);
        double sk = 1.0 / dd_sqrt(2.0 * (double)n);
        y[0] = x[n - 1] * s0;
        for (k = 1; k < n; ++k) {
            y[k] = x[n - 1 - k] * sk;
        }
    } else {
        for (k = 0; k < n; ++k) {
            y[k] = x[n - 1 - k];
        }
    }
    err = dct_iii_unnorm_core(y, n, out);
    if (err == DSP_DCT_OK) {
        for (k = 1; k < n; k += 2) {
            out[k] = -out[k];
        }
    }
    zero_and_free_doubles(y, n);
    return err;
}

static dsp_dct_error_t dst_iv_core(const double *x, size_t n, dsp_dct_norm_t norm, double *out) {
    return dct_dst_iv_core(x, n, norm, 1, out);
}

static dsp_dct_error_t dct_forward_double(const double *x, size_t n, dsp_dct_type_t type,
                                          dsp_dct_norm_t norm, double *out) {
    size_t k;
    dsp_dct_error_t err;
    switch (type) {
        case DSP_DCT_I:
            return dct_i_core(x, n, norm, out);
        case DSP_DCT_II:
            return dct_ii_core(x, n, norm, out);
        case DSP_DCT_III:
            if (norm == DSP_DCT_NORM_ORTHO) {
                double *scaled = alloc_doubles(n);
                double s0 = 1.0 / dd_sqrt((double)n);
                double sk = 1.0 / dd_sqrt(2.0 * (double)n);
                if (scaled == NULL) {
                    return DSP_DCT_ERR_ALLOC;
                }
                scaled[0] = x[0] * s0;
                for (k = 1; k < n; ++k) {
                    scaled[k] = x[k] * sk;
                }
                err = dct_iii_unnorm_core(scaled, n, out);
                zero_and_free_doubles(scaled, n);
                return err;
            }
            return dct_iii_unnorm_core(x, n, out);
        case DSP_DCT_IV:
            return dct_iv_core(x, n, norm, out);
        default:
            return DSP_DCT_ERR_INVALID_INPUT;
    }
}

static dsp_dct_error_t dct_inverse_double(const double *x, size_t n, dsp_dct_type_t type,
                                          dsp_dct_norm_t norm, double *out) {
    size_t k;
    dsp_dct_error_t err;
    if (type == DSP_DCT_III) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dct_forward_double(x, n, DSP_DCT_III, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)n);
            err = dct_iii_unnorm_core(x, n, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    if (type == DSP_DCT_II) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dct_ii_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)n);
            err = dct_ii_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    if (type == DSP_DCT_I) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dct_i_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)(n - 1));
            err = dct_i_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    if (type == DSP_DCT_IV) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dct_iv_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)n);
            err = dct_iv_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    return DSP_DCT_ERR_INVALID_INPUT;
}

static dsp_dct_error_t dst_forward_double(const double *x, size_t n, dsp_dst_type_t type,
                                          dsp_dct_norm_t norm, double *out) {
    switch (type) {
        case DSP_DST_I:
            return dst_i_core(x, n, norm, out);
        case DSP_DST_II:
            return dst_ii_core(x, n, norm, out);
        case DSP_DST_III:
            return dst_iii_core(x, n, norm, out);
        case DSP_DST_IV:
            return dst_iv_core(x, n, norm, out);
        default:
            return DSP_DCT_ERR_INVALID_INPUT;
    }
}

static dsp_dct_error_t dst_inverse_double(const double *x, size_t n, dsp_dst_type_t type,
                                          dsp_dct_norm_t norm, double *out) {
    size_t k;
    dsp_dct_error_t err;
    if (type == DSP_DST_III) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dst_iii_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)n);
            err = dst_iii_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    if (type == DSP_DST_II) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dst_ii_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)n);
            err = dst_ii_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    if (type == DSP_DST_I) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dst_i_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)(n + 1));
            err = dst_i_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    if (type == DSP_DST_IV) {
        if (norm == DSP_DCT_NORM_ORTHO) {
            return dst_iv_core(x, n, DSP_DCT_NORM_ORTHO, out);
        } else {
            double inv = 1.0 / (2.0 * (double)n);
            err = dst_iv_core(x, n, DSP_DCT_NORM_NONE, out);
            if (err == DSP_DCT_OK) {
                for (k = 0; k < n; ++k) {
                    out[k] *= inv;
                }
            }
            return err;
        }
    }
    return DSP_DCT_ERR_INVALID_INPUT;
}

/* ================================================================== */
/* Public 1-D Entry Points                                            */
/* ================================================================== */

static dsp_dct_error_t run_1d_dct(const float *in, size_t len, dsp_dct_type_t type,
                                  dsp_dct_norm_t norm, int is_inv, float *dst_buf) {
    dsp_dct_error_t err;
    double *din = NULL;
    double *dout = NULL;
    size_t i;

    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (!is_valid_dct_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_1d_common(in, len, (type == DSP_DCT_I) ? 2 : 1);
    if (err != DSP_DCT_OK) {
        return err;
    }

    din = alloc_doubles(len);
    dout = alloc_doubles(len);
    if (din == NULL || dout == NULL) {
        zero_and_free_doubles(din, len);
        zero_and_free_doubles(dout, len);
        return DSP_DCT_ERR_ALLOC;
    }
    for (i = 0; i < len; ++i) {
        din[i] = (double)in[i];
    }

    err = is_inv ? dct_inverse_double(din, len, type, norm, dout)
                 : dct_forward_double(din, len, type, norm, dout);
    if (err == DSP_DCT_OK && !dd_narrow_array(dout, len, dst_buf)) {
        err = DSP_DCT_ERR_INVALID_INPUT;
    }

    zero_and_free_doubles(din, len);
    zero_and_free_doubles(dout, len);
    return err;
}

static dsp_dct_error_t run_1d_dst(const float *in, size_t len, dsp_dst_type_t type,
                                  dsp_dct_norm_t norm, int is_inv, float *dst_buf) {
    dsp_dct_error_t err;
    double *din = NULL;
    double *dout = NULL;
    size_t i;

    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (!is_valid_dst_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_1d_common(in, len, (type == DSP_DST_I) ? 2 : 1);
    if (err != DSP_DCT_OK) {
        return err;
    }

    din = alloc_doubles(len);
    dout = alloc_doubles(len);
    if (din == NULL || dout == NULL) {
        zero_and_free_doubles(din, len);
        zero_and_free_doubles(dout, len);
        return DSP_DCT_ERR_ALLOC;
    }
    for (i = 0; i < len; ++i) {
        din[i] = (double)in[i];
    }

    err = is_inv ? dst_inverse_double(din, len, type, norm, dout)
                 : dst_forward_double(din, len, type, norm, dout);
    if (err == DSP_DCT_OK && !dd_narrow_array(dout, len, dst_buf)) {
        err = DSP_DCT_ERR_INVALID_INPUT;
    }

    zero_and_free_doubles(din, len);
    zero_and_free_doubles(dout, len);
    return err;
}

static dsp_dct_error_t dct_1d_alloc_impl(const float *in, size_t len, dsp_dct_type_t type,
                                         dsp_dct_norm_t norm, int is_inv,
                                         float **out, size_t *out_len) {
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL || out_len == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0;
    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (!is_valid_dct_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_1d_common(in, len, (type == DSP_DCT_I) ? 2 : 1);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(len);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_1d_dct(in, len, type, norm, is_inv, tmp);
    if (err != DSP_DCT_OK) {
        zero_and_free_floats(tmp, len);
        return err;
    }
    *out = tmp;
    *out_len = len;
    return DSP_DCT_OK;
}

static dsp_dct_error_t dst_1d_alloc_impl(const float *in, size_t len, dsp_dst_type_t type,
                                         dsp_dct_norm_t norm, int is_inv,
                                         float **out, size_t *out_len) {
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL || out_len == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0;
    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (!is_valid_dst_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_1d_common(in, len, (type == DSP_DST_I) ? 2 : 1);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(len);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_1d_dst(in, len, type, norm, is_inv, tmp);
    if (err != DSP_DCT_OK) {
        zero_and_free_floats(tmp, len);
        return err;
    }
    *out = tmp;
    *out_len = len;
    return DSP_DCT_OK;
}

static dsp_dct_error_t dct_1d_into_impl(const float *in, size_t len, dsp_dct_type_t type,
                                        dsp_dct_norm_t norm, int is_inv, float *out) {
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (!is_valid_dct_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_1d_common(in, len, (type == DSP_DCT_I) ? 2 : 1);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(len);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_1d_dct(in, len, type, norm, is_inv, tmp);
    if (err == DSP_DCT_OK) {
        memcpy(out, tmp, len * sizeof(float));
    }
    zero_and_free_floats(tmp, len);
    return err;
}

static dsp_dct_error_t dst_1d_into_impl(const float *in, size_t len, dsp_dst_type_t type,
                                        dsp_dct_norm_t norm, int is_inv, float *out) {
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    if (!is_valid_dst_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_1d_common(in, len, (type == DSP_DST_I) ? 2 : 1);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(len);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_1d_dst(in, len, type, norm, is_inv, tmp);
    if (err == DSP_DCT_OK) {
        memcpy(out, tmp, len * sizeof(float));
    }
    zero_and_free_floats(tmp, len);
    return err;
}

dsp_dct_error_t dsp_dct_1d(const float *signal, size_t len, dsp_dct_type_t type,
                           dsp_dct_norm_t norm, float **out, size_t *out_len) {
    return dct_1d_alloc_impl(signal, len, type, norm, 0, out, out_len);
}

dsp_dct_error_t dsp_idct_1d(const float *coeffs, size_t len, dsp_dct_type_t type,
                            dsp_dct_norm_t norm, float **out, size_t *out_len) {
    return dct_1d_alloc_impl(coeffs, len, type, norm, 1, out, out_len);
}

dsp_dct_error_t dsp_dst_1d(const float *signal, size_t len, dsp_dst_type_t type,
                           dsp_dct_norm_t norm, float **out, size_t *out_len) {
    return dst_1d_alloc_impl(signal, len, type, norm, 0, out, out_len);
}

dsp_dct_error_t dsp_idst_1d(const float *coeffs, size_t len, dsp_dst_type_t type,
                            dsp_dct_norm_t norm, float **out, size_t *out_len) {
    return dst_1d_alloc_impl(coeffs, len, type, norm, 1, out, out_len);
}

dsp_dct_error_t dsp_dct_1d_into(const float *signal, size_t len, dsp_dct_type_t type,
                                dsp_dct_norm_t norm, float *out) {
    return dct_1d_into_impl(signal, len, type, norm, 0, out);
}

dsp_dct_error_t dsp_idct_1d_into(const float *coeffs, size_t len, dsp_dct_type_t type,
                                 dsp_dct_norm_t norm, float *out) {
    return dct_1d_into_impl(coeffs, len, type, norm, 1, out);
}

dsp_dct_error_t dsp_dst_1d_into(const float *signal, size_t len, dsp_dst_type_t type,
                                dsp_dct_norm_t norm, float *out) {
    return dst_1d_into_impl(signal, len, type, norm, 0, out);
}

dsp_dct_error_t dsp_idst_1d_into(const float *coeffs, size_t len, dsp_dst_type_t type,
                                 dsp_dct_norm_t norm, float *out) {
    return dst_1d_into_impl(coeffs, len, type, norm, 1, out);
}

/* ================================================================== */
/* 2-D Separable Transforms                                           */
/* ================================================================== */

static dsp_dct_error_t validate_2d_dims_and_nonempty(size_t image_len,
                                                     uint32_t height, uint32_t width,
                                                     size_t *total_out) {
    size_t h;
    size_t w;
    size_t total = 0;

    if (height == 0 || width == 0) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    h = (size_t)height;
    w = (size_t)width;
    if (h > DSP_DCT_MAX_LEN || w > DSP_DCT_MAX_LEN) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (!checked_mul_size(h, w, &total) || total > DSP_DCT_MAX_SAMPLES) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (image_len == 0) {
        return DSP_DCT_ERR_EMPTY_INPUT;
    }
    *total_out = total;
    return DSP_DCT_OK;
}

static dsp_dct_error_t validate_2d_common(const float *image, size_t image_len,
                                          uint32_t height, uint32_t width,
                                          int min_axis_len, size_t *total_out) {
    dsp_dct_error_t err;
    size_t total = 0;
    size_t i;

    err = validate_2d_dims_and_nonempty(image_len, height, width, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    if ((size_t)height < (size_t)min_axis_len || (size_t)width < (size_t)min_axis_len) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (image == NULL || image_len != total) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    for (i = 0; i < total; ++i) {
        if (!dd_isfinite_f(image[i])) {
            return DSP_DCT_ERR_INVALID_INPUT;
        }
    }
    *total_out = total;
    return DSP_DCT_OK;
}

static dsp_dct_error_t run_2d_dct(const float *image, size_t image_len,
                                  uint32_t height, uint32_t width,
                                  dsp_dct_type_t type, dsp_dct_norm_t norm,
                                  int is_inv, float *dst_buf) {
    size_t h = (size_t)height;
    size_t w = (size_t)width;
    size_t total = 0;
    double *inter = NULL;
    double *work = NULL;
    double *row_in = NULL;
    double *row_out = NULL;
    double *col_in = NULL;
    double *col_out = NULL;
    size_t r;
    size_t c;
    dsp_dct_error_t err;

    if (!is_valid_dct_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_2d_common(image, image_len, height, width,
                             (type == DSP_DCT_I) ? 2 : 1, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }

    inter = alloc_doubles(total);
    work = alloc_doubles(total);
    row_in = alloc_doubles(w);
    row_out = alloc_doubles(w);
    col_in = alloc_doubles(h);
    col_out = alloc_doubles(h);
    if (inter == NULL || work == NULL || row_in == NULL ||
        row_out == NULL || col_in == NULL || col_out == NULL) {
        err = DSP_DCT_ERR_ALLOC;
        goto cleanup;
    }

    if (!is_inv) {
        /* Forward: row pass then column pass. */
        for (r = 0; r < h; ++r) {
            for (c = 0; c < w; ++c) {
                row_in[c] = (double)image[r * w + c];
            }
            err = dct_forward_double(row_in, w, type, norm, row_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            memcpy(inter + r * w, row_out, w * sizeof(double));
        }
        for (c = 0; c < w; ++c) {
            for (r = 0; r < h; ++r) {
                col_in[r] = inter[r * w + c];
            }
            err = dct_forward_double(col_in, h, type, norm, col_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            for (r = 0; r < h; ++r) {
                work[r * w + c] = col_out[r];
            }
        }
    } else {
        /* Inverse: column pass then row pass. */
        for (c = 0; c < w; ++c) {
            for (r = 0; r < h; ++r) {
                col_in[r] = (double)image[r * w + c];
            }
            err = dct_inverse_double(col_in, h, type, norm, col_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            for (r = 0; r < h; ++r) {
                inter[r * w + c] = col_out[r];
            }
        }
        for (r = 0; r < h; ++r) {
            for (c = 0; c < w; ++c) {
                row_in[c] = inter[r * w + c];
            }
            err = dct_inverse_double(row_in, w, type, norm, row_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            memcpy(work + r * w, row_out, w * sizeof(double));
        }
    }

    if (!dd_narrow_array(work, total, dst_buf)) {
        err = DSP_DCT_ERR_INVALID_INPUT;
    }

cleanup:
    zero_and_free_doubles(inter, total);
    zero_and_free_doubles(work, total);
    zero_and_free_doubles(row_in, w);
    zero_and_free_doubles(row_out, w);
    zero_and_free_doubles(col_in, h);
    zero_and_free_doubles(col_out, h);
    return err;
}

static dsp_dct_error_t run_2d_dst(const float *image, size_t image_len,
                                  uint32_t height, uint32_t width,
                                  dsp_dst_type_t type, dsp_dct_norm_t norm,
                                  int is_inv, float *dst_buf) {
    size_t h = (size_t)height;
    size_t w = (size_t)width;
    size_t total = 0;
    double *inter = NULL;
    double *work = NULL;
    double *row_in = NULL;
    double *row_out = NULL;
    double *col_in = NULL;
    double *col_out = NULL;
    size_t r;
    size_t c;
    dsp_dct_error_t err;

    if (!is_valid_dst_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_2d_common(image, image_len, height, width,
                             (type == DSP_DST_I) ? 2 : 1, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }

    inter = alloc_doubles(total);
    work = alloc_doubles(total);
    row_in = alloc_doubles(w);
    row_out = alloc_doubles(w);
    col_in = alloc_doubles(h);
    col_out = alloc_doubles(h);
    if (inter == NULL || work == NULL || row_in == NULL ||
        row_out == NULL || col_in == NULL || col_out == NULL) {
        err = DSP_DCT_ERR_ALLOC;
        goto cleanup;
    }

    if (!is_inv) {
        for (r = 0; r < h; ++r) {
            for (c = 0; c < w; ++c) {
                row_in[c] = (double)image[r * w + c];
            }
            err = dst_forward_double(row_in, w, type, norm, row_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            memcpy(inter + r * w, row_out, w * sizeof(double));
        }
        for (c = 0; c < w; ++c) {
            for (r = 0; r < h; ++r) {
                col_in[r] = inter[r * w + c];
            }
            err = dst_forward_double(col_in, h, type, norm, col_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            for (r = 0; r < h; ++r) {
                work[r * w + c] = col_out[r];
            }
        }
    } else {
        for (c = 0; c < w; ++c) {
            for (r = 0; r < h; ++r) {
                col_in[r] = (double)image[r * w + c];
            }
            err = dst_inverse_double(col_in, h, type, norm, col_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            for (r = 0; r < h; ++r) {
                inter[r * w + c] = col_out[r];
            }
        }
        for (r = 0; r < h; ++r) {
            for (c = 0; c < w; ++c) {
                row_in[c] = inter[r * w + c];
            }
            err = dst_inverse_double(row_in, w, type, norm, row_out);
            if (err != DSP_DCT_OK) {
                goto cleanup;
            }
            memcpy(work + r * w, row_out, w * sizeof(double));
        }
    }

    if (!dd_narrow_array(work, total, dst_buf)) {
        err = DSP_DCT_ERR_INVALID_INPUT;
    }

cleanup:
    zero_and_free_doubles(inter, total);
    zero_and_free_doubles(work, total);
    zero_and_free_doubles(row_in, w);
    zero_and_free_doubles(row_out, w);
    zero_and_free_doubles(col_in, h);
    zero_and_free_doubles(col_out, h);
    return err;
}

static dsp_dct_error_t dct_2d_alloc_impl(const float *image, size_t image_len,
                                         uint32_t height, uint32_t width,
                                         dsp_dct_type_t type, dsp_dct_norm_t norm,
                                         int is_inv, float **out, size_t *out_len) {
    size_t total = 0;
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL || out_len == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0;
    err = validate_2d_dims_and_nonempty(image_len, height, width, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    if (!is_valid_dct_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_2d_common(image, image_len, height, width,
                             (type == DSP_DCT_I) ? 2 : 1, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(total);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_2d_dct(image, image_len, height, width, type, norm, is_inv, tmp);
    if (err != DSP_DCT_OK) {
        zero_and_free_floats(tmp, total);
        return err;
    }
    *out = tmp;
    *out_len = total;
    return DSP_DCT_OK;
}

static dsp_dct_error_t dst_2d_alloc_impl(const float *image, size_t image_len,
                                         uint32_t height, uint32_t width,
                                         dsp_dst_type_t type, dsp_dct_norm_t norm,
                                         int is_inv, float **out, size_t *out_len) {
    size_t total = 0;
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL || out_len == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0;
    err = validate_2d_dims_and_nonempty(image_len, height, width, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    if (!is_valid_dst_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_2d_common(image, image_len, height, width,
                             (type == DSP_DST_I) ? 2 : 1, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(total);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_2d_dst(image, image_len, height, width, type, norm, is_inv, tmp);
    if (err != DSP_DCT_OK) {
        zero_and_free_floats(tmp, total);
        return err;
    }
    *out = tmp;
    *out_len = total;
    return DSP_DCT_OK;
}

static dsp_dct_error_t dct_2d_into_impl(const float *image, size_t image_len,
                                        uint32_t height, uint32_t width,
                                        dsp_dct_type_t type, dsp_dct_norm_t norm,
                                        int is_inv, float *out) {
    size_t total = 0;
    float *tmp = NULL;
    dsp_dct_error_t err;
    if (out == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_2d_dims_and_nonempty(image_len, height, width, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    if (!is_valid_dct_type(type) || !is_valid_norm(norm)) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    err = validate_2d_common(image, image_len, height, width,
                             (type == DSP_DCT_I) ? 2 : 1, &total);
    if (err != DSP_DCT_OK) {
        return err;
    }
    tmp = alloc_floats(total);
    if (tmp == NULL) {
        return DSP_DCT_ERR_ALLOC;
    }
    err = run_2d_dct(image, image_len, height, width, type, norm, is_inv, tmp);
    if (err == DSP_DCT_OK) {
        memcpy(out, tmp, total * sizeof(float));
    }
    zero_and_free_floats(tmp, total);
    return err;
}

dsp_dct_error_t dsp_dct_2d(const float *image, size_t image_len,
                           uint32_t height, uint32_t width,
                           dsp_dct_type_t type, dsp_dct_norm_t norm,
                           float **out, size_t *out_len) {
    return dct_2d_alloc_impl(image, image_len, height, width, type, norm, 0, out, out_len);
}

dsp_dct_error_t dsp_idct_2d(const float *coeffs, size_t coeffs_len,
                            uint32_t height, uint32_t width,
                            dsp_dct_type_t type, dsp_dct_norm_t norm,
                            float **out, size_t *out_len) {
    return dct_2d_alloc_impl(coeffs, coeffs_len, height, width, type, norm, 1, out, out_len);
}

dsp_dct_error_t dsp_dst_2d(const float *image, size_t image_len,
                           uint32_t height, uint32_t width,
                           dsp_dst_type_t type, dsp_dct_norm_t norm,
                           float **out, size_t *out_len) {
    return dst_2d_alloc_impl(image, image_len, height, width, type, norm, 0, out, out_len);
}

dsp_dct_error_t dsp_idst_2d(const float *coeffs, size_t coeffs_len,
                            uint32_t height, uint32_t width,
                            dsp_dst_type_t type, dsp_dct_norm_t norm,
                            float **out, size_t *out_len) {
    return dst_2d_alloc_impl(coeffs, coeffs_len, height, width, type, norm, 1, out, out_len);
}

dsp_dct_error_t dsp_dct_2d_into(const float *image, size_t image_len,
                                uint32_t height, uint32_t width,
                                dsp_dct_type_t type, dsp_dct_norm_t norm,
                                float *out) {
    return dct_2d_into_impl(image, image_len, height, width, type, norm, 0, out);
}

dsp_dct_error_t dsp_idct_2d_into(const float *coeffs, size_t coeffs_len,
                                 uint32_t height, uint32_t width,
                                 dsp_dct_type_t type, dsp_dct_norm_t norm,
                                 float *out) {
    return dct_2d_into_impl(coeffs, coeffs_len, height, width, type, norm, 1, out);
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

dsp_dct_error_t dsp_dct_parse_type(const char *str, dsp_dct_type_t *out) {
    if (out == NULL || str == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (eq_ci_trimmed(str, "1") || eq_ci_trimmed(str, "i") || eq_ci_trimmed(str, "dct-i")) {
        *out = DSP_DCT_I;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "2") || eq_ci_trimmed(str, "ii") || eq_ci_trimmed(str, "dct-ii")) {
        *out = DSP_DCT_II;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "3") || eq_ci_trimmed(str, "iii") || eq_ci_trimmed(str, "dct-iii")) {
        *out = DSP_DCT_III;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "4") || eq_ci_trimmed(str, "iv") || eq_ci_trimmed(str, "dct-iv")) {
        *out = DSP_DCT_IV;
        return DSP_DCT_OK;
    }
    return DSP_DCT_ERR_INVALID_INPUT;
}

dsp_dct_error_t dsp_dst_parse_type(const char *str, dsp_dst_type_t *out) {
    if (out == NULL || str == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (eq_ci_trimmed(str, "1") || eq_ci_trimmed(str, "i") || eq_ci_trimmed(str, "dst-i")) {
        *out = DSP_DST_I;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "2") || eq_ci_trimmed(str, "ii") || eq_ci_trimmed(str, "dst-ii")) {
        *out = DSP_DST_II;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "3") || eq_ci_trimmed(str, "iii") || eq_ci_trimmed(str, "dst-iii")) {
        *out = DSP_DST_III;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "4") || eq_ci_trimmed(str, "iv") || eq_ci_trimmed(str, "dst-iv")) {
        *out = DSP_DST_IV;
        return DSP_DCT_OK;
    }
    return DSP_DCT_ERR_INVALID_INPUT;
}

dsp_dct_error_t dsp_dct_parse_norm(const char *str, dsp_dct_norm_t *out) {
    if (out == NULL || str == NULL) {
        return DSP_DCT_ERR_INVALID_INPUT;
    }
    if (eq_ci_trimmed(str, "none") || eq_ci_trimmed(str, "raw") ||
        eq_ci_trimmed(str, "unnorm") || eq_ci_trimmed(str, "0")) {
        *out = DSP_DCT_NORM_NONE;
        return DSP_DCT_OK;
    }
    if (eq_ci_trimmed(str, "ortho") || eq_ci_trimmed(str, "unitary") ||
        eq_ci_trimmed(str, "1")) {
        *out = DSP_DCT_NORM_ORTHO;
        return DSP_DCT_OK;
    }
    return DSP_DCT_ERR_INVALID_INPUT;
}
