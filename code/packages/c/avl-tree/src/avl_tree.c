/*
 * avl_tree.c — implementation of the persistent AVL tree (see avl_tree.h).
 * A faithful port of the Rust `avl-tree` crate: the same rotate / rebalance /
 * extract-min algorithms, and the same persistence (updates return a new tree
 * and leave the input untouched — realised here by a deep copy).
 */
#include "avl_tree.h"

#include <stdlib.h> /* malloc, free */

/* ---- small node helpers ----------------------------------------------- */

/* Height of a (possibly absent) subtree: -1 for none, matching the Rust
 * `avl_height` convention so a leaf's cached height is 0. */
static long node_height(const AVLNode *n) { return n ? n->height : -1; }

/* Node count of a (possibly absent) subtree: 0 for none. */
static size_t node_size(const AVLNode *n) { return n ? n->size : 0; }

/* Recompute a node's cached height and size from its children. */
static void update_metadata(AVLNode *n) {
    long lh = node_height(n->left);
    long rh = node_height(n->right);
    n->height = 1 + (lh > rh ? lh : rh);
    n->size = 1 + node_size(n->left) + node_size(n->right);
}

/* balance factor = height(left) - height(right). */
static long node_balance(const AVLNode *n) {
    return node_height(n->left) - node_height(n->right);
}

static AVLNode *node_new(int value) {
    AVLNode *n = malloc(sizeof *n);
    if (!n) {
        return NULL;
    }
    n->value = value;
    n->left = NULL;
    n->right = NULL;
    n->height = 0;
    n->size = 1;
    return n;
}

static void node_free(AVLNode *n) {
    if (!n) {
        return;
    }
    node_free(n->left);
    node_free(n->right);
    free(n);
}

/* Deep-copy a subtree. Returns NULL on allocation failure, having freed any
 * partial copy it built. */
static AVLNode *node_clone_deep(const AVLNode *n) {
    AVLNode *c = malloc(sizeof *c);
    if (!c) {
        return NULL;
    }
    c->value = n->value;
    c->height = n->height;
    c->size = n->size;
    c->left = NULL;
    c->right = NULL;
    if (n->left) {
        c->left = node_clone_deep(n->left);
        if (!c->left) {
            node_free(c);
            return NULL;
        }
    }
    if (n->right) {
        c->right = node_clone_deep(n->right);
        if (!c->right) {
            node_free(c);
            return NULL;
        }
    }
    return c;
}

/* ---- rotations & rebalancing ------------------------------------------ */

/* Rotate `root` left, promoting its right child. Takes ownership of the subtree
 * and returns the new subtree root (unchanged if there is no right child). */
static AVLNode *rotate_left(AVLNode *root) {
    AVLNode *new_root = root->right;
    if (!new_root) {
        return root;
    }
    root->right = new_root->left;
    update_metadata(root);
    new_root->left = root;
    update_metadata(new_root);
    return new_root;
}

/* Rotate `root` right, promoting its left child. */
static AVLNode *rotate_right(AVLNode *root) {
    AVLNode *new_root = root->left;
    if (!new_root) {
        return root;
    }
    root->left = new_root->right;
    update_metadata(root);
    new_root->right = root;
    update_metadata(new_root);
    return new_root;
}

/* Restore the AVL balance invariant at `node` (its subtrees are already
 * balanced). Handles the LL / LR / RR / RL cases. */
static AVLNode *rebalance(AVLNode *node) {
    long bf = node_balance(node);
    if (bf > 1) { /* left-heavy */
        if (node->left && node_balance(node->left) < 0) {
            node->left = rotate_left(node->left); /* LR -> LL */
        }
        return rotate_right(node);
    }
    if (bf < -1) { /* right-heavy */
        if (node->right && node_balance(node->right) > 0) {
            node->right = rotate_right(node->right); /* RL -> RR */
        }
        return rotate_left(node);
    }
    return node;
}

/* ---- insert ----------------------------------------------------------- */

/* Insert `value` into the subtree `root` (which this call owns), returning the
 * new subtree root. On allocation failure sets *ok = 0; the returned tree still
 * contains exactly the pre-existing nodes (no leak, no corruption). Note NULL is
 * only ever returned for an originally-empty subtree, so a parent that writes
 * `child = node_insert(child, ...)` never overwrites a real subtree with NULL. */
static AVLNode *node_insert(AVLNode *root, int value, int *ok) {
    if (!root) {
        AVLNode *n = node_new(value);
        if (!n) {
            *ok = 0;
        }
        return n;
    }
    if (value < root->value) {
        root->left = node_insert(root->left, value, ok);
        if (!*ok) {
            return root;
        }
    } else if (value > root->value) {
        root->right = node_insert(root->right, value, ok);
        if (!*ok) {
            return root;
        }
    } else {
        return root; /* duplicate — set semantics, tree unchanged */
    }
    update_metadata(root);
    return rebalance(root);
}

/* ---- delete ----------------------------------------------------------- */

/* Remove and return the minimum of the subtree `node` (owned), writing that
 * minimum value to *min_out and returning the remaining subtree. */
static AVLNode *extract_min(AVLNode *node, int *min_out) {
    if (!node->left) {
        AVLNode *right = node->right;
        *min_out = node->value;
        free(node);
        return right;
    }
    node->left = extract_min(node->left, min_out);
    update_metadata(node);
    return rebalance(node);
}

/* Delete `value` from the subtree `root` (owned), returning the new root. No
 * allocation, so this cannot fail. */
static AVLNode *node_delete(AVLNode *root, int value) {
    if (!root) {
        return NULL;
    }
    if (value < root->value) {
        root->left = node_delete(root->left, value);
    } else if (value > root->value) {
        root->right = node_delete(root->right, value);
    } else {
        AVLNode *left = root->left;
        AVLNode *right = root->right;
        root->left = NULL;
        root->right = NULL;
        if (!left && !right) {
            free(root);
            return NULL;
        }
        if (left && !right) {
            free(root);
            return left;
        }
        if (!left && right) {
            free(root);
            return right;
        }
        /* Two children: replace with the in-order successor (min of right). */
        {
            int successor;
            AVLNode *new_right = extract_min(right, &successor);
            root->value = successor;
            root->left = left;
            root->right = new_right;
        }
    }
    update_metadata(root);
    return rebalance(root);
}

/* ---- public: construction / destruction ------------------------------- */

AVLTree *avl_empty(void) {
    AVLTree *t = malloc(sizeof *t);
    if (!t) {
        return NULL;
    }
    t->root = NULL;
    return t;
}

void avl_free(AVLTree *t) {
    if (!t) {
        return;
    }
    node_free(t->root);
    free(t);
}

/* ---- public: persistent updates --------------------------------------- */

AVLTree *avl_insert(const AVLTree *t, int value) {
    AVLTree *nt = malloc(sizeof *nt);
    int ok = 1;
    AVLNode *cloned = NULL;
    if (!nt) {
        return NULL;
    }
    if (t && t->root) {
        cloned = node_clone_deep(t->root);
        if (!cloned) { /* clone failed (t->root was non-NULL) */
            free(nt);
            return NULL;
        }
    }
    nt->root = node_insert(cloned, value, &ok);
    if (!ok) {
        node_free(nt->root);
        free(nt);
        return NULL;
    }
    return nt;
}

AVLTree *avl_delete(const AVLTree *t, int value) {
    AVLTree *nt = malloc(sizeof *nt);
    AVLNode *cloned = NULL;
    if (!nt) {
        return NULL;
    }
    if (t && t->root) {
        cloned = node_clone_deep(t->root);
        if (!cloned) {
            free(nt);
            return NULL;
        }
    }
    nt->root = node_delete(cloned, value);
    return nt;
}

/* ---- public: queries -------------------------------------------------- */

const AVLNode *avl_search(const AVLTree *t, int value) {
    const AVLNode *cur = t ? t->root : NULL;
    while (cur) {
        if (value < cur->value) {
            cur = cur->left;
        } else if (value > cur->value) {
            cur = cur->right;
        } else {
            return cur;
        }
    }
    return NULL;
}

int avl_contains(const AVLTree *t, int value) {
    return avl_search(t, value) != NULL;
}

int avl_min_value(const AVLTree *t, int *out) {
    const AVLNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->left) {
        cur = cur->left;
    }
    *out = cur->value;
    return 1;
}

int avl_max_value(const AVLTree *t, int *out) {
    const AVLNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->right) {
        cur = cur->right;
    }
    *out = cur->value;
    return 1;
}

int avl_predecessor(const AVLTree *t, int value, int *out) {
    const AVLNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (value <= cur->value) {
            cur = cur->left;
        } else { /* cur->value < value — a predecessor candidate */
            best = cur->value;
            found = 1;
            cur = cur->right;
        }
    }
    if (found) {
        *out = best;
    }
    return found;
}

int avl_successor(const AVLTree *t, int value, int *out) {
    const AVLNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (value >= cur->value) {
            cur = cur->right;
        } else { /* cur->value > value — a successor candidate */
            best = cur->value;
            found = 1;
            cur = cur->left;
        }
    }
    if (found) {
        *out = best;
    }
    return found;
}

int avl_kth_smallest(const AVLTree *t, size_t k, int *out) {
    const AVLNode *cur = t ? t->root : NULL;
    if (k == 0) {
        return 0;
    }
    while (cur) {
        size_t ls = node_size(cur->left);
        if (k == ls + 1) {
            *out = cur->value;
            return 1;
        }
        if (k <= ls) {
            cur = cur->left;
        } else {
            k = k - ls - 1;
            cur = cur->right;
        }
    }
    return 0;
}

size_t avl_rank(const AVLTree *t, int value) {
    const AVLNode *cur = t ? t->root : NULL;
    size_t rank = 0;
    while (cur) {
        if (value < cur->value) {
            cur = cur->left;
        } else if (value == cur->value) {
            return rank + node_size(cur->left);
        } else {
            rank += node_size(cur->left) + 1;
            cur = cur->right;
        }
    }
    return rank;
}

/* In-order traversal that stops once `buf` (capacity buf_len) is full. */
static void inorder_fill(const AVLNode *n, int *buf, size_t buf_len,
                         size_t *idx) {
    if (!n || *idx >= buf_len) {
        return;
    }
    inorder_fill(n->left, buf, buf_len, idx);
    if (*idx < buf_len) {
        buf[*idx] = n->value;
        (*idx)++;
    }
    inorder_fill(n->right, buf, buf_len, idx);
}

size_t avl_to_sorted_array(const AVLTree *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    if (!buf || buf_len == 0) {
        return 0;
    }
    inorder_fill(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

size_t avl_size(const AVLTree *t) { return node_size(t ? t->root : NULL); }

long avl_height(const AVLTree *t) { return node_height(t ? t->root : NULL); }

long avl_balance_factor(const AVLNode *node) {
    return node ? node_balance(node) : 0;
}

/* ---- public: validation ----------------------------------------------- */

static int validate_bst(const AVLNode *n, const int *min, const int *max) {
    if (!n) {
        return 1;
    }
    if (min && n->value <= *min) {
        return 0;
    }
    if (max && n->value >= *max) {
        return 0;
    }
    return validate_bst(n->left, min, &n->value) &&
           validate_bst(n->right, &n->value, max);
}

int avl_is_valid_bst(const AVLTree *t) {
    return validate_bst(t ? t->root : NULL, NULL, NULL);
}

/* Returns 1 and writes the subtree's (height, size) if it is a valid AVL
 * subtree bounded by (min, max); 0 otherwise. */
static int validate_avl(const AVLNode *n, const int *min, const int *max,
                        long *h_out, size_t *s_out) {
    long lh = 0, rh = 0, height;
    size_t ls = 0, rs = 0, size, diff_h;
    if (!n) {
        *h_out = -1;
        *s_out = 0;
        return 1;
    }
    if (min && n->value <= *min) {
        return 0;
    }
    if (max && n->value >= *max) {
        return 0;
    }
    if (!validate_avl(n->left, min, &n->value, &lh, &ls)) {
        return 0;
    }
    if (!validate_avl(n->right, &n->value, max, &rh, &rs)) {
        return 0;
    }
    height = 1 + (lh > rh ? lh : rh);
    size = 1 + ls + rs;
    if (n->height != height || n->size != size) {
        return 0;
    }
    diff_h = (size_t)(lh > rh ? lh - rh : rh - lh);
    if (diff_h > 1) {
        return 0;
    }
    *h_out = height;
    *s_out = size;
    return 1;
}

int avl_is_valid_avl(const AVLTree *t) {
    long h;
    size_t s;
    return validate_avl(t ? t->root : NULL, NULL, NULL, &h, &s);
}
