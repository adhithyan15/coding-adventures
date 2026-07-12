/*
 * two_layer_network.c — implementation of the two-layer network + backprop.
 * ===========================================================================
 * Matrices are handled through a small internal `Mat` value (flat row-major
 * data + dims); the public API exposes the resulting flat arrays. No <math.h>:
 * the sigmoid uses a from-scratch e^x.
 */
#include "two_layer_network.h"

#include <stdlib.h>
#include <string.h>

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
        return x;
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

static double activate(double value, TlnActivation activation) {
    if (activation == TLN_ACT_LINEAR) {
        return value;
    }
    if (value >= 0.0) {
        double z = d_exp(-value);
        return 1.0 / (1.0 + z);
    }
    double z = d_exp(value);
    return z / (1.0 + z);
}

/* derivative wrt the pre-activation, expressed via the activated output. */
static double derivative(double activated, TlnActivation activation) {
    if (activation == TLN_ACT_LINEAR) {
        return 1.0;
    }
    return activated * (1.0 - activated);
}

/* ---------------------------------------------------------------------------
 *  Internal matrix helper (flat row-major). Ops return 0 / -1 shape / -2 nomem.
 * ------------------------------------------------------------------------- */

typedef struct {
    double *data;
    size_t rows, cols;
} Mat;

static Mat mat_zero(void) {
    Mat m;
    m.data = NULL;
    m.rows = 0;
    m.cols = 0;
    return m;
}

/* Allocate a zero-filled rows×cols matrix, guarding rows*cols overflow. */
static int mat_alloc(size_t rows, size_t cols, Mat *out) {
    if (rows == 0 || cols == 0) {
        return -1;
    }
    if (rows > ((size_t)-1) / cols) {
        return -2;
    }
    double *data = calloc(rows * cols, sizeof(double));
    if (!data) {
        return -2;
    }
    out->data = data;
    out->rows = rows;
    out->cols = cols;
    return 0;
}

static void mat_free(Mat *m) {
    if (m) {
        free(m->data);
        m->data = NULL;
        m->rows = 0;
        m->cols = 0;
    }
}

/* View a borrowed flat array as a Mat (no ownership). */
static Mat mat_view(const double *data, size_t rows, size_t cols) {
    Mat m;
    m.data = (double *)data; /* not mutated by the read-only ops */
    m.rows = rows;
    m.cols = cols;
    return m;
}

static int mat_dot(Mat l, Mat r, Mat *out) {
    if (l.rows == 0 || l.cols == 0 || r.rows == 0 || r.cols == 0) {
        return -1;
    }
    if (l.cols != r.rows) {
        return -1;
    }
    int st = mat_alloc(l.rows, r.cols, out);
    if (st != 0) {
        return st;
    }
    size_t row, col, k;
    for (row = 0; row < l.rows; row++) {
        for (col = 0; col < r.cols; col++) {
            double sum = 0.0;
            for (k = 0; k < l.cols; k++) {
                sum += l.data[row * l.cols + k] * r.data[k * r.cols + col];
            }
            out->data[row * r.cols + col] = sum;
        }
    }
    return 0;
}

static int mat_transpose(Mat m, Mat *out) {
    if (m.rows == 0 || m.cols == 0) {
        return -1;
    }
    int st = mat_alloc(m.cols, m.rows, out);
    if (st != 0) {
        return st;
    }
    size_t row, col;
    for (row = 0; row < m.rows; row++) {
        for (col = 0; col < m.cols; col++) {
            out->data[col * m.rows + row] = m.data[row * m.cols + col];
        }
    }
    return 0;
}

/* out = m with biases[col] added to every row (m is consumed conceptually; we
 * write a fresh matrix). */
static int mat_add_biases(Mat m, const double *biases, Mat *out) {
    int st = mat_alloc(m.rows, m.cols, out);
    if (st != 0) {
        return st;
    }
    size_t row, col;
    for (row = 0; row < m.rows; row++) {
        for (col = 0; col < m.cols; col++) {
            out->data[row * m.cols + col] =
                m.data[row * m.cols + col] + biases[col];
        }
    }
    return 0;
}

static int mat_apply_activation(Mat m, TlnActivation act, Mat *out) {
    int st = mat_alloc(m.rows, m.cols, out);
    if (st != 0) {
        return st;
    }
    size_t i, n = m.rows * m.cols;
    for (i = 0; i < n; i++) {
        out->data[i] = activate(m.data[i], act);
    }
    return 0;
}

/* column_sums -> a 1×cols matrix. */
static int mat_column_sums(Mat m, Mat *out) {
    int st = mat_alloc(1, m.cols, out);
    if (st != 0) {
        return st;
    }
    size_t row, col;
    for (col = 0; col < m.cols; col++) {
        double sum = 0.0;
        for (row = 0; row < m.rows; row++) {
            sum += m.data[row * m.cols + col];
        }
        out->data[col] = sum;
    }
    return 0;
}

static int mat_subtract_scaled(Mat m, Mat grads, double lr, Mat *out) {
    int st = mat_alloc(m.rows, m.cols, out);
    if (st != 0) {
        return st;
    }
    size_t i, n = m.rows * m.cols;
    for (i = 0; i < n; i++) {
        out->data[i] = m.data[i] - lr * grads.data[i];
    }
    return 0;
}

static double mean_squared_error(Mat errors) {
    size_t n = errors.rows * errors.cols;
    double sum = 0.0;
    size_t i;
    for (i = 0; i < n; i++) {
        sum += errors.data[i] * errors.data[i];
    }
    return sum / (double)n;
}

static TlnStatus map_status(int code) {
    if (code == 0) {
        return TLN_OK;
    }
    return code == -1 ? TLN_ERR_SHAPE : TLN_ERR_NOMEM;
}

/* ---------------------------------------------------------------------------
 *  Parameters
 * ------------------------------------------------------------------------- */

static double *dup_doubles(const double *src, size_t n) {
    double *out = malloc(n * sizeof(double)); /* n >= 1 at call sites */
    if (out) {
        memcpy(out, src, n * sizeof(double));
    }
    return out;
}

TlnStatus tln_parameters_init(TlnParameters *out, const double *i2h_weights,
                              size_t input_count, size_t hidden_count,
                              const double *hidden_biases,
                              const double *h2o_weights, size_t output_count,
                              const double *output_biases) {
    if (input_count == 0 || hidden_count == 0 || output_count == 0) {
        return TLN_ERR_SHAPE;
    }
    /* Guard the two weight-matrix element counts against overflow. */
    if (input_count > ((size_t)-1) / hidden_count ||
        hidden_count > ((size_t)-1) / output_count) {
        return TLN_ERR_NOMEM;
    }
    out->input_to_hidden_weights =
        dup_doubles(i2h_weights, input_count * hidden_count);
    out->hidden_biases = dup_doubles(hidden_biases, hidden_count);
    out->hidden_to_output_weights =
        dup_doubles(h2o_weights, hidden_count * output_count);
    out->output_biases = dup_doubles(output_biases, output_count);
    if (!out->input_to_hidden_weights || !out->hidden_biases ||
        !out->hidden_to_output_weights || !out->output_biases) {
        tln_parameters_free(out);
        return TLN_ERR_NOMEM;
    }
    out->input_count = input_count;
    out->hidden_count = hidden_count;
    out->output_count = output_count;
    return TLN_OK;
}

void tln_parameters_free(TlnParameters *p) {
    if (!p) {
        return;
    }
    free(p->input_to_hidden_weights);
    free(p->hidden_biases);
    free(p->hidden_to_output_weights);
    free(p->output_biases);
    p->input_to_hidden_weights = NULL;
    p->hidden_biases = NULL;
    p->hidden_to_output_weights = NULL;
    p->output_biases = NULL;
}

TlnStatus tln_xor_warm_start_parameters(TlnParameters *out) {
    /* 2 inputs -> 2 hidden -> 1 output. */
    double i2h[4] = {4.0, -4.0, 4.0, -4.0}; /* row-major 2×2 */
    double hb[2] = {-2.0, 6.0};
    double h2o[2] = {4.0, 4.0}; /* 2×1 */
    double ob[1] = {-6.0};
    return tln_parameters_init(out, i2h, 2, 2, hb, h2o, 1, ob);
}

/* ---------------------------------------------------------------------------
 *  Forward pass
 * ------------------------------------------------------------------------- */

void tln_forward_pass_free(TlnForwardPass *fp) {
    if (!fp) {
        return;
    }
    free(fp->hidden_raw);
    free(fp->hidden_activations);
    free(fp->output_raw);
    free(fp->predictions);
    fp->hidden_raw = NULL;
    fp->hidden_activations = NULL;
    fp->output_raw = NULL;
    fp->predictions = NULL;
}

/* Internal forward that returns the four intermediate Mats (all owned). */
static int forward_mats(Mat inputs, const TlnParameters *p,
                        TlnActivation hidden_act, TlnActivation output_act,
                        Mat *hidden_raw, Mat *hidden_activations,
                        Mat *output_raw, Mat *predictions) {
    Mat W_ih = mat_view(p->input_to_hidden_weights, p->input_count,
                        p->hidden_count);
    Mat W_ho = mat_view(p->hidden_to_output_weights, p->hidden_count,
                        p->output_count);
    Mat hd = mat_zero(), hr = mat_zero(), ha = mat_zero();
    Mat od = mat_zero(), orr = mat_zero(), pr = mat_zero();
    int st;

    st = mat_dot(inputs, W_ih, &hd); /* samples × hidden */
    if (st != 0) goto fail;
    st = mat_add_biases(hd, p->hidden_biases, &hr);
    if (st != 0) goto fail;
    st = mat_apply_activation(hr, hidden_act, &ha);
    if (st != 0) goto fail;
    st = mat_dot(ha, W_ho, &od); /* samples × output */
    if (st != 0) goto fail;
    st = mat_add_biases(od, p->output_biases, &orr);
    if (st != 0) goto fail;
    st = mat_apply_activation(orr, output_act, &pr);
    if (st != 0) goto fail;

    mat_free(&hd);
    mat_free(&od);
    *hidden_raw = hr;
    *hidden_activations = ha;
    *output_raw = orr;
    *predictions = pr;
    return 0;

fail:
    mat_free(&hd);
    mat_free(&hr);
    mat_free(&ha);
    mat_free(&od);
    mat_free(&orr);
    mat_free(&pr);
    return st;
}

TlnStatus tln_forward(const double *inputs, size_t samples, size_t input_count,
                      const TlnParameters *p, TlnActivation hidden_activation,
                      TlnActivation output_activation, TlnForwardPass *out) {
    if (samples == 0 || input_count == 0 || input_count != p->input_count) {
        return TLN_ERR_SHAPE;
    }
    Mat in = mat_view(inputs, samples, input_count);
    Mat hr, ha, orr, pr;
    int st = forward_mats(in, p, hidden_activation, output_activation, &hr, &ha,
                          &orr, &pr);
    if (st != 0) {
        return map_status(st);
    }
    out->hidden_raw = hr.data;
    out->hidden_activations = ha.data;
    out->output_raw = orr.data;
    out->predictions = pr.data;
    out->samples = samples;
    out->hidden_count = p->hidden_count;
    out->output_count = p->output_count;
    return TLN_OK;
}

/* ---------------------------------------------------------------------------
 *  Training step
 * ------------------------------------------------------------------------- */

void tln_training_step_free(TlnTrainingStep *step) {
    if (!step) {
        return;
    }
    free(step->predictions);
    free(step->errors);
    free(step->output_deltas);
    free(step->hidden_deltas);
    free(step->hidden_to_output_weight_gradients);
    free(step->output_bias_gradients);
    free(step->input_to_hidden_weight_gradients);
    free(step->hidden_bias_gradients);
    tln_parameters_free(&step->next_parameters);
    step->predictions = NULL;
    step->errors = NULL;
    step->output_deltas = NULL;
    step->hidden_deltas = NULL;
    step->hidden_to_output_weight_gradients = NULL;
    step->output_bias_gradients = NULL;
    step->input_to_hidden_weight_gradients = NULL;
    step->hidden_bias_gradients = NULL;
}

TlnStatus tln_train_one_epoch(const double *inputs, size_t samples,
                              size_t input_count, const double *targets,
                              size_t target_rows, size_t target_cols,
                              const TlnParameters *p, double learning_rate,
                              TlnActivation hidden_activation,
                              TlnActivation output_activation,
                              TlnTrainingStep *out) {
    size_t output_count = target_cols;
    size_t hidden_count = p->hidden_count;
    if (samples == 0 || input_count == 0 || target_rows == 0 ||
        target_cols == 0) {
        return TLN_ERR_SHAPE;
    }
    if (input_count != p->input_count || target_rows != samples ||
        output_count != p->output_count) {
        return TLN_ERR_SHAPE;
    }

    Mat in = mat_view(inputs, samples, input_count);
    Mat tg = mat_view(targets, samples, output_count);
    Mat W_ho = mat_view(p->hidden_to_output_weights, hidden_count, output_count);
    Mat W_ih = mat_view(p->input_to_hidden_weights, input_count, hidden_count);

    /* Every owned intermediate is declared and zeroed up front, so the single
     * `cleanup` path can free them all regardless of where a failure occurs. */
    Mat hidden_raw = mat_zero(), hidden_activations = mat_zero();
    Mat output_raw = mat_zero(), predictions = mat_zero();
    Mat errors = mat_zero(), output_deltas = mat_zero();
    Mat ha_T = mat_zero(), h2o_grads = mat_zero(), output_bias_grads = mat_zero();
    Mat W_ho_T = mat_zero(), hidden_errors = mat_zero(), hidden_deltas = mat_zero();
    Mat in_T = mat_zero(), i2h_grads = mat_zero(), hidden_bias_grads = mat_zero();
    Mat next_i2h = mat_zero(), next_h2o = mat_zero();
    double *next_hb = NULL;
    double *next_ob = NULL;
    int st;
    size_t row, col, i;

    /* Forward pass. */
    if ((st = forward_mats(in, p, hidden_activation, output_activation,
                           &hidden_raw, &hidden_activations, &output_raw,
                           &predictions)) != 0) {
        goto cleanup;
    }

    /* Output-layer error and delta. */
    if ((st = mat_alloc(samples, output_count, &errors)) != 0) goto cleanup;
    if ((st = mat_alloc(samples, output_count, &output_deltas)) != 0)
        goto cleanup;
    double scale = 2.0 / (double)(samples * output_count);
    for (row = 0; row < samples; row++) {
        for (col = 0; col < output_count; col++) {
            size_t idx = row * output_count + col;
            double error = predictions.data[idx] - tg.data[idx];
            errors.data[idx] = error;
            output_deltas.data[idx] =
                scale * error *
                derivative(predictions.data[idx], output_activation);
        }
    }

    /* Hidden->output gradients. */
    if ((st = mat_transpose(hidden_activations, &ha_T)) != 0) goto cleanup;
    if ((st = mat_dot(ha_T, output_deltas, &h2o_grads)) != 0) goto cleanup;
    if ((st = mat_column_sums(output_deltas, &output_bias_grads)) != 0)
        goto cleanup;

    /* Backprop into the hidden layer. */
    if ((st = mat_transpose(W_ho, &W_ho_T)) != 0) goto cleanup;
    if ((st = mat_dot(output_deltas, W_ho_T, &hidden_errors)) != 0) goto cleanup;
    if ((st = mat_alloc(samples, hidden_count, &hidden_deltas)) != 0)
        goto cleanup;
    for (row = 0; row < samples; row++) {
        for (col = 0; col < hidden_count; col++) {
            size_t idx = row * hidden_count + col;
            hidden_deltas.data[idx] =
                hidden_errors.data[idx] *
                derivative(hidden_activations.data[idx], hidden_activation);
        }
    }
    if ((st = mat_transpose(in, &in_T)) != 0) goto cleanup;
    if ((st = mat_dot(in_T, hidden_deltas, &i2h_grads)) != 0) goto cleanup;
    if ((st = mat_column_sums(hidden_deltas, &hidden_bias_grads)) != 0)
        goto cleanup;

    /* Next parameters. */
    if ((st = mat_subtract_scaled(W_ih, i2h_grads, learning_rate, &next_i2h)) !=
        0)
        goto cleanup;
    if ((st = mat_subtract_scaled(W_ho, h2o_grads, learning_rate, &next_h2o)) !=
        0)
        goto cleanup;
    next_hb = dup_doubles(p->hidden_biases, hidden_count);
    next_ob = dup_doubles(p->output_biases, output_count);
    if (!next_hb || !next_ob) {
        st = -2;
        goto cleanup;
    }
    for (i = 0; i < hidden_count; i++) {
        next_hb[i] =
            p->hidden_biases[i] - learning_rate * hidden_bias_grads.data[i];
    }
    for (i = 0; i < output_count; i++) {
        next_ob[i] =
            p->output_biases[i] - learning_rate * output_bias_grads.data[i];
    }

    /* Success: transfer ownership of the kept buffers into *out, then free the
     * internal-only temporaries. */
    out->predictions = predictions.data;
    out->errors = errors.data;
    out->output_deltas = output_deltas.data;
    out->hidden_deltas = hidden_deltas.data;
    out->hidden_to_output_weight_gradients = h2o_grads.data;
    out->output_bias_gradients = output_bias_grads.data;
    out->input_to_hidden_weight_gradients = i2h_grads.data;
    out->hidden_bias_gradients = hidden_bias_grads.data;
    out->next_parameters.input_to_hidden_weights = next_i2h.data;
    out->next_parameters.hidden_biases = next_hb;
    out->next_parameters.hidden_to_output_weights = next_h2o.data;
    out->next_parameters.output_biases = next_ob;
    out->next_parameters.input_count = input_count;
    out->next_parameters.hidden_count = hidden_count;
    out->next_parameters.output_count = output_count;
    out->loss = mean_squared_error(errors);
    out->samples = samples;
    out->input_count = input_count;
    out->hidden_count = hidden_count;
    out->output_count = output_count;

    mat_free(&hidden_raw);
    mat_free(&hidden_activations);
    mat_free(&output_raw);
    mat_free(&ha_T);
    mat_free(&W_ho_T);
    mat_free(&hidden_errors);
    mat_free(&in_T);
    return TLN_OK;

cleanup:
    /* Free every owned buffer (kept ones were never transferred on this path). */
    mat_free(&hidden_raw);
    mat_free(&hidden_activations);
    mat_free(&output_raw);
    mat_free(&predictions);
    mat_free(&errors);
    mat_free(&output_deltas);
    mat_free(&ha_T);
    mat_free(&h2o_grads);
    mat_free(&output_bias_grads);
    mat_free(&W_ho_T);
    mat_free(&hidden_errors);
    mat_free(&hidden_deltas);
    mat_free(&in_T);
    mat_free(&i2h_grads);
    mat_free(&hidden_bias_grads);
    mat_free(&next_i2h);
    mat_free(&next_h2o);
    free(next_hb);
    free(next_ob);
    return map_status(st);
}
