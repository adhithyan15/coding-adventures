// Tests for the C++ activation-functions library, using the header-only
// iso_test.h harness (pure ISO). Reference values mirror the Rust crate's own
// tests, at the same 1e-12 tolerance.
#include "iso_test.h"

#include "activation_functions.hpp"

namespace af = ca::activation_functions;

int main() {
    const double eps = 1e-12;

    // ── linear ────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(af::linear(-3.0), -3.0, eps);
    ISO_CHECK_EQ_DBL(af::linear(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::linear(5.0), 5.0, eps);
    ISO_CHECK_EQ_DBL(af::linear_derivative(-3.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af::linear_derivative(5.0), 1.0, eps);

    // ── sigmoid ───────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(af::sigmoid(0.0), 0.5, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid(1.0), 0.7310585786300049, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid(-1.0), 0.2689414213699951, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid(10.0), 0.9999546021312976, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid(-710.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid(710.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid_derivative(0.0), 0.25, eps);
    ISO_CHECK_EQ_DBL(af::sigmoid_derivative(1.0), 0.19661193324148185, eps);

    // ── relu ──────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(af::relu(5.0), 5.0, eps);
    ISO_CHECK_EQ_DBL(af::relu(-3.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::relu(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::relu_derivative(5.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af::relu_derivative(-3.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::relu_derivative(0.0), 0.0, eps);

    // ── leaky relu ────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(af::leaky_relu(5.0), 5.0, eps);
    ISO_CHECK_EQ_DBL(af::leaky_relu(-3.0), -0.03, eps);
    ISO_CHECK_EQ_DBL(af::leaky_relu(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::leaky_relu_derivative(5.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af::leaky_relu_derivative(-3.0), 0.01, eps);
    ISO_CHECK_EQ_DBL(af::leaky_relu_derivative(0.0), 0.01, eps);

    // ── tanh ──────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(af::tanh(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af::tanh(1.0), 0.7615941559557649, eps);
    ISO_CHECK_EQ_DBL(af::tanh(-1.0), -0.7615941559557649, eps);
    ISO_CHECK_EQ_DBL(af::tanh_derivative(0.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af::tanh_derivative(1.0), 0.41997434161402614, eps);

    // ── softplus ──────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(af::softplus(0.0), 0.6931471805599453, eps);  // ln 2
    ISO_CHECK_EQ_DBL(af::softplus(1.0), 1.3132616875182228, eps);
    ISO_CHECK_EQ_DBL(af::softplus(-1.0), 0.31326168751822286, eps);
    ISO_CHECK(af::softplus(1000.0) > 999.0);
    ISO_CHECK_EQ_DBL(af::softplus_derivative(0.0), 0.5, eps);
    ISO_CHECK_EQ_DBL(af::softplus_derivative(1.0), af::sigmoid(1.0), eps);
    ISO_CHECK_EQ_DBL(af::softplus_derivative(-1.0), af::sigmoid(-1.0), eps);

    // ── extreme / non-finite inputs stay defined (no int-cast UB) ─────────
    {
        ISO_CHECK_EQ_DBL(af::softplus(1e300), 1e300, 1.0);
        ISO_CHECK_EQ_DBL(af::softplus(-1e300), 0.0, eps);
        ISO_CHECK_EQ_DBL(af::sigmoid(1e300), 1.0, eps);
        ISO_CHECK_EQ_DBL(af::sigmoid(-1e300), 0.0, eps);
        ISO_CHECK_EQ_DBL(af::tanh(1e300), 1.0, eps);

        volatile double zero = 0.0;
        double nan = zero / zero;
        ISO_CHECK(af::sigmoid(nan) != af::sigmoid(nan));
        ISO_CHECK(af::softplus(nan) != af::softplus(nan));
        ISO_CHECK(af::tanh(nan) != af::tanh(nan));
    }

    return ISO_TEST_RESULT();
}
