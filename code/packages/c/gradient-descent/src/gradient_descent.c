/*
 * gradient_descent.c — one step of stochastic gradient descent, pure ISO C17.
 * ===========================================================================
 *
 * See gradient_descent.h. The update is a single pass over the vectors:
 *
 *     out[i] = weights[i] - learning_rate * gradients[i]
 *
 * Writing into a caller-provided buffer (rather than allocating) keeps the API
 * allocation-free — the caller owns the output storage, and `out` may safely
 * alias `weights` for an in-place update.
 */
#include "gradient_descent.h"

GdStatus gd_sgd(const double *weights, const double *gradients, size_t n,
                double learning_rate, double *out) {
    size_t i;
    if (n == 0) {
        return GD_ERR_LENGTH;
    }
    for (i = 0; i < n; i++) {
        out[i] = weights[i] - (learning_rate * gradients[i]);
    }
    return GD_OK;
}
