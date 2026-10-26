/* A dense network is a sequence of owned matrices. Forward saves activations;
 * backward follows the same chain in reverse. No foreign runtime supplies any
 * arithmetic: Mat owns products/transposes/reductions and float-math owns the
 * elementary nonlinear functions. All work is bounded by the public ceilings.
 */
#include "neural_framework.h"
#include "float_math.h"
#include <stdlib.h>
#include <string.h>
#include <stdio.h>
#include <locale.h>

typedef struct { Mat w, b; NFActivation activation; } Layer;
struct NFNetwork {
    NFMemory memory;
    size_t count;
    uint64_t step;
    Layer layers[16];
};

static void *ordinary_allocate(size_t bytes, void *context) {
    (void)context;
    return malloc(bytes);
}
static void ordinary_release(void *pointer, void *context) {
    (void)context;
    free(pointer);
}
static NFMemory ordinary_memory(void) {
    NFMemory m = {ordinary_allocate, ordinary_release, NULL};
    return m;
}
static int finite_cells(const double *data, size_t count) {
    size_t i;
    if (!data) return 0;
    for (i = 0; i < count; ++i) if (!fm_isfinite(data[i])) return 0;
    return 1;
}
static int finite_mat(const Mat *m) {
    return finite_cells(m->data, m->rows * m->cols);
}
static int valid_activation(NFActivation a) {
    return a >= NF_LINEAR && a <= NF_RELU;
}
/* Division guards precede multiplication. Though today's small limits imply
 * size_t safety, spelling it out keeps future limit edits from creating holes.
 */
static int product(size_t a, size_t b, size_t *out) {
    if (b && a > SIZE_MAX / b) return 0;
    *out = a * b;
    return *out <= SIZE_MAX / sizeof(double);
}
static NFStatus layer_shapes(const NFLayer *layers, size_t count) {
    size_t i, total = 0, cells, nodes = 5;
    if (!layers || !count) return NF_ARGUMENT;
    if (count > NF_MAX_LAYERS) return NF_LIMIT;
    /* Complete the shape pass BEFORE dereferencing any numeric buffer. */
    for (i = 0; i < count; ++i) {
        const NFLayer *l = &layers[i];
        if (!l->inputs || !l->outputs || !valid_activation(l->activation))
            return NF_SHAPE;
        if (l->inputs > NF_MAX_WIDTH || l->outputs > NF_MAX_WIDTH ||
            !product(l->inputs, l->outputs, &cells)) return NF_LIMIT;
        if (i && layers[i - 1].outputs != l->inputs) return NF_SHAPE;
        if (cells > NF_MAX_PARAMETERS - total) return NF_LIMIT;
        total += cells;
        if (l->outputs > NF_MAX_PARAMETERS - total) return NF_LIMIT;
        total += l->outputs;
        /* Every constructible model must also fit the public JSON reader's
         * node envelope: root values, layer fields, rows and numeric leaves.
         */
        if (4 + l->inputs + cells + l->outputs > BJ_MAX_NODES - nodes)
            return NF_LIMIT;
        nodes += 4 + l->inputs + cells + l->outputs;
    }
    return NF_OK;
}
void nf_free(NFNetwork **network) {
    size_t i;
    NFNetwork *n;
    NFMemory memory;
    if (!network || !*network) return;
    n = *network;
    memory = n->memory;
    for (i = 0; i < n->count; ++i) {
        mat_free(&n->layers[i].w); mat_free(&n->layers[i].b);
    }
    memory.release(n, memory.context);
    *network = NULL;
}
NFStatus nf_new_with_memory(const NFLayer *layers, size_t count,
                            const NFMemory *memory, NFNetwork **out) {
    NFNetwork *n;
    size_t i;
    NFStatus status;
    if (!out || *out || !memory || !memory->allocate || !memory->release)
        return NF_ARGUMENT;
    status = layer_shapes(layers, count);
    if (status != NF_OK) return status;
    for (i = 0; i < count; ++i)
        if (!finite_cells(layers[i].weights, layers[i].inputs * layers[i].outputs)
            || !finite_cells(layers[i].biases, layers[i].outputs))
            return NF_NONFINITE;
    n = memory->allocate(sizeof(*n), memory->context);
    if (!n) return NF_ALLOC;
    memset(n, 0, sizeof(*n));
    n->memory = *memory;
    n->count = count;
    for (i = 0; i < count; ++i) {
        n->layers[i].activation = layers[i].activation;
        if (mat_new(layers[i].inputs, layers[i].outputs, layers[i].weights,
                    &n->layers[i].w) != MAT_OK ||
            mat_new(1, layers[i].outputs, layers[i].biases,
                    &n->layers[i].b) != MAT_OK) {
            nf_free(&n);
            return NF_ALLOC;
        }
    }
    *out = n;
    return NF_OK;
}
NFStatus nf_new(const NFLayer *layers, size_t count, NFNetwork **out) {
    NFMemory memory = ordinary_memory();
    return nf_new_with_memory(layers, count, &memory, out);
}
uint64_t nf_step(const NFNetwork *n) { return n ? n->step : 0; }
size_t nf_layers(const NFNetwork *n) { return n ? n->count : 0; }
size_t nf_input_width(const NFNetwork *n) { return n ? n->layers[0].w.rows : 0; }
size_t nf_output_width(const NFNetwork *n) {
    return n ? n->layers[n->count - 1].w.cols : 0;
}
NFStatus nf_layer(const NFNetwork *n, size_t index, Mat *w, Mat *b,
                  NFActivation *activation) {
    if (!n || !w || !b || w == b || w->data || b->data || !activation)
        return NF_ARGUMENT;
    if (index >= n->count) return NF_SHAPE;
    if (mat_clone(&n->layers[index].w, w) != MAT_OK) return NF_ALLOC;
    if (mat_clone(&n->layers[index].b, b) != MAT_OK) {
        mat_free(w);
        return NF_ALLOC;
    }
    *activation = n->layers[index].activation;
    return NF_OK;
}
NFStatus nf_clone(const NFNetwork *n, NFNetwork **out) {
    NFLayer layers[16];
    NFStatus status;
    size_t i;
    if (!n) return NF_ARGUMENT;
    for (i = 0; i < n->count; ++i) {
        layers[i].inputs = n->layers[i].w.rows;
        layers[i].outputs = n->layers[i].w.cols;
        layers[i].weights = n->layers[i].w.data;
        layers[i].biases = n->layers[i].b.data;
        layers[i].activation = n->layers[i].activation;
    }
    status = nf_new_with_memory(layers, n->count, &n->memory, out);
    if (status == NF_OK) (*out)->step = n->step;
    return status;
}
NFStatus nf_seeded(const size_t *widths, const NFActivation *activations,
                   size_t count, uint32_t seed, NFNetwork **out) {
    NFLayer layers[16];
    /* Heap scratch rather than a 500 KiB array on Windows' 1 MiB stack. */
    double *values;
    size_t i, j, total = 0, offset = 0;
    NFStatus status;
    if (!out || *out || !widths || !activations || !count) return NF_ARGUMENT;
    if (count > NF_MAX_LAYERS) return NF_LIMIT;
    memset(layers, 0, sizeof(layers));
    for (i = 0; i < count; ++i) {
        layers[i].inputs = widths[i]; layers[i].outputs = widths[i + 1];
        layers[i].activation = activations[i];
    }
    status = layer_shapes(layers, count);
    if (status != NF_OK) return status;
    for (i = 0; i < count; ++i) total += widths[i] * widths[i + 1] + widths[i + 1];
    values = malloc(total * sizeof(*values));
    if (!values) return NF_ALLOC;
    for (i = 0; i < count; ++i) {
        size_t cells = widths[i] * widths[i + 1];
        double scale = fm_sqrt((double)widths[i]);
        layers[i].weights = values + offset;
        for (j = 0; j < cells; ++j) {
            seed = (uint32_t)(UINT32_C(1664525) * seed + UINT32_C(1013904223));
            values[offset++] = (2 * ((double)seed / 4294967296.0) - 1) / scale;
        }
        layers[i].biases = values + offset;
        for (j = 0; j < widths[i + 1]; ++j) values[offset++] = 0;
    }
    status = nf_new(layers, count, out);
    free(values);
    return status;
}

/* Stable sigmoid never asks exp to compute a large positive exponent. Saved
 * outputs suffice for sigmoid/tanh derivatives; ReLU uses the saved sign.
 */
static double activate(double z, NFActivation a) {
    if (a == NF_SIGMOID) {
        double e = fm_exp(z >= 0 ? -z : z);
        return z >= 0 ? 1 / (1 + e) : e / (1 + e);
    }
    if (a == NF_TANH) return fm_tanh(z);
    if (a == NF_RELU) return z > 0 ? z : 0;
    return z;
}
static double derivative(double value, NFActivation a) {
    if (a == NF_SIGMOID) return value * (1 - value);
    if (a == NF_TANH) return 1 - value * value;
    if (a == NF_RELU) return value > 0 ? 1 : 0;
    return 1;
}
static NFStatus batch(const NFNetwork *n, const double *x, const double *y,
                       size_t samples, int targets) {
    size_t cells;
    if (!n || !x || (targets && !y) || !samples) return NF_ARGUMENT;
    if (samples > NF_MAX_SAMPLES ||
        !product(samples, n->layers[0].w.rows, &cells)) return NF_LIMIT;
    if (!finite_cells(x, cells)) return NF_NONFINITE;
    if (targets) {
        if (!product(samples, n->layers[n->count - 1].w.cols, &cells))
            return NF_LIMIT;
        if (!finite_cells(y, cells)) return NF_NONFINITE;
    }
    return NF_OK;
}
static void tape_free(Mat *tape, size_t count) {
    size_t i;
    for (i = 0; i <= count; ++i) mat_free(&tape[i]);
}
static NFStatus forward(const NFNetwork *n, const double *x, size_t samples,
                         Mat *tape) {
    size_t i, j;
    if (mat_new(samples, n->layers[0].w.rows, x, &tape[0]) != MAT_OK)
        return NF_ALLOC;
    for (i = 0; i < n->count; ++i) {
        Mat *next = &tape[i + 1];
        const Layer *l = &n->layers[i];
        if (mat_dot(&tape[i], &l->w, next) != MAT_OK) return NF_ALLOC;
        /* Check preactivation, too: saturating infinity is not finite training. */
        for (j = 0; j < next->rows * next->cols; ++j) {
            double z = next->data[j] + l->b.data[j % next->cols];
            if (!fm_isfinite(z)) return NF_NONFINITE;
            next->data[j] = activate(z, l->activation);
        }
        if (!finite_mat(next)) return NF_NONFINITE;
    }
    return NF_OK;
}
NFStatus nf_predict(const NFNetwork *n, const double *x, size_t samples, Mat *out) {
    Mat tape[17] = {{0}};
    NFStatus status;
    if (!out || out->data) return NF_ARGUMENT;
    status = batch(n, x, NULL, samples, 0);
    if (status != NF_OK) return status;
    status = forward(n, x, samples, tape);
    if (status == NF_OK) {
        *out = tape[n->count]; memset(&tape[n->count], 0, sizeof(Mat));
    }
    tape_free(tape, n->count);
    return status;
}
void nf_trace_free(NFTrace *t) {
    size_t i;
    if (!t) return;
    /* Fixed slots, not caller-mutable layers, own cleanup. */
    mat_free(&t->predictions);
    for (i = 0; i < NF_MAX_LAYERS; ++i) {
        mat_free(&t->weights[i]); mat_free(&t->biases[i]);
    }
    memset(t, 0, sizeof(*t));
}
static int trace_empty(const NFTrace *t) {
    size_t i;
    if (!t || t->predictions.data || t->layers) return 0;
    for (i = 0; i < NF_MAX_LAYERS; ++i)
        if (t->weights[i].data || t->biases[i].data) return 0;
    return 1;
}
NFStatus nf_gradients(const NFNetwork *n, const double *x, const double *y,
                      size_t samples, NFTrace *out) {
    NFTrace result = {0};
    Mat tape[17] = {{0}}, delta = {0};
    size_t i, j, cells;
    NFStatus status;
    if (!trace_empty(out)) return NF_ARGUMENT;
    status = batch(n, x, y, samples, 1);
    if (status != NF_OK) return status;
    status = forward(n, x, samples, tape);
    if (status != NF_OK) goto done;
    cells = tape[n->count].rows * tape[n->count].cols;
    if (mat_zeros(samples, tape[n->count].cols, &delta) != MAT_OK) {
        status = NF_ALLOC; goto done;
    }
    for (j = 0; j < cells; ++j) {
        double difference = tape[n->count].data[j] - y[j];
        result.loss += difference * difference / (double)cells;
        delta.data[j] = 2 * difference / (double)cells;
    }
    if (!fm_isfinite(result.loss) || !finite_mat(&delta)) {
        status = NF_NONFINITE; goto done;
    }
    result.layers = n->count;
    for (i = n->count; i > 0; --i) {
        const Layer *l = &n->layers[i - 1];
        Mat transposed = {0}, wt = {0}, previous = {0};
        for (j = 0; j < delta.rows * delta.cols; ++j)
            delta.data[j] *= derivative(tape[i].data[j], l->activation);
        if (mat_transpose(&tape[i - 1], &transposed) != MAT_OK ||
            mat_dot(&transposed, &delta, &result.weights[i - 1]) != MAT_OK ||
            mat_sum_cols(&delta, &result.biases[i - 1]) != MAT_OK ||
            (i > 1 && (mat_transpose(&l->w, &wt) != MAT_OK ||
                        mat_dot(&delta, &wt, &previous) != MAT_OK)))
            status = NF_ALLOC;
        else if (!finite_mat(&result.weights[i - 1]) ||
                 !finite_mat(&result.biases[i - 1]) ||
                 (i > 1 && !finite_mat(&previous))) status = NF_NONFINITE;
        mat_free(&transposed); mat_free(&wt); mat_free(&delta);
        delta = previous;
        if (status != NF_OK) goto done;
    }
    result.predictions = tape[n->count];
    memset(&tape[n->count], 0, sizeof(Mat));
    *out = result;
    memset(&result, 0, sizeof(result));
done:
    mat_free(&delta); tape_free(tape, n->count); nf_trace_free(&result);
    return status;
}
NFStatus nf_train_step(NFNetwork *n, const double *x, const double *y,
                       size_t samples, double rate, NFTrace *out) {
    NFTrace trace = {0}, candidate_trace = {0};
    NFNetwork *candidate = NULL;
    NFStatus status;
    size_t i, j;
    if (!n || !trace_empty(out) || !fm_isfinite(rate) || rate <= 0)
        return NF_ARGUMENT;
    if (n->step >= NF_MAX_STEP) return NF_STEP;
    status = nf_gradients(n, x, y, samples, &trace);
    if (status != NF_OK) return status;
    status = nf_clone(n, &candidate);
    if (status != NF_OK) goto done;
    for (i = 0; i < n->count; ++i) {
        Layer *l = &candidate->layers[i];
        for (j = 0; j < l->w.rows * l->w.cols; ++j)
            l->w.data[j] -= rate * trace.weights[i].data[j];
        for (j = 0; j < l->b.cols; ++j)
            l->b.data[j] -= rate * trace.biases[i].data[j];
        if (!finite_mat(&l->w) || !finite_mat(&l->b)) {
            status = NF_NONFINITE; goto done;
        }
    }
    /* A finite parameter vector can still overflow on the actual batch. The
     * candidate's complete evaluation must pass BEFORE exchanging ownership.
     */
    status = nf_gradients(candidate, x, y, samples, &candidate_trace);
    if (status != NF_OK) goto done;
    for (i = 0; i < n->count; ++i) {
        Layer old = n->layers[i];
        n->layers[i] = candidate->layers[i]; candidate->layers[i] = old;
    }
    ++n->step;
    *out = trace; memset(&trace, 0, sizeof(trace));
done:
    nf_free(&candidate); nf_trace_free(&trace); nf_trace_free(&candidate_trace);
    return status;
}
void nf_history_free(NFHistory *h) {
    if (!h) return;
    if (h->losses) h->memory.release(h->losses, h->memory.context);
    memset(h, 0, sizeof(*h));
}
NFStatus nf_fit(NFNetwork *n, const double *x, const double *y, size_t samples,
                size_t epochs, double rate, NFHistory *out) {
    NFHistory h = {0};
    NFTrace t = {0};
    NFStatus status;
    size_t i, bytes;
    if (!n || !out || out->losses || out->count ||
        !fm_isfinite(rate) || rate <= 0) return NF_ARGUMENT;
    if (epochs > NF_MAX_EPOCHS || !product(epochs + 1, sizeof(double), &bytes))
        return NF_LIMIT;
    if ((uint64_t)epochs > NF_MAX_STEP - n->step) return NF_STEP;
    status = nf_gradients(n, x, y, samples, &t);
    if (status != NF_OK) return status;
    h.memory = n->memory;
    h.losses = h.memory.allocate(bytes, h.memory.context);
    if (!h.losses) { nf_trace_free(&t); return NF_ALLOC; }
    h.count = epochs + 1; h.losses[0] = t.loss; nf_trace_free(&t);
    for (i = 0; i < epochs; ++i) {
        status = nf_train_step(n, x, y, samples, rate, &t);
        nf_trace_free(&t);
        if (status != NF_OK) { nf_history_free(&h); return status; }
        status = nf_gradients(n, x, y, samples, &t);
        if (status != NF_OK) { nf_history_free(&h); return status; }
        h.losses[i + 1] = t.loss; nf_trace_free(&t);
    }
    *out = h;
    return NF_OK;
}

/* Schema loading is separate from parsing. Ordered NN47 members retain the
 * evidence needed to reject duplicate names and decoded embedded-NUL names.
 * Checking a closed field set by length+bytes prevents prefix truncation.
 */
static int fields(const BJsonValue *object, const char *const *names,
                   size_t count, const BJsonValue **values) {
    size_t i, j;
    if (bj_kind(object) != BJ_OBJECT || bj_size(object) != count) return 0;
    for (i = 0; i < count; ++i) values[i] = NULL;
    for (i = 0; i < count; ++i) {
        const char *name;
        size_t length;
        const BJsonValue *value;
        if (!bj_member(object, i, &name, &length, &value)) return 0;
        for (j = 0; j < count; ++j)
            if (length == strlen(names[j]) && !memcmp(name, names[j], length)) break;
        if (j == count || values[j]) return 0;
        values[j] = value;
    }
    return 1;
}
static int string_is(const BJsonValue *v, const char *expected) {
    const char *s;
    size_t length;
    return bj_string(v, &s, &length) && length == strlen(expected) &&
        !memcmp(s, expected, length);
}
static const char *const activation_names[] = {"linear", "sigmoid", "tanh", "relu"};
NFStatus nf_from_value(const BJsonValue *checkpoint, NFNetwork **out) {
    const char *const root_names[] = {"format", "version", "step", "layers"};
    const char *const layer_names[] = {"activation", "weights", "biases"};
    const BJsonValue *root[4], *parts[3], *weights[16], *biases[16];
    NFLayer layers[16];
    int64_t version, step;
    size_t i, j, k, count, total = 0, offset = 0;
    double *scratch;
    NFStatus status;
    if (!out || *out) return NF_ARGUMENT;
    if (!fields(checkpoint, root_names, 4, root) ||
        !string_is(root[0], "coding-adventures-dense") ||
        !bj_integer(root[1], &version) || version != 1 ||
        !bj_integer(root[2], &step) || step < 0 || (uint64_t)step > NF_MAX_STEP ||
        bj_kind(root[3]) != BJ_ARRAY || !(count = bj_size(root[3]))) return NF_SCHEMA;
    if (count > NF_MAX_LAYERS) return NF_LIMIT;
    memset(layers, 0, sizeof(layers));
    /* First validate EVERY dimension and parameter total, before scratch copies.
     * Unknown/duplicate fields are rejected even in later layers.
     */
    for (i = 0; i < count; ++i) {
        size_t a;
        if (!fields(bj_at(root[3], i), layer_names, 3, parts)) return NF_SCHEMA;
        for (a = 0; a < 4; ++a) if (string_is(parts[0], activation_names[a])) break;
        if (a == 4 || bj_kind(parts[1]) != BJ_ARRAY || bj_kind(parts[2]) != BJ_ARRAY)
            return NF_SCHEMA;
        layers[i].activation = (NFActivation)a;
        layers[i].inputs = bj_size(parts[1]);
        layers[i].outputs = bj_size(parts[2]);
        weights[i] = parts[1]; biases[i] = parts[2];
    }
    status = layer_shapes(layers, count);
    if (status != NF_OK) return status;
    for (i = 0; i < count; ++i) {
        for (j = 0; j < layers[i].inputs; ++j)
            if (bj_kind(bj_at(weights[i], j)) != BJ_ARRAY ||
                bj_size(bj_at(weights[i], j)) != layers[i].outputs) return NF_SCHEMA;
        total += layers[i].inputs * layers[i].outputs + layers[i].outputs;
    }
    scratch = malloc(total * sizeof(*scratch));
    if (!scratch) return NF_ALLOC;
    status = NF_SCHEMA;
    for (i = 0; i < count; ++i) {
        layers[i].weights = scratch + offset;
        for (j = 0; j < layers[i].inputs; ++j)
            for (k = 0; k < layers[i].outputs; ++k) {
                if (!bj_number(bj_at(bj_at(weights[i], j), k), &scratch[offset]) ||
                    !fm_isfinite(scratch[offset])) goto done_schema;
                ++offset;
            }
        layers[i].biases = scratch + offset;
        for (j = 0; j < layers[i].outputs; ++j) {
            if (!bj_number(bj_at(biases[i], j), &scratch[offset]) ||
                !fm_isfinite(scratch[offset])) goto done_schema;
            ++offset;
        }
    }
    status = nf_new(layers, count, out);
    if (status == NF_OK) (*out)->step = (uint64_t)step;
done_schema:
    free(scratch);
    return status;
}
NFStatus nf_from_json(const char *text, size_t length, NFNetwork **out) {
    BJsonDoc *doc = NULL;
    BJsonStatus parsed;
    NFStatus status;
    if (!out || *out || !text) return NF_ARGUMENT;
    parsed = bj_parse(text, length, &doc, NULL);
    if (parsed != BJ_OK) return parsed == BJ_ALLOC ? NF_ALLOC :
        (parsed >= BJ_BYTE_LIMIT && parsed <= BJ_MEMORY_LIMIT ? NF_LIMIT : NF_SCHEMA);
    status = nf_from_value(bj_root(doc), out);
    bj_doc_free(&doc);
    return status;
}

/* A count-only first pass prevents growth/reallocation and partially published
 * text. The second pass owns an exactly sized buffer. Only closed static names
 * and finite formatted numbers enter it; no untrusted string needs escaping.
 */
typedef struct { char *data; size_t used, capacity; int failed; } Writer;
static void append(Writer *w, const char *s) {
    size_t length = strlen(s);
    if (w->failed || length > BJ_MAX_BYTES - w->used ||
        (w->data && (w->used >= w->capacity || length >= w->capacity - w->used))) {
        w->failed = 1; return;
    }
    if (w->data) memcpy(w->data + w->used, s, length);
    w->used += length;
}
static void number(Writer *w, double value) {
    char buffer[128], normalized[128];
    const char *decimal;
    char *point;
    size_t before, after;
    int length = snprintf(buffer, sizeof(buffer), "%.17g", value);
    if (!fm_isfinite(value) || length < 0 || (size_t)length >= sizeof(buffer)) {
        w->failed = 1; return;
    }
    decimal = localeconv()->decimal_point;
    if (!decimal || !*decimal || strlen(decimal) > 16) { w->failed = 1; return; }
    point = strstr(buffer, decimal);
    if (!point || !strcmp(decimal, ".")) { append(w, buffer); return; }
    before = (size_t)(point - buffer);
    after = strlen(point + strlen(decimal));
    memcpy(normalized, buffer, before); normalized[before] = '.';
    memcpy(normalized + before + 1, point + strlen(decimal), after + 1);
    append(w, normalized);
}
static void serialize(const NFNetwork *n, Writer *w) {
    size_t i, j, k;
    char integer[32];
    append(w, "{\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":");
    (void)snprintf(integer, sizeof(integer), "%llu", (unsigned long long)n->step);
    append(w, integer); append(w, ",\"layers\":[");
    for (i = 0; i < n->count; ++i) {
        const Layer *l = &n->layers[i];
        if (i) append(w, ",");
        append(w, "{\"activation\":\""); append(w, activation_names[l->activation]);
        append(w, "\",\"weights\":[");
        for (j = 0; j < l->w.rows; ++j) {
            if (j) append(w, ",");
            append(w, "[");
            for (k = 0; k < l->w.cols; ++k) {
                if (k) append(w, ",");
                number(w, l->w.data[j * l->w.cols + k]);
            }
            append(w, "]");
        }
        append(w, "],\"biases\":[");
        for (j = 0; j < l->b.cols; ++j) {
            if (j) append(w, ",");
            number(w, l->b.data[j]);
        }
        append(w, "]}");
    }
    append(w, "]}");
}
void nf_text_free(NFText *text) {
    if (!text) return;
    if (text->data) text->memory.release(text->data, text->memory.context);
    memset(text, 0, sizeof(*text));
}
NFStatus nf_to_json(const NFNetwork *n, NFText *out) {
    Writer w = {0};
    NFText text = {0};
    if (!n || !out || out->data || out->length) return NF_ARGUMENT;
    serialize(n, &w);
    if (w.failed) return NF_LIMIT;
    text.memory = n->memory; text.length = w.used;
    text.data = text.memory.allocate(text.length + 1, text.memory.context);
    if (!text.data) return NF_ALLOC;
    w.data = text.data; w.capacity = text.length + 1; w.used = 0;
    serialize(n, &w);
    if (w.failed || w.used != text.length) { nf_text_free(&text); return NF_LIMIT; }
    text.data[text.length] = '\0';
    *out = text;
    return NF_OK;
}
