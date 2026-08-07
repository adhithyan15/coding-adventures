/*
 * red_black_tree.h — a left-leaning red-black (LLRB) tree with order
 * statistics, in pure ISO C17. A faithful port of the Rust `red-black-tree`
 * crate (DT09).
 * ===========================================================================
 *
 * A red-black tree is a binary search tree that colours its nodes red or black
 * and maintains, through rotations and colour flips, two invariants that
 * together force the height to stay O(log n):
 *
 *   1. No red node has a red child.
 *   2. Every root-to-leaf path passes through the same number of black nodes
 *      (the "black height").
 *
 * This is the *left-leaning* variant (Sedgewick): red links always lean left,
 * which reduces the number of cases to a single `fix_up` applied on the way back
 * up from every insert and delete. It is exactly equivalent to a 2-3 tree.
 *
 * Each node caches its subtree `size` (node count), so `rb_kth_smallest`
 * (order statistic) is O(log n).
 *
 * PERSISTENCE. The Rust crate is *persistent*: `insert` and `delete` take the
 * tree by shared reference and return a brand-new tree, leaving the original
 * untouched. This port preserves that behaviour — `rb_insert` and `rb_delete`
 * deep-copy the input tree and mutate the copy. (The Rust `Box` gives deep-clone
 * persistence, not `Rc` structural sharing, so the copy is genuinely
 * independent; we mirror that.) Every tree you obtain must be released with
 * `rb_free`.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Element type is `int`.
 */
#ifndef RED_BLACK_TREE_H
#define RED_BLACK_TREE_H

#include <stddef.h> /* size_t */

typedef enum { RB_RED, RB_BLACK } RBColor;

/* A tree node. Fields are exposed (as in the Rust crate) so callers can walk
 * the structure. */
typedef struct RBNode RBNode;
struct RBNode {
    int value;
    RBColor color;
    RBNode *left;
    RBNode *right;
    size_t size; /* number of nodes in this subtree (>= 1) */
};

/* An owning handle to a tree (possibly empty). */
typedef struct {
    RBNode *root;
} RBTree;

/* ---- construction / destruction --------------------------------------- */

/* rb_empty — allocate a new empty tree. Returns NULL on allocation failure.
 * Release it (and every tree derived from it) with rb_free. */
RBTree *rb_empty(void);

/* rb_free — free a tree and all its nodes. Safe to call with NULL. */
void rb_free(RBTree *t);

/* ---- persistent updates (return a NEW tree; `t` is left unchanged) ----- */

/* rb_insert — return a new tree with `value` inserted (a duplicate value leaves
 * the set unchanged, but you still get an independent copy). Returns NULL on
 * allocation failure; `t` is never modified. */
RBTree *rb_insert(const RBTree *t, int value);

/* rb_delete — return a new tree with `value` removed if present. Returns NULL
 * on allocation failure; `t` is never modified. */
RBTree *rb_delete(const RBTree *t, int value);

/* ---- queries (read-only; do not allocate) ----------------------------- */

/* rb_search — pointer to the node holding `value`, or NULL if absent. Valid
 * until `t` is freed or a new tree is derived from it. */
const RBNode *rb_search(const RBTree *t, int value);

/* rb_contains — 1 if `value` is present, else 0. */
int rb_contains(const RBTree *t, int value);

/* rb_min_value / rb_max_value — write the smallest/largest value to *out and
 * return 1; return 0 (leaving *out untouched) if the tree is empty. */
int rb_min_value(const RBTree *t, int *out);
int rb_max_value(const RBTree *t, int *out);

/* rb_predecessor — largest stored value strictly less than `value`
 * (writes *out, returns 1), or 0 if none exists. */
int rb_predecessor(const RBTree *t, int value, int *out);

/* rb_successor — smallest stored value strictly greater than `value`
 * (writes *out, returns 1), or 0 if none exists. */
int rb_successor(const RBTree *t, int value, int *out);

/* rb_kth_smallest — the k-th smallest value (k is 1-based). Writes *out and
 * returns 1, or returns 0 if k == 0 or k > size. */
int rb_kth_smallest(const RBTree *t, size_t k, int *out);

/* rb_to_sorted_array — copy the values in ascending order into `buf` (capacity
 * `buf_len`) and return the number written, min(size, buf_len). Pass buf == NULL
 * / buf_len == 0 to just learn nothing is written (returns 0; use rb_size). */
size_t rb_to_sorted_array(const RBTree *t, int *buf, size_t buf_len);

/* rb_size — number of values in the tree. */
size_t rb_size(const RBTree *t);

/* rb_black_height — number of black nodes on the path down the left spine
 * (0 for an empty tree). */
size_t rb_black_height(const RBTree *t);

/* rb_is_valid_rb — 1 iff the tree is a valid left-leaning red-black tree: BST
 * ordering, a black root, no red node with a red child, equal black height on
 * both sides of every node, and correct cached sizes. */
int rb_is_valid_rb(const RBTree *t);

#endif /* RED_BLACK_TREE_H */
