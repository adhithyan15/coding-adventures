/*
 * bignum_decimal.h — an exact base-10 number (BigDecimal), built on BigInteger,
 * in pure ISO C17. A faithful port of the `decimal` module of the Rust
 * `bignum-core` crate.
 * ===========================================================================
 *
 * WHAT IT IS. A BigDecimal is a value `mantissa × 10^(-scale)`: an arbitrary-
 * precision integer mantissa (a BigInteger, carrying the sign) scaled by a
 * power of ten. `123.45` is `(mantissa 12345, scale 2)`; `100` is `(mantissa 1,
 * scale -2)`. Everything is held in CANONICAL FORM — the mantissa never ends in
 * a `0` digit (unless the value is exactly zero, which is pinned to `(0, 0)`) —
 * so two decimals are equal iff their (mantissa, scale) pairs match, and
 * `dec_cmp` is a genuine total order.
 *
 * WHY. `f64` cannot represent `0.1` exactly; money, tax, and dosing need exact
 * base-10 arithmetic. `+ − ×` here are EXACT (they never round); only division
 * — the one base-10 operation that need not terminate (`10/3`) — rounds, and
 * only then do you say to how many places (`target_scale`) and how (a
 * `DecRoundingMode`). `dec_to_f64` is the single, clearly-labelled lossy exit.
 *
 * OWNERSHIP. Every constructor and operation returns a NEW heap `BigDecimal *`
 * that the caller releases with `dec_free`. Infallible constructors return NULL
 * on allocation failure; fallible operations return a `DecStatus` and write the
 * result through an out-parameter (left untouched on error). `dec_to_string`
 * returns a malloc'd C string the caller frees.
 *
 * DIVERGENCE FROM RUST. Where the Rust `from_parts`/`div_round` PANIC (scale
 * past the internal ceiling; division by zero), this C port instead returns a
 * status code — C has no unwinding, and a library must not abort the host. The
 * arithmetic is otherwise byte-for-byte identical.
 *
 * PORTABILITY. Pure ISO C17 — no `__int128`, no `<math.h>`/libm (float export
 * goes through `strtod`), no compiler extensions. Builds clean under GCC,
 * Clang, and MSVC with -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef CA_BIGNUM_DECIMAL_H
#define CA_BIGNUM_DECIMAL_H

#include <stddef.h>
#include <stdint.h>

#include "bignum_core.h"

#ifdef __cplusplus
extern "C" {
#endif

/* An exact base-10 number. Opaque: the (mantissa, scale) canonical-form
 * invariant cannot be broken from outside. Read the parts with `dec_mantissa`
 * and `dec_scale`. */
typedef struct BigDecimal BigDecimal;

/* Status of a fallible operation. */
typedef enum {
    DEC_OK = 0,
    DEC_ERR_NOMEM,          /* allocation failed */
    DEC_ERR_DIV_BY_ZERO,    /* divisor was exactly zero */
    DEC_ERR_SCALE_OVERFLOW  /* canonical scale magnitude exceeds the internal
                             * ceiling, or a materialized power of ten would
                             * exceed ~4 billion digits (the Rust `from_parts`
                             * panic, returned as a status here) */
} DecStatus;

/* How to round when a value cannot be represented exactly at the requested
 * scale. The `HALF_*` modes decide only the exact-halfway (`…5`) case; away
 * from it they all round to the nearest representable value. */
typedef enum {
    DEC_ROUND_DOWN,      /* toward zero (truncate):        2.5→2, -2.5→-2 */
    DEC_ROUND_UP,        /* away from zero:                2.1→3, -2.1→-3 */
    DEC_ROUND_FLOOR,     /* toward -infinity:              2.5→2, -2.5→-3 */
    DEC_ROUND_CEILING,   /* toward +infinity:              2.5→3, -2.5→-2 */
    DEC_ROUND_HALF_UP,   /* nearest, ties away from zero:  2.5→3, -2.5→-3 */
    DEC_ROUND_HALF_DOWN, /* nearest, ties toward zero:     2.5→2, -2.5→-2 */
    DEC_ROUND_HALF_EVEN  /* nearest, ties to even ("banker's"): 2.5→2, 1.5→2 */
} DecRoundingMode;

/* Status of a parse. */
typedef enum {
    DEC_PARSE_OK = 0,
    DEC_PARSE_EMPTY,             /* empty, or no digits where digits were required */
    DEC_PARSE_INVALID_DIGIT,     /* a character that is not part of a decimal literal */
    DEC_PARSE_MALFORMED_SHAPE,   /* more than one '.', or more than one 'e'/'E' */
    DEC_PARSE_EXPONENT_OVERFLOW, /* exponent (or resulting scale) out of range */
    DEC_PARSE_NOMEM              /* allocation failed while parsing */
} DecParseStatus;

/* The largest scale magnitude accepted from UNTRUSTED input (`dec_parse`). A
 * security budget, not a precision limit: a value is `mantissa × 10^(-scale)`,
 * and aligning/rendering must materialize `10^(scale gap)`. Bounding the parsed
 * scale keeps any such power well under a megabyte, so a few-byte string like
 * "1e-2000000000" cannot force a multi-gigabyte allocation in a later
 * `+`/`cmp`/`to_string` (none of which can report an error). */
#define DEC_MAX_SCALE ((int64_t)1000000)

/* The hard ceiling every constructor (hence every arithmetic result) enforces.
 * Deliberately WIDER than DEC_MAX_SCALE so that `+ − ×` of two parse-budget
 * operands — whose result scale can reach `2·MAX_SCALE` — never trips it; only
 * a pathological explicit `dec_from_parts` or a long scale-growing chain can. */
#define DEC_INTERNAL_SCALE_LIMIT ((int64_t)8000000)

/* ---- construction (infallible: NULL on OOM) --------------------------- */
BigDecimal *dec_zero(void);
BigDecimal *dec_one(void);
BigDecimal *dec_from_i64(int64_t n);
/* Promote a whole BigInteger to a decimal (scale 0). Clones `n`. */
BigDecimal *dec_from_integer(const BigInteger *n);
BigDecimal *dec_clone(const BigDecimal *a);
void dec_free(BigDecimal *a);

/* Build `mant × 10^(-scale)`, reduce to canonical form, and enforce the
 * internal ceiling. `mant` is cloned. Returns DEC_ERR_SCALE_OVERFLOW if the
 * canonical scale magnitude exceeds DEC_INTERNAL_SCALE_LIMIT (this is the
 * non-panicking analogue of Rust's `checked_from_parts`). */
DecStatus dec_from_parts(const BigInteger *mant, int64_t scale, BigDecimal **out);

/* ---- accessors (borrowed; valid until the decimal is freed) ----------- */
const BigInteger *dec_mantissa(const BigDecimal *a);
int64_t dec_scale(const BigDecimal *a);

/* ---- predicates & sign ------------------------------------------------ */
int dec_is_zero(const BigDecimal *a);
int dec_is_negative(const BigDecimal *a);
int dec_is_positive(const BigDecimal *a);
int dec_signum(const BigDecimal *a); /* -1, 0, +1 */
BigDecimal *dec_abs(const BigDecimal *a);
BigDecimal *dec_neg(const BigDecimal *a);

/* ---- exact arithmetic (+, -, *, ^) and rounding division -------------- */
DecStatus dec_add(const BigDecimal *a, const BigDecimal *b, BigDecimal **out);
DecStatus dec_sub(const BigDecimal *a, const BigDecimal *b, BigDecimal **out);
DecStatus dec_mul(const BigDecimal *a, const BigDecimal *b, BigDecimal **out);
/* Raise to a non-negative integer power (exact). */
DecStatus dec_pow(const BigDecimal *a, uint32_t exp, BigDecimal **out);
/* Divide, rounding the result to exactly `target_scale` places with `mode`.
 * Returns DEC_ERR_DIV_BY_ZERO if `b` is zero. */
DecStatus dec_div_round(const BigDecimal *a, const BigDecimal *b,
                        int64_t target_scale, DecRoundingMode mode,
                        BigDecimal **out);
/* Round to `target_scale` places with `mode`. Increasing the scale is exact. */
DecStatus dec_round_to_scale(const BigDecimal *a, int64_t target_scale,
                             DecRoundingMode mode, BigDecimal **out);

/* ---- ordering --------------------------------------------------------- */
/* Three-way compare: writes -1 (a<b) / 0 / +1 (a>b) through `cmp_out`.
 * Fallible because comparison re-expresses both mantissas at a common scale,
 * which materializes a power of ten and can run out of memory. */
DecStatus dec_cmp(const BigDecimal *a, const BigDecimal *b, int *cmp_out);

/* ---- formatting, parsing, lossy export -------------------------------- */
/* Plain decimal notation, never scientific ("100", "1.23", "0.001", "-0.5",
 * "0"). Malloc'd; NULL on OOM. */
char *dec_to_string(const BigDecimal *a);
/* Parse plain ("123.45", "-0.001", "42") and scientific ("1.5e-3", "6.022E23")
 * notation. Enforces the DEC_MAX_SCALE budget on the stored scale. */
DecParseStatus dec_parse(const char *s, BigDecimal **out);
/* A lossy narrowing to the nearest double (through the value's own decimal
 * string and the correctly-rounded `strtod`). Out-of-range magnitudes saturate
 * to ±inf / 0 exactly as `strtod` does; returns a NaN if the intermediate
 * string cannot be allocated. */
double dec_to_f64(const BigDecimal *a);

#ifdef __cplusplus
}
#endif

#endif /* CA_BIGNUM_DECIMAL_H */
