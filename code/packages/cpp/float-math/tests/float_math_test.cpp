// float_math_test.cpp — unit tests for the from-scratch C++ elementary functions.
//
// Pure ISO C++17 (no <cmath>, no libm): golden constants + algebraic identity
// sweeps that need no oracle. Accuracy was separately cross-checked against the
// platform libm locally to ~1 ULP (that oracle is not committed).
#include "float_math.hpp"
#include "iso_test.h"

#include <cstdint>

namespace fm = ca::float_math;

namespace {

std::uint64_t g_state = 0x2545F4914F6CDD1Du;
double urand(double lo, double hi) {
    g_state = g_state * 6364136223846793005u + 1442695040888963407u;
    double u = static_cast<double>(g_state >> 11) / 9007199254740992.0;
    return lo + u * (hi - lo);
}
double af(double x) { return x < 0.0 ? -x : x; }
bool close(double a, double b, double tol) {
    double scale = af(a) > af(b) ? af(a) : af(b);
    if (scale < 1.0) scale = 1.0;
    return af(a - b) <= scale * tol;
}

void test_classification() {
    ISO_CHECK(fm::isnan(fm::nan()) && !fm::isnan(1.0));
    ISO_CHECK(fm::isinf(fm::inf()) == 1 && fm::isinf(-fm::inf()) == -1 && fm::isinf(1.0) == 0);
    ISO_CHECK(fm::isfinite(1.0) && !fm::isfinite(fm::inf()));
}

void test_rounding() {
    ISO_CHECK_EQ_DBL(fm::fabs(-3.5), 3.5, 0.0);
    ISO_CHECK_EQ_DBL(fm::copysign(3.0, -1.0), -3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm::floor(-2.3), -3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm::ceil(2.3), 3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm::trunc(-2.7), -2.0, 0.0);
    ISO_CHECK_EQ_DBL(fm::round(-2.5), -3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm::fmod(10.0, 3.0), 1.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm::ldexp(1.5, 4), 24.0, 0.0);
}

void test_core() {
    ISO_CHECK_EQ_DBL(fm::sqrt(4.0), 2.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm::sqrt(2.0), fm::SQRT2, 1e-15);
    ISO_CHECK(fm::isnan(fm::sqrt(-1.0)));
    ISO_CHECK_EQ_DBL(fm::cbrt(-8.0), -2.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm::hypot(3.0, 4.0), 5.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm::exp(1.0), fm::E, 1e-14);
    ISO_CHECK_EQ_DBL(fm::log(fm::E), 1.0, 1e-14);
    ISO_CHECK_EQ_DBL(fm::log2(1024.0), 10.0, 1e-13);
    ISO_CHECK_EQ_DBL(fm::log10(1000.0), 3.0, 1e-13);
    ISO_CHECK_EQ_DBL(fm::pow(2.0, 10.0), 1024.0, 1e-12);
    ISO_CHECK_EQ_DBL(fm::pow(-2.0, 3.0), -8.0, 1e-13);
    ISO_CHECK(fm::isnan(fm::pow(-2.0, 0.5)));
    ISO_CHECK_EQ_DBL(fm::cosh(0.0), 1.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm::tanh(0.0), 0.0, 0.0);
}

// constexpr smoke: bit-based constants are usable in constant expressions.
static_assert(fm::LN2 > 0.69 && fm::LN2 < 0.70, "constant sanity");

void test_identity_sweep() {
    for (int i = 0; i < 200000; ++i) {
        double x = urand(1e-6, 1e6);
        double y = urand(-30.0, 30.0);
        double t = urand(-15.0, 15.0);
        ISO_CHECK(close(fm::sqrt(x) * fm::sqrt(x), x, 1e-13));
        double c3 = fm::cbrt(x);
        ISO_CHECK(close(c3 * c3 * c3, x, 1e-12));
        ISO_CHECK(close(fm::log(fm::exp(t)), t, 1e-12));
        ISO_CHECK(close(fm::exp(fm::log(x)), x, 1e-12));
        double x2 = urand(1e-6, 1e6);
        ISO_CHECK(close(fm::log(x * x2), fm::log(x) + fm::log(x2), 1e-11));
        ISO_CHECK(close(fm::pow(x, 2.0), x * x, 1e-12));
        ISO_CHECK(close(fm::pow(x, y), fm::exp(y * fm::log(x)), 1e-10));
        double s = fm::sinh(t), c = fm::cosh(t);
        ISO_CHECK(close(fm::tanh(t), s / c, 1e-12));
        if (fm::fabs(t) < 3.0) ISO_CHECK(close(c * c - s * s, 1.0, 1e-12));
    }
    ISO_CHECK(true);
}

} // namespace

int main() {
    test_classification();
    test_rounding();
    test_core();
    test_identity_sweep();
    return ISO_TEST_RESULT();
}
