/*
 * gf256.h — Galois Field GF(2^8) arithmetic, in pure ISO C17. A faithful port of
 * the Rust `gf256` crate.
 * ===========================================================================
 *
 * GF(2^8) ("GF of 256") is the finite field of 256 elements — the bytes 0..255,
 * but with field arithmetic. Each byte is a degree-<=7 polynomial over GF(2);
 * addition is XOR (characteristic 2, so subtraction is the same), and
 * multiplication reduces the product modulo an irreducible degree-8 polynomial.
 * This field underlies Reed-Solomon codes, QR codes, and AES.
 *
 * Two interfaces, matching the crate:
 *   - Module-level functions fixed to the Reed-Solomon polynomial 0x11D, using
 *     precomputed log/antilog tables (built once on first use).
 *   - `gf256_field` — a field parameterised by any primitive polynomial (e.g.
 *     AES's 0x11B), using table-free Russian-peasant multiplication.
 *
 * Degenerate cases (division by zero, inverse of zero) return 0 here (the Rust
 * crate panics); callers should avoid them.
 *
 * Note: the lazy table build is not thread-safe (the Rust crate uses OnceLock);
 * this port targets single-threaded use, as the pure-ISO harness has no
 * portable threading primitive.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef GF256_H
#define GF256_H

#include <stdint.h> /* uint8_t, uint16_t, uint32_t */

/* The additive and multiplicative identities. */
#define GF256_ZERO ((uint8_t)0)
#define GF256_ONE ((uint8_t)1)

/* The default primitive polynomial x^8 + x^4 + x^3 + x^2 + 1 (Reed-Solomon). */
#define GF256_PRIMITIVE_POLYNOMIAL ((uint16_t)0x11d)

/* ── module-level operations (default field, polynomial 0x11D) ────────────── */
uint8_t gf256_add(uint8_t a, uint8_t b);      /* a XOR b */
uint8_t gf256_subtract(uint8_t a, uint8_t b); /* a XOR b (same as add) */
uint8_t gf256_multiply(uint8_t a, uint8_t b);
uint8_t gf256_divide(uint8_t a, uint8_t b); /* 0 if b == 0 */
uint8_t gf256_power(uint8_t base, uint32_t exp);
uint8_t gf256_inverse(uint8_t a); /* 0 if a == 0 */

/* ── parameterisable field (any primitive polynomial) ─────────────────────── */
typedef struct {
    uint16_t primitive_polynomial;
    uint8_t reduce; /* low byte of the polynomial, the reduction constant */
} gf256_field;

/* gf256_field_new — build a field for `primitive_poly` (e.g. 0x11B for AES). */
gf256_field gf256_field_new(uint16_t primitive_poly);

uint8_t gf256_field_add(const gf256_field *f, uint8_t a, uint8_t b);
uint8_t gf256_field_subtract(const gf256_field *f, uint8_t a, uint8_t b);
uint8_t gf256_field_multiply(const gf256_field *f, uint8_t a, uint8_t b);
uint8_t gf256_field_divide(const gf256_field *f, uint8_t a, uint8_t b);
uint8_t gf256_field_power(const gf256_field *f, uint8_t base, uint32_t exp);
uint8_t gf256_field_inverse(const gf256_field *f, uint8_t a);

#endif /* GF256_H */
