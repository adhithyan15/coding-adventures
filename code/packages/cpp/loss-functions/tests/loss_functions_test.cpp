// Tests for the C++ loss-functions library, using the header-only iso_test.h
// harness (pure ISO). Reference values mirror the Rust crate's own tests.
#include "iso_test.h"

#include <stdexcept>
#include <vector>

#include "loss_functions.hpp"

namespace lf = ca::loss_functions;

int main() {
    const double eps = 1e-6;

    const std::vector<double> y_true = {1.0, 0.0};
    const std::vector<double> y_pred = {0.9, 0.1};

    // ── scalar losses ─────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(lf::mse(y_true, y_pred), 0.010, eps);
    ISO_CHECK_EQ_DBL(lf::mae(y_true, y_pred), 0.100, eps);
    ISO_CHECK_EQ_DBL(lf::bce(y_true, y_pred), 0.1053605, eps);
    ISO_CHECK_EQ_DBL(lf::cce(y_true, y_pred), 0.0526802, eps);

    // ── identical slices -> zero error ────────────────────────────────────
    {
        std::vector<double> id = {1.0, 0.0, 0.5};
        ISO_CHECK_EQ_DBL(lf::mse(id, id), 0.0, eps);
        ISO_CHECK_EQ_DBL(lf::mae(id, id), 0.0, eps);
    }

    // ── length errors throw std::invalid_argument ─────────────────────────
    {
        auto throws = [](auto fn) {
            try {
                fn();
            } catch (const std::invalid_argument&) {
                return true;
            }
            return false;
        };
        ISO_CHECK(throws([&] { return lf::mse({1.0}, y_pred); }));
        ISO_CHECK(throws([&] { return lf::mse({}, std::vector<double>{}); }));
        ISO_CHECK(throws([&] { return lf::bce({1.0}, y_pred); }));
        ISO_CHECK(throws([&] { return lf::cce({}, std::vector<double>{}); }));
    }

    // ── gradients ─────────────────────────────────────────────────────────
    {
        std::vector<double> gt = {1.0, 0.0};
        std::vector<double> gp = {0.8, 0.2};

        auto mse_g = lf::mse_derivative(gt, gp);
        ISO_CHECK_EQ_DBL(mse_g[0], -0.2, eps);
        ISO_CHECK_EQ_DBL(mse_g[1], 0.2, eps);

        auto bce_g = lf::bce_derivative(gt, gp);
        ISO_CHECK_EQ_DBL(bce_g[0], -0.625, eps);
        ISO_CHECK_EQ_DBL(bce_g[1], 0.625, eps);

        auto cce_g = lf::cce_derivative(gt, gp);
        ISO_CHECK_EQ_DBL(cce_g[0], -0.625, eps);
        ISO_CHECK_EQ_DBL(cce_g[1], 0.0, eps);

        std::vector<double> mt = {1.0, 0.0, 0.5};
        std::vector<double> mp = {0.8, 0.2, 0.5};
        auto mae_g = lf::mae_derivative(mt, mp);
        ISO_CHECK_EQ_DBL(mae_g[0], -1.0 / 3.0, eps);
        ISO_CHECK_EQ_DBL(mae_g[1], 1.0 / 3.0, eps);
        ISO_CHECK_EQ_DBL(mae_g[2], 0.0, eps);
    }

    // ── ln reduction reaches the EPSILON clamp without producing inf ──────
    {
        double v = lf::cce({1.0}, {0.0});  // 0 clamped up to EPSILON
        ISO_CHECK_EQ_DBL(v, 16.11809565095832, 1e-4);  // -ln(1e-7)
    }

    return ISO_TEST_RESULT();
}
