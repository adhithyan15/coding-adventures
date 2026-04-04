/*
 * gf256_c.h — C declarations for the Rust gf256-c static library.
 *
 * This header is the public contract between the Rust implementation and any
 * C, C++, or Swift caller. Import this header via a module map to use the
 * GF(2^8) arithmetic functions from Swift.
 *
 * GF(256) ARITHMETIC
 * ------------------
 * GF(2^8) is a finite field with 256 elements (bytes 0–255). All operations
 * are closed — inputs and outputs are always in [0, 255].
 *
 *   Add / Subtract : bitwise XOR (identical in characteristic-2 fields)
 *   Multiply       : log + antilog table lookup, O(1)
 *   Divide         : log subtraction + antilog lookup
 *   Power          : log scaling + antilog lookup
 *   Inverse        : ALOG[255 - LOG[a]]
 *
 * Primitive polynomial: x^8 + x^4 + x^3 + x^2 + 1 = 0x11D = 285
 *
 * ERROR HANDLING
 * --------------
 * Operations that are undefined (divide by zero, inverse of zero) return the
 * sentinel value 0xFF and set a per-thread error flag. Check it immediately
 * after the call with gf256_c_had_error():
 *
 *   uint8_t result = gf256_c_divide(42, 0);
 *   if (gf256_c_had_error()) { /* handle error */ }
 *
 * The error flag is cleared at the start of every gf256_c_* call.
 */

#ifndef GF256_C_H
#define GF256_C_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* ── Field Operations ──────────────────────────────────────────────────────── */

/**
 * Add two GF(256) elements.
 *
 * In GF(2^8), addition is bitwise XOR. No error cases.
 * gf256_c_add(0x53, 0xCA) == 0x99
 */
uint8_t gf256_c_add(uint8_t a, uint8_t b);

/**
 * Subtract two GF(256) elements.
 *
 * Identical to gf256_c_add in characteristic-2 fields (-1 == 1).
 * Provided separately for readability.
 */
uint8_t gf256_c_subtract(uint8_t a, uint8_t b);

/**
 * Multiply two GF(256) elements.
 *
 * Uses log/antilog tables: a * b = ALOG[(LOG[a] + LOG[b]) % 255].
 * Special case: 0 * anything = 0.
 */
uint8_t gf256_c_multiply(uint8_t a, uint8_t b);

/**
 * Divide a by b in GF(256).
 *
 * a / b = ALOG[(LOG[a] - LOG[b] + 255) % 255].
 * Special case: 0 / b = 0.
 *
 * ERROR: b == 0 is undefined. Returns 0xFF and sets the error flag.
 * Check with gf256_c_had_error() immediately after.
 */
uint8_t gf256_c_divide(uint8_t a, uint8_t b);

/**
 * Raise a GF(256) element to a non-negative integer power.
 *
 * base^exp = ALOG[(LOG[base] * exp) % 255].
 * Special cases: 0^0 = 1, 0^exp = 0 (exp > 0), base^0 = 1.
 */
uint8_t gf256_c_power(uint8_t base, uint32_t exp);

/**
 * Compute the multiplicative inverse of a GF(256) element.
 *
 * Returns a^-1 such that a * a^-1 = 1.
 *
 * ERROR: a == 0 has no inverse. Returns 0xFF and sets the error flag.
 * Check with gf256_c_had_error() immediately after.
 */
uint8_t gf256_c_inverse(uint8_t a);

/* ── Error Inspection ──────────────────────────────────────────────────────── */

/**
 * Return 1 if the most recent gf256_c_* call on this thread had an error.
 *
 * The flag is per-thread and is cleared at the start of every gf256_c_* call.
 * Must be called immediately after the operation being checked.
 */
uint8_t gf256_c_had_error(void);

/* ── Constants ─────────────────────────────────────────────────────────────── */

/**
 * Return the primitive polynomial used for GF(256).
 *
 * x^8 + x^4 + x^3 + x^2 + 1 = 285 = 0x11D.
 */
uint32_t gf256_c_primitive_polynomial(void);

#ifdef __cplusplus
}
#endif

#endif /* GF256_C_H */
