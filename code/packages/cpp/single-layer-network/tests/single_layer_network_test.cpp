// Tests for the C++ single-layer-network library, using the header-only
// iso_test.h harness (pure ISO). Vectors mirror the Rust crate's own tests.
#include "iso_test.h"

#include <stdexcept>
#include <vector>

#include "single_layer_network.hpp"

namespace sln = ca::single_layer_network;
using sln::ActivationName;
using sln::Matrix;

int main() {
    const double eps = 1e-6;

    ISO_CHECK_STR_EQ(sln::VERSION, "0.1.0");

    // ── one epoch exposes matrix gradients (Linear, exact values) ─────────
    {
        sln::TrainingStep step = sln::train_one_epoch_with_matrices(
            Matrix{{1.0, 2.0}}, Matrix{{3.0, 5.0}},
            Matrix{{0.0, 0.0}, {0.0, 0.0}}, {0.0, 0.0}, 0.1,
            ActivationName::Linear);

        ISO_CHECK_EQ_DBL(step.predictions[0][0], 0.0, eps);
        ISO_CHECK_EQ_DBL(step.predictions[0][1], 0.0, eps);
        ISO_CHECK_EQ_DBL(step.errors[0][0], -3.0, eps);
        ISO_CHECK_EQ_DBL(step.errors[0][1], -5.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[0][0], -3.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[0][1], -5.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[1][0], -6.0, eps);
        ISO_CHECK_EQ_DBL(step.weight_gradients[1][1], -10.0, eps);
        ISO_CHECK_EQ_DBL(step.next_weights[0][0], 0.3, eps);
        ISO_CHECK_EQ_DBL(step.next_weights[1][1], 1.0, eps);
        ISO_CHECK_EQ_DBL(step.next_biases[0], 0.3, eps);
        ISO_CHECK_EQ_DBL(step.next_biases[1], 0.5, eps);
        ISO_CHECK_EQ_DBL(step.loss, 17.0, eps);
    }

    // ── fit learns m inputs -> n outputs (loss decreases) ─────────────────
    {
        sln::SingleLayerNetwork network(3, 2, ActivationName::Linear);
        auto history = network.fit(
            Matrix{{0.0, 0.0, 1.0}, {1.0, 2.0, 1.0}, {2.0, 1.0, 1.0}},
            Matrix{{1.0, -1.0}, {3.0, 2.0}, {4.0, 1.0}}, 0.05, 500);
        ISO_CHECK_EQ_UINT(history.size(), 500u);
        ISO_CHECK(history.back().loss < history.front().loss);

        Matrix pred = network.predict(Matrix{{1.0, 1.0, 1.0}});
        ISO_CHECK_EQ_UINT(pred.size(), 1u);
        ISO_CHECK_EQ_UINT(pred[0].size(), 2u);
    }

    // ── fit_single_layer_network convenience wrapper ──────────────────────
    {
        auto result = sln::fit_single_layer_network(
            Matrix{{1.0}}, Matrix{{1.0}}, 0.1, 10, ActivationName::Linear);
        ISO_CHECK_EQ_UINT(result.second.size(), 10u);
        ISO_CHECK(result.first.weights.size() == 1u);
    }

    // ── sigmoid activation stays in (0, 1) ────────────────────────────────
    {
        Matrix out = sln::predict_with_parameters(
            Matrix{{0.0}}, Matrix{{0.0}}, {0.0}, ActivationName::Sigmoid);
        ISO_CHECK_EQ_DBL(out[0][0], 0.5, eps);  // sigmoid(0)

        Matrix big = sln::predict_with_parameters(
            Matrix{{1.0}}, Matrix{{1000.0}}, {0.0}, ActivationName::Sigmoid);
        ISO_CHECK(big[0][0] > 0.99 && big[0][0] <= 1.0);
        Matrix small = sln::predict_with_parameters(
            Matrix{{1.0}}, Matrix{{-1000.0}}, {0.0}, ActivationName::Sigmoid);
        ISO_CHECK(small[0][0] >= 0.0 && small[0][0] < 0.01);
    }

    // ── shape errors throw std::invalid_argument ──────────────────────────
    {
        auto throws = [](auto fn) {
            try {
                fn();
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        // input columns (2) != weight rows (3)
        ISO_CHECK(throws([] {
            sln::predict_with_parameters(Matrix{{1.0, 2.0}},
                                         Matrix{{0.0}, {0.0}, {0.0}}, {0.0},
                                         ActivationName::Linear);
        }));
        // empty inputs
        ISO_CHECK(throws([] {
            sln::predict_with_parameters(Matrix{}, Matrix{{0.0}}, {0.0},
                                         ActivationName::Linear);
        }));
        // ragged matrix
        ISO_CHECK(throws([] {
            sln::predict_with_parameters(Matrix{{1.0, 2.0}, {3.0}},
                                         Matrix{{0.0}, {0.0}}, {0.0},
                                         ActivationName::Linear);
        }));
        // bias count mismatch
        ISO_CHECK(throws([] {
            sln::predict_with_parameters(Matrix{{1.0}}, Matrix{{0.0, 0.0}},
                                         {0.0}, ActivationName::Linear);
        }));
    }

    return ISO_TEST_RESULT();
}
