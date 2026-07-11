/*
 * bloom_filter.h — a Bloom filter for probabilistic membership tests, in pure
 * ISO C17. A faithful port of the Rust `bloom-filter` crate (DT22).
 * ===========================================================================
 *
 * A Bloom filter is a compact, probabilistic "set". It can tell you an element
 * is *definitely not* in the set, or *possibly* in it — it never gives a false
 * negative, but it may give a false positive. It stores nothing but a bit array
 * of `m` bits; each element flips `k` bits chosen by `k` hash functions.
 *
 *   add(x):       set the k bits hash_1(x)..hash_k(x)
 *   contains(x):  true iff ALL k of those bits are set
 *
 * Because two different elements can flip overlapping bits, contains() can
 * return true for an element never added (a false positive), with a probability
 * that grows as the filter fills. It can NEVER return false for an element that
 * was added.
 *
 * The k indices come from double hashing, index_i = h1 + i*h2 (mod m), where h1
 * and h2 are derived from two independent hashes (FNV-1a and djb2) run through a
 * finalising bit-mix (fmix32) — matching the Rust crate.
 *
 * Sizing: for `n` expected items and target false-positive rate `p`,
 *   m = ceil(-n ln p / (ln 2)^2)   and   k = round((m/n) ln 2).
 * ISO C has no natural-log in the language and the strict harness does not link
 * libm, so this port carries a small, self-contained `ln` (see the .c file).
 *
 * Memory: the bit array is heap-allocated; pair every successful init with
 * bloom_free. Portability: pure ISO C17 — GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors.
 */
#ifndef BLOOM_FILTER_H
#define BLOOM_FILTER_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* Status codes returned by the initialisers (mirroring BloomFilterError). */
typedef enum {
    BLOOM_OK = 0,
    BLOOM_INVALID_EXPECTED_ITEMS,
    BLOOM_INVALID_FALSE_POSITIVE_RATE,
    BLOOM_INVALID_BIT_COUNT,
    BLOOM_INVALID_HASH_COUNT,
    BLOOM_ALLOC_FAILED
} bloom_status;

typedef struct {
    size_t bit_count;      /* m: number of bits */
    size_t hash_count;     /* k: number of hash functions */
    size_t expected_items; /* n used for sizing (0 when built from raw params) */
    uint8_t *bits;         /* the bit array, (m+7)/8 bytes */
    size_t byte_count;     /* length of `bits` in bytes */
    size_t bits_set;       /* count of 1-bits currently set */
    size_t items_added;    /* number of add() calls */
} bloom_filter;

/* bloom_init — size a filter for `expected_items` and false-positive rate
 * `false_positive_rate` (in the open interval (0,1)). Computes optimal m and k,
 * allocates the bit array. Returns BLOOM_OK, or an error code (no allocation on
 * error). Pair a BLOOM_OK result with bloom_free. */
bloom_status bloom_init(bloom_filter *bf, size_t expected_items,
                        double false_positive_rate);

/* bloom_init_params — build a filter with explicit `bit_count` (m) and
 * `hash_count` (k), both > 0. `expected_items` is recorded as 0 (capacity
 * checks become no-ops). Returns BLOOM_OK or an error code. */
bloom_status bloom_init_params(bloom_filter *bf, size_t bit_count,
                               size_t hash_count);

/* bloom_free — release the bit array and zero the struct. Safe on a
 * zero-initialised or already-freed filter. */
void bloom_free(bloom_filter *bf);

/* bloom_add — insert the `len`-byte element `data`. */
void bloom_add(bloom_filter *bf, const void *data, size_t len);

/* bloom_contains — 1 if `data` is possibly present, 0 if definitely absent. */
int bloom_contains(const bloom_filter *bf, const void *data, size_t len);

/* Accessors. */
size_t bloom_bit_count(const bloom_filter *bf);
size_t bloom_hash_count(const bloom_filter *bf);
size_t bloom_bits_set(const bloom_filter *bf);
size_t bloom_size_bytes(const bloom_filter *bf);
double bloom_fill_ratio(const bloom_filter *bf);
double bloom_estimated_false_positive_rate(const bloom_filter *bf);
int bloom_is_over_capacity(const bloom_filter *bf);

/* Sizing helpers (also usable standalone). */
size_t bloom_optimal_m(size_t n, double p);
size_t bloom_optimal_k(size_t m, size_t n);
size_t bloom_capacity_for_memory(size_t memory_bytes, double p);

#endif /* BLOOM_FILTER_H */
