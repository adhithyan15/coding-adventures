/*
 * b_plus_tree.h — a B+ tree (minimum degree `t`), in pure ISO C17. A faithful
 * port of the Rust `b-plus-tree` crate.
 * ===========================================================================
 *
 * A B+ tree is a B-tree variant tuned for range scans:
 *
 *   - **All values live in leaves.** Internal nodes hold only separator keys for
 *     routing — a separator is a *copy* of the smallest key in its right
 *     subtree, so keys can appear both in an internal node and in a leaf.
 *   - **Leaves form a linked list.** Every leaf has a `next` pointer to the leaf
 *     with the following keys, so a range scan finds one leaf and then walks the
 *     chain — no repeated root-to-leaf descents.
 *
 * This port implements the full algorithm: leaf/internal splitting on insert
 * (propagated bottom-up), and borrow-from-sibling / merge rebalancing on delete,
 * all while keeping the leaf chain in sync.
 *
 *   bpt_insert / bpt_delete / bpt_search    — the map operations
 *   bpt_range_scan / bpt_full_scan          — ordered scans over the leaf chain
 *   bpt_min_key / bpt_max_key / bpt_height / bpt_is_valid — introspection
 *
 * Keys and values are `long`. (The Rust crate is generic; C has no generics, so
 * this port specialises to `long -> long`, matching the crate's tests. The C++
 * sibling package is fully generic.)
 *
 * The `next` chain is an ordinary C pointer — the leaf list the Rust crate builds
 * with `*mut` raw pointers is a natural, pure-ISO structure in C.
 *
 * Ownership: bpt_new allocates; pair it with bpt_free.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef B_PLUS_TREE_H
#define B_PLUS_TREE_H

#include <stddef.h> /* size_t */

typedef struct bpt bpt;

/* bpt_new — a new empty B+ tree with minimum degree `t` (clamped to >= 2).
 * Returns NULL on allocation failure. */
bpt *bpt_new(size_t t);

/* bpt_free — release the tree and all its nodes. NULL-safe. */
void bpt_free(bpt *tree);

/* bpt_insert — insert or overwrite `key` -> `value`. Returns 1 on success, 0 on
 * allocation failure (the tree is left unchanged and valid). */
int bpt_insert(bpt *tree, long key, long value);

/* bpt_delete — remove `key`. Returns 1 if it was present, else 0. */
int bpt_delete(bpt *tree, long key);

/* bpt_search — look up `key`. On a hit writes the value to *out_value (which may
 * be NULL) and returns 1; returns 0 if absent. */
int bpt_search(const bpt *tree, long key, long *out_value);

/* bpt_contains — 1 if `key` is present, else 0. */
int bpt_contains(const bpt *tree, long key);

/* bpt_min_key / bpt_max_key — write the smallest / largest key to *out and
 * return 1; return 0 if the tree is empty. */
int bpt_min_key(const bpt *tree, long *out);
int bpt_max_key(const bpt *tree, long *out);

/* bpt_len — number of key/value pairs. */
size_t bpt_len(const bpt *tree);

/* bpt_is_empty — 1 if the tree holds no pairs. */
int bpt_is_empty(const bpt *tree);

/* bpt_height — 0 when the root is a leaf; +1 per internal level. */
size_t bpt_height(const bpt *tree);

/* bpt_is_valid — 1 if all B+ tree invariants hold (key-count bounds, sorted
 * keys, correct child counts, uniform leaf depth, and a leaf chain that lists
 * every key exactly once in sorted order with a matching size). */
int bpt_is_valid(const bpt *tree);

/* Visitor callback for the scans: one entry's key and value, plus `user`. */
typedef void (*bpt_visit_fn)(long key, long value, void *user);

/* bpt_full_scan — visit every entry in ascending order by walking the leaf
 * chain from the first leaf. */
void bpt_full_scan(const bpt *tree, bpt_visit_fn fn, void *user);

/* bpt_range_scan — visit every entry with low <= key <= high, in order, using
 * the leaf chain. */
void bpt_range_scan(const bpt *tree, long low, long high, bpt_visit_fn fn,
                    void *user);

#endif /* B_PLUS_TREE_H */
