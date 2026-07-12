/*
 * binary_tree.h — a generic binary tree with traversals and shape predicates,
 * in pure ISO C17. A faithful port of the Rust `binary-tree` crate (DT03).
 * ===========================================================================
 *
 * A plain binary tree: each node has a value and up to two children. Unlike a
 * search tree there is no ordering invariant — this is the shared substrate the
 * search-tree family reuses for traversal and shape checks.
 *
 * Shape predicates:
 *   - FULL     — every node has 0 or 2 children.
 *   - COMPLETE — every level is filled except possibly the last, which fills
 *                left-to-right.
 *   - PERFECT  — full AND all leaves at the same depth (n == 2^(h+1) - 1).
 *
 * Traversals: inorder / preorder / postorder (depth-first) and level_order
 * (breadth-first). `to_array` lays the tree out in level order with gaps, and
 * `to_ascii` renders an indented text diagram.
 *
 * OWNERSHIP. Node fields are exposed so callers can build trees by hand
 * (`bt_node_new` + assigning `left`/`right`, then `bt_with_root`). A tree owns
 * its nodes; `bt_free` frees them. `bt_to_ascii` returns a malloc'd string the
 * caller frees.
 *
 * CAVEAT. The depth-first operations (traversals, height, size, find, free,
 * to_ascii) recurse to the tree's height, so a very deep (degenerate) tree can
 * overflow the stack. The breadth-first ones (level_order, is_complete) are
 * iterative. This mirrors the Rust crate.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. Element type is `int`.
 */
#ifndef BINARY_TREE_H
#define BINARY_TREE_H

#include <stddef.h> /* size_t */

typedef struct BinaryTreeNode BinaryTreeNode;
struct BinaryTreeNode {
    int value;
    BinaryTreeNode *left;
    BinaryTreeNode *right;
};

typedef struct {
    BinaryTreeNode *root;
} BinaryTree;

/* ---- construction / destruction --------------------------------------- */

/* bt_new — an empty tree. NULL on allocation failure. */
BinaryTree *bt_new(void);

/* bt_node_new — a leaf node holding `value`. NULL on allocation failure. The
 * caller may assign `left`/`right` (themselves from bt_node_new) to build a
 * subtree, then hand the root to bt_with_root. */
BinaryTreeNode *bt_node_new(int value);

/* bt_with_root — a tree that takes ownership of the node tree `root` (may be
 * NULL for an empty tree). NULL on allocation failure (in which case `root` is
 * NOT freed, so the caller can retry or free it). */
BinaryTree *bt_with_root(BinaryTreeNode *root);

/* bt_from_level_order — build a tree from a level-order layout. `present[i]`
 * being 0 marks a missing node (a gap); otherwise `values[i]` is used. Index i
 * has children 2i+1 and 2i+2. Returns NULL on allocation failure. */
BinaryTree *bt_from_level_order(const int *values, const int *present,
                                size_t n);

/* bt_free — free a tree and all its nodes (safe with NULL). */
void bt_free(BinaryTree *t);

/* bt_free_node — free a node subtree (safe with NULL). For a node tree not yet
 * handed to bt_with_root. */
void bt_free_node(BinaryTreeNode *n);

/* ---- accessors -------------------------------------------------------- */

const BinaryTreeNode *bt_root(const BinaryTree *t);

/* bt_find — the first node (preorder) holding `value`, or NULL. */
const BinaryTreeNode *bt_find(const BinaryTree *t, int value);

/* bt_left_child / bt_right_child — the left/right child of the first node
 * holding `value`, or NULL if the value or that child is absent. */
const BinaryTreeNode *bt_left_child(const BinaryTree *t, int value);
const BinaryTreeNode *bt_right_child(const BinaryTree *t, int value);

/* ---- shape predicates (1 = true, 0 = false) --------------------------- */

int bt_is_full(const BinaryTree *t);
int bt_is_complete(const BinaryTree *t);
int bt_is_perfect(const BinaryTree *t);

/* bt_height — -1 for an empty tree, 0 for a single node. */
long bt_height(const BinaryTree *t);
size_t bt_size(const BinaryTree *t);

/* ---- traversals (copy values into `buf`, return count written) -------- */

size_t bt_inorder(const BinaryTree *t, int *buf, size_t buf_len);
size_t bt_preorder(const BinaryTree *t, int *buf, size_t buf_len);
size_t bt_postorder(const BinaryTree *t, int *buf, size_t buf_len);
size_t bt_level_order(const BinaryTree *t, int *buf, size_t buf_len);

/* bt_to_array — level-order layout with gaps. The full length is 2^(h+1)-1
 * (0 for an empty tree); this writes up to `buf_len` entries, setting
 * `present_out[i]` to 1/0 and `values_out[i]` for present slots, and returns
 * the full length (which may exceed buf_len — nothing past buf_len is written). */
size_t bt_to_array(const BinaryTree *t, int *values_out, int *present_out,
                   size_t buf_len);

/* bt_to_ascii — an indented text diagram (malloc'd, caller frees). Returns NULL
 * on allocation failure; returns an empty string for an empty tree. */
char *bt_to_ascii(const BinaryTree *t);

#endif /* BINARY_TREE_H */
