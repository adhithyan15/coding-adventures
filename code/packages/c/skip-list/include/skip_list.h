/*
 * skip_list.h — an ordered map of int keys to int values, in pure ISO C17. A
 * faithful port of the Rust `skip-list` crate.
 * ===========================================================================
 *
 * NOTE ON THE NAME: the Rust `skip-list` crate is, internally, an ordered map
 * (a balanced tree) that merely REPORTS skip-list-style parameters (max_level,
 * probability, a derived current_max "height"). This C port matches that
 * behavior — it is an ordered key→value map with the same observable API — and
 * keeps the parameters as reported metadata. current_max is derived as
 * ceil(log_base(1/probability)(len)) clamped to [1, max_level], computed without
 * <math.h> so the pure-ISO build needs no libm.
 *
 * Operations: insert/delete/search/contains (O(log n) lookup), order statistics
 * (rank, by_rank), min/max, ordered enumeration, and range queries — all with
 * keys kept in ascending order.
 *
 * The map owns a heap allocation — pair skiplist_init with skiplist_free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef SKIP_LIST_H
#define SKIP_LIST_H

#include <stddef.h> /* size_t */

typedef struct {
    int key;
    int value;
} skiplist_entry;

typedef struct {
    skiplist_entry *entries; /* kept sorted by key */
    size_t size;
    size_t cap;
    size_t max_level;
    double probability;
    size_t current_max; /* reported "height"; cosmetic, derived from size */
} skiplist;

/* skiplist_init — empty map with default parameters (max_level 32, p 0.5). */
int skiplist_init(skiplist *s);

/* skiplist_init_with_params — as above with a chosen max_level and probability
 * (probability is clamped to (0,1); non-finite or out-of-range → 0.5;
 * max_level is at least 1). These affect only the reported current_max. */
int skiplist_init_with_params(skiplist *s, size_t max_level, double probability);

/* skiplist_free — release storage. Safe on a zeroed struct; idempotent. */
void skiplist_free(skiplist *s);

/* skiplist_insert — insert or overwrite key→value. Returns 1, or 0 on
 * allocation failure. */
int skiplist_insert(skiplist *s, int key, int value);

/* skiplist_delete — remove key. Returns 1 if it was present, else 0. */
int skiplist_delete(skiplist *s, int key);

/* skiplist_search — if key is present, write its value to *out, return 1. */
int skiplist_search(const skiplist *s, int key, int *out);

/* skiplist_contains — 1 if key is present, else 0. */
int skiplist_contains(const skiplist *s, int key);

/* skiplist_rank — 0-based position of key in ascending order (written to
 * *out_rank). Returns 1 if present, else 0. */
int skiplist_rank(const skiplist *s, int key, size_t *out_rank);

/* skiplist_by_rank — key at 0-based position `rank` (written to *out_key).
 * Returns 1 if rank < len, else 0. */
int skiplist_by_rank(const skiplist *s, size_t rank, int *out_key);

/* skiplist_min / skiplist_max — smallest / largest key (to *out). Returns 1, or
 * 0 if the map is empty. */
int skiplist_min(const skiplist *s, int *out);
int skiplist_max(const skiplist *s, int *out);

/* Sizes and reported parameters. */
size_t skiplist_len(const skiplist *s);
int skiplist_is_empty(const skiplist *s);
size_t skiplist_max_level(const skiplist *s);
size_t skiplist_current_max(const skiplist *s);
double skiplist_probability(const skiplist *s);

/* Enumeration callback: keys are visited in ascending order. */
typedef void (*skiplist_visit_fn)(int key, int value, void *ud);

/* skiplist_foreach — visit every entry in ascending key order. */
void skiplist_foreach(const skiplist *s, skiplist_visit_fn visit, void *ud);

/* skiplist_range — visit entries whose key is in [lo, hi] (inclusive != 0) or
 * (lo, hi) (inclusive == 0), in ascending order. Empty if lo > hi. */
void skiplist_range(const skiplist *s, int lo, int hi, int inclusive,
                    skiplist_visit_fn visit, void *ud);

#endif /* SKIP_LIST_H */
