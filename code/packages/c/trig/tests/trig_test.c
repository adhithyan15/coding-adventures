/*
 * Tests for the C trig library, using the header-only iso_test.h harness (pure
 * ISO). Values are checked against known references within a small tolerance —
 * the whole point is that our from-scratch series match the real functions.
 */
#include "iso_test.h"

#include <float.h>
#include <math.h>

#include "trig.h"

int main(void) {
    const double eps = 1e-10;

    /* ── sin ─────────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(trig_sin(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(trig_sin(TRIG_PI / 2.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(trig_sin(TRIG_PI), 0.0, eps);
    ISO_CHECK_EQ_DBL(trig_sin(3.0 * TRIG_PI / 2.0), -1.0, eps);
    ISO_CHECK_EQ_DBL(trig_sin(TRIG_PI / 6.0), 0.5, eps); /* sin 30deg = 1/2 */
    /* Odd function, and periodic with 2*PI (exercises range reduction). */
    ISO_CHECK_EQ_DBL(trig_sin(-1.0), -trig_sin(1.0), eps);
    ISO_CHECK_EQ_DBL(trig_sin(1.0 + 10.0 * TRIG_PI), trig_sin(1.0), 1e-9);

    /* ── cos ─────────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(trig_cos(0.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(trig_cos(TRIG_PI), -1.0, eps);
    ISO_CHECK_EQ_DBL(trig_cos(TRIG_PI / 2.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(trig_cos(TRIG_PI / 3.0), 0.5, eps); /* cos 60deg = 1/2 */
    /* Even function. */
    ISO_CHECK_EQ_DBL(trig_cos(-2.0), trig_cos(2.0), eps);
    /* Pythagorean identity sin^2 + cos^2 = 1 at an arbitrary angle. */
    {
        double s = trig_sin(0.7), c = trig_cos(0.7);
        ISO_CHECK_EQ_DBL(s * s + c * c, 1.0, eps);
    }

    /* ── tan ─────────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(trig_tan(0.0), 0.0, eps);
    ISO_CHECK_EQ_DBL(trig_tan(TRIG_PI / 4.0), 1.0, eps);
    ISO_CHECK_EQ_DBL(trig_tan(-TRIG_PI / 4.0), -1.0, eps);
    /* Near the PI/2 pole tan saturates to the large finite sentinel. */
    ISO_CHECK(trig_tan(TRIG_PI / 2.0) > 1.0e300);

    /* ── angle conversion ────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(trig_radians(180.0), TRIG_PI, eps);
    ISO_CHECK_EQ_DBL(trig_radians(90.0), TRIG_PI / 2.0, eps);
    ISO_CHECK_EQ_DBL(trig_degrees(TRIG_PI), 180.0, eps);
    ISO_CHECK_EQ_DBL(trig_degrees(TRIG_PI / 6.0), 30.0, eps);
    /* Round trip. */
    ISO_CHECK_EQ_DBL(trig_degrees(trig_radians(45.0)), 45.0, eps);

    /* ── sqrt (status-code API; Rust panics on negative) ─────────────────── */
    {
        double r;
        ISO_CHECK(trig_sqrt(4.0, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL(r, 2.0, eps);
        ISO_CHECK(trig_sqrt(2.0, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL(r, 1.4142135623730951, eps);
        ISO_CHECK(trig_sqrt(0.0, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL(r, 0.0, eps);
        ISO_CHECK(trig_sqrt(1e12, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL(r, 1000000.0, 1e-4);
        ISO_CHECK(trig_sqrt(1e-100, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL((r - 1e-50) / 1e-50, 0.0, 1e-12);
        ISO_CHECK(trig_sqrt(0x0.0000000000001p-1022, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL((r - 2.2227587494850775e-162) /
                             2.2227587494850775e-162,
                         0.0, 1e-12);
        ISO_CHECK(trig_sqrt(DBL_MAX, &r) == TRIG_OK);
        ISO_CHECK_EQ_DBL((r - 1.3407807929942596e154) /
                             1.3407807929942596e154,
                         0.0, 1e-12);
        ISO_CHECK(trig_sqrt(-0.0, &r) == TRIG_OK);
        ISO_CHECK(r == 0.0 && signbit(r));
        ISO_CHECK(trig_sqrt(INFINITY, &r) == TRIG_OK);
        ISO_CHECK(isinf(r) && r > 0.0);
        ISO_CHECK(trig_sqrt(NAN, &r) == TRIG_OK);
        ISO_CHECK(isnan(r));
        /* Negative input is a domain error and leaves *out untouched. */
        r = -999.0;
        ISO_CHECK(trig_sqrt(-1.0, &r) == TRIG_ERR_DOMAIN);
        ISO_CHECK(r == -999.0);
    }

    /* ── atan ────────────────────────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(trig_atan(0.0), 0.0, eps);
    ISO_CHECK(signbit(trig_atan(-0.0)));
    ISO_CHECK(trig_atan(0x1p-30) == 0x1p-30);
    ISO_CHECK(trig_atan(0x0.0000000000001p-1022) ==
              0x0.0000000000001p-1022);
    ISO_CHECK(trig_atan(-0x0.0000000000001p-1022) ==
              -0x0.0000000000001p-1022);
    ISO_CHECK_EQ_DBL(trig_atan(1.0), TRIG_PI / 4.0, eps);
    ISO_CHECK_EQ_DBL(trig_atan(-1.0), -TRIG_PI / 4.0, eps);
    /* |x| > 1 exercises the layer-1 reduction. */
    ISO_CHECK_EQ_DBL(trig_atan(1000.0), TRIG_PI / 2.0, 1e-3);
    ISO_CHECK_EQ_DBL(trig_atan(-1000.0), -TRIG_PI / 2.0, 1e-3);
    /* atan is the inverse of tan on (-PI/2, PI/2). */
    ISO_CHECK_EQ_DBL(trig_atan(trig_tan(0.5)), 0.5, eps);

    /* ── atan2 (four quadrants) ──────────────────────────────────────────── */
    ISO_CHECK_EQ_DBL(trig_atan2(0.0, 1.0), 0.0, eps);          /* +x axis */
    ISO_CHECK_EQ_DBL(trig_atan2(1.0, 0.0), TRIG_PI / 2.0, eps); /* +y axis */
    ISO_CHECK_EQ_DBL(trig_atan2(0.0, -1.0), TRIG_PI, eps);      /* -x axis */
    ISO_CHECK_EQ_DBL(trig_atan2(-1.0, 0.0), -TRIG_PI / 2.0, eps); /* -y axis */
    ISO_CHECK_EQ_DBL(trig_atan2(1.0, 1.0), TRIG_PI / 4.0, eps);   /* Q1 */
    ISO_CHECK_EQ_DBL(trig_atan2(1.0, -1.0), 3.0 * TRIG_PI / 4.0, eps);  /* Q2 */
    ISO_CHECK_EQ_DBL(trig_atan2(-1.0, -1.0), -3.0 * TRIG_PI / 4.0, eps); /* Q3 */
    ISO_CHECK_EQ_DBL(trig_atan2(-1.0, 1.0), -TRIG_PI / 4.0, eps);       /* Q4 */
    ISO_CHECK_EQ_DBL(trig_atan2(0.0, 0.0), 0.0, eps); /* undefined -> 0 */

    /* ── non-finite inputs stay defined (no UB in range reduction) ───────── */
    {
        /* Build NaN and +inf at runtime (volatile blocks constant folding). */
        volatile double zero = 0.0;
        volatile double huge = 1e308;
        double nan = zero / zero; /* NaN */
        double inf = huge * 10.0; /* +inf */
        /* sin/cos of NaN propagate NaN (NaN != NaN); the point is that the
         * NaN never reaches the double->long long cast in range reduction. */
        ISO_CHECK(trig_sin(nan) != trig_sin(nan));
        ISO_CHECK(trig_cos(nan) != trig_cos(nan));
        /* sin(inf) reduces to inf - inf = NaN — also defined, no cast UB. */
        ISO_CHECK(trig_sin(inf) != trig_sin(inf));
    }

    return ISO_TEST_RESULT();
}
