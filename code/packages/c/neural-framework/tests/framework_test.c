/* Tests precede the implementation: a hand oracle makes backprop inspectable.
 * Later fixture/schema tests extend this executable, not a second scheduler.
 */
#ifdef _MSC_VER
#define _CRT_SECURE_NO_WARNINGS /* Test-only ISO fopen, not nonportable fopen_s. */
#endif
#include "neural_framework.h"
#include "float_math.h"
#include <float.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <locale.h>

static size_t checks, failures;
#define CHECK(x) do { ++checks; if (!(x)) { ++failures; \
    fprintf(stderr, "line %d: %s\n", __LINE__, #x); } } while (0)
static void close_value(double actual, double expected, double tolerance) {
    CHECK(fm_isfinite(actual) && fm_fabs(actual - expected) <= tolerance);
}
static BJsonDoc *fixture(const char *name);
static const BJsonValue *member(const BJsonValue *v, const char *key);
static double numeric(const BJsonValue *v);
static int string_matches(const BJsonValue *v, const char *expected) {
    const char *s;
    size_t length;
    return bj_string(v, &s, &length) && length == strlen(expected) &&
        !memcmp(s, expected, length);
}
static void fixture_array(double *out, size_t length, const BJsonValue *array) {
    size_t j;
    CHECK(bj_kind(array) == BJ_ARRAY && bj_size(array) == length);
    for (j = 0; j < length; ++j) out[j] = numeric(bj_at(array, j));
}
static NFNetwork *paper(void) {
    const double w1[] = {.5, -.5, 1, .5}, b1[] = {0, 0};
    const double w2[] = {1, -1}, b2[] = {.25};
    const NFLayer layers[] = {{2, 2, NF_LINEAR, w1, b1},
                              {2, 1, NF_LINEAR, w2, b2}};
    NFNetwork *n = NULL;
    CHECK(nf_new(layers, 2, &n) == NF_OK);
    return n;
}
static void paper_oracle(void) {
    NFNetwork *n = paper(), *copy = NULL;
    const double x[] = {1, 2}, y[] = {1};
    NFTrace t = {0};
    Mat p = {0}, w = {0}, b = {0};
    NFActivation activation;
    NFHistory h = {0};
    const double dw[] = {2.5, -2.5, 5, -5};
    size_t k;
    CHECK(nf_gradients(n, x, y, 1, &t) == NF_OK);
    close_value(t.loss, 1.5625, 1e-12);
    close_value(t.predictions.data[0], 2.25, 1e-12);
    for (k = 0; k < 4; ++k) close_value(t.weights[0].data[k], dw[k], 1e-12);
    close_value(t.weights[1].data[0], 6.25, 1e-12);
    close_value(t.weights[1].data[1], 1.25, 1e-12);
    close_value(t.biases[0].data[0], 2.5, 1e-12);
    close_value(t.biases[0].data[1], -2.5, 1e-12);
    close_value(t.biases[1].data[0], 2.5, 1e-12);
    CHECK(nf_step(n) == 0);
    nf_trace_free(&t);
    CHECK(nf_fit(n, x, y, 1, 0, .01, &h) == NF_OK);
    CHECK(h.count == 1 && nf_step(n) == 0);
    close_value(h.losses[0], 1.5625, 1e-12);
    nf_history_free(&h);
    CHECK(nf_clone(n, &copy) == NF_OK);
    CHECK(nf_train_step(n, x, y, 1, .01, &t) == NF_OK);
    CHECK(nf_step(n) == 1 && nf_step(copy) == 0);
    close_value(t.loss, 1.5625, 1e-12);
    CHECK(nf_predict(n, x, 1, &p) == NF_OK);
    close_value(p.data[0], 1.77, 1e-12);
    mat_free(&p);
    CHECK(nf_gradients(n, x, y, 1, &t) == NF_ARGUMENT);
    /* A live output must never be overwritten (the previous trace is owned). */
    CHECK(nf_train_step(n, x, y, 1, .01, &t) == NF_ARGUMENT);
    nf_trace_free(&t);
    CHECK(nf_layer(n, 0, &w, &b, &activation) == NF_OK);
    close_value(w.data[0], .475, 1e-12);
    close_value(b.data[0], -.025, 1e-12);
    CHECK(activation == NF_LINEAR);
    w.data[0] = 900;
    mat_free(&w); mat_free(&b);
    CHECK(nf_predict(n, x, 1, &p) == NF_OK);
    close_value(p.data[0], 1.77, 1e-12);
    mat_free(&p);
    nf_trace_free(&t); nf_free(&n); nf_free(&copy); nf_free(&n);
}

/* Central differences visit EVERY weight and bias, not merely a loss slope.
 * Positive ReLU paths ensure its backward pass is active, away from the kink.
 */
static double loss_of(const NFLayer *layers, size_t count, const double *x,
                      const double *y) {
    NFNetwork *n = NULL;
    NFTrace t = {0};
    double loss;
    CHECK(nf_new(layers, count, &n) == NF_OK);
    CHECK(nf_gradients(n, x, y, 2, &t) == NF_OK);
    loss = t.loss;
    nf_trace_free(&t); nf_free(&n);
    return loss;
}
static void differences(void) {
    int a;
    for (a = NF_LINEAR; a <= NF_RELU; ++a) {
        double w1[] = {.2, .3, .4, .5, .6, .7}, b1[] = {.1, .2, .3};
        double w2[] = {.3, .2, .4, .5, .2, .6}, b2[] = {.2, .3};
        double w3[] = {.4, .2, .3, .5}, b3[] = {.1, .2};
        const double x[] = {.2, .5, .7, .4}, y[] = {-.2, -.3, -.4, -.1};
        NFLayer layers[] = {{2, 3, (NFActivation)a, w1, b1},
                            {3, 2, (NFActivation)a, w2, b2},
                            {2, 2, (NFActivation)a, w3, b3}};
        double *arrays[] = {w1, b1, w2, b2, w3, b3};
        size_t lengths[] = {6, 3, 6, 2, 4, 2}, j, k;
        NFNetwork *n = NULL;
        NFTrace t = {0};
        CHECK(nf_new(layers, 3, &n) == NF_OK);
        CHECK(nf_gradients(n, x, y, 2, &t) == NF_OK);
        for (j = 0; j < 6; ++j) for (k = 0; k < lengths[j]; ++k) {
            const double old = arrays[j][k], epsilon = 1e-6;
            double plus, minus, analytic = (j % 2 == 0 ?
                t.weights[j / 2] : t.biases[j / 2]).data[k];
            arrays[j][k] = old + epsilon;
            plus = loss_of(layers, 3, x, y);
            arrays[j][k] = old - epsilon;
            minus = loss_of(layers, 3, x, y);
            arrays[j][k] = old;
            if (a == NF_RELU) CHECK(fm_fabs(analytic) > 1e-8);
            close_value(analytic, (plus - minus) / (2 * epsilon), 1e-7);
        }
        nf_trace_free(&t); nf_free(&n);
    }
}

static void same_network(const NFNetwork *a, const NFNetwork *b) {
    size_t i, j;
    CHECK(nf_step(a) == nf_step(b) && nf_layers(a) == nf_layers(b));
    for (i = 0; i < nf_layers(a); ++i) {
        Mat aw = {0}, ab = {0}, bw = {0}, bb = {0};
        NFActivation aa, ba;
        CHECK(nf_layer(a, i, &aw, &ab, &aa) == NF_OK);
        CHECK(nf_layer(b, i, &bw, &bb, &ba) == NF_OK);
        CHECK(aa == ba && aw.rows == bw.rows && aw.cols == bw.cols && ab.cols == bb.cols);
        for (j = 0; j < aw.rows * aw.cols; ++j) close_value(aw.data[j], bw.data[j], 1e-9);
        for (j = 0; j < ab.cols; ++j) close_value(ab.data[j], bb.data[j], 1e-9);
        mat_free(&aw); mat_free(&ab); mat_free(&bw); mat_free(&bb);
    }
}

static void xor_training(void) {
    BJsonDoc *doc = fixture("../../../specs/fixtures/neural-framework-v1/dense.json");
    const BJsonValue *v = member(bj_root(doc), "xor");
    size_t widths[3], epochs, i;
    uint32_t seed;
    double rate, maximum_loss, x[8], y[4];
    const NFActivation acts[] = {NF_TANH, NF_SIGMOID};
    NFNetwork *n = NULL, *loaded = NULL, *baseline = NULL;
    NFHistory h = {0};
    NFText text = {0};
    size_t step, j;
    CHECK(bj_size(member(v, "widths")) == 3);
    for (i = 0; i < 3; ++i) widths[i] = (size_t)numeric(bj_at(member(v, "widths"), i));
    CHECK(widths[0] == 2 && widths[1] == 4 && widths[2] == 1);
    CHECK(string_matches(bj_at(member(v, "activations"), 0), "tanh"));
    CHECK(string_matches(bj_at(member(v, "activations"), 1), "sigmoid"));
    for (i = 0; i < 4; ++i) {
        fixture_array(x + 2 * i, 2, bj_at(member(v, "inputs"), i));
        fixture_array(y + i, 1, bj_at(member(v, "targets"), i));
    }
    seed = (uint32_t)numeric(member(v, "seed"));
    epochs = (size_t)numeric(member(v, "epochs"));
    rate = numeric(member(v, "learning_rate"));
    maximum_loss = numeric(member(v, "maximum_final_loss"));
    CHECK(seed == 7 && epochs == 3000 && rate == 1 && maximum_loss == .01);
    CHECK(nf_seeded(widths, acts, 2, seed, &n) == NF_OK);
    CHECK(nf_fit(n, x, y, 4, epochs, rate, &h) == NF_OK);
    CHECK(h.count == 3001 && nf_step(n) == 3000);
    CHECK(h.losses[epochs] < maximum_loss);
    printf("native C XOR: step=%llu loss=%.17g\n",
           (unsigned long long)nf_step(n), h.losses[3000]);
    nf_history_free(&h);
    CHECK(nf_to_json(n, &text) == NF_OK);
    CHECK(nf_from_json(text.data, text.length, &loaded) == NF_OK);
    nf_text_free(&text);
    for (step = 3001; step <= 3002; ++step) {
        NFTrace resumed = {0}, direct = {0};
        CHECK(nf_fit(loaded, x, y, 4, 1, rate, &h) == NF_OK); nf_history_free(&h);
        CHECK(nf_seeded(widths, acts, 2, seed, &baseline) == NF_OK);
        CHECK(nf_fit(baseline, x, y, 4, step, rate, &h) == NF_OK); nf_history_free(&h);
        same_network(loaded, baseline);
        CHECK(nf_gradients(loaded, x, y, 4, &resumed) == NF_OK);
        CHECK(nf_gradients(baseline, x, y, 4, &direct) == NF_OK);
        close_value(resumed.loss, direct.loss, 1e-9);
        for (j = 0; j < 4; ++j)
            close_value(resumed.predictions.data[j], direct.predictions.data[j], 1e-9);
        nf_trace_free(&resumed); nf_trace_free(&direct); nf_free(&baseline);
        /* Each return step crosses serialization, not merely a live clone. */
        CHECK(nf_to_json(loaded, &text) == NF_OK); nf_free(&loaded);
        CHECK(nf_from_json(text.data, text.length, &loaded) == NF_OK); nf_text_free(&text);
    }
    nf_free(&loaded); nf_free(&n); bj_doc_free(&doc);
}

static const BJsonValue *member(const BJsonValue *v, const char *key) {
    size_t i;
    for (i = 0; i < bj_size(v); ++i) {
        const char *name;
        size_t length;
        const BJsonValue *value;
        if (bj_member(v, i, &name, &length, &value) && length == strlen(key) &&
            !memcmp(name, key, length)) return value;
    }
    CHECK(0);
    return NULL;
}
static BJsonDoc *fixture(const char *name) {
    FILE *f = fopen(name, "rb");
    char *text;
    long length;
    BJsonDoc *doc = NULL;
    CHECK(f != NULL);
    if (!f) return NULL;
    CHECK(fseek(f, 0, SEEK_END) == 0);
    length = ftell(f);
    CHECK(length > 0 && (unsigned long)length <= BJ_MAX_BYTES);
    rewind(f);
    if (length <= 0 || (unsigned long)length > BJ_MAX_BYTES) { fclose(f); return NULL; }
    text = malloc((size_t)length);
    CHECK(text != NULL);
    if (!text) { fclose(f); return NULL; }
    CHECK(fread(text, 1, (size_t)length, f) == (size_t)length);
    CHECK(fclose(f) == 0);
    CHECK(bj_parse(text, (size_t)length, &doc, NULL) == BJ_OK);
    free(text);
    return doc;
}
static double numeric(const BJsonValue *v) {
    double number = 0;
    CHECK(bj_number(v, &number));
    return number;
}
static void compare_array(const double *data, size_t count, const BJsonValue *v) {
    size_t j;
    CHECK(bj_kind(v) == BJ_ARRAY && bj_size(v) == count);
    for (j = 0; j < count; ++j) close_value(data[j], numeric(bj_at(v, j)), 1e-12);
}
static void compare_matrix(const Mat *m, const BJsonValue *v) {
    size_t i;
    CHECK(bj_size(v) == m->rows);
    for (i = 0; i < m->rows; ++i)
        compare_array(m->data + i * m->cols, m->cols, bj_at(v, i));
}
static void shared_fixtures(void) {
    BJsonDoc *doc = fixture("../../../specs/fixtures/neural-framework-v1/dense.json");
    const BJsonValue *root = bj_root(doc), *p = member(root, "paper");
    const BJsonValue *seed = member(root, "seed");
    NFNetwork *n = NULL, *round = NULL;
    NFTrace t = {0};
    NFText text = {0};
    Mat w = {0}, b = {0};
    NFActivation activation;
    double x[2], y[1];
    size_t widths[2];
    const NFActivation acts[] = {NF_TANH};
    size_t i;
    fixture_array(x, 2, bj_at(member(p, "inputs"), 0));
    fixture_array(y, 1, bj_at(member(p, "targets"), 0));
    widths[0] = (size_t)numeric(bj_at(member(seed, "widths"), 0));
    widths[1] = (size_t)numeric(bj_at(member(seed, "widths"), 1));
    CHECK(string_matches(bj_at(member(seed, "activations"), 0), "tanh"));
    CHECK(nf_from_value(member(p, "checkpoint"), &n) == NF_OK);
    CHECK(nf_gradients(n, x, y, 1, &t) == NF_OK);
    close_value(t.loss, numeric(member(p, "loss")), 1e-12);
    compare_matrix(&t.predictions, member(p, "predictions"));
    for (i = 0; i < t.layers; ++i) {
        compare_matrix(&t.weights[i], bj_at(member(p, "weight_gradients"), i));
        compare_array(t.biases[i].data, t.biases[i].cols,
                      bj_at(member(p, "bias_gradients"), i));
    }
    nf_trace_free(&t);
    CHECK(nf_train_step(n, x, y, 1, numeric(member(p, "learning_rate")), &t) == NF_OK);
    nf_trace_free(&t);
    CHECK(nf_to_json(n, &text) == NF_OK);
    CHECK(nf_from_json(text.data, text.length, &round) == NF_OK);
    CHECK(nf_gradients(round, x, y, 1, &t) == NF_OK);
    compare_matrix(&t.predictions, member(p, "updated_predictions"));
    close_value(t.loss, numeric(member(p, "updated_loss")), 1e-12);
    for (i = 0; i < nf_layers(round); ++i) {
        const BJsonValue *l = bj_at(member(member(p, "updated_checkpoint"), "layers"), i);
        CHECK(nf_layer(round, i, &w, &b, &activation) == NF_OK);
        compare_matrix(&w, member(l, "weights"));
        compare_array(b.data, b.cols, member(l, "biases"));
        mat_free(&w); mat_free(&b);
    }
    nf_trace_free(&t); nf_free(&n); nf_free(&round); nf_text_free(&text);
    CHECK(nf_seeded(widths, acts, 1, (uint32_t)numeric(member(seed, "value")), &n) == NF_OK);
    CHECK(nf_layer(n, 0, &w, &b, &activation) == NF_OK);
    compare_matrix(&w, member(seed, "first_weights"));
    close_value(b.data[0], 0, 0); close_value(b.data[1], 0, 0);
    mat_free(&w); mat_free(&b); nf_free(&n); bj_doc_free(&doc);

    doc = fixture("../../../specs/fixtures/neural-framework-v1/checkpoints.json");
    CHECK(bj_size(bj_root(doc)) == 2 && string_matches(member(bj_root(doc), "contract"), "NN40"));
    root = member(bj_root(doc), "cases");
    CHECK(bj_size(root) == 55);
    {
        size_t accepts = 0, rejects = 0, nulls = 0;
        const char *const null_ids[] = {"null-weight-cell", "null-weight-cell-mixed",
            "null-bias-cell", "null-bias-cell-mixed", "null-weight-cell-later-layer",
            "null-bias-cell-later-layer"};
        unsigned seen = 0;
        const char *ids[55]; size_t lengths[55];
        for (i = 0; i < bj_size(root); ++i) {
            const BJsonValue *c = bj_at(root, i);
            const char *id;
            size_t length;
            int accept = -1;
            NFStatus status;
            CHECK(bj_boolean(member(c, "accept"), &accept));
            CHECK(bj_string(member(c, "id"), &id, &length));
            CHECK(bj_size(c) == 3);
            {
                size_t j;
                for (j = 0; j < i; ++j)
                    CHECK(length != lengths[j] || memcmp(id, ids[j], length) != 0);
                ids[i] = id; lengths[i] = length;
                for (j = 0; j < 6; ++j) if (length == strlen(null_ids[j]) &&
                        !memcmp(id, null_ids[j], length)) {
                    CHECK(!accept && !(seen & (1u << j)));
                    seen |= 1u << j; ++nulls;
                }
            }
            status = nf_from_value(member(c, "checkpoint"), &n);
            CHECK((status == NF_OK) == accept);
            if (accept) {
                ++accepts;
                CHECK(nf_to_json(n, &text) == NF_OK);
                CHECK(nf_from_json(text.data, text.length, &round) == NF_OK);
                CHECK(nf_step(n) == nf_step(round));
            } else { ++rejects; CHECK(n == NULL); }
            nf_free(&n); nf_free(&round); nf_text_free(&text);
        }
        CHECK(accepts == 3 && rejects == 52 && nulls == 6 && seen == 63);
        printf("public C schema loader: %llu accepted, %llu rejected, %llu null cases\n",
               (unsigned long long)accepts, (unsigned long long)rejects,
               (unsigned long long)nulls);
    }
    bj_doc_free(&doc);
}

static void boundaries(void) {
    NFNetwork *n = NULL;
    double w[] = {0}, b[] = {0}, x[] = {2}, y[] = {1};
    NFLayer l = {1, 1, NF_LINEAR, w, b};
    NFTrace t = {0};
    NFHistory h = {0};
    Mat m = {0}, bias = {0};
    NFActivation a;
    NFText before = {0}, after = {0};
    size_t widths[] = {1, SIZE_MAX};
    NFActivation acts[] = {NF_LINEAR};
    CHECK(nf_step(NULL) == 0 && nf_layers(NULL) == 0);
    CHECK(nf_input_width(NULL) == 0 && nf_output_width(NULL) == 0);
    nf_free(NULL); nf_trace_free(NULL); nf_history_free(NULL); nf_text_free(NULL);
    CHECK(nf_new(NULL, 1, &n) == NF_ARGUMENT);
    CHECK(nf_new(&l, 0, &n) == NF_ARGUMENT);
    CHECK(nf_new(&l, 17, &n) == NF_LIMIT);
    CHECK(nf_new(&l, 1, NULL) == NF_ARGUMENT);
    l.inputs = SIZE_MAX; CHECK(nf_new(&l, 1, &n) == NF_LIMIT); l.inputs = 0;
    CHECK(nf_new(&l, 1, &n) == NF_SHAPE); l.inputs = 1;
    l.activation = (NFActivation)99; CHECK(nf_new(&l, 1, &n) == NF_SHAPE);
    l.activation = NF_LINEAR;
    w[0] = fm_nan(); CHECK(nf_new(&l, 1, &n) == NF_NONFINITE); w[0] = 0;
    l.weights = NULL; CHECK(nf_new(&l, 1, &n) == NF_NONFINITE); l.weights = w;
    CHECK(nf_seeded(widths, acts, 1, 0, &n) == NF_LIMIT);
    CHECK(nf_seeded(NULL, acts, 1, 0, &n) == NF_ARGUMENT);
    CHECK(nf_seeded(widths, acts, 17, 0, &n) == NF_LIMIT);
    CHECK(nf_new(&l, 1, &n) == NF_OK);
    CHECK(nf_input_width(n) == 1 && nf_output_width(n) == 1);
    w[0] = 900; b[0] = 900; /* Constructor snapshot must already own zeroes. */
    CHECK(nf_predict(n, x, 1, &m) == NF_OK); close_value(m.data[0], 0, 0);
    CHECK(nf_predict(n, x, 1, &m) == NF_ARGUMENT); mat_free(&m);
    CHECK(nf_predict(n, x, 0, &m) == NF_ARGUMENT);
    CHECK(nf_predict(n, x, SIZE_MAX, &m) == NF_LIMIT);
    CHECK(nf_predict(NULL, x, 1, &m) == NF_ARGUMENT);
    CHECK(nf_layer(n, 1, &m, &bias, &a) == NF_SHAPE);
    CHECK(nf_layer(n, 0, &m, &m, &a) == NF_ARGUMENT);
    x[0] = fm_inf(); CHECK(nf_predict(n, x, 1, &m) == NF_NONFINITE); x[0] = 2;
    y[0] = fm_nan(); CHECK(nf_gradients(n, x, y, 1, &t) == NF_NONFINITE); y[0] = 1;
    CHECK(nf_train_step(n, x, y, 1, 0, &t) == NF_ARGUMENT);
    CHECK(nf_train_step(n, x, y, 1, -1, &t) == NF_ARGUMENT);
    CHECK(nf_train_step(n, x, y, 1, fm_inf(), &t) == NF_ARGUMENT);
    CHECK(nf_fit(n, x, y, 1, SIZE_MAX, .1, &h) == NF_LIMIT);
    CHECK(nf_fit(n, x, y, 1, 0, 0, &h) == NF_ARGUMENT);
    CHECK(nf_to_json(n, &before) == NF_OK);
    /* Finite candidates, nonfinite candidate loss: not just a parameter check. */
    CHECK(nf_train_step(n, x, y, 1, 1e154, &t) == NF_NONFINITE);
    CHECK(nf_step(n) == 0 && t.layers == 0);
    CHECK(nf_train_step(n, x, y, 1, DBL_MAX, &t) == NF_NONFINITE);
    CHECK(nf_to_json(n, &after) == NF_OK);
    CHECK(before.length == after.length && !memcmp(before.data, after.data, before.length));
    nf_text_free(&before); nf_text_free(&after); nf_free(&n);
    {
        const char *maxstep = "{\"format\":\"coding-adventures-dense\",\"version\":1,"
          "\"step\":9007199254740991,\"layers\":[{\"activation\":\"linear\",\"weights\":[[0]],\"biases\":[0]}]}";
        CHECK(nf_from_json(maxstep, strlen(maxstep), &n) == NF_OK);
        CHECK(nf_step(n) == NF_MAX_STEP);
        CHECK(nf_train_step(n, x, y, 1, .1, &t) == NF_STEP);
        CHECK(nf_fit(n, x, y, 1, 1, .1, &h) == NF_STEP);
        CHECK(nf_fit(n, x, y, 1, 0, .1, &h) == NF_OK);
        nf_history_free(&h); nf_free(&n);
    }
    /* ReLU at the kink is separate from the ACTIVE finite difference networks. */
    w[0] = 0; b[0] = 0; l.activation = NF_RELU;
    CHECK(nf_new(&l, 1, &n) == NF_OK);
    CHECK(nf_gradients(n, x, y, 1, &t) == NF_OK);
    close_value(t.weights[0].data[0], 0, 0); close_value(t.biases[0].data[0], 0, 0);
    nf_trace_free(&t); nf_free(&n);
}

static void text_boundaries_and_locale(void) {
    static const char *const metadata[] = {
        "\"format\":\"coding-adventures-dense\",\"version\":1.0,\"step\":0",
        "\"format\":\"coding-adventures-dense\",\"version\":1e0,\"step\":0",
        "\"format\":\"coding-adventures-dense\",\"version\":true,\"step\":0",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":0.0",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":0e0",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":false",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":null",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":\"0\"",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":9007199254740992",
        "\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":-1",
        "\"format\":\"coding-adventures-dense\",\"format\":\"coding-adventures-dense\",\"step\":0",
        "\"format\\u0000x\":\"coding-adventures-dense\",\"version\":1,\"step\":0"
    };
    static const char *const bad_layers[] = {
        "{\"activation\":\"linear\",\"activation\":\"linear\",\"biases\":[0]}",
        "{\"activation\\u0000\":\"linear\",\"weights\":[[0]],\"biases\":[0]}",
        "{\"activation\":\"linear\\u0000\",\"weights\":[[0]],\"biases\":[0]}"
    };
    NFNetwork *n = NULL;
    char buffer[1024], nesting[263];
    size_t i;
    for (i = 0; i < sizeof(metadata) / sizeof(metadata[0]); ++i) {
        int length = snprintf(buffer, sizeof(buffer), "{%s,\"layers\":[{\"activation\":\"linear\",\"weights\":[[0]],\"biases\":[0]}]}", metadata[i]);
        CHECK(length > 0 && (size_t)length < sizeof(buffer));
        CHECK(nf_from_json(buffer, strlen(buffer), &n) == NF_SCHEMA && n == NULL);
    }
    for (i = 0; i < sizeof(bad_layers) / sizeof(bad_layers[0]); ++i) {
        int length = snprintf(buffer, sizeof(buffer), "{\"format\":\"coding-adventures-dense\",\"version\":1,\"step\":0,\"layers\":[%s]}", bad_layers[i]);
        CHECK(length > 0 && (size_t)length < sizeof(buffer));
        CHECK(nf_from_json(buffer, strlen(buffer), &n) == NF_SCHEMA && n == NULL);
    }
    CHECK(nf_from_json("{} {}", 5, &n) == NF_SCHEMA && n == NULL);
    CHECK(nf_from_json("", BJ_MAX_BYTES + 1, &n) == NF_LIMIT && n == NULL);
    memset(nesting, '[', 131); nesting[131] = '0';
    memset(nesting + 132, ']', 131);
    CHECK(nf_from_json(nesting, sizeof(nesting), &n) == NF_LIMIT && n == NULL);
    CHECK(nf_from_json(NULL, 0, &n) == NF_ARGUMENT);
    CHECK(nf_from_value(NULL, &n) == NF_SCHEMA);
    CHECK(nf_to_json(NULL, &(NFText){0}) == NF_ARGUMENT);
    {
        const char *locales[] = {"de_DE.UTF-8", "German_Germany.1252", "de-DE", "C"};
        const double weights[] = {DBL_MAX, DBL_MIN, -0.0, 1.0 / 3.0};
        const double biases[] = {5e-324, -.0};
        NFLayer layer = {2, 2, NF_LINEAR, weights, biases};
        CHECK(nf_new(&layer, 1, &n) == NF_OK);
        for (i = 0; i < sizeof(locales) / sizeof(locales[0]); ++i) {
            const char *selected = setlocale(LC_NUMERIC, locales[i]);
            if (selected) {
                char saved[256];
                NFNetwork *loaded = NULL;
                NFText text = {0};
                Mat w = {0}, b = {0};
                NFActivation a;
                size_t length = strlen(selected);
                CHECK(length < sizeof(saved));
                if (length >= sizeof(saved)) continue;
                memcpy(saved, selected, length + 1);
                CHECK(nf_to_json(n, &text) == NF_OK);
                CHECK(nf_to_json(n, &text) == NF_ARGUMENT);
                CHECK(nf_from_json(text.data, text.length, &loaded) == NF_OK);
                CHECK(nf_from_json(text.data, text.length, &loaded) == NF_ARGUMENT);
                CHECK(!strcmp(saved, setlocale(LC_NUMERIC, NULL)));
                CHECK(nf_layer(loaded, 0, &w, &b, &a) == NF_OK);
                CHECK(!memcmp(w.data, weights, sizeof(weights)));
                CHECK(!memcmp(b.data, biases, sizeof(biases)));
                nf_text_free(&text); mat_free(&w); mat_free(&b); nf_free(&loaded);
            }
        }
        CHECK(setlocale(LC_NUMERIC, "C") != NULL);
        nf_free(&n);
    }
    /* Largest structural limits reject BEFORE touching nonexistent numeric
     * buffers; a valid wide model is independently serializable/reloadable.
     */
    {
        NFLayer layers[16];
        const size_t widths[] = {64, 64};
        const NFActivation activations[] = {NF_LINEAR};
        NFNetwork *loaded = NULL;
        NFText text = {0};
        for (i = 0; i < 16; ++i) {
            layers[i].inputs = 64; layers[i].outputs = 64;
            layers[i].activation = NF_LINEAR;
            layers[i].weights = NULL; layers[i].biases = NULL;
        }
        CHECK(nf_new(layers, 16, &n) == NF_LIMIT && n == NULL);
        layers[0].outputs = 2;
        CHECK(nf_new(layers, 2, &n) == NF_SHAPE);
        CHECK(nf_seeded(widths, activations, 1, UINT32_MAX, &n) == NF_OK);
        CHECK(nf_to_json(n, &text) == NF_OK);
        CHECK(nf_from_json(text.data, text.length, &loaded) == NF_OK);
        same_network(n, loaded);
        nf_free(&loaded); nf_free(&n); nf_text_free(&text);
    }
}

typedef struct { size_t call, fail, live; } Fault;
static void *fault_allocate(size_t bytes, void *context) {
    Fault *f = context;
    void *p;
    if (++f->call == f->fail) return NULL;
    p = malloc(bytes); if (p) ++f->live;
    return p;
}
static void fault_release(void *p, void *context) {
    Fault *f = context;
    if (p) { CHECK(f->live > 0); --f->live; free(p); }
}
static void allocator_failures(void) {
    size_t fail;
    for (fail = 1; fail <= 8; ++fail) {
        Fault f = {0, fail, 0};
        NFMemory memory = {fault_allocate, fault_release, &f};
        NFNetwork *n = NULL, *copy = NULL;
        const double w[] = {.1}, b[] = {0}, x[] = {1}, y[] = {1};
        NFLayer layer = {1, 1, NF_SIGMOID, w, b};
        NFHistory h = {0};
        NFText text = {0};
        NFStatus status = nf_new_with_memory(&layer, 1, &memory, &n);
        if (status == NF_OK) {
            (void)nf_clone(n, &copy);
            (void)nf_fit(n, x, y, 1, 2, .1, &h);
            (void)nf_to_json(n, &text);
        } else CHECK(status == NF_ALLOC && n == NULL);
        nf_text_free(&text); nf_history_free(&h); nf_free(&copy); nf_free(&n);
        CHECK(f.live == 0);
    }
}

int main(void) {
    paper_oracle(); differences(); xor_training(); shared_fixtures(); boundaries();
    text_boundaries_and_locale(); allocator_failures();
    printf("neural-framework: %llu checks, %llu failed\n",
           (unsigned long long)checks, (unsigned long long)failures);
    return failures ? EXIT_FAILURE : EXIT_SUCCESS;
}
