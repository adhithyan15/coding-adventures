// Tests for the C++ trig library, using the header-only iso_test.h harness
// (pure ISO). Values are checked against known references within a small
// tolerance — our from-scratch series should match the real functions.
#include "iso_test.h"

#include <cmath>
#include <limits>
#include <stdexcept>

#include "trig.hpp"

namespace t = ca::trig;

int main() {
    const double eps = 1e-10;

    // ── sin ──────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(t::sin(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(t::sin(t::PI / 2.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(t::sin(t::PI), 0.0, eps);
    ISO_CHECK_EQ_DBL(t::sin(3.0 * t::PI / 2.0), -1.0, eps);
    ISO_CHECK_EQ_DBL(t::sin(t::PI / 6.0), 0.5, eps);  // sin 30deg = 1/2
    ISO_CHECK_EQ_DBL(t::sin(-1.0), -t::sin(1.0), eps);
    ISO_CHECK_EQ_DBL(t::sin(1.0 + 10.0 * t::PI), t::sin(1.0), 1e-9);

    // ── cos ──────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(t::cos(0.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(t::cos(t::PI), -1.0, eps);
    ISO_CHECK_EQ_DBL(t::cos(t::PI / 2.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(t::cos(t::PI / 3.0), 0.5, eps);  // cos 60deg = 1/2
    ISO_CHECK_EQ_DBL(t::cos(-2.0), t::cos(2.0), eps);
    {
        double s = t::sin(0.7), c = t::cos(0.7);
        ISO_CHECK_EQ_DBL(s * s + c * c, 1.0, eps);  // sin^2 + cos^2 = 1
    }

    // ── tan ──────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(t::tan(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(t::tan(t::PI / 4.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(t::tan(-t::PI / 4.0), -1.0, eps);
    ISO_CHECK(t::tan(t::PI / 2.0) > 1.0e300);  // saturates near the pole

    // ── angle conversion ─────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(t::radians(180.0), t::PI, eps);
    ISO_CHECK_EQ_DBL(t::radians(90.0), t::PI / 2.0, eps);
    ISO_CHECK_EQ_DBL(t::degrees(t::PI), 180.0, eps);
    ISO_CHECK_EQ_DBL(t::degrees(t::PI / 6.0), 30.0, eps);
    ISO_CHECK_EQ_DBL(t::degrees(t::radians(45.0)), 45.0, eps);

    // ── sqrt (throws on negative; Rust panics) ───────────────────────────
    ISO_CHECK_EQ_DBL(t::sqrt(4.0), 2.0, eps);
    ISO_CHECK_EQ_DBL(t::sqrt(2.0), 1.4142135623730951, eps);
    ISO_CHECK_EQ_DBL(t::sqrt(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(t::sqrt(1e12), 1000000.0, 1e-4);
    ISO_CHECK_EQ_DBL((t::sqrt(1e-100) - 1e-50) / 1e-50, 0.0, 1e-12);
    ISO_CHECK_EQ_DBL(
        (t::sqrt(std::numeric_limits<double>::denorm_min()) -
         2.2227587494850775e-162) /
            2.2227587494850775e-162,
        0.0, 1e-12);
    ISO_CHECK_EQ_DBL(
        (t::sqrt(std::numeric_limits<double>::max()) -
         1.3407807929942596e154) /
            1.3407807929942596e154,
        0.0, 1e-12);
    ISO_CHECK(std::signbit(t::sqrt(-0.0)));
    ISO_CHECK(std::isinf(t::sqrt(std::numeric_limits<double>::infinity())));
    ISO_CHECK(std::isnan(t::sqrt(std::numeric_limits<double>::quiet_NaN())));
    {
        bool threw = false;
        try {
            (void)t::sqrt(-1.0);
        } catch (const std::domain_error&) {
            threw = true;
        }
        ISO_CHECK(threw);
    }

    // ── atan ─────────────────────────────────────────────────────────────
    ISO_CHECK_EQ_DBL(t::atan(0.0), 0.0, eps);
    ISO_CHECK(std::signbit(t::atan(-0.0)));
    ISO_CHECK(t::atan(0x1p-30) == 0x1p-30);
    ISO_CHECK(t::atan(std::numeric_limits<double>::denorm_min()) ==
              std::numeric_limits<double>::denorm_min());
    ISO_CHECK(t::atan(-std::numeric_limits<double>::denorm_min()) ==
              -std::numeric_limits<double>::denorm_min());
    ISO_CHECK_EQ_DBL(t::atan(1.0), t::PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(t::atan(-1.0), -t::PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(t::atan(1000.0), t::PI / 2.0, 1e-3);
    ISO_CHECK_EQ_DBL(t::atan(-1000.0), -t::PI / 2.0, 1e-3);
    ISO_CHECK_EQ_DBL(t::atan(t::tan(0.5)), 0.5, eps);

    // ── atan2 (four quadrants) ───────────────────────────────────────────
    ISO_CHECK_EQ_DBL(t::atan2(0.0, 1.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(1.0, 0.0), t::PI / 2.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(0.0, -1.0), t::PI, eps);
    ISO_CHECK_EQ_DBL(t::atan2(-1.0, 0.0), -t::PI / 2.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(1.0, 1.0), t::PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(1.0, -1.0), 3.0 * t::PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(-1.0, -1.0), -3.0 * t::PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(-1.0, 1.0), -t::PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(t::atan2(0.0, 0.0), 0.0, eps);

    // ── non-finite inputs stay defined (no UB in range reduction) ─────────
    {
        volatile double zero = 0.0;
        volatile double huge = 1e308;
        double nan = zero / zero;  // NaN
        double inf = huge * 10.0;  // +inf
        ISO_CHECK(t::sin(nan) != t::sin(nan));
        ISO_CHECK(t::cos(nan) != t::cos(nan));
        ISO_CHECK(t::sin(inf) != t::sin(inf));
    }

    return ISO_TEST_RESULT();
}
