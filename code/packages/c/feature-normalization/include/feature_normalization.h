/*
 * feature_normalization.h — column-wise feature scaling, in pure ISO C17.
 * A faithful port of the Rust `feature-normalization` crate.
 * ===========================================================================
 *
 * Two classic scalers used to put the columns of a data matrix on comparable
 * scales before feeding a model:
 *
 *   StandardScaler (z-score)    z = (x - mean) / stddev      per column
 *   MinMaxScaler   (unit range) u = (x - min)  / (max - min) per column
 *
 * Each is a two-step fit/transform: `fit` learns the per-column statistics from
 * a training matrix; `transform` applies them (to that matrix or any other with
 * the same width). A column with zero spread (stddev == 0, or max == min) maps
 * to 0.0, exactly as in the Rust crate.
 *
 * MATRIX REPRESENTATION. Matrices are passed as a flat row-major array of
 * `nrows * ncols` doubles (element (r, c) at data[r*ncols + c]). This is the
 * idiomatic C shape; because width is explicit, ragged rows are not
 * representable (the Rust "all rows must have the same width" check has no
 * analogue here — a single `FN_ERR_EMPTY` covers nrows==0 or ncols==0).
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<_, &'static str>`; this port
 * returns an `FnStatus` code. The population standard deviation (divide by n,
 * not n-1) matches the Rust crate.
 *
 * PORTABILITY. Pure ISO C17 — no <math.h> (sqrt is computed by Newton's
 * method), no compiler extensions. Builds clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_FEATURE_NORMALIZATION_H
#define CA_FEATURE_NORMALIZATION_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status of a fallible operation. */
typedef enum {
    FN_OK = 0,
    FN_ERR_EMPTY,          /* matrix has zero rows or zero columns */
    FN_ERR_WIDTH_MISMATCH, /* transform matrix width != fitted scaler width */
    FN_ERR_NOMEM           /* allocation failed */
} FnStatus;

/* Per-column mean and (population) standard deviation. Owns two `width`-long
 * arrays; release with fn_standard_scaler_free. */
typedef struct {
    double *means;
    double *standard_deviations;
    size_t width;
} FnStandardScaler;

/* Per-column minimum and maximum. Owns two `width`-long arrays; release with
 * fn_min_max_scaler_free. */
typedef struct {
    double *minimums;
    double *maximums;
    size_t width;
} FnMinMaxScaler;

/* ── StandardScaler ──────────────────────────────────────────────────────── */

/* Learn per-column mean and stddev from `data` (nrows x ncols, row-major).
 * On FN_OK, *out owns freshly allocated arrays. */
FnStatus fn_fit_standard_scaler(const double *data, size_t nrows, size_t ncols,
                                FnStandardScaler *out);
void fn_standard_scaler_free(FnStandardScaler *s);

/* Apply a fitted StandardScaler to `data` (nrows x ncols), writing the scaled
 * matrix into the caller-provided `out` (also nrows*ncols doubles). The scaler
 * width must equal ncols. */
FnStatus fn_transform_standard(const double *data, size_t nrows, size_t ncols,
                               const FnStandardScaler *s, double *out);

/* ── MinMaxScaler ────────────────────────────────────────────────────────── */

FnStatus fn_fit_min_max_scaler(const double *data, size_t nrows, size_t ncols,
                               FnMinMaxScaler *out);
void fn_min_max_scaler_free(FnMinMaxScaler *s);

FnStatus fn_transform_min_max(const double *data, size_t nrows, size_t ncols,
                              const FnMinMaxScaler *s, double *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_FEATURE_NORMALIZATION_H */
