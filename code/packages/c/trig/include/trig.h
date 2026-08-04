/*
 * trig.h — trigonometric functions from first principles, in pure ISO C17.
 * A faithful port of the Rust `trig` crate.
 * ===========================================================================
 *
 * Every value is computed from BASIC ARITHMETIC — no <math.h>, no libm, no
 * `sin`/`cos`/`sqrt` from the C standard library. The point of the crate is to
 * show *how* these functions are calculated:
 *
 *   - sin / cos      Maclaurin (Taylor-at-zero) series, after reducing the
 *                    argument into [-PI, PI] so the series converges fast:
 *                        sin(x) = x - x^3/3! + x^5/5! - ...
 *                        cos(x) = 1 - x^2/2! + x^4/4! - ...
 *   - sqrt           Newton's (Babylonian) method: the average of `guess` and
 *                    `x/guess` is a better guess; quadratic convergence.
 *   - tan            sin(x) / cos(x), guarding the cos(x)=0 poles.
 *   - atan / atan2   Taylor series with two layers of range reduction.
 *   - radians/degrees  the linear conversions (PI/180 and 180/PI).
 *
 * DIVERGENCE FROM RUST. Rust's `sqrt` panics on a negative input; this port
 * returns a status code instead (`trig_sqrt` writes to an out-parameter and
 * returns `TRIG_OK` or `TRIG_ERR_DOMAIN`). Every other function is total and
 * returns a `double` directly, exactly like the Rust crate.
 *
 * PORTABILITY. Pure ISO C17 — no compiler extensions, no <math.h>. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_TRIG_H
#define CA_TRIG_H

#ifdef __cplusplus
extern "C" {
#endif

/* The ratio of a circle's circumference to its diameter — hand-written to f64
 * precision so the library is fully self-contained. */
#define TRIG_PI 3.141592653589793

/* Status of the one fallible function (`trig_sqrt`). */
typedef enum {
    TRIG_OK = 0,
    TRIG_ERR_DOMAIN /* input outside the function's domain (e.g. sqrt of < 0) */
} TrigStatus;

/* sin / cos of `x` (radians), via a 20-term Maclaurin series. */
double trig_sin(double x);
double trig_cos(double x);

/* tan(x) = sin(x)/cos(x); near a pole (|cos x| < 1e-15) returns +/-1e308. */
double trig_tan(double x);

/* Angle conversions. */
double trig_radians(double deg); /* deg * (PI/180) */
double trig_degrees(double rad); /* rad * (180/PI) */

/* Square root via Newton's method. Writes the root to *out and returns TRIG_OK,
 * or returns TRIG_ERR_DOMAIN (leaving *out untouched) when x < 0. */
TrigStatus trig_sqrt(double x, double *out);

/* Arctangent in (-PI/2, PI/2), via a Taylor series with range reduction. */
double trig_atan(double x);
/* Four-quadrant arctangent of (y, x) in (-PI, PI]. */
double trig_atan2(double y, double x);

#ifdef __cplusplus
}
#endif

#endif /* CA_TRIG_H */
