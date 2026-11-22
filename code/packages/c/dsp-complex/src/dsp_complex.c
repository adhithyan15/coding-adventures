/*
 * dsp_complex.c — Interleaved Complex Tensor & Scalar Operations (pure ISO C17).
 * ============================================================================
 *
 * Implements the `dsp-complex` contract with zero `<math.h>` / `-lm`
 * dependencies and full VER15 CWE hardening (CWE-20, 120, 125, 190, 400,
 * 415, 416, 476, 787).
 */
#include "dsp_complex.h"

#include <float.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* ========================================================================== */
/* § 1. Secure Wipe, Checked Arithmetic, and Pure-ISO Math Helpers            */
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

int dsp_complex_is_finite(float x) {
    /* Pure IEEE-754: NaN fails (x == x); +/-Inf fails ((x - x) == 0.0f). */
    return (x == x) && ((x - x) == 0.0f);
}

static int is_finite_double(double x) {
    return (x == x) && ((x - x) == 0.0);
}

int dsp_complex_checked_add(size_t a, size_t b, size_t *out) {
    if (out == NULL) {
        return 0;
    }
    if (SIZE_MAX - a < b) {
        return 0;
    }
    *out = a + b;
    return 1;
}

int dsp_complex_checked_mul(size_t a, size_t b, size_t *out) {
    if (out == NULL) {
        return 0;
    }
    if (a != 0 && b > SIZE_MAX / a) {
        return 0;
    }
    *out = a * b;
    return 1;
}

static double abs_d(double x) {
    return x < 0.0 ? -x : x;
}

/* Pure ISO C sqrt on non-negative finite double using 4x scaling + Heron. */
static double sqrt_d(double x) {
    double m;
    double scale = 1.0;
    double g;
    int iter;
    if (!is_finite_double(x) || x <= 0.0) {
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
    for (iter = 0; iter < 9; ++iter) {
        g = 0.5 * (g + m / g);
    }
    return g * scale;
}

/* Overflow-safe hypot in double without <math.h>: scales by max(|a|, |b|). */
static double hypot_d(float a, float b) {
    double da = abs_d((double)a);
    double db = abs_d((double)b);
    double hi = da > db ? da : db;
    double lo = da > db ? db : da;
    double r;
    if (!dsp_complex_is_finite(a) || !dsp_complex_is_finite(b)) {
        return -1.0;
    }
    if (hi == 0.0) {
        return 0.0;
    }
    r = lo / hi;
    return hi * sqrt_d(1.0 + r * r);
}

static float hypot_f(float a, float b) {
    double val = hypot_d(a, b);
    if (val < 0.0 || !is_finite_double(val)) {
        return 0.0f;
    }
    if (val > (double)FLT_MAX) {
        return FLT_MAX;
    }
    return (float)val;
}

/* Pure ISO C atan(z) for z in [0, 1] using pi/6 argument reduction. */
static double atan_unit_d(double z) {
    /* tan(pi/12) = 2 - sqrt(3) and sqrt(3) */
    static const double TAN_PI_12 = 0.26794919243112270647;
    static const double SQRT_3 = 1.73205080756887729353;
    static const double PI_6 = 0.52359877559829887308;
    int shifted = 0;
    double u;
    double u2;
    double term;
    double sum;
    int k;

    if (z > TAN_PI_12) {
        u = (z * SQRT_3 - 1.0) / (SQRT_3 + z);
        shifted = 1;
    } else {
        u = z;
    }
    /* Alternating Maclaurin series on |u| <= 0.268: 10 terms gives < 1e-13. */
    u2 = u * u;
    term = u;
    sum = u;
    for (k = 1; k <= 10; ++k) {
        term *= -u2;
        sum += term / (double)(2 * k + 1);
    }
    return shifted ? (PI_6 + sum) : sum;
}

/* Pure ISO C atan2f without <math.h>. */
static float atan2_f(float y, float x) {
    static const double PI = 3.14159265358979323846;
    static const double HALF_PI = 1.57079632679489661923;
    double dy = (double)y;
    double dx = (double)x;
    double ay = abs_d(dy);
    double ax = abs_d(dx);
    double ang;

    if (!dsp_complex_is_finite(y) || !dsp_complex_is_finite(x)) {
        return 0.0f;
    }
    if (ax == 0.0 && ay == 0.0) {
        return 0.0f;
    }
    if (ax == 0.0) {
        return dy > 0.0 ? (float)HALF_PI : (float)(-HALF_PI);
    }
    if (ay == 0.0) {
        return dx < 0.0 ? (float)PI : 0.0f;
    }

    if (ay <= ax) {
        ang = atan_unit_d(ay / ax);
    } else {
        ang = HALF_PI - atan_unit_d(ax / ay);
    }

    if (dx < 0.0) {
        ang = PI - ang;
    }
    if (dy < 0.0) {
        ang = -ang;
    }
    return (float)ang;
}

/* Validate tensor structural invariant before read/write access (CWE-476/400). */
static int validate_tensor(const dsp_complex_tensor_t *t) {
    if (t == NULL) {
        return 0;
    }
    if (t->len > DSP_COMPLEX_MAX_SAMPLES) {
        return 0;
    }
    if (t->len > 0 && t->data == NULL) {
        return 0;
    }
    return 1;
}

static void zero_tensor_out(dsp_complex_tensor_t *out) {
    if (out != NULL) {
        out->data = NULL;
        out->len = 0;
    }
}

static dsp_complex_error_t alloc_tensor(size_t n, dsp_complex_tensor_t *out) {
    size_t floats_needed = 0;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    zero_tensor_out(out);
    if (n > DSP_COMPLEX_MAX_SAMPLES) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (n == 0) {
        return DSP_COMPLEX_OK;
    }
    if (!dsp_complex_checked_mul(n, 2u, &floats_needed)) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    out->data = (float *)calloc(floats_needed, sizeof(float));
    if (out->data == NULL) {
        return DSP_COMPLEX_ERR_ALLOC;
    }
    out->len = n;
    return DSP_COMPLEX_OK;
}

/* ========================================================================== */
/* § 2. Diagnostics                                                           */
/* ========================================================================== */

const char *dsp_complex_error_str(dsp_complex_error_t err) {
    switch (err) {
    case DSP_COMPLEX_OK:
        return "ok";
    case DSP_COMPLEX_ERR_INVALID_INPUT:
        return "invalid input";
    case DSP_COMPLEX_ERR_LENGTH_MISMATCH:
        return "length mismatch";
    case DSP_COMPLEX_ERR_ODD_INTERLEAVED_LENGTH:
        return "odd interleaved buffer length";
    case DSP_COMPLEX_ERR_OUT_OF_RANGE:
        return "index out of range";
    case DSP_COMPLEX_ERR_ALLOC:
        return "allocation failed";
    }
    return "unknown error";
}

/* ========================================================================== */
/* § 3. Scalar Complex Arithmetic                                             */
/* ========================================================================== */

dsp_complex_t dsp_c_make(float re, float im) {
    dsp_complex_t c;
    c.re = re;
    c.im = im;
    return c;
}

dsp_complex_t dsp_c_add(dsp_complex_t a, dsp_complex_t b) {
    return dsp_c_make(a.re + b.re, a.im + b.im);
}

dsp_complex_t dsp_c_sub(dsp_complex_t a, dsp_complex_t b) {
    return dsp_c_make(a.re - b.re, a.im - b.im);
}

dsp_complex_t dsp_c_mul(dsp_complex_t a, dsp_complex_t b) {
    return dsp_c_make(a.re * b.re - a.im * b.im, a.re * b.im + a.im * b.re);
}

dsp_complex_t dsp_c_conj(dsp_complex_t a) {
    return dsp_c_make(a.re, -a.im);
}

float dsp_c_abs(dsp_complex_t a) {
    return hypot_f(a.re, a.im);
}

float dsp_c_arg(dsp_complex_t a) {
    return atan2_f(a.im, a.re);
}

/* ========================================================================== */
/* § 4. Tensor Lifecycle & Checked Constructors                               */
/* ========================================================================== */

dsp_complex_error_t dsp_complex_tensor_zeros(size_t n, dsp_complex_tensor_t *out) {
    return alloc_tensor(n, out);
}

dsp_complex_error_t dsp_complex_tensor_from_real(const float *real, size_t n,
                                                 dsp_complex_tensor_t *out) {
    dsp_complex_error_t err;
    size_t i;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    zero_tensor_out(out);
    if (n > DSP_COMPLEX_MAX_SAMPLES) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (n == 0) {
        return DSP_COMPLEX_OK;
    }
    if (real == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    for (i = 0; i < n; ++i) {
        if (!dsp_complex_is_finite(real[i])) {
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
    }
    err = alloc_tensor(n, out);
    if (err != DSP_COMPLEX_OK) {
        return err;
    }
    for (i = 0; i < n; ++i) {
        out->data[2u * i] = real[i];
        out->data[2u * i + 1u] = 0.0f;
    }
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_from_real_imag(const float *real,
                                                      size_t real_len,
                                                      const float *imag,
                                                      size_t imag_len,
                                                      dsp_complex_tensor_t *out) {
    dsp_complex_error_t err;
    size_t i;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    zero_tensor_out(out);
    if (real_len != imag_len) {
        return DSP_COMPLEX_ERR_LENGTH_MISMATCH;
    }
    if (real_len > DSP_COMPLEX_MAX_SAMPLES) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (real_len == 0) {
        return DSP_COMPLEX_OK;
    }
    if (real == NULL || imag == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    for (i = 0; i < real_len; ++i) {
        if (!dsp_complex_is_finite(real[i]) || !dsp_complex_is_finite(imag[i])) {
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
    }
    err = alloc_tensor(real_len, out);
    if (err != DSP_COMPLEX_OK) {
        return err;
    }
    for (i = 0; i < real_len; ++i) {
        out->data[2u * i] = real[i];
        out->data[2u * i + 1u] = imag[i];
    }
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_from_interleaved(const float *data,
                                                        size_t float_len,
                                                        dsp_complex_tensor_t *out) {
    dsp_complex_error_t err;
    size_t n;
    size_t i;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    zero_tensor_out(out);
    if ((float_len % 2u) != 0u) {
        return DSP_COMPLEX_ERR_ODD_INTERLEAVED_LENGTH;
    }
    n = float_len / 2u;
    if (n > DSP_COMPLEX_MAX_SAMPLES) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (n == 0) {
        return DSP_COMPLEX_OK;
    }
    if (data == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    for (i = 0; i < float_len; ++i) {
        if (!dsp_complex_is_finite(data[i])) {
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
    }
    err = alloc_tensor(n, out);
    if (err != DSP_COMPLEX_OK) {
        return err;
    }
    memcpy(out->data, data, float_len * sizeof(float));
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_clone(const dsp_complex_tensor_t *src,
                                             dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(src)) {
        if (out != src) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (src->len == 0u) {
        if (out == src) {
            dsp_complex_tensor_free(out);
        } else {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_OK;
    }
    err = dsp_complex_tensor_from_interleaved(src->data, src->len * 2u, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (out != src) {
            zero_tensor_out(out);
        }
        return err;
    }
    if (out == src) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}

void dsp_complex_tensor_free(dsp_complex_tensor_t *tensor) {
    size_t float_len = 0;
    size_t byte_len = 0;
    if (tensor == NULL) {
        return;
    }
    if (tensor->data != NULL) {
        if (tensor->len <= DSP_COMPLEX_MAX_SAMPLES &&
            dsp_complex_checked_mul(tensor->len, 2u, &float_len) &&
            dsp_complex_checked_mul(float_len, sizeof(float), &byte_len)) {
            secure_zero_bytes(tensor->data, byte_len);
        }
        free(tensor->data);
    }
    tensor->data = NULL;
    tensor->len = 0;
}

void dsp_complex_free_buffer(float *buf, size_t float_len) {
    size_t byte_len = 0;
    if (buf == NULL) {
        return;
    }
    if (dsp_complex_checked_mul(float_len, sizeof(float), &byte_len)) {
        secure_zero_bytes(buf, byte_len);
    }
    free(buf);
}

void dsp_complex_free(void *p) {
    free(p);
}

/* ========================================================================== */
/* § 5. Accessors & Safe Bounds Indexing                                      */
/* ========================================================================== */

size_t dsp_complex_tensor_len(const dsp_complex_tensor_t *t) {
    if (!validate_tensor(t)) {
        return 0;
    }
    return t->len;
}

int dsp_complex_tensor_is_empty(const dsp_complex_tensor_t *t) {
    if (!validate_tensor(t)) {
        return 1;
    }
    return t->len == 0 ? 1 : 0;
}

const float *dsp_complex_tensor_as_interleaved(const dsp_complex_tensor_t *t) {
    if (!validate_tensor(t) || t->len == 0) {
        return NULL;
    }
    return t->data;
}

dsp_complex_error_t dsp_complex_tensor_get(const dsp_complex_tensor_t *t,
                                           size_t idx, dsp_complex_t *out) {
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    out->re = 0.0f;
    out->im = 0.0f;
    if (!validate_tensor(t)) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (idx >= t->len) {
        return DSP_COMPLEX_ERR_OUT_OF_RANGE;
    }
    out->re = t->data[2u * idx];
    out->im = t->data[2u * idx + 1u];
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_set(dsp_complex_tensor_t *t, size_t idx,
                                           dsp_complex_t val) {
    if (!validate_tensor(t)) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (idx >= t->len) {
        return DSP_COMPLEX_ERR_OUT_OF_RANGE;
    }
    if (!dsp_complex_is_finite(val.re) || !dsp_complex_is_finite(val.im)) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    t->data[2u * idx] = val.re;
    t->data[2u * idx + 1u] = val.im;
    return DSP_COMPLEX_OK;
}

/* ========================================================================== */
/* § 6. Projections & Element-Wise Tensor Operations                          */
/* ========================================================================== */

static dsp_complex_error_t alloc_proj_buffer(const dsp_complex_tensor_t *t,
                                             float **out, size_t *out_len) {
    size_t i;
    if (out == NULL || out_len == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    *out = NULL;
    *out_len = 0;
    if (!validate_tensor(t)) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (t->len == 0) {
        return DSP_COMPLEX_OK;
    }
    for (i = 0u; i < t->len * 2u; ++i) {
        if (!dsp_complex_is_finite(t->data[i])) {
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
    }
    *out = (float *)calloc(t->len, sizeof(float));
    if (*out == NULL) {
        return DSP_COMPLEX_ERR_ALLOC;
    }
    *out_len = t->len;
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_real(const dsp_complex_tensor_t *t,
                                            float **out, size_t *out_len) {
    dsp_complex_error_t err = alloc_proj_buffer(t, out, out_len);
    size_t i;
    if (err != DSP_COMPLEX_OK || *out_len == 0) {
        return err;
    }
    for (i = 0; i < t->len; ++i) {
        (*out)[i] = t->data[2u * i];
    }
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_imag(const dsp_complex_tensor_t *t,
                                            float **out, size_t *out_len) {
    dsp_complex_error_t err = alloc_proj_buffer(t, out, out_len);
    size_t i;
    if (err != DSP_COMPLEX_OK || *out_len == 0) {
        return err;
    }
    for (i = 0; i < t->len; ++i) {
        (*out)[i] = t->data[2u * i + 1u];
    }
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_magnitude(const dsp_complex_tensor_t *t,
                                                 float **out, size_t *out_len) {
    dsp_complex_error_t err = alloc_proj_buffer(t, out, out_len);
    size_t i;
    if (err != DSP_COMPLEX_OK || *out_len == 0) {
        return err;
    }
    for (i = 0; i < t->len; ++i) {
        double m = hypot_d(t->data[2u * i], t->data[2u * i + 1u]);
        if (m < 0.0 || !is_finite_double(m) || m > (double)FLT_MAX) {
            dsp_complex_free_buffer(*out, *out_len);
            *out = NULL;
            *out_len = 0u;
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        (*out)[i] = (float)m;
    }
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_phase(const dsp_complex_tensor_t *t,
                                             float **out, size_t *out_len) {
    dsp_complex_error_t err = alloc_proj_buffer(t, out, out_len);
    size_t i;
    if (err != DSP_COMPLEX_OK || *out_len == 0) {
        return err;
    }
    for (i = 0; i < t->len; ++i) {
        (*out)[i] = atan2_f(t->data[2u * i + 1u], t->data[2u * i]);
    }
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_conjugate(const dsp_complex_tensor_t *t,
                                                 dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    size_t i;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(t)) {
        if (out != t) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    err = alloc_tensor(t->len, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (out != t) {
            zero_tensor_out(out);
        }
        return err;
    }
    for (i = 0; i < t->len; ++i) {
        if (!dsp_complex_is_finite(t->data[2u * i]) ||
            !dsp_complex_is_finite(t->data[2u * i + 1u])) {
            dsp_complex_tensor_free(&tmp);
            if (out != t) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        tmp.data[2u * i] = t->data[2u * i];
        tmp.data[2u * i + 1u] = -t->data[2u * i + 1u];
    }
    if (out == t) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_scale(const dsp_complex_tensor_t *t,
                                             float factor,
                                             dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    size_t i;
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(t) || !dsp_complex_is_finite(factor)) {
        if (out != t) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    err = alloc_tensor(t->len, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (out != t) {
            zero_tensor_out(out);
        }
        return err;
    }
    for (i = 0; i < t->len * 2u; ++i) {
        float v = t->data[i] * factor;
        if (!dsp_complex_is_finite(v)) {
            dsp_complex_tensor_free(&tmp);
            if (out != t) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        tmp.data[i] = v;
    }
    if (out == t) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_add(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    size_t i;
    int aliases_input = (out == a || out == b);
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(a) || !validate_tensor(b)) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (a->len != b->len) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_LENGTH_MISMATCH;
    }
    err = alloc_tensor(a->len, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return err;
    }
    for (i = 0; i < a->len * 2u; ++i) {
        float v;
        if (!dsp_complex_is_finite(a->data[i]) || !dsp_complex_is_finite(b->data[i])) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        v = a->data[i] + b->data[i];
        if (!dsp_complex_is_finite(v)) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        tmp.data[i] = v;
    }
    if (aliases_input) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_sub(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    size_t i;
    int aliases_input = (out == a || out == b);
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(a) || !validate_tensor(b)) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (a->len != b->len) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_LENGTH_MISMATCH;
    }
    err = alloc_tensor(a->len, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return err;
    }
    for (i = 0; i < a->len * 2u; ++i) {
        float v;
        if (!dsp_complex_is_finite(a->data[i]) || !dsp_complex_is_finite(b->data[i])) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        v = a->data[i] - b->data[i];
        if (!dsp_complex_is_finite(v)) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        tmp.data[i] = v;
    }
    if (aliases_input) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_mul(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    size_t i;
    int aliases_input = (out == a || out == b);
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(a) || !validate_tensor(b)) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (a->len != b->len) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_LENGTH_MISMATCH;
    }
    err = alloc_tensor(a->len, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return err;
    }
    for (i = 0; i < a->len; ++i) {
        float ar = a->data[2u * i];
        float ai = a->data[2u * i + 1u];
        float br = b->data[2u * i];
        float bi = b->data[2u * i + 1u];
        float re;
        float im;
        if (!dsp_complex_is_finite(ar) || !dsp_complex_is_finite(ai) ||
            !dsp_complex_is_finite(br) || !dsp_complex_is_finite(bi)) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        re = ar * br - ai * bi;
        im = ar * bi + ai * br;
        if (!dsp_complex_is_finite(re) || !dsp_complex_is_finite(im)) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        tmp.data[2u * i] = re;
        tmp.data[2u * i + 1u] = im;
    }
    if (aliases_input) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}

dsp_complex_error_t dsp_complex_tensor_div(const dsp_complex_tensor_t *a,
                                           const dsp_complex_tensor_t *b,
                                           dsp_complex_tensor_t *out) {
    dsp_complex_tensor_t tmp = {NULL, 0u};
    dsp_complex_error_t err;
    size_t i;
    int aliases_input = (out == a || out == b);
    if (out == NULL) {
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (!validate_tensor(a) || !validate_tensor(b)) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_INVALID_INPUT;
    }
    if (a->len != b->len) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return DSP_COMPLEX_ERR_LENGTH_MISMATCH;
    }
    err = alloc_tensor(a->len, &tmp);
    if (err != DSP_COMPLEX_OK) {
        if (!aliases_input) {
            zero_tensor_out(out);
        }
        return err;
    }
    for (i = 0; i < a->len; ++i) {
        float arf = a->data[2u * i];
        float aif = a->data[2u * i + 1u];
        float brf = b->data[2u * i];
        float bif = b->data[2u * i + 1u];
        double ar;
        double ai;
        double br;
        double bi;
        double denom;
        double re_d;
        double im_d;
        if (!dsp_complex_is_finite(arf) || !dsp_complex_is_finite(aif) ||
            !dsp_complex_is_finite(brf) || !dsp_complex_is_finite(bif)) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        ar = (double)arf;
        ai = (double)aif;
        br = (double)brf;
        bi = (double)bif;
        denom = br * br + bi * bi;
        if (denom == 0.0 || !is_finite_double(denom)) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        re_d = (ar * br + ai * bi) / denom;
        im_d = (ai * br - ar * bi) / denom;
        if (!is_finite_double(re_d) || !is_finite_double(im_d) ||
            abs_d(re_d) > (double)FLT_MAX || abs_d(im_d) > (double)FLT_MAX) {
            dsp_complex_tensor_free(&tmp);
            if (!aliases_input) {
                zero_tensor_out(out);
            }
            return DSP_COMPLEX_ERR_INVALID_INPUT;
        }
        tmp.data[2u * i] = (float)re_d;
        tmp.data[2u * i + 1u] = (float)im_d;
    }
    if (aliases_input) {
        dsp_complex_tensor_free(out);
    }
    *out = tmp;
    return DSP_COMPLEX_OK;
}
