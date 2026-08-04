/*
 * wide_int.h — portable 128-bit integers, built from two 64-bit halves
 * (pure ISO C17).
 * ---------------------------------------------------------------------------
 *
 * Rust has native `u128`/`i128`; C does not. GCC and Clang offer `__int128` as
 * an *extension*, but MSVC has no 128-bit integer type at all, and `__int128`
 * is rejected under `-pedantic-errors`. So a *portable* 128-bit integer must be
 * synthesised from two `uint64_t` halves — which is exactly what this library
 * does, using nothing but standard 64-bit arithmetic. It is the substrate the
 * campaign's `u128`-using crates build on.
 *
 * Representation: a value is `hi * 2^64 + lo`. Signed values (`wi_i128`) use the
 * same two 64-bit words interpreted as two's complement — so add/sub/mul and the
 * bitwise ops are bit-identical to the unsigned versions; only division,
 * comparison, and shift-right differ by sign.
 *
 * Every operation is total and well-defined (no undefined shifts, no reliance on
 * `__int128`), so the same code runs identically under GCC, Clang, and MSVC.
 */
#ifndef WIDE_INT_H
#define WIDE_INT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* A 128-bit value = hi * 2^64 + lo. */
typedef struct {
    uint64_t hi;
    uint64_t lo;
} wi_u128;

/* Same layout, interpreted as a signed two's-complement 128-bit integer. */
typedef struct {
    uint64_t hi;
    uint64_t lo;
} wi_i128;

/* ------------------------------------------------------------------ */
/* Construction / accessors (unsigned)                                */
/* ------------------------------------------------------------------ */

wi_u128 wi_u128_make(uint64_t hi, uint64_t lo);
wi_u128 wi_u128_from_u64(uint64_t v);
wi_u128 wi_u128_zero(void);
wi_u128 wi_u128_max(void);   /* 2^128 - 1 */
uint64_t wi_u128_lo(wi_u128 a);
uint64_t wi_u128_hi(wi_u128 a);
/* Low 64 bits as u64 (truncating) — convenience for round-trips. */
uint64_t wi_u128_to_u64(wi_u128 a);

/* ------------------------------------------------------------------ */
/* Arithmetic (unsigned, all modulo 2^128)                            */
/* ------------------------------------------------------------------ */

wi_u128 wi_u128_add(wi_u128 a, wi_u128 b);
wi_u128 wi_u128_sub(wi_u128 a, wi_u128 b);
wi_u128 wi_u128_mul(wi_u128 a, wi_u128 b);

/* Widening 64x64 -> 128 multiply — the core primitive (exact, no wraparound). */
wi_u128 wi_mul_u64(uint64_t a, uint64_t b);

/* Unsigned division with remainder. Returns 0 on success (and fills *q, *r), or
 * 1 if `b` is zero (in which case *q and *r are left unchanged). */
int wi_u128_divmod(wi_u128 a, wi_u128 b, wi_u128 *q, wi_u128 *r);

/* ------------------------------------------------------------------ */
/* Bitwise / shifts (unsigned)                                        */
/* ------------------------------------------------------------------ */

wi_u128 wi_u128_and(wi_u128 a, wi_u128 b);
wi_u128 wi_u128_or(wi_u128 a, wi_u128 b);
wi_u128 wi_u128_xor(wi_u128 a, wi_u128 b);
wi_u128 wi_u128_not(wi_u128 a);
/* Shifts by `n` bits; n >= 128 yields 0. shr is logical (zero-fill). */
wi_u128 wi_u128_shl(wi_u128 a, unsigned n);
wi_u128 wi_u128_shr(wi_u128 a, unsigned n);

/* ------------------------------------------------------------------ */
/* Comparison (unsigned)                                              */
/* ------------------------------------------------------------------ */

int wi_u128_cmp(wi_u128 a, wi_u128 b); /* -1, 0, or 1 */
int wi_u128_eq(wi_u128 a, wi_u128 b);
int wi_u128_is_zero(wi_u128 a);

/* ------------------------------------------------------------------ */
/* Formatting (unsigned)                                              */
/* ------------------------------------------------------------------ */

/* Write the decimal representation into `buf` (needs 40 bytes: up to 39 digits
 * + NUL). Returns the string length (excluding the NUL). */
size_t wi_u128_to_dec(wi_u128 a, char *buf);
/* Write a lowercase 32-hex-digit (zero-padded) representation into `buf` (needs
 * 33 bytes). Returns 32. */
size_t wi_u128_to_hex(wi_u128 a, char *buf);

/* ------------------------------------------------------------------ */
/* Signed (wi_i128, two's complement)                                 */
/* ------------------------------------------------------------------ */

wi_i128 wi_i128_from_i64(int64_t v);
wi_i128 wi_i128_make(uint64_t hi, uint64_t lo);
wi_i128 wi_i128_zero(void);
int wi_i128_is_negative(wi_i128 a);

/* Reinterpret between the signed and unsigned views (same bits). */
wi_u128 wi_i128_bits(wi_i128 a);
wi_i128 wi_u128_as_i128(wi_u128 a);

wi_i128 wi_i128_add(wi_i128 a, wi_i128 b);
wi_i128 wi_i128_sub(wi_i128 a, wi_i128 b);
wi_i128 wi_i128_mul(wi_i128 a, wi_i128 b);
wi_i128 wi_i128_neg(wi_i128 a);

/* Signed division truncating toward zero (C semantics). Returns 0 on success,
 * 1 on division by zero. Remainder takes the sign of the dividend. */
int wi_i128_divmod(wi_i128 a, wi_i128 b, wi_i128 *q, wi_i128 *r);

int wi_i128_cmp(wi_i128 a, wi_i128 b); /* -1, 0, or 1 (signed) */
int wi_i128_eq(wi_i128 a, wi_i128 b);
/* Arithmetic shift right (sign-extending); n >= 128 yields all-sign. */
wi_i128 wi_i128_sar(wi_i128 a, unsigned n);

/* Decimal, with a leading '-' for negatives (needs 41 bytes). */
size_t wi_i128_to_dec(wi_i128 a, char *buf);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* WIDE_INT_H */
