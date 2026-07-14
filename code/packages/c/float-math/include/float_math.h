/*
 * float_math.h — elementary floating-point functions, from scratch (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * A from-first-principles replacement for the parts of the C math library the
 * campaign's ports need — WITHOUT linking libm. Every function is computed from
 * nothing but +, -, *, /, comparisons, and IEEE-754 bit manipulation (via
 * memcpy, so no strict-aliasing or type-punning UB). Nothing here calls the
 * platform's `<math.h>`; the whole point is that a math-using port depends on
 * THIS library instead of libm, keeping the pure-ISO lane self-contained and
 * identical across GCC, Clang, and MSVC.
 *
 * Companion to the `trig` crate (sin/cos/tan/atan): this one covers roots,
 * exponentials, logarithms, powers, and hyperbolics.
 *
 * Accuracy target: solid double precision (~1e-15 relative on the normal range),
 * verified against the platform libm as a local, non-shipped oracle.
 */
#ifndef FLOAT_MATH_H
#define FLOAT_MATH_H

#ifdef __cplusplus
extern "C" {
#endif

/* High-precision constants (nearest double to the true value). */
#define FM_PI     3.141592653589793238462643383279503
#define FM_E      2.718281828459045235360287471352662
#define FM_LN2    0.693147180559945309417232121458177
#define FM_LN10   2.302585092994045684017991454684364
#define FM_LOG2E  1.442695040888963407359924681001892
#define FM_LOG10E 0.434294481903251827651128918916605
#define FM_SQRT2  1.414213562373095048801688724209698

/* ------------------------------------------------------------------ */
/* Classification (no <math.h>)                                       */
/* ------------------------------------------------------------------ */

int fm_isnan(double x);
int fm_isinf(double x);     /* +1 for +inf, -1 for -inf, 0 otherwise */
int fm_isfinite(double x);
double fm_inf(void);        /* +infinity */
double fm_nan(void);        /* a quiet NaN */

/* ------------------------------------------------------------------ */
/* Sign / rounding / remainder (exact, bit- or arithmetic-based)      */
/* ------------------------------------------------------------------ */

double fm_fabs(double x);
double fm_copysign(double mag, double sgn);
double fm_floor(double x);
double fm_ceil(double x);
double fm_trunc(double x);
double fm_round(double x);  /* round half away from zero */
double fm_fmod(double x, double y);

/* Scale by a power of two: x * 2^n (exact for representable results). */
double fm_ldexp(double x, int n);
/* Decompose x = frac * 2^exp with frac in [0.5, 1); writes *exp. */
double fm_frexp(double x, int *exp);

/* ------------------------------------------------------------------ */
/* Roots                                                              */
/* ------------------------------------------------------------------ */

double fm_sqrt(double x);   /* NaN for x < 0 */
double fm_cbrt(double x);   /* defined for negative x */
double fm_hypot(double x, double y); /* sqrt(x^2+y^2) without overflow */

/* ------------------------------------------------------------------ */
/* Exponentials / logarithms                                          */
/* ------------------------------------------------------------------ */

double fm_exp(double x);
double fm_expm1(double x);  /* exp(x) - 1, accurate near 0 */
double fm_log(double x);    /* natural log; NaN for x<0, -inf for 0 */
double fm_log2(double x);
double fm_log10(double x);
double fm_log_base(double x, double base);

/* ------------------------------------------------------------------ */
/* Power                                                              */
/* ------------------------------------------------------------------ */

/* x^y. Integer y is handled exactly by squaring (and works for x<0);
 * non-integer y with x<0 is a domain error (NaN). */
double fm_pow(double x, double y);

/* ------------------------------------------------------------------ */
/* Hyperbolics                                                        */
/* ------------------------------------------------------------------ */

double fm_sinh(double x);
double fm_cosh(double x);
double fm_tanh(double x);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* FLOAT_MATH_H */
