/*
 * Tests for the C activation-functions library, using the header-only
 * iso_test.h harness (pure ISO). Reference values mirror the Rust crate's own
 * tests; the same 1e-12 tolerance is used, which our from-scratch e^x / tanh /
 * ln(1+x) comfortably meet.
 */
#include "iso_test.h"

#include "activation_functions.h"

int main(void) {
    const double eps = 1e-12;

    /* ── linear ──────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(af_linear(-3.0), -3.0, eps);
    ISO_CHECK_EQ_DBL(af_linear(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af_linear(5.0), 5.0, eps);
    ISO_CHECK_EQ_DBL(af_linear_derivative(-3.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af_linear_derivative(5.0), 1.0, eps);

    /* ── sigmoid ─────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(af_sigmoid(0.0), 0.5, eps);
    ISO_CHECK_EQ_DBL(af_sigmoid(1.0), 0.7310585786300049, eps);
    ISO_CHECK_EQ_DBL(af_sigmoid(-1.0), 0.2689414213699951, eps);
    ISO_CHECK_EQ_DBL(af_sigmoid(10.0), 0.9999546021312976, eps);
    ISO_CHECK_EQ_DBL(af_sigmoid(-710.0), 0.0, eps);  /* saturated */
    ISO_CHECK_EQ_DBL(af_sigmoid(710.0), 1.0, eps);   /* saturated */
    ISO_CHECK_EQ_DBL(af_sigmoid_derivative(0.0), 0.25, eps);
    ISO_CHECK_EQ_DBL(af_sigmoid_derivative(1.0), 0.19661193324148185, eps);

    /* ── relu ────────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(af_relu(5.0), 5.0, eps);
    ISO_CHECK_EQ_DBL(af_relu(-3.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af_relu(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af_relu_derivative(5.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af_relu_derivative(-3.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af_relu_derivative(0.0), 0.0, eps);

    /* ── leaky relu ──────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(af_leaky_relu(5.0), 5.0, eps);
    ISO_CHECK_EQ_DBL(af_leaky_relu(-3.0), -0.03, eps);
    ISO_CHECK_EQ_DBL(af_leaky_relu(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af_leaky_relu_derivative(5.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af_leaky_relu_derivative(-3.0), 0.01, eps);
    ISO_CHECK_EQ_DBL(af_leaky_relu_derivative(0.0), 0.01, eps);

    /* ── tanh ────────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(af_tanh(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(af_tanh(1.0), 0.7615941559557649, eps);
    ISO_CHECK_EQ_DBL(af_tanh(-1.0), -0.7615941559557649, eps);
    ISO_CHECK_EQ_DBL(af_tanh_derivative(0.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(af_tanh_derivative(1.0), 0.41997434161402614, eps);

    /* ── softplus ────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(af_softplus(0.0), 0.6931471805599453, eps); /* ln 2 */
    ISO_CHECK_EQ_DBL(af_softplus(1.0), 1.3132616875182228, eps);
    ISO_CHECK_EQ_DBL(af_softplus(-1.0), 0.31326168751822286, eps);
    ISO_CHECK(af_softplus(1000.0) > 999.0); /* ~= x for large x */
    ISO_CHECK_EQ_DBL(af_softplus_derivative(0.0), 0.5, eps);
    ISO_CHECK_EQ_DBL(af_softplus_derivative(1.0), af_sigmoid(1.0), eps);
    ISO_CHECK_EQ_DBL(af_softplus_derivative(-1.0), af_sigmoid(-1.0), eps);

    /* ── extreme / non-finite inputs stay defined (no int-cast UB) ───────── */
    {
        /* Huge finite magnitudes: softplus(±1e300) internally calls
         * exp(-1e300); the underflow guard keeps it out of the double->int
         * cast. softplus(+1e300) ~= x; softplus(-1e300) ~= 0. */
        ISO_CHECK_EQ_DBL(af_softplus(1e300), 1e300, 1.0);
        ISO_CHECK_EQ_DBL(af_softplus(-1e300), 0.0, eps);
        ISO_CHECK_EQ_DBL(af_sigmoid(1e300), 1.0, eps);
        ISO_CHECK_EQ_DBL(af_sigmoid(-1e300), 0.0, eps);
        ISO_CHECK_EQ_DBL(af_tanh(1e300), 1.0, eps);

        /* NaN propagates through the exp-based paths rather than trapping. */
        volatile double zero = 0.0;
        double nan = zero / zero;
        ISO_CHECK(af_sigmoid(nan) != af_sigmoid(nan));
        ISO_CHECK(af_softplus(nan) != af_softplus(nan));
        ISO_CHECK(af_tanh(nan) != af_tanh(nan));
    }

    return ISO_TEST_RESULT();
}
