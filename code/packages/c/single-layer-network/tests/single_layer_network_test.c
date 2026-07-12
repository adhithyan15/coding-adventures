/*
 * Tests for the C single-layer-network library, using the header-only
 * iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include "single_layer_network.h"

int main(void) {
    const double eps = 1e-6;

    ISO_CHECK_STR_EQ(SLN_VERSION, "0.1.0");

    /* ── one epoch exposes the matrix gradients (Linear, exact values) ───── */
    {
        double inputs[2] = {1.0, 2.0};           /* 1 sample x 2 features */
        double targets[2] = {3.0, 5.0};          /* 1 sample x 2 outputs  */
        double weights[4] = {0.0, 0.0, 0.0, 0.0}; /* 2 x 2                 */
        double biases[2] = {0.0, 0.0};

        SlnTrainingStep step;
        ISO_CHECK(sln_train_one_epoch(inputs, 1, 2, targets, 1, 2, weights, 2,
                                      2, biases, 2, 0.1, SLN_ACT_LINEAR,
                                      &step) == SLN_OK);

        /* predictions == [[0, 0]] */
        ISO_CHECK_EQ_DBL(step.predictions[0], 0.0, eps);
        ISO_CHECK_EQ_DBL(step.predictions[1], 0.0, eps);
        /* errors == [[-3, -5]] */
        ISO_CHECK_EQ_DBL(step.errors[0], -3.0, eps);
        ISO_CHECK_EQ_DBL(step.errors[1], -5.0, eps);
        /* weight_gradients == [[-3, -5], [-6, -10]] */
        ISO_CHECK_EQ_DBL(step.weight_gradients[0], -3.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[1], -5.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[2], -6.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[3], -10.0, eps);
        /* next_weights[0][0] == 0.3, next_weights[1][1] == 1.0 */
        ISO_CHECK_EQ_DBL(step.next_weights[0], 0.3, eps);
        ISO_CHECK_EQ_DBL(step.next_weights[3], 1.0, eps);
        /* bias gradients = column sums of deltas = [-3, -5]; next = 0.3, 0.5 */
        ISO_CHECK_EQ_DBL(step.bias_gradients[0], -3.0, eps);
        ISO_CHECK_EQ_DBL(step.bias_gradients[1], -5.0, eps);
        ISO_CHECK_EQ_DBL(step.next_biases[0], 0.3, eps);
        ISO_CHECK_EQ_DBL(step.next_biases[1], 0.5, eps);
        /* loss = mean of error^2 = (9 + 25) / 2 = 17 */
        ISO_CHECK_EQ_DBL(step.loss, 17.0, eps);

        sln_training_step_free(&step);
    }

    /* ── fit learns m inputs -> n outputs (loss decreases) ───────────────── */
    {
        SlnNetwork net;
        ISO_CHECK(sln_network_init(&net, 3, 2, SLN_ACT_LINEAR) == SLN_OK);

        double inputs[9] = {
            0.0, 0.0, 1.0,
            1.0, 2.0, 1.0,
            2.0, 1.0, 1.0};
        double targets[6] = {
            1.0, -1.0,
            3.0, 2.0,
            4.0, 1.0};

        SlnTrainingStep *history = NULL;
        size_t count = 0;
        ISO_CHECK(sln_network_fit(&net, inputs, 3, 3, targets, 3, 2, 0.05, 500,
                                  &history, &count) == SLN_OK);
        ISO_CHECK_EQ_UINT(count, 500u);
        /* Training reduces the loss. */
        ISO_CHECK(history[count - 1].loss < history[0].loss);

        /* Prediction shape is 1 x 2. */
        double query[3] = {1.0, 1.0, 1.0};
        double pred[2];
        ISO_CHECK(sln_network_predict(&net, query, 1, 3, pred) == SLN_OK);
        /* (values are finite reals; exact fit not asserted, only shape/decay) */

        sln_history_free(history, count);
        sln_network_free(&net);
    }

    /* ── sigmoid activation stays in (0, 1) ──────────────────────────────── */
    {
        double inputs[1] = {0.0};   /* 1 sample x 1 feature */
        double weights[1] = {0.0};  /* 1 x 1 */
        double biases[1] = {0.0};
        double out[1];
        /* sigmoid(0) = 0.5 */
        ISO_CHECK(sln_predict(inputs, 1, 1, weights, 1, 1, biases, 1,
                              SLN_ACT_SIGMOID, out) == SLN_OK);
        ISO_CHECK_EQ_DBL(out[0], 0.5, eps);

        /* A large positive pre-activation saturates near 1, large negative
         * near 0 — both defined, no overflow. */
        double big_w[1] = {1000.0};
        double one[1] = {1.0};
        ISO_CHECK(sln_predict(one, 1, 1, big_w, 1, 1, biases, 1,
                              SLN_ACT_SIGMOID, out) == SLN_OK);
        ISO_CHECK(out[0] > 0.99 && out[0] <= 1.0);
        double neg_w[1] = {-1000.0};
        ISO_CHECK(sln_predict(one, 1, 1, neg_w, 1, 1, biases, 1,
                              SLN_ACT_SIGMOID, out) == SLN_OK);
        ISO_CHECK(out[0] >= 0.0 && out[0] < 0.01);
    }

    /* ── shape errors are reported ───────────────────────────────────────── */
    {
        double inputs[2] = {1.0, 2.0};
        double weights[4] = {0.0, 0.0, 0.0, 0.0};
        double biases[2] = {0.0, 0.0};
        double out[2];
        /* input_count (3) != weight_rows (2). */
        ISO_CHECK(sln_predict(inputs, 1, 3, weights, 2, 2, biases, 2,
                              SLN_ACT_LINEAR, out) == SLN_ERR_SHAPE);
        /* empty matrix. */
        ISO_CHECK(sln_predict(inputs, 0, 2, weights, 2, 2, biases, 2,
                              SLN_ACT_LINEAR, out) == SLN_ERR_SHAPE);
        /* bias count mismatch. */
        ISO_CHECK(sln_predict(inputs, 1, 2, weights, 2, 2, biases, 1,
                              SLN_ACT_LINEAR, out) == SLN_ERR_SHAPE);
    }

    return ISO_TEST_RESULT();
}
