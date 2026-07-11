/*
 * segment_tree.h — a segment tree over ints with a caller-supplied associative
 * combine operation, in pure ISO C17. A faithful port of the Rust
 * `segment-tree` crate.
 * ===========================================================================
 *
 * A segment tree answers "combine all elements in the range [left, right]" in
 * O(log n) and supports point updates in O(log n). The combine operation is any
 * associative binary function with an identity element — sum (+, 0), min (min,
 * +inf), max (max, -inf), gcd, and so on. Pass the operation and its identity to
 * segment_tree_init, or use the sum/min/max convenience builders.
 *
 * Internally the tree is a 1-indexed array of up to 4n nodes: node k covers a
 * contiguous segment; its children 2k and 2k+1 cover the two halves. Queries and
 * updates walk O(log n) of those nodes.
 *
 * Ranges are INCLUSIVE and 0-based: query(0, 2) combines elements 0, 1, 2.
 *
 * The tree owns a heap allocation — pair segment_tree_init with
 * segment_tree_free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef SEGMENT_TREE_H
#define SEGMENT_TREE_H

#include <stddef.h> /* size_t */

/* An associative binary operation over ints (e.g. addition, min, max). */
typedef int (*segment_tree_combine)(int a, int b);

/* The tree. Treat the fields as opaque; use the functions below. */
typedef struct {
    int *tree; /* 1-indexed node array, length 4n+4 (or 1 when n == 0) */
    size_t n;
    segment_tree_combine combine;
    int identity; /* neutral element of `combine` */
} segment_tree;

/* segment_tree_init — build a tree over `values[0..n]` using `combine` (with
 * neutral element `identity`). Returns 1 on success, 0 on allocation failure. */
int segment_tree_init(segment_tree *t, const int *values, size_t n,
                      segment_tree_combine combine, int identity);

/* Convenience builders for the three most common operations. */
int segment_tree_init_sum(segment_tree *t, const int *values, size_t n);
int segment_tree_init_min(segment_tree *t, const int *values, size_t n);
int segment_tree_init_max(segment_tree *t, const int *values, size_t n);

/* segment_tree_free — release storage. Safe on a zeroed struct; idempotent. */
void segment_tree_free(segment_tree *t);

/* segment_tree_query — combine elements in the inclusive range [left, right].
 * Returns `identity` for an empty tree or an out-of-range/inverted range
 * (left > right or right >= n), so it never reads out of bounds. */
int segment_tree_query(const segment_tree *t, size_t left, size_t right);

/* segment_tree_update — set element `index` (0-based) to `value` and refresh
 * the covering nodes. Out-of-range indices are ignored. */
void segment_tree_update(segment_tree *t, size_t index, int value);

/* segment_tree_len — number of elements. segment_tree_is_empty — 1 if n == 0. */
size_t segment_tree_len(const segment_tree *t);
int segment_tree_is_empty(const segment_tree *t);

#endif /* SEGMENT_TREE_H */
