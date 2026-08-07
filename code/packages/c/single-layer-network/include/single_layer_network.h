/*
 * single_layer_network.h — a single dense neural-network layer with batch
 * gradient descent, in pure ISO C17. A faithful port of the Rust
 * `single-layer-network` crate.
 * ===========================================================================
 *
 * One dense layer maps `input_count` features to `output_count` outputs:
 *
 *     prediction[o] = activate( bias[o] + Σ_i input[i] * weight[i][o] )
 *
 * Training is full-batch mean-squared-error gradient descent. One epoch:
 *   1. forward pass -> predictions
 *   2. error = prediction - target;  delta = (2/(N·O)) · error · act'(pred)
 *   3. weight_grad[i][o] = Σ_row input[row][i] · delta[row][o]
 *      bias_grad[o]      = Σ_row delta[row][o]
 *   4. next = param - learning_rate · grad
 * with loss = mean of error².
 *
 * MATRICES are flat row-major `double` arrays: an r×c matrix M has M(row,col)
 * at m[row*c + col]. (Because dimensions are explicit, ragged rows are not
 * representable — the Rust "must be rectangular" check has no analogue here.)
 *
 * ACTIVATIONS: Linear (identity) and Sigmoid (numerically stable, computed from
 * a libm-free e^x). Sigmoid's derivative uses the output: out·(1-out).
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port returns an
 * `SlnStatus` code and writes results through out-parameters. Owned buffers are
 * released with the matching `*_free`.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_SINGLE_LAYER_NETWORK_H
#define CA_SINGLE_LAYER_NETWORK_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SLN_VERSION "0.1.0"

typedef enum { SLN_ACT_LINEAR, SLN_ACT_SIGMOID } SlnActivation;

typedef enum {
    SLN_OK = 0,
    SLN_ERR_SHAPE, /* an empty or mismatched matrix / vector shape */
    SLN_ERR_NOMEM
} SlnStatus;

/* Predict outputs for a batch. `inputs` is samples×input_count, `weights` is
 * input_count×output_count, `biases` has output_count entries; `out` (provided
 * by the caller) is samples×output_count. */
SlnStatus sln_predict(const double *inputs, size_t samples, size_t input_count,
                      const double *weights, size_t weight_rows,
                      size_t output_count, const double *biases,
                      size_t n_biases, SlnActivation activation, double *out);

/* The full record of one training epoch (all matrices malloc'd; release with
 * sln_training_step_free). */
typedef struct {
    double *predictions;      /* samples × output_count */
    double *errors;           /* samples × output_count */
    double *weight_gradients; /* input_count × output_count */
    double *bias_gradients;   /* output_count */
    double *next_weights;     /* input_count × output_count */
    double *next_biases;      /* output_count */
    double loss;
    size_t samples;
    size_t input_count;
    size_t output_count;
} SlnTrainingStep;

void sln_training_step_free(SlnTrainingStep *step);

/* Run one epoch of batch gradient descent, filling *out (release it). */
SlnStatus sln_train_one_epoch(const double *inputs, size_t samples,
                              size_t input_count, const double *targets,
                              size_t target_rows, size_t target_cols,
                              const double *weights, size_t weight_rows,
                              size_t weight_cols, const double *biases,
                              size_t n_biases, double learning_rate,
                              SlnActivation activation, SlnTrainingStep *out);

/* A network: owned weights (input_count×output_count) and biases
 * (output_count), plus an activation. */
typedef struct {
    double *weights;
    double *biases;
    size_t input_count;
    size_t output_count;
    SlnActivation activation;
} SlnNetwork;

/* Initialize with zero weights and biases. Returns SLN_OK or SLN_ERR_NOMEM. */
SlnStatus sln_network_init(SlnNetwork *net, size_t input_count,
                           size_t output_count, SlnActivation activation);
void sln_network_free(SlnNetwork *net);

/* Predict with the network's current parameters into caller-provided `out`
 * (samples × output_count). */
SlnStatus sln_network_predict(const SlnNetwork *net, const double *inputs,
                              size_t samples, size_t input_count, double *out);

/* Train for `epochs` epochs, updating the network in place and returning a
 * malloc'd history of `*history_count_out` steps (release with
 * sln_history_free). */
SlnStatus sln_network_fit(SlnNetwork *net, const double *inputs, size_t samples,
                          size_t input_count, const double *targets,
                          size_t target_rows, size_t target_cols,
                          double learning_rate, size_t epochs,
                          SlnTrainingStep **history_out,
                          size_t *history_count_out);
void sln_history_free(SlnTrainingStep *history, size_t count);

#ifdef __cplusplus
}
#endif

#endif /* CA_SINGLE_LAYER_NETWORK_H */
