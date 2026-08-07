/*
 * loss_functions.c — implementation of the four losses and their gradients.
 * ===========================================================================
 * No <math.h>: the natural logarithm is computed from scratch (range reduction
 * to [1, 2) then an atanh series), so the package links without libm.
 */
#include "loss_functions.h"

/* ---------------------------------------------------------------------------
 *  <math.h>-free helpers
 * ------------------------------------------------------------------------- */

static double d_abs(double x) { return x < 0.0 ? -x : x; }

/* Clamp x into [lo, hi]. */
static double d_clamp(double x, double lo, double hi) {
    if (x < lo) {
        return lo;
    }
    if (x > hi) {
        return hi;
    }
    return x;
}

/* Natural log for x > 0. Reduce x = m * 2^e with m in [1, 2), so
 * ln(x) = e*ln2 + ln(m), and ln(m) = 2*atanh(u) with u = (m-1)/(m+1) in
 * [0, 1/3) — a fast, accurate series. Callers only pass x in [EPSILON, 1]. */
static double d_ln(double x) {
    int e = 0;
    double m = x;
    while (m < 1.0) {
        m *= 2.0;
        e--;
    }
    while (m >= 2.0) {
        m *= 0.5;
        e++;
    }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u;
    double sum = u;
    int n;
    for (n = 1; n <= 40; n++) {
        term *= u2;
        double add = term / (double)(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) {
            break;
        }
    }
    const double LN2 = 0.6931471805599453;
    return (double)e * LN2 + 2.0 * sum;
}

/* Shared validation: equal, non-zero lengths (mirrors the Rust check). */
static LossStatus check_lengths(size_t n_true, size_t n_pred) {
    if (n_true != n_pred || n_true == 0) {
        return LOSS_ERR_LENGTH;
    }
    return LOSS_OK;
}

/* ---------------------------------------------------------------------------
 *  Scalar losses
 * ------------------------------------------------------------------------- */

LossStatus loss_mse(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double sum = 0.0;
    size_t i;
    for (i = 0; i < n_true; i++) {
        double diff = y_true[i] - y_pred[i];
        sum += diff * diff;
    }
    *out = sum / (double)n_true;
    return LOSS_OK;
}

LossStatus loss_mae(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double sum = 0.0;
    size_t i;
    for (i = 0; i < n_true; i++) {
        sum += d_abs(y_true[i] - y_pred[i]);
    }
    *out = sum / (double)n_true;
    return LOSS_OK;
}

LossStatus loss_bce(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double sum = 0.0;
    size_t i;
    for (i = 0; i < n_true; i++) {
        double p = d_clamp(y_pred[i], LOSS_EPSILON, 1.0 - LOSS_EPSILON);
        sum += y_true[i] * d_ln(p) + (1.0 - y_true[i]) * d_ln(1.0 - p);
    }
    *out = -sum / (double)n_true;
    return LOSS_OK;
}

LossStatus loss_cce(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double sum = 0.0;
    size_t i;
    for (i = 0; i < n_true; i++) {
        double p = d_clamp(y_pred[i], LOSS_EPSILON, 1.0 - LOSS_EPSILON);
        sum += y_true[i] * d_ln(p);
    }
    *out = -sum / (double)n_true;
    return LOSS_OK;
}

/* ---------------------------------------------------------------------------
 *  Gradients (per-element, written into out[0..n))
 * ------------------------------------------------------------------------- */

LossStatus loss_mse_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred,
                               double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double n = (double)n_true;
    size_t i;
    for (i = 0; i < n_true; i++) {
        out[i] = (2.0 / n) * (y_pred[i] - y_true[i]);
    }
    return LOSS_OK;
}

LossStatus loss_mae_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred,
                               double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double n = (double)n_true;
    size_t i;
    for (i = 0; i < n_true; i++) {
        if (y_pred[i] > y_true[i]) {
            out[i] = 1.0 / n;
        } else if (y_pred[i] < y_true[i]) {
            out[i] = -1.0 / n;
        } else {
            out[i] = 0.0;
        }
    }
    return LOSS_OK;
}

LossStatus loss_bce_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred,
                               double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double n = (double)n_true;
    size_t i;
    for (i = 0; i < n_true; i++) {
        double p = d_clamp(y_pred[i], LOSS_EPSILON, 1.0 - LOSS_EPSILON);
        out[i] = (1.0 / n) * ((p - y_true[i]) / (p * (1.0 - p)));
    }
    return LOSS_OK;
}

LossStatus loss_cce_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred,
                               double *out) {
    LossStatus st = check_lengths(n_true, n_pred);
    if (st != LOSS_OK) {
        return st;
    }
    double n = (double)n_true;
    size_t i;
    for (i = 0; i < n_true; i++) {
        double p = d_clamp(y_pred[i], LOSS_EPSILON, 1.0 - LOSS_EPSILON);
        out[i] = (-1.0 / n) * (y_true[i] / p);
    }
    return LOSS_OK;
}
