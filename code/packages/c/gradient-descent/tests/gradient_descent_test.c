/*
 * Tests for gradient-descent, using the header-only iso_test.h harness.
 * Vectors mirror the Rust crate's unit tests, plus a few extra cases.
 */
#include "iso_test.h"

#include "gradient_descent.h"

int main(void) {
    /* ── the crate's core vector ────────────────────────────────────────────*/
    {
        static const double w[] = {1.0, -0.5, 2.0};
        static const double g[] = {0.1, -0.2, 0.0};
        double out[3];
        ISO_CHECK_EQ_INT(gd_sgd(w, g, 3, 0.1, out), GD_OK);
        ISO_CHECK_EQ_DBL(out[0], 0.99, 1e-6);
        ISO_CHECK_EQ_DBL(out[1], -0.48, 1e-6);
        ISO_CHECK_EQ_DBL(out[2], 2.0, 1e-6);
    }

    /* ── error: empty vectors ───────────────────────────────────────────────*/
    {
        double out[1];
        ISO_CHECK_EQ_INT(gd_sgd(out, out, 0, 0.1, out), GD_ERR_LENGTH);
    }

    /* ── extra: zero gradient leaves weights unchanged ─────────────────────*/
    {
        static const double w[] = {3.0, -7.0};
        static const double g[] = {0.0, 0.0};
        double out[2];
        ISO_CHECK_EQ_INT(gd_sgd(w, g, 2, 0.5, out), GD_OK);
        ISO_CHECK_EQ_DBL(out[0], 3.0, 1e-12);
        ISO_CHECK_EQ_DBL(out[1], -7.0, 1e-12);
    }

    /* ── extra: a larger learning rate takes a bigger step ─────────────────*/
    {
        static const double w[] = {10.0};
        static const double g[] = {2.0};
        double out[1];
        ISO_CHECK_EQ_INT(gd_sgd(w, g, 1, 1.5, out), GD_OK);
        ISO_CHECK_EQ_DBL(out[0], 7.0, 1e-9); /* 10 - 1.5*2 */
    }

    /* ── extra: a negative gradient moves the weight up ────────────────────*/
    {
        static const double w[] = {0.0};
        static const double g[] = {-4.0};
        double out[1];
        ISO_CHECK_EQ_INT(gd_sgd(w, g, 1, 0.25, out), GD_OK);
        ISO_CHECK_EQ_DBL(out[0], 1.0, 1e-9); /* 0 - 0.25*(-4) */
    }

    /* ── extra: in-place update (out aliases weights) ──────────────────────*/
    {
        double buf[3] = {5.0, 5.0, 5.0};
        static const double g[] = {1.0, 2.0, 3.0};
        ISO_CHECK_EQ_INT(gd_sgd(buf, g, 3, 1.0, buf), GD_OK);
        ISO_CHECK_EQ_DBL(buf[0], 4.0, 1e-9);
        ISO_CHECK_EQ_DBL(buf[1], 3.0, 1e-9);
        ISO_CHECK_EQ_DBL(buf[2], 2.0, 1e-9);
    }

    return ISO_TEST_RESULT();
}
