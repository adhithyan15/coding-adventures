/*
 * matrix.h — a small, dependency-free 2D matrix of `double`, in pure ISO C17.
 * =========================================================================
 *
 * A faithful port of the Rust `matrix` crate. The Rust type stores its
 * elements as `Vec<Vec<f64>>` (a vector of row vectors); here we store the
 * same values in a single **row-major** heap block of `rows * cols` doubles.
 * Element (i, j) lives at `data[i * cols + j]`.
 *
 * ## Design, mirrored from the Rust crate
 *
 *   1. **Immutable by default.** Every operation returns a *new* matrix; the
 *      inputs (taken by `const Mat *`) are never mutated. The caller owns each
 *      returned matrix and must release it with `mat_free`.
 *
 *   2. **No libm.** The only "math" the crate needs — `sqrt`, `abs`, and a
 *      general `pow` — is computed from scratch (see matrix.c). This keeps the
 *      package pure ISO C with no `<math.h>` and no `-lm` link dependency.
 *
 *   3. **Status codes for fallible operations.** Rust returns
 *      `Result<Matrix, _>` for dimension mismatches and out-of-bounds access;
 *      C cannot throw, so those functions return a `MatStatus` and write their
 *      result through an out-parameter. Infallible operations (scale, sum, …)
 *      return the value directly.
 *
 * ## Ownership contract (READ THIS)
 *
 *   - Producers (`mat_zeros`, `mat_add`, `mat_dot`, …) allocate a fresh matrix.
 *     On `MAT_OK` the out-parameter owns heap memory; free it with `mat_free`.
 *     On any error status the out-parameter is left empty (its `data` is NULL),
 *     so calling `mat_free` on it is always safe.
 *   - `mat_free(NULL)` and a repeated `mat_free` are safe: freeing zeroes the
 *     handle so a second call is a no-op.
 *
 * This is pure ISO C17: it compiles under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_MATRIX_H
#define CA_MATRIX_H

#include <stddef.h> /* size_t */

#ifdef __cplusplus
extern "C" {
#endif

/* ── The matrix handle ────────────────────────────────────────────────────
 * `data` is a row-major block of `rows*cols` doubles (NULL for an empty 0x0
 * matrix or after `mat_free`). A zero-sized matrix has data == NULL and
 * rows == cols == 0. */
typedef struct {
    double *data;
    size_t rows;
    size_t cols;
} Mat;

/* Status codes returned by fallible operations. */
typedef enum {
    MAT_OK = 0,     /* success */
    MAT_ERR_DIM,    /* dimension mismatch (add/subtract/dot/reshape) */
    MAT_ERR_BOUNDS, /* row/col/index out of range (get/set/row/col/slice) */
    MAT_ERR_ALLOC   /* out of memory, or a size_t multiply overflowed */
} MatStatus;

/* ── Lifetime ─────────────────────────────────────────────────────────────*/

/* Release a matrix's storage and reset it to the empty state. Safe on an
 * already-freed or zero-initialised handle, and on NULL. */
void mat_free(Mat *m);

/* ── Constructors ─────────────────────────────────────────────────────────*/

/* Build a rows*cols matrix by copying `values` (row-major, length rows*cols).
 * `values` may be NULL only when rows*cols == 0. */
MatStatus mat_new(size_t rows, size_t cols, const double *values, Mat *out);

/* One-row matrix from a length-`cols` array (Rust `new_1d`). */
MatStatus mat_new_1d(const double *values, size_t cols, Mat *out);

/* 1x1 matrix holding a single value (Rust `new_scalar`). */
MatStatus mat_new_scalar(double value, Mat *out);

/* rows*cols matrix of zeros (Rust `zeros`). */
MatStatus mat_zeros(size_t rows, size_t cols, Mat *out);

/* n x n identity (Rust `identity`). */
MatStatus mat_identity(size_t n, Mat *out);

/* n x n diagonal matrix from `values` (length n) (Rust `from_diagonal`). */
MatStatus mat_from_diagonal(const double *values, size_t n, Mat *out);

/* Deep copy. */
MatStatus mat_clone(const Mat *m, Mat *out);

/* ── Element access ───────────────────────────────────────────────────────*/

/* Read element (row, col). MAT_ERR_BOUNDS if out of range; else writes
 * *out_value. */
MatStatus mat_get(const Mat *m, size_t row, size_t col, double *out_value);

/* Copy of `m` with element (row, col) replaced by `value` (immutable set).
 * MAT_ERR_BOUNDS if out of range. */
MatStatus mat_set(const Mat *m, size_t row, size_t col, double value, Mat *out);

/* ── Basic arithmetic ─────────────────────────────────────────────────────*/

MatStatus mat_add(const Mat *a, const Mat *b, Mat *out);      /* same shape */
MatStatus mat_add_scalar(const Mat *a, double scalar, Mat *out);
MatStatus mat_subtract(const Mat *a, const Mat *b, Mat *out); /* same shape */
MatStatus mat_scale(const Mat *a, double scalar, Mat *out);
MatStatus mat_transpose(const Mat *a, Mat *out);
MatStatus mat_dot(const Mat *a, const Mat *b, Mat *out);      /* a.cols==b.rows */

/* ── Reductions ───────────────────────────────────────────────────────────*/

double mat_sum(const Mat *m);     /* sum of all elements */
double mat_mean(const Mat *m);    /* sum / (rows*cols); 0 for an empty matrix */
double mat_min_val(const Mat *m); /* min element (0.0 for an empty matrix) */
double mat_max_val(const Mat *m); /* max element (0.0 for an empty matrix) */

MatStatus mat_sum_rows(const Mat *m, Mat *out); /* rows x 1 column of row sums */
MatStatus mat_sum_cols(const Mat *m, Mat *out); /* 1 x cols row of column sums */

/* First (row, col) of the min / max element, row-major order. For an empty
 * matrix both indices are set to 0. */
void mat_argmin(const Mat *m, size_t *out_row, size_t *out_col);
void mat_argmax(const Mat *m, size_t *out_row, size_t *out_col);

/* ── Element-wise math ────────────────────────────────────────────────────*/

/* Apply `f` to every element (Rust `map`). */
MatStatus mat_map(const Mat *m, double (*f)(double), Mat *out);
MatStatus mat_sqrt(const Mat *m, Mat *out);            /* element-wise sqrt  */
MatStatus mat_abs(const Mat *m, Mat *out);             /* element-wise |x|   */
MatStatus mat_pow(const Mat *m, double exp, Mat *out); /* element-wise x^exp */

/* ── Shape operations ─────────────────────────────────────────────────────*/

MatStatus mat_flatten(const Mat *m, Mat *out); /* 1 x (r*c) */
MatStatus mat_reshape(const Mat *m, size_t rows, size_t cols, Mat *out);
MatStatus mat_row(const Mat *m, size_t i, Mat *out); /* 1 x cols */
MatStatus mat_col(const Mat *m, size_t j, Mat *out); /* rows x 1 */
MatStatus mat_slice(const Mat *m, size_t r0, size_t r1, size_t c0, size_t c1,
                    Mat *out); /* [r0:r1, c0:c1) */

/* ── Comparison ───────────────────────────────────────────────────────────*/

int mat_equals(const Mat *a, const Mat *b);                 /* exact, bool 0/1 */
int mat_close(const Mat *a, const Mat *b, double tolerance); /* |a-b|<=tol */

#ifdef __cplusplus
}
#endif

#endif /* CA_MATRIX_H */
