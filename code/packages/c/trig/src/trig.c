/*
 * trig.c -- implementation of the from-first-principles trig library.
 * ===========================================================================
 *
 * No <math.h>: the whole library depends on nothing but +, -, *, / and
 * comparisons.  See trig.h for the mathematical background of each function.
 *
 * This file implements the reference algorithm TRIG-REF-1 of PHY00a, bit for
 * bit.  "Bit for bit" is a strong promise: every one of the 62,154 vectors in
 * code/specs/fixtures/trig-reference-v1 is checked by tests/conformance_test.c
 * and must match to the last bit.  Seventeen language ports share that one
 * algorithm, so a Warrant can claim "this function returns exactly these bits"
 * and have the claim hold in every lane.
 *
 * The analogy is a metre bar.  Every workshop cuts its own "one metre" rod;
 * they all look right and differ by a hair.  TRIG-REF-1 is the master bar and
 * the vectors are the calibration marks.
 */
#include "trig.h"

#include <float.h>

/* ===========================================================================
 *  The arithmetic model (PHY00a 3.1) -- and how C keeps to it
 * ===========================================================================
 *
 * TRIG-REF-1 is defined as a sequence of IEEE-754 binary64 operations, each
 * rounded ONCE, round-to-nearest-even.  Three things can silently break that,
 * and each is refused here instead of hoped away.
 *
 * 1. FUSED MULTIPLY-ADD.  A compiler may turn `a*b + c` into one instruction
 *    that rounds once instead of twice.  That is more accurate and it is a
 *    DIFFERENT answer.  Every fusable site in this file is written as two
 *    statements, but a compiler may fuse across statements, so we also switch
 *    contraction off.  The pragma is per compiler, because an unknown pragma
 *    is a -Wunknown-pragmas diagnostic and the build uses -Werror:
 *      - clang (and clang-cl, Apple clang): the ISO C pragma FP_CONTRACT.
 *      - GCC: ignores the ISO pragma; its own `optimize` attribute works.
 *      - MSVC: `fp_contract(off)`.
 *    (clang's `-ffp-contract=fast` command-line flag overrides the pragma; the
 *    package build never passes it.)
 *
 * 2. EXTENDED PRECISION.  On 32-bit x87, intermediates live in 80-bit
 *    registers and `(t + MAGIC) - MAGIC` stops rounding to an integer.
 *    FLT_EVAL_METHOD == 0 means "evaluate in the operand's own type"; we
 *    refuse to compile otherwise.  The CI targets are x86-64 (SSE2) and arm64.
 *
 * 3. FAST-MATH.  -ffast-math reassociates and substitutes reciprocals.
 *    GCC and clang define __FAST_MATH__ under it; we refuse that too.
 */
#if defined(__clang__)
#pragma STDC FP_CONTRACT OFF
#elif defined(__GNUC__)
#pragma GCC optimize("fp-contract=off")
#elif defined(_MSC_VER)
#pragma fp_contract(off)
#endif

#if defined(__FAST_MATH__)
#error "trig requires strict IEEE-754 arithmetic: do not build with -ffast-math"
#endif

_Static_assert(FLT_EVAL_METHOD == 0,
               "trig requires FLT_EVAL_METHOD == 0 (no x87 extended precision)");
_Static_assert(FLT_RADIX == 2 && DBL_MANT_DIG == 53,
               "trig requires IEEE-754 binary64 doubles");

/* ---------------------------------------------------------------------------
 *  Constants (PHY00a 3.2)
 * ---------------------------------------------------------------------------
 * Every constant is a HEX FLOAT literal, so the text IS the bit pattern: no
 * decimal parser is involved and nothing can round differently.  The bit
 * patterns are in the comments; the conformance vectors pin them.
 */

/* Pi and friends. */
#define TRIG_HALF_PI 0x1.921fb54442d18p+0 /* 0x3FF921FB54442D18 = PI / 2 */

/* Range reduction.  INV_PIO2 is the double nearest 2/pi. */
static const double INV_PIO2 = 0x1.45f306dc9c883p-1; /* 0x3FE45F306DC9C883 */

/* pi/2 as a SUM of five pieces (Cody and Waite, 1980).  The first four have at
 * most 24 significant bits, so k * PIO2_i is EXACT whenever |k| < 2^29
 * (24 + 29 = 53 bits).  The fifth carries the rest, rounded to 53 bits.  Their
 * sum is pi/2 to about 2^-150. */
static const double PIO2_1 = 0x1.921fb4p+0;       /* 0x3FF921FB40000000 */
static const double PIO2_2 = 0x1.4442dp-24;       /* 0x3E74442D00000000 */
static const double PIO2_3 = 0x1.846988p-48;      /* 0x3CF8469880000000 */
static const double PIO2_4 = 0x1.8cc516p-72;      /* 0x3B78CC5160000000 */
static const double PIO2_5 = 0x1.01b839a25204ap-96; /* 0x39F01B839A25204A */

/* MAGIC = 1.5 * 2^52.  Adding it to |t| < 2^51 pushes the fraction bits off
 * the end of the significand, so (t + MAGIC) - MAGIC is t rounded to the
 * nearest integer, ties to even, using only an add and a subtract. */
static const double MAGIC = 0x1.8p+52; /* 0x4338000000000000 */

/* The domain: |x| <= 2^29.  Beyond it the quotient no longer fits the exact
 * products above, and sin/cos/tan return NaN rather than a plausible lie. */
static const double LIMIT = 0x1p+29; /* 0x41C0000000000000 */

/* Below 2^-27, sin(x) == x to the last bit (x^3/6 < half an ulp of x) and
 * cos(x) == 1. */
static const double TINY = 0x1p-27; /* 0x3E40000000000000 */

/* sin series coefficients S[n] = (-1)^n / (2n+1)!, n = 1..9, each rounded once.
 *   n  value    n  value
 *   1  -1/3!    6  +1/13!
 *   2  +1/5!    7  -1/15!
 *   3  -1/7!    8  +1/17!
 *   4  +1/9!    9  -1/19!
 *   5  -1/11!
 * Index 0 is unused so that S[n] reads like the mathematics. */
static const double S[10] = {
    0.0,
    -0x1.5555555555555p-3,  /* S1 -1/3!  0xBFC5555555555555 */
    0x1.1111111111111p-7,   /* S2 +1/5!  0x3F81111111111111 */
    -0x1.a01a01a01a01ap-13, /* S3 -1/7!  0xBF2A01A01A01A01A */
    0x1.71de3a556c734p-19,  /* S4 +1/9!  0x3EC71DE3A556C734 */
    -0x1.ae64567f544e4p-26, /* S5 -1/11! 0xBE5AE64567F544E4 */
    0x1.6124613a86d09p-33,  /* S6 +1/13! 0x3DE6124613A86D09 */
    -0x1.ae7f3e733b81fp-41, /* S7 -1/15! 0xBD6AE7F3E733B81F */
    0x1.952c77030ad4ap-49,  /* S8 +1/17! 0x3CE952C77030AD4A */
    -0x1.2f49b46814157p-57  /* S9 -1/19! 0xBC62F49B46814157 */
};

/* cos series coefficients C[n] = (-1)^n / (2n)!, n = 2..9 (the n = 1 term,
 * -1/2, is applied as 0.5 * z, which is exact). */
static const double C[10] = {
    0.0,
    0.0,
    0x1.5555555555555p-5,   /* C2 +1/4!  0x3FA5555555555555 */
    -0x1.6c16c16c16c17p-10, /* C3 -1/6!  0xBF56C16C16C16C17 */
    0x1.a01a01a01a01ap-16,  /* C4 +1/8!  0x3EFA01A01A01A01A */
    -0x1.27e4fb7789f5cp-22, /* C5 -1/10! 0xBE927E4FB7789F5C */
    0x1.1eed8eff8d898p-29,  /* C6 +1/12! 0x3E21EED8EFF8D898 */
    -0x1.93974a8c07c9dp-37, /* C7 -1/14! 0xBDA93974A8C07C9D */
    0x1.ae7f3e733b81fp-45,  /* C8 +1/16! 0x3D2AE7F3E733B81F */
    -0x1.6827863b97d97p-53  /* C9 -1/18! 0xBCA6827863B97D97 */
};

/* Degree conversions, pre-divided: the doubles nearest pi/180 and 180/pi, so
 * radians() and degrees() are ONE rounding each (not two, as `d * PI / 180`). */
static const double RAD_PER_DEG = 0x1.1df46a2529d39p-6; /* 0x3F91DF46A2529D39 */
static const double DEG_PER_RAD = 0x1.ca5dc1a63c1f8p+5; /* 0x404CA5DC1A63C1F8 */

/* ---------------------------------------------------------------------------
 *  Tiny <math.h>-free helpers
 * ------------------------------------------------------------------------- */

/* |x| for a double, without fabs.  Note -0.0 stays -0.0 (x < 0 is false) and a
 * NaN stays a NaN; every caller only compares the result with a positive
 * threshold, where that does not matter. */
static double d_abs(double x) { return x < 0.0 ? -x : x; }

/* A quiet NaN, built at run time.  `volatile` stops the compiler folding
 * 0.0/0.0 at compile time (where -pedantic and MSVC's /W4 object).  The sign
 * and payload of a NaN are not portable, so every consumer tests "is it a
 * NaN", never its bits (VER04p 3.3). */
static double make_nan(void) {
    volatile double zero = 0.0;
    return zero / zero;
}

/* Square root via Newton's method (no domain check — callers guarantee x >= 0).
 * See trig_sqrt for the documented public entry point. */
static double sqrt_unchecked(double x) {
    if (x == 0.0) {
        return x;
    }
    if (x > DBL_MAX) {
        return x;
    }
    double scaled = x;
    double result_scale = 1.0;
    while (scaled < 0.25) {
        scaled *= 4.0;
        result_scale *= 0.5;
    }
    while (scaled >= 4.0) {
        scaled *= 0.25;
        result_scale *= 2.0;
    }
    double guess = scaled >= 1.0 ? scaled : 1.0;
    int i;
    for (i = 0; i < 60; i++) {
        double next = (guess + scaled / guess) / 2.0;
        /* Stop when the improvement is negligible. The relative term handles
         * large values; the 1e-300 floor keeps subnormal inputs safe. */
        if (d_abs(next - guess) < 1e-15 * guess + 1e-300) {
            return next * result_scale;
        }
        guess = next;
    }
    return guess * result_scale;
}

/* ---------------------------------------------------------------------------
 *  Range reduction (PHY00a 3.4)
 * ---------------------------------------------------------------------------
 * A sine is a repeating wave: knowing it on [-pi/4, pi/4] and which quarter-
 * turn you are in is enough to know it everywhere.  Reducing answers two
 * questions: how many quarter-turns fit in x (an integer k), and what is left
 * over (r = x - k*pi/2, with |r| <= pi/4)?
 *
 *   Step 1, the quotient:    t = x * (2/pi)
 *                            k = (t + MAGIC) - MAGIC      nearest integer
 *
 *   Step 2, the remainder, by Cody-Waite subtraction of five slices of pi/2:
 *
 *       x ----------------------------------------------------------- angle
 *       |-- k*PIO2_1 --|                                              24 bits
 *                      |-- k*PIO2_2 --|                               next 24
 *                                     |-- k*PIO2_3 --|                next 24
 *                                                    |- k*PIO2_4 -|   next 24
 *                                                                 |PIO2_5|
 *       r = what is left, |r| <= pi/4 (plus a few ulps)
 *
 * Each product k*PIO2_i (i = 1..4) is exact because PIO2_i has at most 24 bits
 * and |k| < 2^29.  The first two subtractions are exact; the last three round.
 * Every product is written as its own statement so that no compiler may fuse
 * it with the subtraction (the contraction pragma above is the second guard).
 *
 * Nothing here calls floor, round, trunc or fmod: only +, - and *.
 */
static void reduce(double x, double *k_out, double *r_out) {
    double t = x * INV_PIO2;
    double k = (t + MAGIC) - MAGIC;
    double p;
    double r = x;
    p = k * PIO2_1;
    r = r - p;
    p = k * PIO2_2;
    r = r - p;
    p = k * PIO2_3;
    r = r - p;
    p = k * PIO2_4;
    r = r - p;
    p = k * PIO2_5;
    r = r - p;
    *k_out = k;
    *r_out = r;
}

/* ---------------------------------------------------------------------------
 *  The series kernels (PHY00a 3.5): Horner's rule on stored coefficients
 * ---------------------------------------------------------------------------
 * On |r| <= pi/4, with z = r*r:
 *
 *   sin(r) = r + r*z*(S1 + S2 z + S3 z^2 + ... + S9 z^8)    r^1 .. r^19
 *   cos(r) = 1 - z/2 + z^2*(C2 + C3 z + ... + C9 z^7)        r^0 .. r^18
 *
 * Horner evaluates the bracket innermost-first, one multiply and one add per
 * step, adding the smallest terms first (the accurate order).  There is no
 * division and no "build each term from the last" recurrence, so there is only
 * one way to group the arithmetic.  The factorials are visible in S and C.
 */
static double sin_kernel(double r) {
    double z = r * r;
    double p = S[9];
    double w;
    int n;
    for (n = 8; n >= 1; n--) {
        p = p * z;
        p = p + S[n];
    }
    w = r * z;
    w = w * p;
    return r + w;
}

static double cos_kernel(double r) {
    double z = r * r;
    double q = C[9];
    double u, v;
    int n;
    for (n = 8; n >= 2; n--) {
        q = q * z;
        q = q + C[n];
    }
    u = 0.5 * z; /* exact */
    u = 1.0 - u;
    v = z * z;
    v = v * q;
    return u + v;
}

/* ---------------------------------------------------------------------------
 *  sin / cos (PHY00a 3.3 and 3.6)
 * ---------------------------------------------------------------------------
 * After reduction, x = r + k*(pi/2), so the quadrant q = k mod 4 picks which
 * of four identities applies:
 *
 *      q   sin(x)       cos(x)
 *      0    sin r        cos r
 *      1    cos r       -sin r
 *      2   -sin r       -cos r
 *      3   -cos r        sin r
 *
 * `(int)k & 3` is exact: after the domain guard k is an integer-valued double
 * with |k| < 2^29, so the cast is defined, and `& 3` on two's complement is
 * the FLOOR modulus 4 (k = -1 gives 3, k = -5 gives 3).
 *
 * THE GUARD MUST BE A NEGATED `<=`.  A NaN fails every comparison, so
 * `!(|x| <= LIMIT)` is true for NaN, both infinities and every finite value
 * beyond 2^29: one test covers all three.  The tempting `|x| > LIMIT` lets a
 * NaN through to the integer cast, which is undefined behaviour.
 */
double trig_sin(double x) {
    double k, r;
    if (!(d_abs(x) <= LIMIT)) {
        return make_nan(); /* NaN, +-inf, or |x| > 2^29: outside the domain */
    }
    if (d_abs(x) < TINY) {
        return x; /* keeps -0 and every subnormal; makes sin(-0) = -0 */
    }
    reduce(x, &k, &r);
    switch ((int)k & 3) {
    case 0:
        return sin_kernel(r);
    case 1:
        return cos_kernel(r);
    case 2:
        return -sin_kernel(r);
    default:
        return -cos_kernel(r);
    }
}

double trig_cos(double x) {
    double k, r;
    if (!(d_abs(x) <= LIMIT)) {
        return make_nan();
    }
    if (d_abs(x) < TINY) {
        return 1.0;
    }
    reduce(x, &k, &r);
    switch ((int)k & 3) {
    case 0:
        return cos_kernel(r);
    case 1:
        return -sin_kernel(r);
    case 2:
        return -cos_kernel(r);
    default:
        return sin_kernel(r);
    }
}

/* ---------------------------------------------------------------------------
 *  Angle conversion (PHY00a 3.7): one multiply by a pinned constant
 * ------------------------------------------------------------------------- */

double trig_radians(double deg) { return deg * RAD_PER_DEG; }
double trig_degrees(double rad) { return rad * DEG_PER_RAD; }

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
 *  tan = sin / cos (PHY00a 3.8)
 * ------------------------------------------------------------------------- */

double trig_tan(double x) {
    double s = trig_sin(x);
    double c = trig_cos(x);
    /* Near a pole (cos x ~ 0) return the largest finite magnitude with the
     * sign of sin, rather than dividing by ~0.  This is a PINNED LEGACY
     * CONVENTION (PHY00 3.6), not an approximation of the tangent.  The test is
     * false when c is NaN, so NaN / NaN falls through and stays NaN. */
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
    /* atan(x) rounds exactly to x here; avoid halving subnormals and retain -0. */
    if (d_abs(x) <= 0x1p-27) {
        return x;
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
    /* Added by TRIG-REF-1 (PHY00a 3.9): a NaN in either position is NaN.
     * Before, a NaN fell through every comparison below to the final branch and
     * returned 0.0 -- a finite answer to an undefined question.  `x != x` is
     * true only for a NaN. */
    if (x != x || y != y) {
        return x + y; /* NaN + anything is NaN */
    }
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
