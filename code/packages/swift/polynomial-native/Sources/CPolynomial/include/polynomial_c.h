/*
 * polynomial_c.h — C declarations for the Rust polynomial-c static library.
 *
 * This header is the public contract between the Rust implementation and any
 * C, C++, or Swift caller. Import this header via a module map to use the
 * polynomial arithmetic functions from Swift.
 *
 * MEMORY PROTOCOL
 * ---------------
 * The caller owns all memory. Array-returning functions write into a
 * caller-provided output buffer and return the number of elements written.
 *
 * Allocate worst-case sizes:
 *
 *   poly_c_normalize   out_cap >= len
 *   poly_c_add         out_cap >= max(a_len, b_len)
 *   poly_c_subtract    out_cap >= max(a_len, b_len)
 *   poly_c_multiply    out_cap >= a_len + b_len - 1
 *   poly_c_divide      out_cap >= dividend_len
 *   poly_c_modulo      out_cap >= divisor_len
 *   poly_c_gcd         out_cap >= max(a_len, b_len)
 *   poly_c_divmod      quot_cap >= dividend_len, rem_cap >= divisor_len
 *
 * ERROR HANDLING
 * --------------
 * Most functions silently return 0 elements on error (e.g., zero divisor).
 * poly_c_divmod returns -1 on error (zero divisor polynomial), 0 on success.
 * Use poly_c_divmod for reliable error detection on division operations.
 *
 * POLYNOMIAL REPRESENTATION
 * -------------------------
 * Polynomials are arrays of double where index = degree of that term:
 *
 *   {3.0, 0.0, 2.0}  →  3 + 0·x + 2·x²  =  3 + 2x²
 *   {1.0, 2.0, 3.0}  →  1 + 2x + 3x²
 *   {} (len=0)        →  the zero polynomial
 */

#ifndef POLYNOMIAL_C_H
#define POLYNOMIAL_C_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Fundamentals ──────────────────────────────────────────────────────────── */

/**
 * Normalize a polynomial: strip trailing near-zero coefficients.
 *
 * Returns the number of elements written to `out`.
 * Example: {1.0, 0.0, 0.0} → writes {1.0}, returns 1.
 */
size_t poly_c_normalize(
    const double *coeffs, size_t len,
    double *out, size_t out_cap
);

/**
 * Return the degree of a polynomial.
 *
 * Degree is the index of the highest non-zero coefficient.
 * The zero polynomial returns 0 by convention.
 */
size_t poly_c_degree(const double *coeffs, size_t len);

/**
 * Evaluate a polynomial at x using Horner's method (O(n) time).
 *
 * Returns the value p(x).
 */
double poly_c_evaluate(const double *coeffs, size_t len, double x);

/* ── Addition & Subtraction ────────────────────────────────────────────────── */

/**
 * Add two polynomials term-by-term.
 *
 * Worst-case output length: max(a_len, b_len).
 * Returns the number of elements written to `out`.
 */
size_t poly_c_add(
    const double *a, size_t a_len,
    const double *b, size_t b_len,
    double *out, size_t out_cap
);

/**
 * Subtract polynomial b from polynomial a term-by-term.
 *
 * Worst-case output length: max(a_len, b_len).
 * Returns the number of elements written to `out`.
 */
size_t poly_c_subtract(
    const double *a, size_t a_len,
    const double *b, size_t b_len,
    double *out, size_t out_cap
);

/* ── Multiplication ────────────────────────────────────────────────────────── */

/**
 * Multiply two polynomials by polynomial convolution.
 *
 * Worst-case output length: a_len + b_len - 1 (or 0 if either is empty).
 * Returns the number of elements written to `out`.
 */
size_t poly_c_multiply(
    const double *a, size_t a_len,
    const double *b, size_t b_len,
    double *out, size_t out_cap
);

/* ── Division ──────────────────────────────────────────────────────────────── */

/**
 * Polynomial long division: dividend = divisor × quotient + remainder.
 *
 * Writes quotient and remainder into separate caller-provided buffers.
 * Sets *quot_len_out and *rem_len_out to the number of elements written.
 *
 * Returns 0 on success.
 * Returns -1 if divisor is the zero polynomial (division by zero).
 *
 * Buffer sizing:
 *   quot_cap >= dividend_len
 *   rem_cap  >= divisor_len
 */
int poly_c_divmod(
    const double *dividend, size_t dividend_len,
    const double *divisor,  size_t divisor_len,
    double *quot_out, size_t quot_cap, size_t *quot_len_out,
    double *rem_out,  size_t rem_cap,  size_t *rem_len_out
);

/**
 * Return the quotient of dividend / divisor.
 *
 * Worst-case output length: dividend_len.
 * Returns the number of elements written, or 0 on error (zero divisor).
 *
 * Note: a return value of 0 is ambiguous (could be error or zero quotient).
 * Use poly_c_divmod for reliable error detection.
 */
size_t poly_c_divide(
    const double *a, size_t a_len,
    const double *b, size_t b_len,
    double *out, size_t out_cap
);

/**
 * Return the remainder of dividend / divisor.
 *
 * Worst-case output length: divisor_len.
 * Returns the number of elements written, or 0 on error (zero divisor).
 */
size_t poly_c_modulo(
    const double *a, size_t a_len,
    const double *b, size_t b_len,
    double *out, size_t out_cap
);

/* ── Greatest Common Divisor ───────────────────────────────────────────────── */

/**
 * Compute the GCD of two polynomials using the Euclidean algorithm.
 *
 * Worst-case output length: max(a_len, b_len).
 * Returns the number of elements written to `out`.
 */
size_t poly_c_gcd(
    const double *a, size_t a_len,
    const double *b, size_t b_len,
    double *out, size_t out_cap
);

#ifdef __cplusplus
}
#endif

#endif /* POLYNOMIAL_C_H */
