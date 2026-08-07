/*
 * red_black_tree.c — implementation of the persistent left-leaning red-black
 * tree (see red_black_tree.h). A faithful port of the Rust `red-black-tree`
 * crate: the same rotate / fix_up / move-red / delete-min algorithms and the
 * same persistence (updates return a new tree and leave the input untouched —
 * realised here by a deep copy).
 */
#include "red_black_tree.h"

#include <stdlib.h> /* malloc, free */

/* ---- small node helpers ----------------------------------------------- */

static size_t node_size(const RBNode *n) { return n ? n->size : 0; }

static void update_size(RBNode *n) {
    n->size = 1 + node_size(n->left) + node_size(n->right);
}

/* is_red — a node is red iff it exists and is coloured red (an absent node is
 * black, matching the Rust `is_red` over Option). */
static int is_red(const RBNode *n) { return n && n->color == RB_RED; }

/* is_red_left — true iff `n` exists, has a left child, and that child is red. */
static int is_red_left(const RBNode *n) {
    return n && n->left && n->left->color == RB_RED;
}

static RBColor flip(RBColor c) { return c == RB_RED ? RB_BLACK : RB_RED; }

static RBNode *node_new(int value, RBColor color) {
    RBNode *n = malloc(sizeof *n);
    if (!n) {
        return NULL;
    }
    n->value = value;
    n->color = color;
    n->left = NULL;
    n->right = NULL;
    n->size = 1;
    return n;
}

static void node_free(RBNode *n) {
    if (!n) {
        return;
    }
    node_free(n->left);
    node_free(n->right);
    free(n);
}

/* Deep-copy a subtree. Returns NULL on allocation failure, having freed any
 * partial copy it built. */
static RBNode *node_clone_deep(const RBNode *n) {
    RBNode *c = malloc(sizeof *c);
    if (!c) {
        return NULL;
    }
    c->value = n->value;
    c->color = n->color;
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

/* ---- rotations & colour flips (LLRB) ---------------------------------- */

/* Rotate `root` left, promoting its right child and swapping the red link to
 * the left. Returns the new subtree root (unchanged if there is no right
 * child). Takes ownership of the subtree. */
static RBNode *rotate_left(RBNode *root) {
    RBNode *new_root = root->right;
    RBColor root_color;
    if (!new_root) {
        return root;
    }
    root_color = root->color;
    root->right = new_root->left;
    root->color = RB_RED;
    update_size(root);
    new_root->left = root;
    new_root->color = root_color;
    update_size(new_root);
    return new_root;
}

/* Rotate `root` right, promoting its left child. */
static RBNode *rotate_right(RBNode *root) {
    RBNode *new_root = root->left;
    RBColor root_color;
    if (!new_root) {
        return root;
    }
    root_color = root->color;
    root->left = new_root->right;
    root->color = RB_RED;
    update_size(root);
    new_root->right = root;
    new_root->color = root_color;
    update_size(new_root);
    return new_root;
}

/* Flip the colours of `node` and both of its children. */
static void flip_colors(RBNode *node) {
    node->color = flip(node->color);
    if (node->left) {
        node->left->color = flip(node->left->color);
    }
    if (node->right) {
        node->right->color = flip(node->right->color);
    }
}

/* Restore the LLRB invariants at `node` on the way back up: lean red links
 * left, split 4-nodes. Also refreshes the cached size. */
static RBNode *fix_up(RBNode *node) {
    if (is_red(node->right) && !is_red(node->left)) {
        node = rotate_left(node);
    }
    if (is_red(node->left) && is_red_left(node->left)) {
        node = rotate_right(node);
    }
    if (is_red(node->left) && is_red(node->right)) {
        flip_colors(node);
    }
    update_size(node);
    return node;
}

/* ---- insert ----------------------------------------------------------- */

/* Insert `value` into subtree `root` (owned), returning the new root. On
 * allocation failure sets *ok = 0; the tree keeps exactly its pre-existing
 * nodes. NULL is only returned for an originally-empty subtree, so a parent
 * writing `child = insert_rec(child, ...)` never overwrites a real subtree. */
static RBNode *insert_rec(RBNode *root, int value, int *ok) {
    if (!root) {
        RBNode *n = node_new(value, RB_RED);
        if (!n) {
            *ok = 0;
        }
        return n;
    }
    if (value < root->value) {
        root->left = insert_rec(root->left, value, ok);
        if (!*ok) {
            return root;
        }
    } else if (value > root->value) {
        root->right = insert_rec(root->right, value, ok);
        if (!*ok) {
            return root;
        }
    } else {
        return root; /* duplicate — set semantics, tree unchanged */
    }
    return fix_up(root);
}

/* ---- delete (LLRB) ---------------------------------------------------- */

/* Ensure `node`'s left child (or one of its children) is red so a red node can
 * be pushed down the left spine, per the LLRB delete algorithm. */
static RBNode *move_red_left(RBNode *node) {
    flip_colors(node);
    if (is_red_left(node->right)) {
        if (node->right) {
            node->right = rotate_right(node->right);
        }
        node = rotate_left(node);
        flip_colors(node);
    }
    return node;
}

/* Symmetric helper for descending the right spine. */
static RBNode *move_red_right(RBNode *node) {
    flip_colors(node);
    if (is_red_left(node->left)) {
        node = rotate_right(node);
        flip_colors(node);
    }
    return node;
}

/* Remove the minimum of subtree `node` (owned), writing it to *min_out and
 * returning the remaining subtree. No allocation. */
static RBNode *delete_min(RBNode *node, int *min_out) {
    if (!node->left) {
        RBNode *right = node->right;
        *min_out = node->value;
        free(node); /* node's left is NULL; the node itself is removed */
        return right;
    }
    if (!is_red(node->left) && !is_red_left(node->left)) {
        node = move_red_left(node);
    }
    node->left = delete_min(node->left, min_out);
    return fix_up(node);
}

/* Delete `value` from subtree `node` (owned), returning the new root. No
 * allocation, so this cannot fail. */
static RBNode *delete_rec(RBNode *node, int value) {
    if (!node) {
        return NULL;
    }
    if (value < node->value) {
        if (!is_red(node->left) && !is_red_left(node->left)) {
            node = move_red_left(node);
        }
        node->left = delete_rec(node->left, value);
    } else {
        if (is_red(node->left)) {
            node = rotate_right(node);
        }
        if (value == node->value && node->right == NULL) {
            node_free(node); /* leaf being removed (its left is NULL here) */
            return NULL;
        }
        if (!is_red(node->right) && !is_red_left(node->right)) {
            node = move_red_right(node);
        }
        if (value == node->value) {
            int successor;
            RBNode *right = node->right; /* guaranteed non-NULL here */
            node->right = delete_min(right, &successor);
            node->value = successor;
        } else {
            node->right = delete_rec(node->right, value);
        }
    }
    return fix_up(node);
}

/* ---- public: construction / destruction ------------------------------- */

RBTree *rb_empty(void) {
    RBTree *t = malloc(sizeof *t);
    if (!t) {
        return NULL;
    }
    t->root = NULL;
    return t;
}

void rb_free(RBTree *t) {
    if (!t) {
        return;
    }
    node_free(t->root);
    free(t);
}

/* ---- public: persistent updates --------------------------------------- */

RBTree *rb_insert(const RBTree *t, int value) {
    RBTree *nt = malloc(sizeof *nt);
    int ok = 1;
    RBNode *cloned = NULL;
    RBNode *r;
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
    r = insert_rec(cloned, value, &ok);
    if (!ok) {
        node_free(r);
        free(nt);
        return NULL;
    }
    if (r) {
        r->color = RB_BLACK; /* the root is always black */
    }
    nt->root = r;
    return nt;
}

RBTree *rb_delete(const RBTree *t, int value) {
    RBTree *nt = malloc(sizeof *nt);
    RBNode *cloned = NULL;
    RBNode *r;
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
    r = delete_rec(cloned, value);
    if (r) {
        r->color = RB_BLACK;
    }
    nt->root = r;
    return nt;
}

/* ---- public: queries -------------------------------------------------- */

const RBNode *rb_search(const RBTree *t, int value) {
    const RBNode *cur = t ? t->root : NULL;
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

int rb_contains(const RBTree *t, int value) {
    return rb_search(t, value) != NULL;
}

int rb_min_value(const RBTree *t, int *out) {
    const RBNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->left) {
        cur = cur->left;
    }
    *out = cur->value;
    return 1;
}

int rb_max_value(const RBTree *t, int *out) {
    const RBNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->right) {
        cur = cur->right;
    }
    *out = cur->value;
    return 1;
}

int rb_predecessor(const RBTree *t, int value, int *out) {
    const RBNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (value <= cur->value) {
            cur = cur->left;
        } else {
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

int rb_successor(const RBTree *t, int value, int *out) {
    const RBNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (value >= cur->value) {
            cur = cur->right;
        } else {
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

int rb_kth_smallest(const RBTree *t, size_t k, int *out) {
    const RBNode *cur = t ? t->root : NULL;
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

/* In-order traversal that stops once `buf` (capacity buf_len) is full. */
static void inorder_fill(const RBNode *n, int *buf, size_t buf_len,
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

size_t rb_to_sorted_array(const RBTree *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    if (!buf || buf_len == 0) {
        return 0;
    }
    inorder_fill(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

size_t rb_size(const RBTree *t) { return node_size(t ? t->root : NULL); }

size_t rb_black_height(const RBTree *t) {
    const RBNode *cur = t ? t->root : NULL;
    size_t height = 0;
    while (cur) {
        if (cur->color == RB_BLACK) {
            height++;
        }
        cur = cur->left;
    }
    return height;
}

/* ---- public: validation ----------------------------------------------- */

/* Returns 1 and writes the subtree's black height if it is a valid LLRB subtree
 * bounded by (min, max); 0 otherwise. */
static int validate(const RBNode *n, const int *min, const int *max,
                    size_t *bh_out) {
    size_t lh = 0, rh = 0;
    if (!n) {
        *bh_out = 1;
        return 1;
    }
    if (min && n->value <= *min) {
        return 0;
    }
    if (max && n->value >= *max) {
        return 0;
    }
    if (n->color == RB_RED && (is_red(n->left) || is_red(n->right))) {
        return 0;
    }
    if (!validate(n->left, min, &n->value, &lh)) {
        return 0;
    }
    if (!validate(n->right, &n->value, max, &rh)) {
        return 0;
    }
    if (lh != rh) {
        return 0;
    }
    if (1 + node_size(n->left) + node_size(n->right) != n->size) {
        return 0;
    }
    *bh_out = lh + (n->color == RB_BLACK ? 1u : 0u);
    return 1;
}

int rb_is_valid_rb(const RBTree *t) {
    const RBNode *root = t ? t->root : NULL;
    size_t bh;
    if (root && root->color != RB_BLACK) {
        return 0;
    }
    return validate(root, NULL, NULL, &bh);
}
