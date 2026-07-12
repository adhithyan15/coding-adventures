/*
 * two_layer_network.h — a two-layer (one hidden layer) neural network with
 * backpropagation, in pure ISO C17. A faithful port of the Rust
 * `two-layer-network` crate.
 * ===========================================================================
 *
 * A hidden layer is what lets a network learn NON-linearly-separable functions
 * such as XOR (a single layer cannot). The forward pass is:
 *
 *     hidden_raw  = inputs · W_ih + b_h
 *     hidden      = activation(hidden_raw)
 *     output_raw  = hidden · W_ho + b_o
 *     prediction  = activation(output_raw)
 *
 * Training is one full-batch mean-squared-error step. Backpropagation runs the
 * error backward through both layers (output deltas, then hidden deltas via the
 * transposed output weights) and returns every gradient plus the next
 * parameters. Activations: Linear and Sigmoid (numerically stable, from a
 * libm-free e^x).
 *
 * MATRICES are flat row-major `double` arrays: an r×c matrix M has M(row,col)
 * at m[row*c + col].
 *
 * DIVERGENCE FROM RUST. Rust returns `Result<_, String>`; this port returns a
 * `TlnStatus` and writes results through out-parameters. Owned buffers are
 * released with the matching `*_free`.
 *
 * PORTABILITY. Pure ISO C17, no <math.h>. Builds clean under GCC, Clang, and
 * MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_TWO_LAYER_NETWORK_H
#define CA_TWO_LAYER_NETWORK_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#define TLN_VERSION "0.1.0"

typedef enum { TLN_ACT_LINEAR, TLN_ACT_SIGMOID } TlnActivation;

typedef enum {
    TLN_OK = 0,
    TLN_ERR_SHAPE, /* an empty or mismatched matrix / vector shape */
    TLN_ERR_NOMEM
} TlnStatus;

/* The network's learnable parameters (all owned). */
typedef struct {
    double *input_to_hidden_weights;  /* input_count × hidden_count */
    double *hidden_biases;            /* hidden_count */
    double *hidden_to_output_weights; /* hidden_count × output_count */
    double *output_biases;            /* output_count */
    size_t input_count;
    size_t hidden_count;
    size_t output_count;
} TlnParameters;

/* Deep-copy the four arrays into a fresh TlnParameters (arrays are copied, not
 * borrowed). Returns TLN_OK or TLN_ERR_NOMEM. */
TlnStatus tln_parameters_init(TlnParameters *out, const double *i2h_weights,
                              size_t input_count, size_t hidden_count,
                              const double *hidden_biases,
                              const double *h2o_weights, size_t output_count,
                              const double *output_biases);
void tln_parameters_free(TlnParameters *p);

/* The well-known hand-tuned XOR warm-start parameters (2-2-1 network). */
TlnStatus tln_xor_warm_start_parameters(TlnParameters *out);

/* The intermediate values of one forward pass (all samples × width). */
typedef struct {
    double *hidden_raw;         /* samples × hidden_count */
    double *hidden_activations; /* samples × hidden_count */
    double *output_raw;         /* samples × output_count */
    double *predictions;        /* samples × output_count */
    size_t samples;
    size_t hidden_count;
    size_t output_count;
} TlnForwardPass;

void tln_forward_pass_free(TlnForwardPass *fp);

/* Run the forward pass, filling *out (release with tln_forward_pass_free). */
TlnStatus tln_forward(const double *inputs, size_t samples, size_t input_count,
                      const TlnParameters *p, TlnActivation hidden_activation,
                      TlnActivation output_activation, TlnForwardPass *out);

/* The full record of one training epoch (all buffers owned). */
typedef struct {
    double *predictions;   /* samples × output_count */
    double *errors;        /* samples × output_count */
    double *output_deltas; /* samples × output_count */
    double *hidden_deltas; /* samples × hidden_count */
    double *hidden_to_output_weight_gradients; /* hidden_count × output_count */
    double *output_bias_gradients;             /* output_count */
    double *input_to_hidden_weight_gradients;  /* input_count × hidden_count */
    double *hidden_bias_gradients;             /* hidden_count */
    TlnParameters next_parameters;
    double loss;
    size_t samples;
    size_t input_count;
    size_t hidden_count;
    size_t output_count;
} TlnTrainingStep;

void tln_training_step_free(TlnTrainingStep *step);

/* One epoch of full-batch backpropagation, filling *out (release it). */
TlnStatus tln_train_one_epoch(const double *inputs, size_t samples,
                              size_t input_count, const double *targets,
                              size_t target_rows, size_t target_cols,
                              const TlnParameters *p, double learning_rate,
                              TlnActivation hidden_activation,
                              TlnActivation output_activation,
                              TlnTrainingStep *out);

#ifdef __cplusplus
}
#endif

#endif /* CA_TWO_LAYER_NETWORK_H */
