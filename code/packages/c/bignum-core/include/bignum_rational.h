/*
 * bignum_rational.h — an exact fraction (BigRational), built on BigInteger, in
 * pure ISO C17. A faithful port of the `rational` module of the Rust
 * `bignum-core` crate.
 * ===========================================================================
 *
 * WHAT IT IS. A BigRational is an EXACT rational number: an arbitrary-precision
 * integer numerator over an arbitrary-precision integer denominator, always in
 * canonical form —
 *   - lowest terms (numerator and denominator divided through by their gcd),
 *   - the sign carried on the numerator (the denominator is always positive),
 *   - and every representation of zero collapsed to 0/1.
 * So two rationals are equal iff their (numerator, denominator) pairs match, and
 * `rat_cmp` is a genuine total order.
 *
 * WHY. `double` cannot represent 1/3, and `0.1 + 0.2` is not `0.3`. A
 * BigRational holds `1/3` and `3/10` exactly; arithmetic never rounds.
 *
 * OWNERSHIP. Every constructor and operation returns a NEW heap `BigRational *`
 * the caller releases with `rat_free`. Infallible constructors return NULL on
 * allocation failure; fallible operations return a `RatStatus` and write the
 * result through an out-parameter (untouched on error). `rat_to_string` returns
 * a malloc'd C string the caller frees.
 *
 * DIVERGENCE FROM RUST. Where the Rust `new`/`div`/`recip` PANIC (zero
 * denominator, divide by zero), this C port returns a status code — a library
 * must not abort its host. `rat_to_f64` narrows to the nearest `double`; the
 * Rust crate routes this through its `BigDouble` float rung, whereas this port
 * (which does not yet include that rung) computes the same correctly-rounded
 * result through the exact base-10 `BigDecimal` division and `strtod`. It is
 * correct for every rational of practical magnitude and saturates to ±inf / 0
 * beyond `double`'s range, exactly as the Rust method documents.
 *
 * PORTABILITY. Pure ISO C17 — no `__int128`, no `<math.h>`/libm. Builds clean
 * under GCC, Clang, and MSVC with -pedantic-errors / /permissive- and
 * warnings-as-errors.
 */
#ifndef CA_BIGNUM_RATIONAL_H
#define CA_BIGNUM_RATIONAL_H

#include <stddef.h>
#include <stdint.h>

#include "bignum_core.h"

#ifdef __cplusplus
extern "C" {
#endif

/* An exact fraction. Opaque: the canonical-form invariant (lowest terms,
 * positive denominator, 0 == 0/1) cannot be broken from outside. Read the parts
 * with `rat_numerator` / `rat_denominator`. */
typedef struct BigRational BigRational;

/* Status of a fallible operation. */
typedef enum {
    RAT_OK = 0,
    RAT_ERR_NOMEM,             /* allocation failed */
    RAT_ERR_ZERO_DENOMINATOR,  /* a denominator was zero (n/0 is not a number) */
    RAT_ERR_DIV_BY_ZERO,       /* divide, or reciprocal, by the zero rational */
    RAT_ERR_POW_TOO_LARGE      /* try_pow projection exceeds the bit ceiling */
} RatStatus;

/* Status of a parse. */
typedef enum {
    RAT_PARSE_OK = 0,
    RAT_PARSE_EMPTY,             /* empty numerator or denominator */
    RAT_PARSE_TOO_MANY_SLASHES,  /* more than one '/' */
    RAT_PARSE_INVALID_INTEGER,   /* a part was not a base-10 integer */
    RAT_PARSE_ZERO_DENOMINATOR,  /* the denominator parsed to zero */
    RAT_PARSE_NOMEM              /* allocation failed while parsing */
} RatParseStatus;

/* ---- construction (infallible: NULL on OOM) --------------------------- */
BigRational *rat_zero(void); /* 0/1 */
BigRational *rat_one(void);  /* 1/1 */
BigRational *rat_from_i64(int64_t n);
/* Promote a whole BigInteger to n/1. Clones `n`. */
BigRational *rat_from_integer(const BigInteger *n);
/* Build num/den from two primitives, canonicalized. Returns NULL if `den` is
 * zero (the Rust `from_ints` panic) or on OOM. */
BigRational *rat_from_ints(int64_t num, int64_t den);
BigRational *rat_clone(const BigRational *a);
void rat_free(BigRational *a);

/* Build num/den, reduce to canonical form. `num` and `den` are cloned. Returns
 * RAT_ERR_ZERO_DENOMINATOR if `den` is zero (Rust's `checked_new`). */
RatStatus rat_new(const BigInteger *num, const BigInteger *den,
                  BigRational **out);

/* ---- accessors (borrowed; valid until the rational is freed) ---------- */
const BigInteger *rat_numerator(const BigRational *a);   /* carries the sign */
const BigInteger *rat_denominator(const BigRational *a); /* always > 0 */

/* ---- predicates & sign ------------------------------------------------ */
int rat_is_zero(const BigRational *a);
int rat_is_integer(const BigRational *a); /* denominator == 1 */
int rat_is_negative(const BigRational *a);
int rat_is_positive(const BigRational *a);
int rat_signum(const BigRational *a); /* -1, 0, +1 */
BigRational *rat_abs(const BigRational *a);
/* The reciprocal 1/a. Returns RAT_ERR_DIV_BY_ZERO if `a` is zero. */
RatStatus rat_recip(const BigRational *a, BigRational **out);

/* ---- exact arithmetic ------------------------------------------------- */
RatStatus rat_add(const BigRational *a, const BigRational *b, BigRational **out);
RatStatus rat_sub(const BigRational *a, const BigRational *b, BigRational **out);
RatStatus rat_mul(const BigRational *a, const BigRational *b, BigRational **out);
/* a / b. Returns RAT_ERR_DIV_BY_ZERO if `b` is zero. */
RatStatus rat_div(const BigRational *a, const BigRational *b, BigRational **out);
/* Raise to an integer power (a negative exponent takes the reciprocal). Returns
 * RAT_ERR_ZERO_DENOMINATOR on a negative power of zero (that is 1/0). UNBOUNDED
 * in the result size — use `rat_try_pow` for an untrusted exponent. */
RatStatus rat_pow(const BigRational *a, int32_t exp, BigRational **out);
/* DoS-safe pow: refuses up front (RAT_ERR_POW_TOO_LARGE) if either the
 * numerator or denominator of the result would exceed `max_bits` bits. */
RatStatus rat_try_pow(const BigRational *a, int32_t exp, uint64_t max_bits,
                      BigRational **out);

/* ---- ordering --------------------------------------------------------- */
/* Three-way compare via cross-multiplication: writes -1 / 0 / +1 through
 * `cmp_out`. Fallible because cross-multiplication allocates. */
RatStatus rat_cmp(const BigRational *a, const BigRational *b, int *cmp_out);

/* ---- formatting, parsing, lossy export -------------------------------- */
/* "numerator/denominator", or just "numerator" for a whole number. Malloc'd;
 * NULL on OOM. */
char *rat_to_string(const BigRational *a);
/* Parse "num/den" or a bare integer "num" (base 10); a bare integer n → n/1.
 * Whitespace is not trimmed. */
RatParseStatus rat_parse(const char *s, BigRational **out);
/* A lossy narrowing to the nearest double (round-half-even). Values beyond
 * double's range saturate to ±inf; tiny ones to 0. */
double rat_to_f64(const BigRational *a);

#ifdef __cplusplus
}
#endif

#endif /* CA_BIGNUM_RATIONAL_H */
