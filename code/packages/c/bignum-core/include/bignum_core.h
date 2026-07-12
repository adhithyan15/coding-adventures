/*
 * bignum_core.h — arbitrary-precision signed integers (BigInteger), in pure ISO
 * C17. A faithful port of the `BigInteger` core of the Rust `bignum-core` crate.
 * ===========================================================================
 *
 * A `BigInteger` is a sign-magnitude arbitrary-precision integer: a sign
 * (-1 / 0 / +1) plus a magnitude stored as little-endian base-2^32 limbs with
 * no trailing zero limb (so zero is the empty magnitude — there is never a
 * "-0"). All arithmetic is done with 32-bit limbs and a 64-bit accumulator, so
 * the code needs no 128-bit integers.
 *
 * The algorithms are the grade-school ones: column add/subtract, schoolbook
 * O(n·m) multiply, and — for division — Knuth's Algorithm D (TAOCP §4.3.1),
 * long division generalized from base 10 to base 2^32. Division truncates
 * toward zero and the remainder takes the dividend's sign, matching C integer
 * `/` and `%`.
 *
 * OWNERSHIP. Every operation returns a NEW heap `BigInteger *` that the caller
 * frees with `bigint_free`. Functions return NULL on allocation failure.
 * `bigint_to_str_radix` returns a malloc'd string the caller frees.
 *
 * This ports the integer core; the crate's decimal / float rungs build on it.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef BIGNUM_CORE_H
#define BIGNUM_CORE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint32_t, uint64_t, int64_t */

/* An arbitrary-precision signed integer (opaque; heap-allocated). */
typedef struct BigInteger BigInteger;

/* Status codes for the fallible operations. */
typedef enum {
    BIGINT_OK = 0,
    BIGINT_ALLOC_ERROR,         /* out of memory */
    BIGINT_DIV_BY_ZERO,         /* division / remainder by zero */
    BIGINT_POW_TOO_LARGE,       /* try_pow projection exceeds the ceiling */
    BIGINT_PARSE_EMPTY,         /* empty string, or a lone sign */
    BIGINT_PARSE_INVALID_DIGIT, /* a character was not a digit in the radix */
    BIGINT_PARSE_INVALID_RADIX  /* radix outside 2..=36 */
} BigIntStatus;

/* ---- construction / destruction --------------------------------------- */

BigInteger *bigint_zero(void);
BigInteger *bigint_one(void);
BigInteger *bigint_from_i64(int64_t value);
BigInteger *bigint_from_u64(uint64_t value);
BigInteger *bigint_clone(const BigInteger *a);
void bigint_free(BigInteger *a);

/* ---- queries ---------------------------------------------------------- */

int bigint_is_zero(const BigInteger *a);
int bigint_is_negative(const BigInteger *a);
int bigint_is_positive(const BigInteger *a);
int bigint_signum(const BigInteger *a); /* -1, 0, +1 */
size_t bigint_num_limbs(const BigInteger *a);
uint64_t bigint_bit_len(const BigInteger *a);

/* bigint_cmp — three-way compare: -1 if a<b, 0 if a==b, +1 if a>b. */
int bigint_cmp(const BigInteger *a, const BigInteger *b);

/* ---- sign transforms -------------------------------------------------- */

BigInteger *bigint_abs(const BigInteger *a);
BigInteger *bigint_neg(const BigInteger *a);

/* ---- arithmetic (return a new BigInteger, NULL on OOM) ---------------- */

BigInteger *bigint_add(const BigInteger *a, const BigInteger *b);
BigInteger *bigint_sub(const BigInteger *a, const BigInteger *b);
BigInteger *bigint_mul(const BigInteger *a, const BigInteger *b);

/* bigint_div_rem — truncating division. Writes fresh quotient/remainder to
 * *q_out and *r_out and returns BIGINT_OK; returns BIGINT_DIV_BY_ZERO (writing
 * nothing) if `b` is zero, or BIGINT_ALLOC_ERROR on OOM. Either out-pointer may
 * be NULL to discard that half. */
BigIntStatus bigint_div_rem(const BigInteger *a, const BigInteger *b,
                            BigInteger **q_out, BigInteger **r_out);
BigIntStatus bigint_div(const BigInteger *a, const BigInteger *b,
                        BigInteger **out);
BigIntStatus bigint_rem(const BigInteger *a, const BigInteger *b,
                        BigInteger **out);

/* bigint_pow — `a` raised to `exp` by exponentiation-by-squaring. NULL on OOM.
 * WARNING: the result grows linearly in `exp`; use bigint_try_pow for an
 * untrusted exponent. */
BigInteger *bigint_pow(const BigInteger *a, uint32_t exp);

/* bigint_try_pow — pow with an O(1) up-front size guard. If the projected bit
 * length (bit_len(a)*exp) exceeds `max_bits`, returns BIGINT_POW_TOO_LARGE
 * (writing the projection to *projected_out if non-NULL) without allocating.
 * Otherwise writes the result to *out and returns BIGINT_OK (or BIGINT_ALLOC_
 * ERROR). */
BigIntStatus bigint_try_pow(const BigInteger *a, uint32_t exp, uint64_t max_bits,
                            BigInteger **out, uint64_t *projected_out);

/* bigint_gcd — the non-negative greatest common divisor (Euclid). NULL on OOM. */
BigInteger *bigint_gcd(const BigInteger *a, const BigInteger *b);

/* ---- parsing / formatting --------------------------------------------- */

/* bigint_parse_radix — parse `s` in `radix` (2..=36), optional leading +/-.
 * Writes the result to *out and returns BIGINT_OK, or a BIGINT_PARSE_* /
 * BIGINT_ALLOC_ERROR status. If `bad_char_out` is non-NULL it receives the
 * offending character on BIGINT_PARSE_INVALID_DIGIT. */
BigIntStatus bigint_parse_radix(const char *s, uint32_t radix, BigInteger **out,
                                char *bad_char_out);

/* bigint_to_str_radix — render in `radix` (2..=36), lowercase, leading '-' for
 * negatives; "0" for zero. malloc'd, caller frees. NULL on bad radix or OOM. */
char *bigint_to_str_radix(const BigInteger *a, uint32_t radix);

/* bigint_to_string — base-10 rendering (malloc'd, caller frees). */
char *bigint_to_string(const BigInteger *a);

#endif /* BIGNUM_CORE_H */
