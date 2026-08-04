// Tests for gradient-descent, using the header-only iso_test.h harness.
// Vectors mirror the Rust crate's unit tests, plus a few extra cases.
#include "iso_test.h"

#include <vector>

#include "gradient_descent.hpp"

namespace gd = ca::gradient_descent;

int main() {
    // ── the crate's core vector ────────────────────────────────────────────
    {
        std::vector<double> w = {1.0, -0.5, 2.0};
        std::vector<double> g = {0.1, -0.2, 0.0};
        std::vector<double> r = gd::sgd(w, g, 0.1);
        ISO_CHECK_EQ_UINT(r.size(), 3u);
        ISO_CHECK_EQ_DBL(r[0], 0.99, 1e-6);
        ISO_CHECK_EQ_DBL(r[1], -0.48, 1e-6);
        ISO_CHECK_EQ_DBL(r[2], 2.0, 1e-6);
    }

    // ── errors: mismatched / empty ─────────────────────────────────────────
    {
        bool threw = false;
        try {
            gd::sgd({1.0}, {}, 0.1);
        } catch (const gd::GradientDescentError&) {
            threw = true;
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            gd::sgd({}, {}, 0.1);
        } catch (const gd::GradientDescentError&) {
            threw = true;
        }
        ISO_CHECK(threw);

        threw = false;
        try {
            gd::sgd({1.0, 2.0}, {1.0}, 0.1);  // length mismatch
        } catch (const gd::GradientDescentError&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── extra: zero gradient leaves weights unchanged ──────────────────────
    {
        std::vector<double> r = gd::sgd({3.0, -7.0}, {0.0, 0.0}, 0.5);
        ISO_CHECK_EQ_DBL(r[0], 3.0, 1e-12);
        ISO_CHECK_EQ_DBL(r[1], -7.0, 1e-12);
    }

    // ── extra: a larger learning rate takes a bigger step ──────────────────
    {
        std::vector<double> r = gd::sgd({10.0}, {2.0}, 1.5);
        ISO_CHECK_EQ_DBL(r[0], 7.0, 1e-9);  // 10 - 1.5*2
    }

    // ── extra: a negative gradient moves the weight up ─────────────────────
    {
        std::vector<double> r = gd::sgd({0.0}, {-4.0}, 0.25);
        ISO_CHECK_EQ_DBL(r[0], 1.0, 1e-9);  // 0 - 0.25*(-4)
    }

    return ISO_TEST_RESULT();
}
