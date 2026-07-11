/*
 * fenwick_tree.h — a Fenwick tree (Binary Indexed Tree) over doubles, in pure
 * ISO C17. A faithful port of the Rust `fenwick-tree` crate.
 * ===========================================================================
 *
 * A Fenwick tree answers two operations in O(log n) each over an array of
 * numbers:
 *   • update(i, delta) — add `delta` to element i
 *   • prefix_sum(i)    — sum of elements 1..=i
 * from which range sums, point queries, and "find the smallest index whose
 * prefix sum reaches k" all follow.
 *
 * The trick is that each slot `bit[i]` stores the sum of a run of elements
 * ending at i whose length is the lowest set bit of i (`i & -i`). Walking by
 * repeatedly adding/removing that lowest bit visits O(log n) slots.
 *
 * INDEXING IS 1-BASED, matching the crate: valid element indices are 1..=n.
 * prefix_sum additionally accepts 0 (the empty prefix, sum 0).
 *
 * This tree owns a heap allocation, so pair every fenwick_init* with
 * fenwick_free. Every fallible function returns a fenwick_status; the computed
 * value (if any) is written through an out-parameter.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef FENWICK_TREE_H
#define FENWICK_TREE_H

#include <stddef.h> /* size_t */

/* Status codes. FENWICK_OK is 0; every error is negative. */
typedef enum {
    FENWICK_OK = 0,
    FENWICK_INDEX_OUT_OF_RANGE = -1,
    FENWICK_INVALID_RANGE = -2,     /* left > right */
    FENWICK_EMPTY_TREE = -3,        /* find_kth on an empty tree */
    FENWICK_NON_POSITIVE_TARGET = -4,
    FENWICK_TARGET_EXCEEDS_TOTAL = -5,
    FENWICK_ALLOC_FAILED = -6
} fenwick_status;

/* The tree. Treat the fields as opaque; use the functions below. `bit` has
 * length n+1 and is 1-indexed (bit[0] is unused). */
typedef struct {
    size_t n;
    double *bit;
} fenwick_tree;

/* fenwick_init — create an all-zero tree of `n` elements.
 * Returns FENWICK_OK or FENWICK_ALLOC_FAILED. */
fenwick_status fenwick_init(fenwick_tree *t, size_t n);

/* fenwick_init_from_slice — create a tree of `count` elements initialised to
 * `values[0..count]`. Returns FENWICK_OK or FENWICK_ALLOC_FAILED. */
fenwick_status fenwick_init_from_slice(fenwick_tree *t, const double *values,
                                       size_t count);

/* fenwick_free — release the tree's storage. Safe to call on a zeroed struct
 * and idempotent (nulls the pointer). */
void fenwick_free(fenwick_tree *t);

/* fenwick_update — add `delta` to element `index` (1..=n).
 * Returns FENWICK_OK or FENWICK_INDEX_OUT_OF_RANGE. */
fenwick_status fenwick_update(fenwick_tree *t, size_t index, double delta);

/* fenwick_prefix_sum — sum of elements 1..=index into *out. `index` may be 0
 * (empty prefix → 0). Returns FENWICK_OK or FENWICK_INDEX_OUT_OF_RANGE. */
fenwick_status fenwick_prefix_sum(const fenwick_tree *t, size_t index,
                                  double *out);

/* fenwick_range_sum — sum of elements left..=right (both 1..=n) into *out.
 * Returns FENWICK_OK, FENWICK_INVALID_RANGE, or FENWICK_INDEX_OUT_OF_RANGE. */
fenwick_status fenwick_range_sum(const fenwick_tree *t, size_t left,
                                 size_t right, double *out);

/* fenwick_point_query — value of element `index` (1..=n) into *out. */
fenwick_status fenwick_point_query(const fenwick_tree *t, size_t index,
                                   double *out);

/* fenwick_find_kth — smallest index whose prefix sum is >= `target` (a
 * cumulative-frequency search; all elements are assumed non-negative). Writes
 * the 1-based index into *out. Returns FENWICK_OK, FENWICK_EMPTY_TREE,
 * FENWICK_NON_POSITIVE_TARGET, or FENWICK_TARGET_EXCEEDS_TOTAL. */
fenwick_status fenwick_find_kth(const fenwick_tree *t, double target,
                                size_t *out);

/* fenwick_len — number of elements. fenwick_is_empty — 1 if n == 0 else 0. */
size_t fenwick_len(const fenwick_tree *t);
int fenwick_is_empty(const fenwick_tree *t);

#endif /* FENWICK_TREE_H */
