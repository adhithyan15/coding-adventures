/*
 * binary_search_tree.h — an unbalanced binary search tree with order
 * statistics, in pure ISO C17. A faithful port of the Rust `binary-search-tree`
 * crate (DT07).
 * ===========================================================================
 *
 * A binary search tree keeps values ordered so that, for every node, everything
 * in its left subtree is smaller and everything in its right subtree is larger.
 * Search / insert / delete are O(h) where h is the height — O(log n) for a
 * balanced tree, O(n) worst case for a degenerate one. Each node caches its
 * subtree `size` (node count), making `bst_rank` and `bst_kth_smallest` (order
 * statistics) O(h).
 *
 * `bst_from_sorted_array` builds a height-balanced tree from a sorted array by
 * recursively choosing the middle element as each subtree root.
 *
 * PERSISTENCE. Like the Rust crate, updates are *persistent*: `bst_insert` and
 * `bst_delete` return a NEW tree and leave the input untouched (they deep-copy,
 * then mutate the copy). Every tree you obtain must be released with `bst_free`.
 *
 * CAVEAT. This tree does not self-balance, so its height is bounded only by
 * insertion order — inserting already-sorted keys builds a degenerate (linked-
 * list) tree of height n. All operations here recurse to the tree's height, so
 * a very tall tree can overflow the stack. For adversarial or large sorted
 * input, prefer `bst_from_sorted_array` (log-depth) or the self-balancing
 * `avl-tree` package. This mirrors the Rust crate's behaviour.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Element type is `int`.
 */
#ifndef BINARY_SEARCH_TREE_H
#define BINARY_SEARCH_TREE_H

#include <stddef.h> /* size_t */

/* A tree node. Fields are exposed (as in the Rust crate) so callers can walk
 * the structure. */
typedef struct BSTNode BSTNode;
struct BSTNode {
    int value;
    BSTNode *left;
    BSTNode *right;
    size_t size; /* number of nodes in this subtree (>= 1) */
};

/* An owning handle to a tree (possibly empty). */
typedef struct {
    BSTNode *root;
} BST;

/* ---- construction / destruction --------------------------------------- */

/* bst_empty — allocate a new empty tree. Returns NULL on allocation failure. */
BST *bst_empty(void);

/* bst_from_sorted_array — a balanced tree over the (assumed sorted) `values`.
 * Returns NULL on allocation failure. */
BST *bst_from_sorted_array(const int *values, size_t n);

/* bst_free — free a tree and all its nodes. Safe to call with NULL. */
void bst_free(BST *t);

/* ---- persistent updates (return a NEW tree; `t` is left unchanged) ----- */

BST *bst_insert(const BST *t, int value);
BST *bst_delete(const BST *t, int value);

/* ---- queries (read-only; do not allocate) ----------------------------- */

const BSTNode *bst_search(const BST *t, int value);
int bst_contains(const BST *t, int value);

/* Extremes / neighbours: write to *out and return 1, or return 0 if absent. */
int bst_min_value(const BST *t, int *out);
int bst_max_value(const BST *t, int *out);
int bst_predecessor(const BST *t, int value, int *out);
int bst_successor(const BST *t, int value, int *out);

/* bst_kth_smallest — the k-th smallest value (k is 1-based); writes *out and
 * returns 1, or 0 if k == 0 or k > size. */
int bst_kth_smallest(const BST *t, size_t k, int *out);

/* bst_rank — number of stored values strictly less than `value`. */
size_t bst_rank(const BST *t, int value);

/* bst_to_sorted_array — copy the values in ascending order into `buf` (capacity
 * buf_len); returns the number written, min(size, buf_len). */
size_t bst_to_sorted_array(const BST *t, int *buf, size_t buf_len);

size_t bst_size(const BST *t);

/* bst_height — height of the tree: -1 when empty, 0 for a single node. */
long bst_height(const BST *t);

/* bst_is_valid — 1 iff the tree obeys the binary-search-tree ordering. */
int bst_is_valid(const BST *t);

#endif /* BINARY_SEARCH_TREE_H */
