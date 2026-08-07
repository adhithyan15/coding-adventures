/*
 * float_math_test.c — unit tests for the from-scratch elementary functions.
 *
 * Pure ISO C17 (no <math.h>, no libm): golden constants plus algebraic identity
 * sweeps that need no oracle — exp(log x) == x, cosh^2 - sinh^2 == 1,
 * sqrt(x)^2 == x, pow(x,2) == x*x, and so on. (Accuracy was separately
 * cross-checked against the platform libm locally to ~1 ULP; that oracle is not
 * committed, since libm may not be linked and this lane forbids it.)
 */
#include "float_math.h"
#include "iso_test.h"

#include <stdint.h>

/* Deterministic LCG → a double in [lo, hi). */
static uint64_t g_state = 0x2545F4914F6CDD1Du;
static double urand(double lo, double hi) {
    double u;
    g_state = g_state * 6364136223846793005u + 1442695040888963407u;
    u = (double)(g_state >> 11) / 9007199254740992.0; /* [0,1) with 53-bit mantissa */
    return lo + u * (hi - lo);
}

static double fabs_(double x) { return x < 0.0 ? -x : x; }
/* Relative closeness (falls back to absolute near zero). */
static int close(double a, double b, double tol) {
    double scale = fabs_(a) > fabs_(b) ? fabs_(a) : fabs_(b);
    if (scale < 1.0) {
        scale = 1.0;
    }
    return fabs_(a - b) <= scale * tol;
}

static void test_classification(void) {
    ISO_CHECK(fm_isnan(fm_nan()));
    ISO_CHECK(!fm_isnan(1.0));
    ISO_CHECK(fm_isinf(fm_inf()) == 1);
    ISO_CHECK(fm_isinf(-fm_inf()) == -1);
    ISO_CHECK(fm_isinf(1.0) == 0);
    ISO_CHECK(fm_isfinite(1.0) && !fm_isfinite(fm_inf()) && !fm_isfinite(fm_nan()));
}

static void test_rounding(void) {
    ISO_CHECK_EQ_DBL(fm_fabs(-3.5), 3.5, 0.0);
    ISO_CHECK_EQ_DBL(fm_copysign(3.0, -1.0), -3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_floor(2.7), 2.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_floor(-2.3), -3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_ceil(2.3), 3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_ceil(-2.7), -2.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_trunc(-2.7), -2.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_round(2.5), 3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_round(-2.5), -3.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_round(2.4), 2.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_fmod(10.0, 3.0), 1.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_fmod(-10.0, 3.0), -1.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_ldexp(1.5, 4), 24.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_ldexp(3.0, -1), 1.5, 0.0);
}

static void test_roots(void) {
    ISO_CHECK_EQ_DBL(fm_sqrt(4.0), 2.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_sqrt(2.0), FM_SQRT2, 1e-15);
    ISO_CHECK_EQ_DBL(fm_sqrt(0.0), 0.0, 0.0);
    ISO_CHECK(fm_isnan(fm_sqrt(-1.0)));
    ISO_CHECK_EQ_DBL(fm_cbrt(27.0), 3.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_cbrt(-8.0), -2.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_hypot(3.0, 4.0), 5.0, 1e-15);
}

static void test_exp_log(void) {
    ISO_CHECK_EQ_DBL(fm_exp(0.0), 1.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_exp(1.0), FM_E, 1e-14);
    ISO_CHECK_EQ_DBL(fm_log(FM_E), 1.0, 1e-14);
    ISO_CHECK_EQ_DBL(fm_log(1.0), 0.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_log(2.0), FM_LN2, 1e-14);
    ISO_CHECK_EQ_DBL(fm_log2(1024.0), 10.0, 1e-13);
    ISO_CHECK_EQ_DBL(fm_log10(1000.0), 3.0, 1e-13);
    ISO_CHECK_EQ_DBL(fm_expm1(0.0), 0.0, 0.0);
    ISO_CHECK(close(fm_expm1(1e-10), 1e-10, 1e-9)); /* accurate near 0 */
}

static void test_pow(void) {
    ISO_CHECK_EQ_DBL(fm_pow(2.0, 10.0), 1024.0, 1e-12);
    ISO_CHECK_EQ_DBL(fm_pow(2.0, -1.0), 0.5, 1e-15);
    ISO_CHECK_EQ_DBL(fm_pow(9.0, 0.5), 3.0, 1e-13);
    ISO_CHECK_EQ_DBL(fm_pow(5.0, 0.0), 1.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_pow(-2.0, 3.0), -8.0, 1e-13);
    ISO_CHECK_EQ_DBL(fm_pow(-2.0, 2.0), 4.0, 1e-13);
    ISO_CHECK(fm_isnan(fm_pow(-2.0, 0.5))); /* negative base, fractional power */
    ISO_CHECK_EQ_DBL(fm_pow(0.0, 3.0), 0.0, 0.0);
}

static void test_hyperbolics(void) {
    ISO_CHECK_EQ_DBL(fm_sinh(0.0), 0.0, 0.0);
    ISO_CHECK_EQ_DBL(fm_cosh(0.0), 1.0, 1e-15);
    ISO_CHECK_EQ_DBL(fm_tanh(0.0), 0.0, 0.0);
    /* Identity cosh^2 - sinh^2 == 1 at a specific point. */
    {
        double s = fm_sinh(2.0);
        double c = fm_cosh(2.0);
        ISO_CHECK(close(c * c - s * s, 1.0, 1e-13));
    }
}

/* Identity sweep — no oracle, so it stays pure ISO. */
static void test_identity_sweep(void) {
    int i;
    for (i = 0; i < 200000; ++i) {
        double x = urand(1e-6, 1e6);   /* positive domain */
        double y = urand(-30.0, 30.0);
        double t = urand(-15.0, 15.0);

        /* Roots. */
        ISO_CHECK(close(fm_sqrt(x) * fm_sqrt(x), x, 1e-13));
        {
            double c = fm_cbrt(x);
            ISO_CHECK(close(c * c * c, x, 1e-12));
        }
        /* exp/log are inverses; log turns products into sums. */
        ISO_CHECK(close(fm_log(fm_exp(t)), t, 1e-12));
        ISO_CHECK(close(fm_exp(fm_log(x)), x, 1e-12));
        {
            double x2 = urand(1e-6, 1e6);
            ISO_CHECK(close(fm_log(x * x2), fm_log(x) + fm_log(x2), 1e-11));
        }
        /* exp(a+b) == exp(a)*exp(b). */
        {
            double a = urand(-10.0, 10.0);
            double b = urand(-10.0, 10.0);
            ISO_CHECK(close(fm_exp(a + b), fm_exp(a) * fm_exp(b), 1e-12));
        }
        /* pow consistency: x^2 == x*x, x^y == exp(y*log x). */
        ISO_CHECK(close(fm_pow(x, 2.0), x * x, 1e-12));
        ISO_CHECK(close(fm_pow(x, y), fm_exp(y * fm_log(x)), 1e-10));
        /* Hyperbolic relations. tanh == sinh/cosh is well-conditioned
         * everywhere; cosh^2 - sinh^2 == 1 is only checked for small |t|, since
         * for large t it subtracts two huge near-equal values and the identity
         * (not the library) loses all its digits to cancellation. */
        {
            double s = fm_sinh(t);
            double c = fm_cosh(t);
            ISO_CHECK(close(fm_tanh(t), s / c, 1e-12));
            if (fm_fabs(t) < 3.0) {
                ISO_CHECK(close(c * c - s * s, 1.0, 1e-12));
            }
        }
    }
    ISO_CHECK(1);
}

int main(void) {
    test_classification();
    test_rounding();
    test_roots();
    test_exp_log();
    test_pow();
    test_hyperbolics();
    test_identity_sweep();
    return ISO_TEST_RESULT();
}
