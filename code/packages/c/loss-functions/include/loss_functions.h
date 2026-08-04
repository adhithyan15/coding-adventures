/*
 * loss_functions.h — machine-learning loss functions and their gradients, in
 * pure ISO C17. A faithful port of the Rust `loss-functions` crate.
 * ===========================================================================
 *
 * A loss function scores how far a model's predictions `y_pred` are from the
 * ground truth `y_true`; its derivative (gradient) is what training descends.
 * This library provides the four classics, each as a scalar loss and a
 * per-element gradient:
 *
 *   MSE  mean squared error       (1/n) Σ (t - p)^2          regression
 *   MAE  mean absolute error      (1/n) Σ |t - p|            robust regression
 *   BCE  binary cross-entropy     -(1/n) Σ [t·ln p + (1-t)·ln(1-p)]   2-class
 *   CCE  categorical cross-entropy -(1/n) Σ [t·ln p]                  k-class
 *
 * Cross-entropy clamps each prediction to [EPSILON, 1-EPSILON] before the log,
 * so ln(0) = -inf never occurs (EPSILON = 1e-7, matching the Rust crate).
 *
 * NO libm: the one transcendental (ln) is computed from scratch, so the package
 * links with no math library. Results match the Rust std methods to well within
 * the 1e-6 tolerance the crate's own tests use.
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<_, &'static str>`; this port
 * returns a `LossStatus` code. The scalar losses write to a `double *out`; the
 * gradients write `n` values into a caller-provided `out` array. Both input
 * arrays carry their own length so an unequal-length call is still an error, as
 * in Rust.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>, no compiler extensions. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_LOSS_FUNCTIONS_H
#define CA_LOSS_FUNCTIONS_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Clamp bound for predictions before a logarithm (matches the Rust constant). */
#define LOSS_EPSILON 1e-7

/* Status of a fallible operation. */
typedef enum {
    LOSS_OK = 0,
    LOSS_ERR_LENGTH /* the two arrays differ in length, or are empty */
} LossStatus;

/* ── Scalar losses: write the loss to *out ───────────────────────────────── */
LossStatus loss_mse(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out);
LossStatus loss_mae(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out);
LossStatus loss_bce(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out);
LossStatus loss_cce(const double *y_true, size_t n_true, const double *y_pred,
                    size_t n_pred, double *out);

/* ── Gradients: write n per-element derivatives into `out` (length n) ─────── */
LossStatus loss_mse_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred, double *out);
LossStatus loss_mae_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred, double *out);
LossStatus loss_bce_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred, double *out);
LossStatus loss_cce_derivative(const double *y_true, size_t n_true,
                               const double *y_pred, size_t n_pred, double *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_LOSS_FUNCTIONS_H */
