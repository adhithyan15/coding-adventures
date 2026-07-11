/*
 * tree_set.h — an ordered set built on a balanced-tree backend, in pure ISO
 * C17. A faithful port of the Rust `tree-set` crate.
 * ===========================================================================
 *
 * A set stores each value at most once and keeps its elements in sorted order.
 * The Rust crate is generic over its backend (any balanced ordered tree); this
 * port uses the crate's DEFAULT backend, the sibling `avl-tree`, so every
 * operation is O(log n) and the elements come out sorted for free.
 *
 * On top of the backend the set provides the usual algebra — union,
 * intersection, difference, symmetric difference — plus subset / superset /
 * disjoint tests and range queries. Following the Rust crate, those are all
 * computed from the two operands' sorted sequences (a linear merge), so the
 * result never depends on which balanced-tree backend is used.
 *
 * PERSISTENCE. Like the Rust crate (and the underlying avl-tree), updates are
 * *persistent*: `tset_insert`, `tset_remove`, and the algebra operations return
 * a NEW set and leave their inputs untouched. Every set you obtain must be
 * released with `tset_free`.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Element type is `int`. Depends on the
 * sibling `avl-tree` package (declared via `# build-tool: deps=c/avl-tree`).
 */
#ifndef TREE_SET_H
#define TREE_SET_H

#include <stddef.h> /* size_t */

#include "avl_tree.h" /* the ordered-tree backend */

/* An owning handle to a set. `backend` is the balanced tree that stores it. */
typedef struct {
    AVLTree *backend;
} TreeSet;

/* ---- construction / destruction --------------------------------------- */

/* tset_empty — allocate a new empty set. Returns NULL on allocation failure.
 * Release it (and every set derived from it) with tset_free. */
TreeSet *tset_empty(void);

/* tset_free — free a set and its backend. Safe to call with NULL. */
void tset_free(TreeSet *s);

/* tset_from_array — a new set containing the distinct values of `values`
 * (duplicates collapse). Returns NULL on allocation failure. */
TreeSet *tset_from_array(const int *values, size_t n);

/* ---- persistent updates (return a NEW set; inputs left unchanged) ------ */

/* tset_insert — a new set with `value` added (a no-op on membership if already
 * present). Returns NULL on allocation failure. */
TreeSet *tset_insert(const TreeSet *s, int value);

/* tset_remove — a new set with `value` removed if present. Returns NULL on
 * allocation failure. */
TreeSet *tset_remove(const TreeSet *s, int value);

/* ---- membership & order queries (read-only) --------------------------- */

/* tset_size — number of elements. */
size_t tset_size(const TreeSet *s);

/* tset_is_empty — 1 if the set has no elements, else 0. */
int tset_is_empty(const TreeSet *s);

/* tset_contains — 1 if `value` is present, else 0. */
int tset_contains(const TreeSet *s, int value);

/* tset_min_value / tset_max_value — smallest / largest element (via *out,
 * returns 1), or 0 if the set is empty. */
int tset_min_value(const TreeSet *s, int *out);
int tset_max_value(const TreeSet *s, int *out);

/* tset_predecessor — largest element strictly less than `value`
 * (writes *out, returns 1), or 0 if none. */
int tset_predecessor(const TreeSet *s, int value, int *out);

/* tset_successor — smallest element strictly greater than `value`
 * (writes *out, returns 1), or 0 if none. */
int tset_successor(const TreeSet *s, int value, int *out);

/* tset_kth_smallest — the k-th smallest element (k is 1-based); writes *out and
 * returns 1, or 0 if k == 0 or k > size. */
int tset_kth_smallest(const TreeSet *s, size_t k, int *out);

/* tset_rank — number of elements strictly less than `value`. */
size_t tset_rank(const TreeSet *s, int value);

/* tset_to_sorted_array — copy the elements in ascending order into `buf`
 * (capacity buf_len); returns the number written, min(size, buf_len). */
size_t tset_to_sorted_array(const TreeSet *s, int *buf, size_t buf_len);

/* tset_range — copy the elements between `min` and `max` (inclusive when
 * `inclusive` != 0, else strictly between) in ascending order into `buf`;
 * returns the number written, capped at buf_len. Empty if min > max. */
size_t tset_range(const TreeSet *s, int min, int max, int inclusive, int *buf,
                  size_t buf_len);

/* ---- set algebra (return a NEW set; NULL on allocation failure) -------- */

TreeSet *tset_union(const TreeSet *a, const TreeSet *b);
TreeSet *tset_intersection(const TreeSet *a, const TreeSet *b);
TreeSet *tset_difference(const TreeSet *a, const TreeSet *b);
TreeSet *tset_symmetric_difference(const TreeSet *a, const TreeSet *b);

/* ---- set relations (read-only) ---------------------------------------- */

/* tset_is_subset — 1 iff every element of `a` is in `b`. */
int tset_is_subset(const TreeSet *a, const TreeSet *b);

/* tset_is_superset — 1 iff every element of `b` is in `a`. */
int tset_is_superset(const TreeSet *a, const TreeSet *b);

/* tset_is_disjoint — 1 iff `a` and `b` share no element. */
int tset_is_disjoint(const TreeSet *a, const TreeSet *b);

/* tset_equals — 1 iff `a` and `b` contain exactly the same elements. */
int tset_equals(const TreeSet *a, const TreeSet *b);

#endif /* TREE_SET_H */
