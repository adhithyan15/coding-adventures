/*
 * b_tree.h — a B-tree (minimum degree `t`), in pure ISO C17. A faithful port of
 * the Rust `b-tree` crate.
 * ===========================================================================
 *
 * A B-tree is a balanced search tree tuned for large, shallow fan-out: every
 * node holds many keys, so the tree stays short and lookups touch few nodes.
 * With minimum degree `t`, every non-root node holds between `t-1` and `2t-1`
 * keys and between `t` and `2t` children; all leaves sit at the same depth.
 *
 * This port implements the full CLRS algorithm: proactive top-down splitting on
 * insert, and pre-fill (rotate from a sibling, or merge) on the way down for
 * delete. It keeps the tree valid at every step.
 *
 *   btree_insert / btree_delete / btree_search   — the map operations
 *   btree_min_key / btree_max_key                — extremes
 *   btree_inorder / btree_range_query            — ordered traversal
 *   btree_height / btree_is_valid / btree_len     — introspection
 *
 * Keys and values are `long`. (The Rust crate is generic over an ordered key
 * and any value; C has no generics, so this port specialises to `long -> long`
 * — a sorted integer map — which is exactly what the crate's tests exercise.
 * The C++ sibling package is fully generic.)
 *
 * Ownership: btree_new allocates; pair it with btree_free.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef B_TREE_H
#define B_TREE_H

#include <stddef.h> /* size_t */

typedef struct btree btree;

/* btree_new — a new empty B-tree with minimum degree `t` (clamped to >= 2).
 * Returns NULL on allocation failure. */
btree *btree_new(size_t t);

/* btree_free — release the tree and all its nodes. NULL-safe. */
void btree_free(btree *tree);

/* btree_insert — insert or overwrite `key` -> `value`. Returns 1 on success, 0
 * on allocation failure (the tree is left valid, just without the new key). */
int btree_insert(btree *tree, long key, long value);

/* btree_delete — remove `key`. Returns 1 if it was present, else 0. */
int btree_delete(btree *tree, long key);

/* btree_search — look up `key`. On a hit writes the value to *out_value (which
 * may be NULL) and returns 1; returns 0 if absent. */
int btree_search(const btree *tree, long key, long *out_value);

/* btree_contains — 1 if `key` is present, else 0. */
int btree_contains(const btree *tree, long key);

/* btree_min_key / btree_max_key — write the smallest / largest key to *out and
 * return 1; return 0 if the tree is empty. */
int btree_min_key(const btree *tree, long *out);
int btree_max_key(const btree *tree, long *out);

/* btree_len — number of key/value pairs. */
size_t btree_len(const btree *tree);

/* btree_is_empty — 1 if the tree holds no pairs. */
int btree_is_empty(const btree *tree);

/* btree_height — 0 for empty or a single-leaf tree; +1 per internal level. */
size_t btree_height(const btree *tree);

/* btree_is_valid — 1 if all B-tree structural invariants hold (sorted keys,
 * key-count bounds, uniform leaf depth, correct child counts). */
int btree_is_valid(const btree *tree);

/* Visitor callback for the ordered traversals: receives one entry's key and
 * value plus the caller's `user` pointer. */
typedef void (*btree_visit_fn)(long key, long value, void *user);

/* btree_inorder — visit every entry in ascending key order. */
void btree_inorder(const btree *tree, btree_visit_fn fn, void *user);

/* btree_range_query — visit every entry with low <= key <= high, in order. */
void btree_range_query(const btree *tree, long low, long high,
                       btree_visit_fn fn, void *user);

#endif /* B_TREE_H */
