/*
 * Tests for the C two-layer-network library, using the header-only iso_test.h
 * harness (pure ISO). Vectors mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include <stddef.h>

#include "two_layer_network.h"

/* Build the sample_parameters used by the Rust teaching-example test. */
static TlnStatus sample_parameters(size_t input_count, size_t hidden_count,
                                   TlnParameters *out) {
    /* i2h[feature][hidden] = 0.17*(feature+1) - 0.11*(hidden+1) */
    double i2h[64];
    double hb[16];
    double h2o[16]; /* hidden × 1 */
    double ob[1] = {0.02};
    size_t f, h;
    for (f = 0; f < input_count; f++) {
        for (h = 0; h < hidden_count; h++) {
            i2h[f * hidden_count + h] =
                0.17 * (double)(f + 1) - 0.11 * (double)(h + 1);
        }
    }
    for (h = 0; h < hidden_count; h++) {
        hb[h] = 0.05 * ((double)h - 1.0);
        h2o[h] = 0.13 * (double)(h + 1) - 0.25;
    }
    return tln_parameters_init(out, i2h, input_count, hidden_count, hb, h2o, 1,
                               ob);
}

int main(void) {
    ISO_CHECK_STR_EQ(TLN_VERSION, "0.1.0");

    double xor_inputs[8] = {0.0, 0.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0};
    double xor_targets[4] = {0.0, 1.0, 1.0, 0.0};

    /* ── forward pass exposes hidden activations (XOR warm start) ─────────── */
    {
        TlnParameters p;
        ISO_CHECK(tln_xor_warm_start_parameters(&p) == TLN_OK);
        TlnForwardPass fp;
        ISO_CHECK(tln_forward(xor_inputs, 4, 2, &p, TLN_ACT_SIGMOID,
                              TLN_ACT_SIGMOID, &fp) == TLN_OK);
        ISO_CHECK_EQ_UINT(fp.samples, 4u);
        ISO_CHECK_EQ_UINT(fp.hidden_count, 2u);
        ISO_CHECK_EQ_UINT(fp.output_count, 1u);
        /* XOR: input [0,1] -> ~1, input [0,0] -> ~0. */
        ISO_CHECK(fp.predictions[1] > 0.7); /* sample 1, output 0 */
        ISO_CHECK(fp.predictions[0] < 0.3); /* sample 0, output 0 */
        /* Sanity: all activations/predictions are in (0, 1). */
        ISO_CHECK(fp.predictions[2] > 0.7); /* [1,0] -> ~1 */
        ISO_CHECK(fp.predictions[3] < 0.3); /* [1,1] -> ~0 */
        tln_forward_pass_free(&fp);
        tln_parameters_free(&p);
    }

    /* ── training step exposes both layers' gradients ────────────────────── */
    {
        TlnParameters p;
        ISO_CHECK(tln_xor_warm_start_parameters(&p) == TLN_OK);
        TlnTrainingStep step;
        ISO_CHECK(tln_train_one_epoch(xor_inputs, 4, 2, xor_targets, 4, 1, &p,
                                      0.5, TLN_ACT_SIGMOID, TLN_ACT_SIGMOID,
                                      &step) == TLN_OK);
        /* input_to_hidden gradients: 2 × 2 (input_count × hidden_count). */
        ISO_CHECK_EQ_UINT(step.input_count, 2u);
        ISO_CHECK_EQ_UINT(step.hidden_count, 2u);
        /* hidden_to_output gradients: 2 × 1. */
        ISO_CHECK_EQ_UINT(step.output_count, 1u);
        ISO_CHECK(step.loss >= 0.0);
        /* next_parameters carry the same shapes. */
        ISO_CHECK_EQ_UINT(step.next_parameters.input_count, 2u);
        ISO_CHECK_EQ_UINT(step.next_parameters.hidden_count, 2u);
        ISO_CHECK_EQ_UINT(step.next_parameters.output_count, 1u);
        tln_training_step_free(&step);
        tln_parameters_free(&p);
    }

    /* ── teaching examples each run one training step (loss >= 0) ─────────── */
    {
        /* "absolute value": 5 samples, 1 feature, 4 hidden. */
        double av_in[5] = {-1.0, -0.5, 0.0, 0.5, 1.0};
        double av_tg[5] = {1.0, 0.5, 0.0, 0.5, 1.0};
        TlnParameters p;
        ISO_CHECK(sample_parameters(1, 4, &p) == TLN_OK);
        TlnTrainingStep step;
        ISO_CHECK(tln_train_one_epoch(av_in, 5, 1, av_tg, 5, 1, &p, 0.4,
                                      TLN_ACT_SIGMOID, TLN_ACT_SIGMOID,
                                      &step) == TLN_OK);
        ISO_CHECK(step.loss >= 0.0);
        ISO_CHECK_EQ_UINT(step.input_count, 1u);   /* i2h grads rows */
        ISO_CHECK_EQ_UINT(step.hidden_count, 4u);  /* h2o grads rows */
        tln_training_step_free(&step);
        tln_parameters_free(&p);

        /* "interaction features": 4 samples, 3 features, 5 hidden. */
        double if_in[12] = {0.2, 0.25, 0.0, 0.6, 0.5, 1.0,
                            1.0, 0.75, 1.0, 1.0, 1.0, 0.0};
        double if_tg[4] = {0.08, 0.72, 0.96, 0.76};
        ISO_CHECK(sample_parameters(3, 5, &p) == TLN_OK);
        ISO_CHECK(tln_train_one_epoch(if_in, 4, 3, if_tg, 4, 1, &p, 0.4,
                                      TLN_ACT_SIGMOID, TLN_ACT_SIGMOID,
                                      &step) == TLN_OK);
        ISO_CHECK(step.loss >= 0.0);
        ISO_CHECK_EQ_UINT(step.input_count, 3u);
        ISO_CHECK_EQ_UINT(step.hidden_count, 5u);
        tln_training_step_free(&step);
        tln_parameters_free(&p);
    }

    /* ── linear activation gives an exact known gradient ─────────────────── */
    {
        /* A tiny 1-in, 1-hidden, 1-out linear network with zero params:
         * everything is 0, so prediction 0, error = -target, and gradients are
         * deterministic — mainly a smoke test of the two-layer wiring. */
        double in[1] = {1.0};
        double tg[1] = {2.0};
        double i2h[1] = {0.0}, hb[1] = {0.0}, h2o[1] = {0.0}, ob[1] = {0.0};
        TlnParameters p;
        ISO_CHECK(tln_parameters_init(&p, i2h, 1, 1, hb, h2o, 1, ob) == TLN_OK);
        TlnTrainingStep step;
        ISO_CHECK(tln_train_one_epoch(in, 1, 1, tg, 1, 1, &p, 0.1,
                                      TLN_ACT_LINEAR, TLN_ACT_LINEAR,
                                      &step) == TLN_OK);
        /* prediction 0, error = 0 - 2 = -2, loss = 4. */
        ISO_CHECK_EQ_DBL(step.predictions[0], 0.0, 1e-12);
        ISO_CHECK_EQ_DBL(step.errors[0], -2.0, 1e-12);
        ISO_CHECK_EQ_DBL(step.loss, 4.0, 1e-12);
        tln_training_step_free(&step);
        tln_parameters_free(&p);
    }

    /* ── shape error is reported ─────────────────────────────────────────── */
    {
        TlnParameters p;
        ISO_CHECK(tln_xor_warm_start_parameters(&p) == TLN_OK);
        TlnForwardPass fp;
        /* input_count 3 != params input_count 2. */
        ISO_CHECK(tln_forward(xor_inputs, 4, 3, &p, TLN_ACT_SIGMOID,
                              TLN_ACT_SIGMOID, &fp) == TLN_ERR_SHAPE);
        tln_parameters_free(&p);
    }

    return ISO_TEST_RESULT();
}
