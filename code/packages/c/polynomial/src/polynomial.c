/*
 * polynomial.c — implementation of coefficient-array polynomial arithmetic (see
 * polynomial.h). A faithful port of the Rust `polynomial` crate. All arithmetic
 * is on doubles; the only "special" operation is a manual absolute value so no
 * libm is needed.
 */
#include "polynomial.h"

#include <float.h>  /* DBL_EPSILON */
#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, memset */

/* Coefficients with magnitude <= this are treated as zero (the crate's value). */
#define ZERO_THRESHOLD (DBL_EPSILON * 1e6)

static double dabs(double x) { return x < 0.0 ? -x : x; }

/* Normalized length of `p`: strip trailing (high-degree) near-zero coeffs. */
static size_t norm_len(const double *p, size_t n) {
    while (n > 0 && dabs(p[n - 1]) <= ZERO_THRESHOLD) {
        n--;
    }
    return n;
}

size_t poly_normalize(const double *p, size_t n, double *out) {
    size_t len = norm_len(p, n);
    if (len) {
        memcpy(out, p, len * sizeof(double));
    }
    return len;
}

size_t poly_degree(const double *p, size_t n) {
    size_t len = norm_len(p, n);
    return len == 0 ? 0 : len - 1;
}

/* Common core for add (sign = +1) and subtract (sign = -1). */
static size_t add_sub(const double *a, size_t na, const double *b, size_t nb,
                      double sign, double *out) {
    size_t len = na > nb ? na : nb;
    size_t i;
    for (i = 0; i < len; i++) {
        double ai = i < na ? a[i] : 0.0;
        double bi = i < nb ? b[i] : 0.0;
        out[i] = ai + sign * bi;
    }
    return norm_len(out, len);
}

size_t poly_add(const double *a, size_t na, const double *b, size_t nb,
                double *out) {
    return add_sub(a, na, b, nb, 1.0, out);
}
size_t poly_subtract(const double *a, size_t na, const double *b, size_t nb,
                     double *out) {
    return add_sub(a, na, b, nb, -1.0, out);
}

size_t poly_multiply(const double *a, size_t na, const double *b, size_t nb,
                     double *out) {
    size_t len, i, j;
    if (na == 0 || nb == 0) {
        return 0;
    }
    len = na + nb - 1;
    for (i = 0; i < len; i++) {
        out[i] = 0.0;
    }
    for (i = 0; i < na; i++) {
        for (j = 0; j < nb; j++) {
            out[i + j] += a[i] * b[j];
        }
    }
    return norm_len(out, len);
}

int poly_divmod(const double *dividend, size_t nd, const double *divisor,
                size_t nv, double *quot, size_t *quot_len, double *rem,
                size_t *rem_len) {
    size_t nb = norm_len(divisor, nv);
    size_t na = norm_len(dividend, nd);
    size_t deg_a, deg_b, qn, deg_rem, i;
    double lead_b;
    if (nb == 0) {
        return 0; /* division by the zero polynomial */
    }
    if (na < nb) {
        *quot_len = 0;
        if (na) {
            memcpy(rem, dividend, na * sizeof(double));
        }
        *rem_len = na;
        return 1;
    }
    deg_a = na - 1;
    deg_b = nb - 1;
    lead_b = divisor[deg_b];
    /* rem starts as the normalized dividend; quot is zeroed. */
    memcpy(rem, dividend, na * sizeof(double));
    qn = deg_a - deg_b + 1;
    for (i = 0; i < qn; i++) {
        quot[i] = 0.0;
    }
    deg_rem = deg_a;
    for (;;) {
        double lead_rem, coeff;
        size_t power, j;
        ptrdiff_t sd;
        if (deg_rem < deg_b) {
            break;
        }
        lead_rem = rem[deg_rem];
        coeff = lead_rem / lead_b;
        power = deg_rem - deg_b;
        quot[power] = coeff;
        for (j = 0; j <= deg_b; j++) {
            rem[power + j] -= coeff * divisor[j];
        }
        /* Walk the remainder's active degree down past new near-zero terms. */
        sd = (ptrdiff_t)deg_rem - 1;
        while (sd >= 0 && dabs(rem[(size_t)sd]) <= ZERO_THRESHOLD) {
            sd--;
        }
        if (sd < 0) {
            break;
        }
        deg_rem = (size_t)sd;
    }
    *quot_len = norm_len(quot, qn);
    *rem_len = norm_len(rem, na);
    return 1;
}

int poly_divide(const double *dividend, size_t nd, const double *divisor,
                size_t nv, double *quot, size_t *quot_len) {
    /* divmod needs a remainder buffer; allocate a throwaway one. */
    double *rem = (double *)malloc((nd ? nd : 1) * sizeof(double));
    size_t rem_len;
    int ok;
    if (rem == NULL) {
        return 0;
    }
    ok = poly_divmod(dividend, nd, divisor, nv, quot, quot_len, rem, &rem_len);
    free(rem);
    return ok;
}

int poly_modulo(const double *dividend, size_t nd, const double *divisor,
                size_t nv, double *rem, size_t *rem_len) {
    double *quot = (double *)malloc((nd ? nd : 1) * sizeof(double));
    size_t quot_len;
    int ok;
    if (quot == NULL) {
        return 0;
    }
    ok = poly_divmod(dividend, nd, divisor, nv, quot, &quot_len, rem, rem_len);
    free(quot);
    return ok;
}

double poly_evaluate(const double *p, size_t n, double x) {
    size_t len = norm_len(p, n);
    double acc = 0.0;
    size_t i;
    for (i = len; i > 0; i--) {
        acc = acc * x + p[i - 1];
    }
    return acc;
}

size_t poly_gcd(const double *a, size_t na, const double *b, size_t nb,
                double *out) {
    size_t cap = na > nb ? na : nb;
    double *u, *v, *r, *q;
    size_t ul, vl;
    if (cap == 0) {
        return 0; /* gcd(0, 0) = 0 */
    }
    if (cap > SIZE_MAX / sizeof(double)) {
        return 0;
    }
    u = (double *)malloc(cap * sizeof(double));
    v = (double *)malloc(cap * sizeof(double));
    r = (double *)malloc(cap * sizeof(double));
    q = (double *)malloc(cap * sizeof(double));
    if (u == NULL || v == NULL || r == NULL || q == NULL) {
        free(u);
        free(v);
        free(r);
        free(q);
        return 0;
    }
    ul = poly_normalize(a, na, u);
    vl = poly_normalize(b, nb, v);
    while (vl != 0) {
        size_t rl, ql;
        /* r = u mod v (divmod needs quot/rem capacity = u length <= cap). */
        poly_divmod(u, ul, v, vl, q, &ql, r, &rl);
        memcpy(u, v, vl * sizeof(double)); /* u = v */
        ul = vl;
        if (rl) {
            memcpy(v, r, rl * sizeof(double)); /* v = r */
        }
        vl = rl;
    }
    if (ul) {
        memcpy(out, u, ul * sizeof(double));
    }
    free(u);
    free(v);
    free(r);
    free(q);
    return ul;
}
