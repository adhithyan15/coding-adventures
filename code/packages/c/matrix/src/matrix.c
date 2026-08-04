/*
 * matrix.c — implementation of the pure-ISO C matrix library.
 * ===========================================================
 *
 * Storage is a single row-major heap block (see matrix.h). Every "producing"
 * routine allocates a new block, so the arithmetic reads inputs and writes a
 * disjoint output — the Rust crate's immutability, expressed in C.
 *
 * The transcendentals (`sqrt`, `pow`) are computed from scratch because the
 * pure-ISO build links no `<math.h>` / libm. They reproduce the Rust f64
 * results to ~1e-12 relative, which is far inside every test tolerance.
 */
#include "matrix.h"

#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memcpy */

/* ── From-scratch floating-point helpers (no libm) ────────────────────────*/

static double d_abs(double x) { return x < 0.0 ? -x : x; }

/* Newton–Raphson square root. sqrt(x<=0) is defined here as 0.0 (the Rust
 * f64::sqrt would yield NaN for x<0, but the crate only ever square-roots
 * non-negative data, and producing NaN without <math.h> risks UB). */
static double d_sqrt(double x) {
    if (x <= 0.0) return 0.0;
    double guess = x >= 1.0 ? x : 1.0;
    for (int i = 0; i < 100; i++) {
        double next = 0.5 * (guess + x / guess);
        if (d_abs(next - guess) <= 1e-15 * guess) return next;
        guess = next;
    }
    return guess;
}

/* 2^k for integer k, by binary exponentiation (exact for |k| within the
 * double exponent range). */
static double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) result *= base;
        base *= base;
        n >>= 1;
    }
    return result;
}

/* e^x via Cody–Waite range reduction: x = k*ln2 + r, e^x = 2^k * e^r, with r
 * small enough for a short Taylor series. Guards run BEFORE the (int) cast so
 * an out-of-range argument can never overflow the conversion. */
static double d_exp(double x) {
    if (x != x) return x;                  /* NaN propagates */
    if (x == 0.0) return 1.0;
    if (x > 709.782712893384) return 1.7976931348623157e308;  /* overflow */
    if (x < -745.13321910194) return 0.0;                     /* underflow */
    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375;
    const double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - (double)k * C1) - (double)k * C2;
    double term = 1.0, sum = 1.0;
    for (int i = 1; i <= 17; i++) {
        term *= r / (double)i;
        sum += term;
    }
    return sum * pow2i(k);
}

/* ln(x): reduce x = m*2^e with m in [1,2), then ln(x) = e*ln2 + 2*atanh(u),
 * u = (m-1)/(m+1). The top guards keep the reduction loops from spinning on
 * non-finite / non-positive inputs. */
static double d_ln(double x) {
    if (x != x) return x;                                          /* NaN */
    if (x <= 0.0) return -1.7976931348623157e308;                 /* ~ -inf */
    if (x > 1.7976931348623157e308) return 1.7976931348623157e308; /* +inf */
    int e = 0;
    double m = x;
    while (m < 1.0) { m *= 2.0; e--; }
    while (m >= 2.0) { m *= 0.5; e++; }
    double u = (m - 1.0) / (m + 1.0);
    double u2 = u * u;
    double term = u, sum = u;
    for (int n = 1; n <= 60; n++) {
        term *= u2;
        double add = term / (double)(2 * n + 1);
        sum += add;
        if (d_abs(add) < 1e-17) break;
    }
    const double LN2 = 0.6931471805599453;
    return (double)e * LN2 + 2.0 * sum;
}

/* x^y, matching Rust's f64::powf on the inputs this crate produces:
 *   - y == 0            -> 1
 *   - x == 0            -> 0 (y>0) / big (y<0)
 *   - integer y         -> exact repeated multiply (works for x < 0)
 *   - x > 0             -> exp(y*ln(x))
 *   - x < 0, y non-int  -> Rust yields NaN; we return 0.0 (documented; the
 *                          crate never takes a fractional power of a negative).
 */
static double d_pow(double x, double y) {
    if (y == 0.0) return 1.0;
    if (x == 0.0) return y > 0.0 ? 0.0 : 1.7976931348623157e308;
    /* Integer-exponent fast path: exact and sign-correct. */
    if (d_abs(y) < 1e15) {
        double ry = y < 0.0 ? -(double)(long long)(-y) : (double)(long long)y;
        if (ry == y) {
            long long n = (long long)ry;
            int neg = n < 0;
            unsigned long long k = (unsigned long long)(neg ? -n : n);
            double result = 1.0, base = x;
            while (k > 0) {
                if (k & 1ULL) result *= base;
                base *= base;
                k >>= 1;
            }
            return neg ? 1.0 / result : result;
        }
    }
    if (x > 0.0) return d_exp(y * d_ln(x));
    return 0.0; /* fractional power of a negative base (unused) */
}

/* ── Internal allocation with overflow guards ─────────────────────────────*/

/* Number of elements rows*cols, guarding the size_t multiply against
 * overflow. Returns 1 and writes *n on success, 0 on overflow. */
static int checked_count(size_t rows, size_t cols, size_t *n) {
    if (rows != 0 && cols > (size_t)-1 / rows) return 0;
    *n = rows * cols;
    return 1;
}

/* Allocate an uninitialised (raw) or zeroed matrix. On overflow / OOM the out
 * matrix is left empty and MAT_ERR_ALLOC is returned. */
static MatStatus alloc_mat(size_t rows, size_t cols, int zeroed, Mat *out) {
    size_t n;
    out->data = NULL;
    out->rows = 0;
    out->cols = 0;
    if (!checked_count(rows, cols, &n)) return MAT_ERR_ALLOC;
    if (n == 0) {
        /* A legitimately empty matrix keeps its (rows, cols) shape but owns no
         * storage. */
        out->rows = rows;
        out->cols = cols;
        return MAT_OK;
    }
    /* Allocate n doubles. `checked_count` proved n <= SIZE_MAX but NOT that
     * n*sizeof(double) fits, so guard the byte size too: calloc does its own
     * n*size overflow check, but the malloc branch multiplies explicitly and
     * must be guarded here. */
    if (n > ((size_t)-1) / sizeof(double)) return MAT_ERR_ALLOC;
    out->data = zeroed ? (double *)calloc(n, sizeof(double))
                       : (double *)malloc(n * sizeof(double));
    if (out->data == NULL) return MAT_ERR_ALLOC;
    out->rows = rows;
    out->cols = cols;
    return MAT_OK;
}

/* ── Lifetime ─────────────────────────────────────────────────────────────*/

void mat_free(Mat *m) {
    if (m == NULL) return;
    free(m->data);
    m->data = NULL;
    m->rows = 0;
    m->cols = 0;
}

/* ── Constructors ─────────────────────────────────────────────────────────*/

MatStatus mat_new(size_t rows, size_t cols, const double *values, Mat *out) {
    MatStatus st = alloc_mat(rows, cols, /*zeroed=*/0, out);
    if (st != MAT_OK) return st;
    size_t n = rows * cols;
    if (n > 0) memcpy(out->data, values, n * sizeof(double));
    return MAT_OK;
}

MatStatus mat_new_1d(const double *values, size_t cols, Mat *out) {
    return mat_new(1, cols, values, out);
}

MatStatus mat_new_scalar(double value, Mat *out) {
    return mat_new(1, 1, &value, out);
}

MatStatus mat_zeros(size_t rows, size_t cols, Mat *out) {
    return alloc_mat(rows, cols, /*zeroed=*/1, out);
}

MatStatus mat_identity(size_t n, Mat *out) {
    MatStatus st = mat_zeros(n, n, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < n; i++) out->data[i * n + i] = 1.0;
    return MAT_OK;
}

MatStatus mat_from_diagonal(const double *values, size_t n, Mat *out) {
    MatStatus st = mat_zeros(n, n, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < n; i++) out->data[i * n + i] = values[i];
    return MAT_OK;
}

MatStatus mat_clone(const Mat *m, Mat *out) {
    return mat_new(m->rows, m->cols, m->data, out);
}

/* ── Element access ───────────────────────────────────────────────────────*/

MatStatus mat_get(const Mat *m, size_t row, size_t col, double *out_value) {
    if (row >= m->rows || col >= m->cols) return MAT_ERR_BOUNDS;
    *out_value = m->data[row * m->cols + col];
    return MAT_OK;
}

MatStatus mat_set(const Mat *m, size_t row, size_t col, double value,
                  Mat *out) {
    if (row >= m->rows || col >= m->cols) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_BOUNDS;
    }
    MatStatus st = mat_clone(m, out);
    if (st != MAT_OK) return st;
    out->data[row * m->cols + col] = value;
    return MAT_OK;
}

/* ── Basic arithmetic ─────────────────────────────────────────────────────*/

/* Shared body for add/subtract: same-shape element-wise combine. `sign` is
 * +1 for add, -1 for subtract. */
static MatStatus elementwise(const Mat *a, const Mat *b, double sign,
                             Mat *out) {
    if (a->rows != b->rows || a->cols != b->cols) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_DIM;
    }
    MatStatus st = alloc_mat(a->rows, a->cols, 0, out);
    if (st != MAT_OK) return st;
    size_t n = a->rows * a->cols;
    for (size_t i = 0; i < n; i++) out->data[i] = a->data[i] + sign * b->data[i];
    return MAT_OK;
}

MatStatus mat_add(const Mat *a, const Mat *b, Mat *out) {
    return elementwise(a, b, 1.0, out);
}

MatStatus mat_subtract(const Mat *a, const Mat *b, Mat *out) {
    return elementwise(a, b, -1.0, out);
}

MatStatus mat_add_scalar(const Mat *a, double scalar, Mat *out) {
    MatStatus st = alloc_mat(a->rows, a->cols, 0, out);
    if (st != MAT_OK) return st;
    size_t n = a->rows * a->cols;
    for (size_t i = 0; i < n; i++) out->data[i] = a->data[i] + scalar;
    return MAT_OK;
}

MatStatus mat_scale(const Mat *a, double scalar, Mat *out) {
    MatStatus st = alloc_mat(a->rows, a->cols, 0, out);
    if (st != MAT_OK) return st;
    size_t n = a->rows * a->cols;
    for (size_t i = 0; i < n; i++) out->data[i] = a->data[i] * scalar;
    return MAT_OK;
}

MatStatus mat_transpose(const Mat *a, Mat *out) {
    if (a->rows == 0) return mat_zeros(0, 0, out);
    MatStatus st = alloc_mat(a->cols, a->rows, 0, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < a->rows; i++)
        for (size_t j = 0; j < a->cols; j++)
            out->data[j * a->rows + i] = a->data[i * a->cols + j];
    return MAT_OK;
}

MatStatus mat_dot(const Mat *a, const Mat *b, Mat *out) {
    if (a->cols != b->rows) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_DIM;
    }
    MatStatus st = mat_zeros(a->rows, b->cols, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < a->rows; i++)
        for (size_t j = 0; j < b->cols; j++) {
            double acc = 0.0;
            for (size_t k = 0; k < a->cols; k++)
                acc += a->data[i * a->cols + k] * b->data[k * b->cols + j];
            out->data[i * b->cols + j] = acc;
        }
    return MAT_OK;
}

/* ── Reductions ───────────────────────────────────────────────────────────*/

double mat_sum(const Mat *m) {
    double total = 0.0;
    size_t n = m->rows * m->cols;
    for (size_t i = 0; i < n; i++) total += m->data[i];
    return total;
}

double mat_mean(const Mat *m) {
    size_t n = m->rows * m->cols;
    if (n == 0) return 0.0;
    return mat_sum(m) / (double)n;
}

double mat_min_val(const Mat *m) {
    size_t n = m->rows * m->cols;
    if (n == 0) return 0.0;
    double v = m->data[0];
    for (size_t i = 1; i < n; i++)
        if (m->data[i] < v) v = m->data[i];
    return v;
}

double mat_max_val(const Mat *m) {
    size_t n = m->rows * m->cols;
    if (n == 0) return 0.0;
    double v = m->data[0];
    for (size_t i = 1; i < n; i++)
        if (m->data[i] > v) v = m->data[i];
    return v;
}

MatStatus mat_sum_rows(const Mat *m, Mat *out) {
    MatStatus st = alloc_mat(m->rows, 1, 0, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < m->rows; i++) {
        double s = 0.0;
        for (size_t j = 0; j < m->cols; j++) s += m->data[i * m->cols + j];
        out->data[i] = s;
    }
    return MAT_OK;
}

MatStatus mat_sum_cols(const Mat *m, Mat *out) {
    MatStatus st = mat_zeros(1, m->cols, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < m->rows; i++)
        for (size_t j = 0; j < m->cols; j++)
            out->data[j] += m->data[i * m->cols + j];
    return MAT_OK;
}

void mat_argmin(const Mat *m, size_t *out_row, size_t *out_col) {
    *out_row = 0;
    *out_col = 0;
    size_t n = m->rows * m->cols;
    if (n == 0) return;
    double best = m->data[0];
    for (size_t i = 0; i < m->rows; i++)
        for (size_t j = 0; j < m->cols; j++) {
            double v = m->data[i * m->cols + j];
            if (v < best) {
                best = v;
                *out_row = i;
                *out_col = j;
            }
        }
}

void mat_argmax(const Mat *m, size_t *out_row, size_t *out_col) {
    *out_row = 0;
    *out_col = 0;
    size_t n = m->rows * m->cols;
    if (n == 0) return;
    double best = m->data[0];
    for (size_t i = 0; i < m->rows; i++)
        for (size_t j = 0; j < m->cols; j++) {
            double v = m->data[i * m->cols + j];
            if (v > best) {
                best = v;
                *out_row = i;
                *out_col = j;
            }
        }
}

/* ── Element-wise math ────────────────────────────────────────────────────*/

MatStatus mat_map(const Mat *m, double (*f)(double), Mat *out) {
    MatStatus st = alloc_mat(m->rows, m->cols, 0, out);
    if (st != MAT_OK) return st;
    size_t n = m->rows * m->cols;
    for (size_t i = 0; i < n; i++) out->data[i] = f(m->data[i]);
    return MAT_OK;
}

MatStatus mat_sqrt(const Mat *m, Mat *out) { return mat_map(m, d_sqrt, out); }
MatStatus mat_abs(const Mat *m, Mat *out) { return mat_map(m, d_abs, out); }

MatStatus mat_pow(const Mat *m, double exp, Mat *out) {
    MatStatus st = alloc_mat(m->rows, m->cols, 0, out);
    if (st != MAT_OK) return st;
    size_t n = m->rows * m->cols;
    for (size_t i = 0; i < n; i++) out->data[i] = d_pow(m->data[i], exp);
    return MAT_OK;
}

/* ── Shape operations ─────────────────────────────────────────────────────*/

MatStatus mat_flatten(const Mat *m, Mat *out) {
    /* Row-major storage IS the flattened order, so this is a straight copy
     * into a 1 x (rows*cols) shape. */
    size_t n;
    if (!checked_count(m->rows, m->cols, &n)) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_ALLOC;
    }
    return mat_new(1, n, m->data, out);
}

MatStatus mat_reshape(const Mat *m, size_t rows, size_t cols, Mat *out) {
    size_t want, have;
    if (!checked_count(rows, cols, &want) ||
        !checked_count(m->rows, m->cols, &have)) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_ALLOC;
    }
    if (want != have) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_DIM;
    }
    /* Same element count, same row-major order — copy the block, restamp the
     * shape. */
    return mat_new(rows, cols, m->data, out);
}

MatStatus mat_row(const Mat *m, size_t i, Mat *out) {
    if (i >= m->rows) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_BOUNDS;
    }
    return mat_new(1, m->cols, m->data + i * m->cols, out);
}

MatStatus mat_col(const Mat *m, size_t j, Mat *out) {
    if (j >= m->cols) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_BOUNDS;
    }
    MatStatus st = alloc_mat(m->rows, 1, 0, out);
    if (st != MAT_OK) return st;
    for (size_t i = 0; i < m->rows; i++) out->data[i] = m->data[i * m->cols + j];
    return MAT_OK;
}

MatStatus mat_slice(const Mat *m, size_t r0, size_t r1, size_t c0, size_t c1,
                    Mat *out) {
    /* Half-open [r0:r1, c0:c1); reject empty or out-of-range ranges (mirrors
     * the Rust guard r0>=r1 || c0>=c1 || r1>rows || c1>cols). */
    if (r0 >= r1 || c0 >= c1 || r1 > m->rows || c1 > m->cols) {
        out->data = NULL;
        out->rows = 0;
        out->cols = 0;
        return MAT_ERR_BOUNDS;
    }
    MatStatus st = alloc_mat(r1 - r0, c1 - c0, 0, out);
    if (st != MAT_OK) return st;
    for (size_t i = r0; i < r1; i++)
        for (size_t j = c0; j < c1; j++)
            out->data[(i - r0) * (c1 - c0) + (j - c0)] = m->data[i * m->cols + j];
    return MAT_OK;
}

/* ── Comparison ───────────────────────────────────────────────────────────*/

int mat_equals(const Mat *a, const Mat *b) {
    if (a->rows != b->rows || a->cols != b->cols) return 0;
    size_t n = a->rows * a->cols;
    for (size_t i = 0; i < n; i++)
        if (a->data[i] != b->data[i]) return 0;
    return 1;
}

int mat_close(const Mat *a, const Mat *b, double tolerance) {
    if (a->rows != b->rows || a->cols != b->cols) return 0;
    size_t n = a->rows * a->cols;
    for (size_t i = 0; i < n; i++)
        if (d_abs(a->data[i] - b->data[i]) > tolerance) return 0;
    return 1;
}
