/*
 * hyperloglog/hyperloglog.h — approximate distinct-count (cardinality) estimator.
 * ===========================================================================
 *
 * The C port of the Rust `hyperloglog` crate (DT21), and the third bucket-A port
 * of the CCPP02 campaign: a pure-ISO crate that needs no OS, so it rides the
 * `iso-harness` (links nothing, `-pedantic-errors` / `/permissive-`).
 *
 * WHAT IT DOES. HyperLogLog answers "roughly how many *distinct* items have I
 * seen?" using a tiny fixed amount of memory, however many items stream past. It
 * never stores the items. Each item is hashed; the leading run of zero bits in
 * the hash is a cheap proxy for rarity, and the maximum such run seen per bucket,
 * combined across buckets with a harmonic mean, estimates the cardinality. The
 * accuracy/memory trade-off is set once by the *precision* p (4..16): there are
 * 2^p one-byte registers, and the relative error is about 1.04 / sqrt(2^p).
 *
 * PURE, BUT NOT ALONE. This crate is pure-ISO yet composes two other pure-ISO
 * packages rather than re-deriving them: the 64-bit FNV-1a hash from
 * `c/hash-functions` and the from-scratch elementary math (ln / sqrt / log2 /
 * ceil / round) from `c/float-math`. Nothing here links a math library — the
 * math is computed from scratch, honouring the lane's no-libm rule.
 *
 * OWNERSHIP. A `hll` owns its register array; release every sketch you create
 * (via `hll_create*` or `hll_merge`) with `hll_destroy` (safe on NULL).
 */
#ifndef HYPERLOGLOG_HYPERLOGLOG_H
#define HYPERLOGLOG_HYPERLOGLOG_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

#ifdef __cplusplus
extern "C" {
#endif

/* Precision bounds (register count is 2^precision). */
#define HLL_MIN_PRECISION 4
#define HLL_MAX_PRECISION 16
#define HLL_DEFAULT_PRECISION 14

/* Every result a sketch operation can produce. */
typedef enum {
    HLL_OK = 0,
    HLL_ERR_INVALID_PRECISION, /* precision outside [4, 16] */
    HLL_ERR_PRECISION_MISMATCH, /* merge of two different precisions */
    HLL_ERR_NOMEM,
    HLL_ERR_INVALID /* NULL out-parameter, etc. */
} hll_status;

/* Opaque sketch. */
typedef struct hll hll;

/* Create a sketch with the given precision (HLL_ERR_INVALID_PRECISION outside
 * [4, 16]) / the default precision 14. HLL_ERR_NOMEM / HLL_ERR_INVALID. */
hll_status hll_create(uint8_t precision, hll **out);
hll_status hll_create_default(hll **out);

/* Release a sketch (safe on NULL). */
void hll_destroy(hll *h);

/* Observe one element (a byte range, or a NUL-terminated string). Idempotent for
 * a repeated identical element. HLL_ERR_INVALID on NULL args (bytes may be NULL
 * only when len == 0). */
hll_status hll_add_bytes(hll *h, const unsigned char *bytes, size_t len);
hll_status hll_add_str(hll *h, const char *s);

/*
 * Estimate the number of distinct elements observed. Combines the registers with
 * a harmonic mean, applies linear-counting for small cardinalities and the
 * large-range correction near 2^32, and rounds to a non-negative integer.
 * Returns 0 for a NULL sketch. (No allocation — infallible.)
 */
size_t hll_count(const hll *h);

/* True when the estimate is zero (NULL sketch → true). */
int hll_is_empty(const hll *h);

/*
 * Merge two sketches (the register-wise maximum — a union of what each has seen)
 * into a fresh sketch. Both must share a precision, else HLL_ERR_PRECISION_MISMATCH.
 * HLL_ERR_NOMEM / HLL_ERR_INVALID. The result is a new sketch the caller owns.
 */
hll_status hll_merge(const hll *a, const hll *b, hll **out);

/* The configured precision / register count (0 for a NULL sketch). */
uint8_t hll_precision(const hll *h);
size_t hll_num_registers(const hll *h);

/* Expected relative error for this sketch / for a given precision (1.04/sqrt(m)).
 * A precision outside [4, 16] is clamped into range. */
double hll_error_rate(const hll *h);
double hll_error_rate_for_precision(uint8_t precision);

/* Register memory for a precision, in bytes: (2^precision * 6) / 8. A precision
 * outside [4, 16] is clamped into range. */
size_t hll_memory_bytes(uint8_t precision);

/* The smallest precision (clamped to [4, 16]) whose expected error is within
 * `desired_error`. A non-positive or non-finite target yields HLL_MAX_PRECISION. */
uint8_t hll_optimal_precision(double desired_error);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* HYPERLOGLOG_HYPERLOGLOG_H */
