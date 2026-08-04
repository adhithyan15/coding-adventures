/*
 * feature_normalization.c — implementation of the two column scalers.
 * ===========================================================================
 * See feature_normalization.h for the API and the mathematics. No <math.h>:
 * the one square root (for the standard deviation) is computed by Newton's
 * method, so the package depends on nothing but basic arithmetic.
 */
#include "feature_normalization.h"

#include <stdlib.h>

/* ---------------------------------------------------------------------------
 *  Small helpers
 * ------------------------------------------------------------------------- */

/* Square root via Newton's (Babylonian) method — no libm. Callers only ever
 * pass a non-negative variance, so no domain check is needed. */
static double newton_sqrt(double x) {
    if (x <= 0.0) {
        return 0.0;
    }
    double guess = x >= 1.0 ? x : 1.0;
    int i;
    for (i = 0; i < 60; i++) {
        double next = (guess + x / guess) / 2.0;
        double diff = next - guess;
        if (diff < 0.0) {
            diff = -diff;
        }
        if (diff < 1e-15 * guess + 1e-300) {
            return next;
        }
        guess = next;
    }
    return guess;
}

/* Allocate a zero-initialized array of `n` doubles, guarding the multiply
 * against overflow via calloc's checked n*size. Returns NULL on failure. */
static double *alloc_doubles(size_t n) {
    /* n is a matrix width (>= 1 when we get here); calloc(0,..) is avoided. */
    return (double *)calloc(n, sizeof(double));
}

/* Validate a flat matrix: reject zero rows or zero columns. */
static FnStatus validate_matrix(size_t nrows, size_t ncols) {
    if (nrows == 0 || ncols == 0) {
        return FN_ERR_EMPTY;
    }
    return FN_OK;
}

/* ---------------------------------------------------------------------------
 *  StandardScaler
 * ------------------------------------------------------------------------- */

FnStatus fn_fit_standard_scaler(const double *data, size_t nrows, size_t ncols,
                                FnStandardScaler *out) {
    FnStatus st = validate_matrix(nrows, ncols);
    if (st != FN_OK) {
        return st;
    }

    double *means = alloc_doubles(ncols);
    double *sds = alloc_doubles(ncols);
    if (!means || !sds) {
        free(means);
        free(sds);
        return FN_ERR_NOMEM;
    }

    /* Column sums -> means. */
    size_t r, c;
    for (r = 0; r < nrows; r++) {
        for (c = 0; c < ncols; c++) {
            means[c] += data[r * ncols + c];
        }
    }
    for (c = 0; c < ncols; c++) {
        means[c] /= (double)nrows;
    }

    /* Sum of squared deviations -> population variance -> stddev. */
    for (r = 0; r < nrows; r++) {
        for (c = 0; c < ncols; c++) {
            double diff = data[r * ncols + c] - means[c];
            sds[c] += diff * diff;
        }
    }
    for (c = 0; c < ncols; c++) {
        sds[c] = newton_sqrt(sds[c] / (double)nrows);
    }

    out->means = means;
    out->standard_deviations = sds;
    out->width = ncols;
    return FN_OK;
}

void fn_standard_scaler_free(FnStandardScaler *s) {
    if (!s) {
        return;
    }
    free(s->means);
    free(s->standard_deviations);
    s->means = NULL;
    s->standard_deviations = NULL;
    s->width = 0;
}

FnStatus fn_transform_standard(const double *data, size_t nrows, size_t ncols,
                               const FnStandardScaler *s, double *out) {
    FnStatus st = validate_matrix(nrows, ncols);
    if (st != FN_OK) {
        return st;
    }
    if (ncols != s->width) {
        return FN_ERR_WIDTH_MISMATCH;
    }
    size_t r, c;
    for (r = 0; r < nrows; r++) {
        for (c = 0; c < ncols; c++) {
            size_t i = r * ncols + c;
            /* A column with no spread maps to 0 (avoids divide-by-zero). */
            if (s->standard_deviations[c] == 0.0) {
                out[i] = 0.0;
            } else {
                out[i] = (data[i] - s->means[c]) / s->standard_deviations[c];
            }
        }
    }
    return FN_OK;
}

/* ---------------------------------------------------------------------------
 *  MinMaxScaler
 * ------------------------------------------------------------------------- */

FnStatus fn_fit_min_max_scaler(const double *data, size_t nrows, size_t ncols,
                               FnMinMaxScaler *out) {
    FnStatus st = validate_matrix(nrows, ncols);
    if (st != FN_OK) {
        return st;
    }

    double *mins = alloc_doubles(ncols);
    double *maxs = alloc_doubles(ncols);
    if (!mins || !maxs) {
        free(mins);
        free(maxs);
        return FN_ERR_NOMEM;
    }

    /* Seed from the first row, then fold in the rest. */
    size_t r, c;
    for (c = 0; c < ncols; c++) {
        mins[c] = data[c];
        maxs[c] = data[c];
    }
    for (r = 1; r < nrows; r++) {
        for (c = 0; c < ncols; c++) {
            double v = data[r * ncols + c];
            if (v < mins[c]) {
                mins[c] = v;
            }
            if (v > maxs[c]) {
                maxs[c] = v;
            }
        }
    }

    out->minimums = mins;
    out->maximums = maxs;
    out->width = ncols;
    return FN_OK;
}

void fn_min_max_scaler_free(FnMinMaxScaler *s) {
    if (!s) {
        return;
    }
    free(s->minimums);
    free(s->maximums);
    s->minimums = NULL;
    s->maximums = NULL;
    s->width = 0;
}

FnStatus fn_transform_min_max(const double *data, size_t nrows, size_t ncols,
                              const FnMinMaxScaler *s, double *out) {
    FnStatus st = validate_matrix(nrows, ncols);
    if (st != FN_OK) {
        return st;
    }
    if (ncols != s->width) {
        return FN_ERR_WIDTH_MISMATCH;
    }
    size_t r, c;
    for (r = 0; r < nrows; r++) {
        for (c = 0; c < ncols; c++) {
            size_t i = r * ncols + c;
            double span = s->maximums[c] - s->minimums[c];
            if (span == 0.0) {
                out[i] = 0.0;
            } else {
                out[i] = (data[i] - s->minimums[c]) / span;
            }
        }
    }
    return FN_OK;
}
