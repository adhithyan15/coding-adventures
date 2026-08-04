/*
 * avl_tree.h — a self-balancing AVL tree with order statistics, in pure ISO
 * C17. A faithful port of the Rust `avl-tree` crate (DT08).
 * ===========================================================================
 *
 * An AVL tree is a binary search tree that keeps itself balanced: after every
 * insert or delete it restores the invariant that, for every node, the heights
 * of its two subtrees differ by at most one. That bound guarantees O(log n)
 * search / insert / delete no matter what order the keys arrive in.
 *
 *   balance factor = height(left) - height(right)      must be in {-1, 0, +1}
 *
 * When an insert or delete pushes a node's balance factor to +2 or -2, one or
 * two rotations bring it back into range. An "LR"/"RL" case first rotates the
 * child to reduce to the simple "LL"/"RR" case, then rotates the node.
 *
 * PERSISTENCE. The Rust crate is *persistent*: `insert` and `delete` take the
 * tree by shared reference and return a brand-new tree, leaving the original
 * untouched. This port preserves that exact observable behaviour — `avl_insert`
 * and `avl_delete` deep-copy the input tree and mutate the copy, so any handle
 * you already hold keeps its old contents. (The Rust `Box` gives deep-clone
 * persistence, not `Rc` structural sharing, so the copy is genuinely
 * independent; we mirror that.) Every tree you obtain must be released with
 * `avl_free`.
 *
 * Each node also caches its subtree `height` and `size` (node count); the size
 * cache is what makes `avl_rank` and `avl_kth_smallest` (order statistics)
 * O(log n).
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Element type is `int`.
 */
#ifndef AVL_TREE_H
#define AVL_TREE_H

#include <stddef.h> /* size_t */

/* A tree node. Fields are exposed (as in the Rust crate) so callers can walk
 * the structure; `height` is -1 for an absent child, 0 for a leaf. */
typedef struct AVLNode AVLNode;
struct AVLNode {
    int value;
    AVLNode *left;
    AVLNode *right;
    long height; /* Rust isize: -1 (empty child) .. */
    size_t size; /* number of nodes in this subtree (>= 1) */
};

/* An owning handle to a tree (possibly empty). */
typedef struct {
    AVLNode *root;
} AVLTree;

/* ---- construction / destruction --------------------------------------- */

/* avl_empty — allocate a new empty tree. Returns NULL on allocation failure.
 * Release it (and every tree derived from it) with avl_free. */
AVLTree *avl_empty(void);

/* avl_free — free a tree and all its nodes. Safe to call with NULL. */
void avl_free(AVLTree *t);

/* ---- persistent updates (return a NEW tree; `t` is left unchanged) ----- */

/* avl_insert — return a new tree with `value` inserted (a duplicate value
 * leaves the set unchanged, but you still get an independent copy). Returns
 * NULL on allocation failure; `t` is never modified. */
AVLTree *avl_insert(const AVLTree *t, int value);

/* avl_delete — return a new tree with `value` removed if present. Returns NULL
 * on allocation failure; `t` is never modified. */
AVLTree *avl_delete(const AVLTree *t, int value);

/* ---- queries (read-only; do not allocate) ----------------------------- */

/* avl_search — pointer to the node holding `value`, or NULL if absent. The
 * pointer is valid until `t` is freed or a new tree is derived from it. */
const AVLNode *avl_search(const AVLTree *t, int value);

/* avl_contains — 1 if `value` is present, else 0. */
int avl_contains(const AVLTree *t, int value);

/* avl_min_value / avl_max_value — write the smallest/largest value to *out and
 * return 1; return 0 (and leave *out untouched) if the tree is empty. */
int avl_min_value(const AVLTree *t, int *out);
int avl_max_value(const AVLTree *t, int *out);

/* avl_predecessor — largest stored value strictly less than `value`
 * (writes *out, returns 1), or 0 if none exists. */
int avl_predecessor(const AVLTree *t, int value, int *out);

/* avl_successor — smallest stored value strictly greater than `value`
 * (writes *out, returns 1), or 0 if none exists. */
int avl_successor(const AVLTree *t, int value, int *out);

/* avl_kth_smallest — the k-th smallest value (k is 1-based). Writes *out and
 * returns 1, or returns 0 if k == 0 or k > size. */
int avl_kth_smallest(const AVLTree *t, size_t k, int *out);

/* avl_rank — number of stored values strictly less than `value` (i.e. the
 * index `value` would occupy in sorted order). */
size_t avl_rank(const AVLTree *t, int value);

/* avl_to_sorted_array — copy the values in ascending order into `buf` (capacity
 * `buf_len`) and return the number written, which is min(size, buf_len). Pass
 * buf == NULL / buf_len == 0 to just learn the size (returns 0 then; use
 * avl_size). */
size_t avl_to_sorted_array(const AVLTree *t, int *buf, size_t buf_len);

/* avl_size — number of values in the tree. */
size_t avl_size(const AVLTree *t);

/* avl_height — height of the tree: -1 when empty, 0 for a single node. */
long avl_height(const AVLTree *t);

/* avl_balance_factor — height(left) - height(right) for `node`. */
long avl_balance_factor(const AVLNode *node);

/* avl_is_valid_bst — 1 iff the tree obeys the binary-search-tree ordering. */
int avl_is_valid_bst(const AVLTree *t);

/* avl_is_valid_avl — 1 iff the tree is a valid AVL tree: BST ordering AND the
 * balance invariant AND correct cached height/size at every node. */
int avl_is_valid_avl(const AVLTree *t);

#endif /* AVL_TREE_H */
