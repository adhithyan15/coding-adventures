/*
 * trig.c — implementation of the from-first-principles trig library.
 * ===========================================================================
 *
 * No <math.h>: we roll our own absolute value, truncation, and floating-point
 * remainder so the whole library depends on nothing but +, -, *, / and
 * comparisons. See trig.h for the mathematical background of each function.
 */
#include "trig.h"

/* 2*PI — a full revolution; pre-computed to save a multiply in range reduction. */
#define TRIG_TWO_PI (2.0 * TRIG_PI)
#define TRIG_HALF_PI (TRIG_PI / 2.0)

/* ---------------------------------------------------------------------------
 *  Tiny <math.h>-free helpers
 * ------------------------------------------------------------------------- */

/* |x| for a double, without fabs. Note -0.0 stays -0.0 (x < 0 is false); that
 * matches the Rust `.abs()` uses here, which only ever compare against a
 * positive threshold. */
static double d_abs(double x) { return x < 0.0 ? -x : x; }

/* Truncate `q` toward zero (like the integer part of the value).
 *
 * A double with |q| >= 2^53 has no fractional bits — it is already integral —
 * so we return it unchanged (and thereby avoid overflowing int64 territory).
 * Below that, |q| fits comfortably in a long long, whose float->int conversion
 * truncates toward zero per the C standard. */
static double d_trunc(double q) {
    const double two53 = 9007199254740992.0; /* 2^53 */
    /* Pass through anything NOT strictly inside (-2^53, 2^53). Written as a
     * negated in-range test (rather than `q >= two53 || q <= -two53`) so that a
     * NaN — which fails every comparison — takes this branch too, rather than
     * falling through to the cast: converting NaN to an integer is UB. */
    if (!(q > -two53 && q < two53)) {
        return q;
    }
    return (double)(long long)q;
}

/* Floating-point remainder x mod m, matching Rust's `%` on f64:
 *   x % m = x - m * trunc(x / m)
 * The result has the same sign as x and magnitude < |m|. */
static double d_fmod(double x, double m) { return x - m * d_trunc(x / m); }

/* Square root via Newton's method (no domain check — callers guarantee x >= 0).
 * See trig_sqrt for the documented public entry point. */
static double sqrt_unchecked(double x) {
    if (x == 0.0) {
        return 0.0;
    }
    /* Start high enough to converge quickly: x for x >= 1, else 1.0 so the
     * first step does not divide by a tiny number. */
    double guess = x >= 1.0 ? x : 1.0;
    int i;
    for (i = 0; i < 60; i++) {
        double next = (guess + x / guess) / 2.0;
        /* Stop when the improvement is negligible. The relative term handles
         * large values; the 1e-300 floor keeps subnormal inputs safe. */
        if (d_abs(next - guess) < 1e-15 * guess + 1e-300) {
            return next;
        }
        guess = next;
    }
    return guess;
}

/* Reduce `x` into [-PI, PI], preserving the value of any 2*PI-periodic
 * function so the Maclaurin series converges quickly. */
static double range_reduce(double x) {
    double r = d_fmod(x, TRIG_TWO_PI); /* now in (-2PI, 2PI) */
    if (r > TRIG_PI) {
        r -= TRIG_TWO_PI;
    }
    if (r < -TRIG_PI) {
        r += TRIG_TWO_PI;
    }
    return r;
}

/* ---------------------------------------------------------------------------
 *  sin / cos — Maclaurin series
 * ------------------------------------------------------------------------- */

double trig_sin(double x) {
    double rx = range_reduce(x);
    double x_squared = rx * rx;
    /* term tracks the current series term (starts at the k=0 term, x). */
    double term = rx;
    double sum = term;
    int k;
    for (k = 1; k < 20; k++) {
        /* term_k = term_{k-1} * -x^2 / ((2k)(2k+1)) — flips sign and folds in
         * the next two factorial factors, avoiding factorials and powers. */
        double denom = (double)(2 * k) * (double)(2 * k + 1);
        term *= -x_squared / denom;
        sum += term;
    }
    return sum;
}

double trig_cos(double x) {
    double rx = range_reduce(x);
    double x_squared = rx * rx;
    /* term starts at 1 (the k=0 term of the cosine series). */
    double term = 1.0;
    double sum = term;
    int k;
    for (k = 1; k < 20; k++) {
        double denom = (double)(2 * k - 1) * (double)(2 * k);
        term *= -x_squared / denom;
        sum += term;
    }
    return sum;
}

/* ---------------------------------------------------------------------------
 *  Angle conversion
 * ------------------------------------------------------------------------- */

double trig_radians(double deg) { return deg * (TRIG_PI / 180.0); }
double trig_degrees(double rad) { return rad * (180.0 / TRIG_PI); }

/* ---------------------------------------------------------------------------
 *  sqrt (public, domain-checked)
 * ------------------------------------------------------------------------- */

TrigStatus trig_sqrt(double x, double *out) {
    if (x < 0.0) {
        return TRIG_ERR_DOMAIN; /* real square root undefined for x < 0 */
    }
    *out = sqrt_unchecked(x);
    return TRIG_OK;
}

/* ---------------------------------------------------------------------------
 *  tan = sin / cos
 * ------------------------------------------------------------------------- */

double trig_tan(double x) {
    double s = trig_sin(x);
    double c = trig_cos(x);
    /* Near a pole (cos x ~ 0) return the largest finite magnitude with the
     * sign of sin, rather than dividing by ~0. */
    if (d_abs(c) < 1e-15) {
        return s > 0.0 ? 1.0e308 : -1.0e308;
    }
    return s / c;
}

/* ---------------------------------------------------------------------------
 *  atan / atan2 — Taylor series with two-layer range reduction
 * ------------------------------------------------------------------------- */

/* Inner atan for |x| <= 1: half-angle reduction, then the Taylor series.
 *   atan(x) = 2 * atan( x / (1 + sqrt(1 + x^2)) )
 * shrinks |x| to <= tan(PI/8) ~ 0.414 so the series converges in ~15 terms. */
static double atan_core(double x) {
    /* 1 + x^2 >= 1 > 0, so sqrt_unchecked is safe here. */
    double reduced = x / (1.0 + sqrt_unchecked(1.0 + x * x));
    double t = reduced;
    double t_sq = t * t;
    double term = t;
    double result = t;
    int n;
    for (n = 1; n <= 30; n++) {
        /* term_n = term_{n-1} * -t^2 * (2n-1)/(2n+1). */
        term = term * (-t_sq) * (double)(2 * n - 1) / (double)(2 * n + 1);
        result += term;
        if (d_abs(term) < 1e-17) {
            break; /* negligible; stop early */
        }
    }
    return 2.0 * result; /* undo the half-angle halving */
}

double trig_atan(double x) {
    if (x == 0.0) {
        return 0.0;
    }
    /* Layer-1 reduction for |x| > 1: atan(x) = ±PI/2 - atan(1/x). */
    if (x > 1.0) {
        return TRIG_HALF_PI - atan_core(1.0 / x);
    }
    if (x < -1.0) {
        return -TRIG_HALF_PI - atan_core(1.0 / x);
    }
    return atan_core(x);
}

double trig_atan2(double y, double x) {
    if (x > 0.0) {
        return trig_atan(y / x);
    }
    if (x < 0.0 && y >= 0.0) {
        return trig_atan(y / x) + TRIG_PI;
    }
    if (x < 0.0 && y < 0.0) {
        return trig_atan(y / x) - TRIG_PI;
    }
    if (x == 0.0 && y > 0.0) {
        return TRIG_HALF_PI;
    }
    if (x == 0.0 && y < 0.0) {
        return -TRIG_HALF_PI;
    }
    return 0.0; /* (0,0): undefined; 0 by convention */
}
