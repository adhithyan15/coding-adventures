/*
 * Tests for the C loss-functions library, using the header-only iso_test.h
 * harness (pure ISO). Reference values mirror the Rust crate's own tests.
 */
#include "iso_test.h"

#include "loss_functions.h"

int main(void) {
    const double eps = 1e-6;

    double y_true[2] = {1.0, 0.0};
    double y_pred[2] = {0.9, 0.1};

    /* ── scalar losses ───────────────────────────────────────────────────── */
    {
        double v;
        ISO_CHECK(loss_mse(y_true, 2, y_pred, 2, &v) == LOSS_OK);
        ISO_CHECK_EQ_DBL(v, 0.010, eps);
        ISO_CHECK(loss_mae(y_true, 2, y_pred, 2, &v) == LOSS_OK);
        ISO_CHECK_EQ_DBL(v, 0.100, eps);
        ISO_CHECK(loss_bce(y_true, 2, y_pred, 2, &v) == LOSS_OK);
        ISO_CHECK_EQ_DBL(v, 0.1053605, eps);
        ISO_CHECK(loss_cce(y_true, 2, y_pred, 2, &v) == LOSS_OK);
        ISO_CHECK_EQ_DBL(v, 0.0526802, eps);
    }

    /* ── identical slices -> zero error (MSE, MAE) ───────────────────────── */
    {
        double id[3] = {1.0, 0.0, 0.5};
        double v;
        ISO_CHECK(loss_mse(id, 3, id, 3, &v) == LOSS_OK);
        ISO_CHECK_EQ_DBL(v, 0.0, eps);
        ISO_CHECK(loss_mae(id, 3, id, 3, &v) == LOSS_OK);
        ISO_CHECK_EQ_DBL(v, 0.0, eps);
    }

    /* ── length errors: mismatch and empty ───────────────────────────────── */
    {
        double v;
        ISO_CHECK(loss_mse(y_true, 1, y_pred, 2, &v) == LOSS_ERR_LENGTH);
        ISO_CHECK(loss_mse(y_true, 0, y_pred, 0, &v) == LOSS_ERR_LENGTH);
        ISO_CHECK(loss_mae(y_true, 1, y_pred, 2, &v) == LOSS_ERR_LENGTH);
        ISO_CHECK(loss_bce(y_true, 0, y_pred, 0, &v) == LOSS_ERR_LENGTH);
        ISO_CHECK(loss_cce(y_true, 1, y_pred, 2, &v) == LOSS_ERR_LENGTH);
    }

    /* ── gradients ───────────────────────────────────────────────────────── */
    {
        double gt[2] = {1.0, 0.0};
        double gp[2] = {0.8, 0.2};
        double grad[3];

        ISO_CHECK(loss_mse_derivative(gt, 2, gp, 2, grad) == LOSS_OK);
        ISO_CHECK_EQ_DBL(grad[0], -0.2, eps);
        ISO_CHECK_EQ_DBL(grad[1], 0.2, eps);

        ISO_CHECK(loss_bce_derivative(gt, 2, gp, 2, grad) == LOSS_OK);
        ISO_CHECK_EQ_DBL(grad[0], -0.625, eps);
        ISO_CHECK_EQ_DBL(grad[1], 0.625, eps);

        ISO_CHECK(loss_cce_derivative(gt, 2, gp, 2, grad) == LOSS_OK);
        ISO_CHECK_EQ_DBL(grad[0], -0.625, eps);
        ISO_CHECK_EQ_DBL(grad[1], 0.0, eps);

        /* MAE derivative: sign of (pred - true), scaled by 1/n; ties -> 0. */
        double mt[3] = {1.0, 0.0, 0.5};
        double mp[3] = {0.8, 0.2, 0.5};
        ISO_CHECK(loss_mae_derivative(mt, 3, mp, 3, grad) == LOSS_OK);
        ISO_CHECK_EQ_DBL(grad[0], -1.0 / 3.0, eps);
        ISO_CHECK_EQ_DBL(grad[1], 1.0 / 3.0, eps);
        ISO_CHECK_EQ_DBL(grad[2], 0.0, eps);

        /* A gradient with mismatched lengths is rejected. */
        ISO_CHECK(loss_mse_derivative(gt, 2, gp, 1, grad) == LOSS_ERR_LENGTH);
    }

    /* ── ln reduction reaches the EPSILON clamp without producing inf ─────── */
    {
        double t[1] = {1.0};
        double p[1] = {0.0}; /* clamped up to EPSILON before the log */
        double v;
        ISO_CHECK(loss_cce(t, 1, p, 1, &v) == LOSS_OK);
        /* -ln(1e-7) = 16.1181; a finite, sensible value. */
        ISO_CHECK_EQ_DBL(v, 16.11809565095832, 1e-4);
    }

    return ISO_TEST_RESULT();
}
