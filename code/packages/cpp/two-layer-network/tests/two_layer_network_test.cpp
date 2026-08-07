// Tests for the C++ two-layer-network library, using the header-only iso_test.h
// harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <cstddef>
#include <stdexcept>
#include <vector>

#include "two_layer_network.hpp"

namespace tln = ca::two_layer_network;
using tln::ActivationName;
using tln::Matrix;
using tln::Parameters;

static Matrix xor_inputs() {
    return {{0.0, 0.0}, {0.0, 1.0}, {1.0, 0.0}, {1.0, 1.0}};
}
static Matrix xor_targets() { return {{0.0}, {1.0}, {1.0}, {0.0}}; }

// The Rust teaching-example sample_parameters.
static Parameters sample_parameters(std::size_t input_count,
                                    std::size_t hidden_count) {
    Parameters p;
    for (std::size_t f = 0; f < input_count; f++) {
        std::vector<double> row;
        for (std::size_t h = 0; h < hidden_count; h++)
            row.push_back(0.17 * static_cast<double>(f + 1) -
                          0.11 * static_cast<double>(h + 1));
        p.input_to_hidden_weights.push_back(row);
    }
    for (std::size_t h = 0; h < hidden_count; h++) {
        p.hidden_biases.push_back(0.05 * (static_cast<double>(h) - 1.0));
        p.hidden_to_output_weights.push_back(
            {0.13 * static_cast<double>(h + 1) - 0.25});
    }
    p.output_biases = {0.02};
    return p;
}

int main() {
    ISO_CHECK_STR_EQ(tln::VERSION, "0.1.0");

    // ── forward pass exposes hidden activations (XOR warm start) ──────────
    {
        tln::ForwardPass passed =
            tln::forward(xor_inputs(), tln::xor_warm_start_parameters(),
                         ActivationName::Sigmoid, ActivationName::Sigmoid);
        ISO_CHECK_EQ_UINT(passed.hidden_activations.size(), 4u);
        ISO_CHECK_EQ_UINT(passed.hidden_activations[0].size(), 2u);
        ISO_CHECK(passed.predictions[1][0] > 0.7);  // XOR [0,1] -> ~1
        ISO_CHECK(passed.predictions[0][0] < 0.3);  // XOR [0,0] -> ~0
        ISO_CHECK(passed.predictions[2][0] > 0.7);  // [1,0] -> ~1
        ISO_CHECK(passed.predictions[3][0] < 0.3);  // [1,1] -> ~0
    }

    // ── training step exposes both layers' gradients ──────────────────────
    {
        tln::TrainingStep step = tln::train_one_epoch(
            xor_inputs(), xor_targets(), tln::xor_warm_start_parameters(), 0.5,
            ActivationName::Sigmoid, ActivationName::Sigmoid);
        ISO_CHECK_EQ_UINT(step.input_to_hidden_weight_gradients.size(), 2u);
        ISO_CHECK_EQ_UINT(step.input_to_hidden_weight_gradients[0].size(), 2u);
        ISO_CHECK_EQ_UINT(step.hidden_to_output_weight_gradients.size(), 2u);
        ISO_CHECK_EQ_UINT(step.hidden_to_output_weight_gradients[0].size(), 1u);
        ISO_CHECK(step.loss >= 0.0);
    }

    // ── teaching examples each run one training step (loss >= 0) ──────────
    {
        struct Case {
            Matrix inputs;
            Matrix targets;
            std::size_t hidden;
        };
        std::vector<Case> cases = {
            {{{-1.0}, {-0.5}, {0.0}, {0.5}, {1.0}},
             {{1.0}, {0.5}, {0.0}, {0.5}, {1.0}},
             4},
            {{{0.2, 0.25, 0.0}, {0.6, 0.5, 1.0}, {1.0, 0.75, 1.0}, {1.0, 1.0, 0.0}},
             {{0.08}, {0.72}, {0.96}, {0.76}},
             5},
        };
        for (const Case& c : cases) {
            tln::TrainingStep step = tln::train_one_epoch(
                c.inputs, c.targets,
                sample_parameters(c.inputs[0].size(), c.hidden), 0.4,
                ActivationName::Sigmoid, ActivationName::Sigmoid);
            ISO_CHECK(step.loss >= 0.0);
            ISO_CHECK_EQ_UINT(step.input_to_hidden_weight_gradients.size(),
                              c.inputs[0].size());
            ISO_CHECK_EQ_UINT(step.hidden_to_output_weight_gradients.size(),
                              c.hidden);
        }
    }

    // ── linear activation gives an exact known error/loss ─────────────────
    {
        Parameters p;
        p.input_to_hidden_weights = {{0.0}};
        p.hidden_biases = {0.0};
        p.hidden_to_output_weights = {{0.0}};
        p.output_biases = {0.0};
        tln::TrainingStep step = tln::train_one_epoch(
            Matrix{{1.0}}, Matrix{{2.0}}, p, 0.1, ActivationName::Linear,
            ActivationName::Linear);
        ISO_CHECK_EQ_DBL(step.predictions[0][0], 0.0, 1e-12);
        ISO_CHECK_EQ_DBL(step.errors[0][0], -2.0, 1e-12);
        ISO_CHECK_EQ_DBL(step.loss, 4.0, 1e-12);
    }

    // ── shape error throws std::invalid_argument ──────────────────────────
    {
        bool threw = false;
        try {
            // ragged inputs
            tln::forward(Matrix{{1.0, 2.0}, {3.0}},
                         tln::xor_warm_start_parameters(),
                         ActivationName::Sigmoid, ActivationName::Sigmoid);
        } catch (const std::invalid_argument&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    return ISO_TEST_RESULT();
}
