/*
 * polynomial.h — coefficient-array polynomial arithmetic over doubles, in pure
 * ISO C17. A faithful port of the Rust `polynomial` crate.
 * ===========================================================================
 *
 * A polynomial a0 + a1*x + a2*x^2 + ... is represented as a **little-endian**
 * array of coefficients: `poly[i]` is the coefficient of x^i. The zero
 * polynomial is the empty array (length 0).
 *
 *   poly_normalize   — strip trailing (high-degree) near-zero coefficients
 *   poly_degree      — index of the highest non-zero coefficient
 *   poly_add / poly_subtract / poly_multiply — arithmetic
 *   poly_divmod / poly_divide / poly_modulo  — long division
 *   poly_evaluate    — Horner evaluation at a point
 *   poly_gcd         — Euclidean GCD of two polynomials
 *
 * Buffers are caller-provided (nothing but `poly_gcd` allocates, and it frees
 * its own scratch). Each operation returns the length of the normalized result
 * written to `out`; size the output as noted per function. A coefficient with
 * magnitude <= DBL_EPSILON*1e6 is treated as zero (the crate's threshold).
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions (no libm).
 */
#ifndef POLYNOMIAL_H
#define POLYNOMIAL_H

#include <stddef.h> /* size_t */

/* poly_normalize — copy `p` (n coeffs) to `out` (capacity n) with trailing
 * near-zero coefficients stripped; returns the normalized length. */
size_t poly_normalize(const double *p, size_t n, double *out);

/* poly_degree — the index of the highest non-zero coefficient (0 for the zero
 * polynomial). */
size_t poly_degree(const double *p, size_t n);

/* poly_add / poly_subtract — a +/- b, normalized. `out` capacity >= max(na, nb).
 * Returns the result length. */
size_t poly_add(const double *a, size_t na, const double *b, size_t nb,
                double *out);
size_t poly_subtract(const double *a, size_t na, const double *b, size_t nb,
                     double *out);

/* poly_multiply — a * b, normalized. `out` capacity >= na + nb - 1 (0 if either
 * is empty). Returns the result length. */
size_t poly_multiply(const double *a, size_t na, const double *b, size_t nb,
                     double *out);

/* poly_divmod — long division: dividend = divisor*quotient + remainder, with
 * degree(remainder) < degree(divisor). `quot` and `rem` each need capacity nd
 * (the dividend length). Returns 1 on success, 0 if `divisor` is the zero
 * polynomial. */
int poly_divmod(const double *dividend, size_t nd, const double *divisor,
                size_t nv, double *quot, size_t *quot_len, double *rem,
                size_t *rem_len);

/* poly_divide / poly_modulo — the quotient / remainder alone. Same buffer
 * requirement (capacity nd) and return convention as poly_divmod. */
int poly_divide(const double *dividend, size_t nd, const double *divisor,
                size_t nv, double *quot, size_t *quot_len);
int poly_modulo(const double *dividend, size_t nd, const double *divisor,
                size_t nv, double *rem, size_t *rem_len);

/* poly_evaluate — value of the polynomial at `x` (Horner's method). */
double poly_evaluate(const double *p, size_t n, double x);

/* poly_gcd — the polynomial GCD of `a` and `b` (Euclidean algorithm). `out`
 * capacity >= max(na, nb). Returns the result length (0 for the zero GCD or on
 * an internal allocation failure). */
size_t poly_gcd(const double *a, size_t na, const double *b, size_t nb,
                double *out);

#endif /* POLYNOMIAL_H */
