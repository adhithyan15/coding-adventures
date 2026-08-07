/*
 * gradient_descent.h — one step of stochastic gradient descent, pure ISO C17.
 * ===========================================================================
 *
 * A faithful port of the Rust `gradient-descent` crate.
 *
 * Stochastic gradient descent (SGD) is the workhorse of machine-learning
 * optimisation. Given model **weights** and the **gradient** of the loss with
 * respect to each weight, it nudges every weight a small step *downhill* — in
 * the direction that reduces the loss:
 *
 *     new_weight[i] = weight[i] - learning_rate * gradient[i]
 *
 * The `learning_rate` (a small positive scalar like 0.01) sets the step size.
 * This routine performs exactly one such update over the whole vector.
 *
 * Pure ISO C17: no <math.h>, no compiler extensions.
 */
#ifndef GRADIENT_DESCENT_H
#define GRADIENT_DESCENT_H

#include <stddef.h> /* size_t */

#ifdef __cplusplus
extern "C" {
#endif

typedef enum {
    GD_OK = 0,
    GD_ERR_LENGTH /* vectors differ in length, or are empty */
} GdStatus;

/*
 * Apply one SGD update, writing `weights[i] - learning_rate * gradients[i]`
 * into `out` (which the caller must size to hold `n` doubles; it may alias
 * `weights`). `n` is the shared length of both input vectors.
 *
 * Returns GD_ERR_LENGTH if `n == 0` (matching the Rust crate, which also
 * rejects mismatched lengths — a mismatch is the caller's to detect here,
 * since the C API takes a single shared length).
 */
GdStatus gd_sgd(const double *weights, const double *gradients, size_t n,
                double learning_rate, double *out);

#ifdef __cplusplus
}
#endif

#endif /* GRADIENT_DESCENT_H */
