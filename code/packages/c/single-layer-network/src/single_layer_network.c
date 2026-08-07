/*
 * single_layer_network.c — implementation of the single dense layer + SGD.
 * ===========================================================================
 * Matrices are flat row-major double arrays. No <math.h>: the sigmoid uses a
 * from-scratch e^x. See single_layer_network.h for the math.
 */
#include "single_layer_network.h"

#include <stdlib.h>

/* ---------------------------------------------------------------------------
 *  <math.h>-free exp + activations
 * ------------------------------------------------------------------------- */

static double pow2i(int k) {
    double result = 1.0;
    double base = k < 0 ? 0.5 : 2.0;
    int n = k < 0 ? -k : k;
    while (n > 0) {
        if (n & 1) {
            result *= base;
        }
        base *= base;
        n >>= 1;
    }
    return result;
}

static double d_exp(double x) {
    if (x != x) {
        return x; /* NaN */
    }
    if (x == 0.0) {
        return 1.0;
    }
    if (x > 709.782712893384) {
        return 1.7976931348623157e308;
    }
    if (x < -745.13321910194) {
        return 0.0;
    }
    const double INV_LN2 = 1.4426950408889634;
    const double C1 = 0.693359375;
    const double C2 = -2.1219444005469058277e-4;
    double kf = x * INV_LN2;
    int k = (int)(kf >= 0.0 ? kf + 0.5 : kf - 0.5);
    double r = (x - (double)k * C1) - (double)k * C2;
    double term = 1.0;
    double sum = 1.0;
    int i;
    for (i = 1; i <= 17; i++) {
        term *= r / (double)i;
        sum += term;
    }
    return sum * pow2i(k);
}

static double activate(double value, SlnActivation activation) {
    if (activation == SLN_ACT_LINEAR) {
        return value;
    }
    /* Sigmoid, computed in the numerically stable direction. */
    if (value >= 0.0) {
        double z = d_exp(-value);
        return 1.0 / (1.0 + z);
    }
    double z = d_exp(value);
    return z / (1.0 + z);
}

static double derivative_from_output(double output, SlnActivation activation) {
    if (activation == SLN_ACT_LINEAR) {
        return 1.0;
    }
    return output * (1.0 - output);
}

/* ---------------------------------------------------------------------------
 *  Allocation + shape helpers
 * ------------------------------------------------------------------------- */

/* Allocate a zero-filled rows×cols matrix, guarding the rows*cols multiply
 * against overflow. Both dims are >= 1 when we reach here. */
static double *alloc_matrix(size_t rows, size_t cols) {
    if (cols != 0 && rows > ((size_t)-1) / cols) {
        return NULL;
    }
    size_t total = rows * cols;
    return (double *)calloc(total ? total : 1, sizeof(double));
}

/* A flat matrix passed as (data, rows, cols) is "valid" when both dims are
 * non-zero (ragged is impossible with explicit dims). */
static int shape_ok(size_t rows, size_t cols) {
    return rows != 0 && cols != 0;
}

/* ---------------------------------------------------------------------------
 *  Predict
 * ------------------------------------------------------------------------- */

SlnStatus sln_predict(const double *inputs, size_t samples, size_t input_count,
                      const double *weights, size_t weight_rows,
                      size_t output_count, const double *biases,
                      size_t n_biases, SlnActivation activation, double *out) {
    if (!shape_ok(samples, input_count) ||
        !shape_ok(weight_rows, output_count)) {
        return SLN_ERR_SHAPE;
    }
    if (input_count != weight_rows || n_biases != output_count) {
        return SLN_ERR_SHAPE;
    }
    size_t row, output, input;
    for (row = 0; row < samples; row++) {
        for (output = 0; output < output_count; output++) {
            double total = biases[output];
            for (input = 0; input < input_count; input++) {
                total += inputs[row * input_count + input] *
                         weights[input * output_count + output];
            }
            out[row * output_count + output] = activate(total, activation);
        }
    }
    return SLN_OK;
}

/* ---------------------------------------------------------------------------
 *  One training epoch
 * ------------------------------------------------------------------------- */

void sln_training_step_free(SlnTrainingStep *step) {
    if (!step) {
        return;
    }
    free(step->predictions);
    free(step->errors);
    free(step->weight_gradients);
    free(step->bias_gradients);
    free(step->next_weights);
    free(step->next_biases);
    step->predictions = NULL;
    step->errors = NULL;
    step->weight_gradients = NULL;
    step->bias_gradients = NULL;
    step->next_weights = NULL;
    step->next_biases = NULL;
}

SlnStatus sln_train_one_epoch(const double *inputs, size_t samples,
                              size_t input_count, const double *targets,
                              size_t target_rows, size_t target_cols,
                              const double *weights, size_t weight_rows,
                              size_t weight_cols, const double *biases,
                              size_t n_biases, double learning_rate,
                              SlnActivation activation, SlnTrainingStep *out) {
    size_t output_count = target_cols;
    if (!shape_ok(samples, input_count) ||
        !shape_ok(target_rows, target_cols) ||
        !shape_ok(weight_rows, weight_cols)) {
        return SLN_ERR_SHAPE;
    }
    if (target_rows != samples || weight_rows != input_count ||
        weight_cols != output_count || n_biases != output_count) {
        return SLN_ERR_SHAPE;
    }

    /* Allocate every output buffer up front; free all on any failure. */
    double *predictions = alloc_matrix(samples, output_count);
    double *errors = alloc_matrix(samples, output_count);
    double *deltas = alloc_matrix(samples, output_count);
    double *weight_gradients = alloc_matrix(input_count, output_count);
    double *next_weights = alloc_matrix(input_count, output_count);
    double *bias_gradients = alloc_matrix(1, output_count);
    double *next_biases = alloc_matrix(1, output_count);
    if (!predictions || !errors || !deltas || !weight_gradients ||
        !next_weights || !bias_gradients || !next_biases) {
        free(predictions);
        free(errors);
        free(deltas);
        free(weight_gradients);
        free(next_weights);
        free(bias_gradients);
        free(next_biases);
        return SLN_ERR_NOMEM;
    }

    SlnStatus st = sln_predict(inputs, samples, input_count, weights,
                               weight_rows, output_count, biases, n_biases,
                               activation, predictions);
    if (st != SLN_OK) {
        free(predictions);
        free(errors);
        free(deltas);
        free(weight_gradients);
        free(next_weights);
        free(bias_gradients);
        free(next_biases);
        return st;
    }

    double scale = 2.0 / (double)(samples * output_count);
    double loss_total = 0.0;
    size_t row, output, input;
    for (row = 0; row < samples; row++) {
        for (output = 0; output < output_count; output++) {
            size_t idx = row * output_count + output;
            double error = predictions[idx] - targets[idx];
            errors[idx] = error;
            deltas[idx] =
                scale * error * derivative_from_output(predictions[idx],
                                                       activation);
            loss_total += error * error;
        }
    }

    for (input = 0; input < input_count; input++) {
        for (output = 0; output < output_count; output++) {
            size_t widx = input * output_count + output;
            double grad = 0.0;
            for (row = 0; row < samples; row++) {
                grad += inputs[row * input_count + input] *
                        deltas[row * output_count + output];
            }
            weight_gradients[widx] = grad;
            next_weights[widx] = weights[widx] - learning_rate * grad;
        }
    }

    for (output = 0; output < output_count; output++) {
        double grad = 0.0;
        for (row = 0; row < samples; row++) {
            grad += deltas[row * output_count + output];
        }
        bias_gradients[output] = grad;
        next_biases[output] = biases[output] - learning_rate * grad;
    }

    free(deltas); /* internal only */

    out->predictions = predictions;
    out->errors = errors;
    out->weight_gradients = weight_gradients;
    out->bias_gradients = bias_gradients;
    out->next_weights = next_weights;
    out->next_biases = next_biases;
    out->loss = loss_total / (double)(samples * output_count);
    out->samples = samples;
    out->input_count = input_count;
    out->output_count = output_count;
    return SLN_OK;
}

/* ---------------------------------------------------------------------------
 *  Network
 * ------------------------------------------------------------------------- */

SlnStatus sln_network_init(SlnNetwork *net, size_t input_count,
                           size_t output_count, SlnActivation activation) {
    net->weights = alloc_matrix(input_count, output_count);
    net->biases = alloc_matrix(1, output_count);
    if (!net->weights || !net->biases) {
        free(net->weights);
        free(net->biases);
        net->weights = NULL;
        net->biases = NULL;
        return SLN_ERR_NOMEM;
    }
    net->input_count = input_count;
    net->output_count = output_count;
    net->activation = activation;
    return SLN_OK;
}

void sln_network_free(SlnNetwork *net) {
    if (!net) {
        return;
    }
    free(net->weights);
    free(net->biases);
    net->weights = NULL;
    net->biases = NULL;
    net->input_count = 0;
    net->output_count = 0;
}

SlnStatus sln_network_predict(const SlnNetwork *net, const double *inputs,
                              size_t samples, size_t input_count, double *out) {
    return sln_predict(inputs, samples, input_count, net->weights,
                       net->input_count, net->output_count, net->biases,
                       net->output_count, net->activation, out);
}

SlnStatus sln_network_fit(SlnNetwork *net, const double *inputs, size_t samples,
                          size_t input_count, const double *targets,
                          size_t target_rows, size_t target_cols,
                          double learning_rate, size_t epochs,
                          SlnTrainingStep **history_out,
                          size_t *history_count_out) {
    *history_out = NULL;
    *history_count_out = 0;

    SlnTrainingStep *history = NULL;
    if (epochs > 0) {
        if (epochs > ((size_t)-1) / sizeof(SlnTrainingStep)) {
            return SLN_ERR_NOMEM;
        }
        history = calloc(epochs, sizeof(SlnTrainingStep));
        if (!history) {
            return SLN_ERR_NOMEM;
        }
    }

    size_t e;
    for (e = 0; e < epochs; e++) {
        SlnTrainingStep step;
        SlnStatus st = sln_train_one_epoch(
            inputs, samples, input_count, targets, target_rows, target_cols,
            net->weights, net->input_count, net->output_count, net->biases,
            net->output_count, learning_rate, net->activation, &step);
        if (st != SLN_OK) {
            sln_history_free(history, e);
            return st;
        }
        /* Adopt the updated parameters (copy from the step's next_*). */
        size_t wcount = net->input_count * net->output_count;
        size_t i;
        for (i = 0; i < wcount; i++) {
            net->weights[i] = step.next_weights[i];
        }
        for (i = 0; i < net->output_count; i++) {
            net->biases[i] = step.next_biases[i];
        }
        history[e] = step;
    }

    *history_out = history;
    *history_count_out = epochs;
    return SLN_OK;
}

void sln_history_free(SlnTrainingStep *history, size_t count) {
    if (!history) {
        return;
    }
    size_t i;
    for (i = 0; i < count; i++) {
        sln_training_step_free(&history[i]);
    }
    free(history);
}
