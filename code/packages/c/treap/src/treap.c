/*
 * treap.c — implementation of the persistent treap (see treap.h). A faithful
 * port of the Rust `treap` crate: the same rotate-on-insert, merge-on-delete,
 * split/merge, and order-statistic algorithms, and the same persistence
 * (updates deep-copy then work on the copy).
 */
#include "treap.h"

#include <stdint.h> /* uint32_t */
#include <stdlib.h> /* malloc, free */

/* ---- node helpers ----------------------------------------------------- */

static size_t node_size(const TreapNode *n) { return n ? n->size : 0; }

static void update_metadata(TreapNode *n) {
    n->size = 1 + node_size(n->left) + node_size(n->right);
}

static TreapNode *node_new(int key, double priority) {
    TreapNode *n = malloc(sizeof *n);
    if (!n) {
        return NULL;
    }
    n->key = key;
    n->priority = priority;
    n->left = NULL;
    n->right = NULL;
    n->size = 1;
    return n;
}

static void node_free(TreapNode *n) {
    if (!n) {
        return;
    }
    node_free(n->left);
    node_free(n->right);
    free(n);
}

/* Deep-copy a subtree; NULL on allocation failure (partial copy is freed). */
static TreapNode *node_clone_deep(const TreapNode *n) {
    TreapNode *c;
    if (!n) {
        return NULL;
    }
    c = malloc(sizeof *c);
    if (!c) {
        return NULL;
    }
    c->key = n->key;
    c->priority = n->priority;
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

/* ---- default-priority PRNG -------------------------------------------- */

/* next_priority — a value in [0, 1] from a deterministic xorshift generator.
 * The Rust crate holds the seed in an AtomicU32 (fetch_add) for cross-thread
 * safety; a pure-ISO single-threaded port uses a plain static counter with the
 * identical arithmetic. u32 multiply wraps mod 2^32 by definition, so there is
 * no signed overflow / UB. */
static double next_priority(void) {
    static uint32_t seed = 0x9E3779B9u;
    uint32_t state = seed; /* fetch_add returns the value *before* the add */
    seed += 0x9E3779B9u;   /* uint32_t addition wraps (well-defined) */
    state ^= state >> 13;
    state ^= state << 17;
    state ^= state >> 5;
    {
        uint32_t mixed = state * 0x85EBCA6Bu; /* wrapping multiply */
        return (double)mixed / (double)UINT32_MAX;
    }
}

/* ---- rotations -------------------------------------------------------- */

/* Rotate `root` left, lifting its right child. Ownership: takes `root`, returns
 * the new subtree root. */
static TreapNode *rotate_left(TreapNode *root) {
    TreapNode *new_root = root->right;
    if (!new_root) {
        return root;
    }
    root->right = new_root->left;
    update_metadata(root);
    new_root->left = root;
    update_metadata(new_root);
    return new_root;
}

/* Rotate `root` right, lifting its left child. */
static TreapNode *rotate_right(TreapNode *root) {
    TreapNode *new_root = root->left;
    if (!new_root) {
        return root;
    }
    root->left = new_root->right;
    update_metadata(root);
    new_root->right = root;
    update_metadata(new_root);
    return new_root;
}

/* ---- insert / delete / merge / split ---------------------------------- */

/* Insert into subtree `root` (owned); returns the new root. On allocation
 * failure sets *ok = 0. */
static TreapNode *insert_rec(TreapNode *root, int key, double priority,
                            int *ok) {
    if (!root) {
        TreapNode *n = node_new(key, priority);
        if (!n) {
            *ok = 0;
        }
        return n;
    }
    if (key < root->key) {
        root->left = insert_rec(root->left, key, priority, ok);
        if (!*ok) {
            return root;
        }
        if (root->left && root->left->priority > root->priority) {
            root = rotate_right(root);
        }
    } else if (key > root->key) {
        root->right = insert_rec(root->right, key, priority, ok);
        if (!*ok) {
            return root;
        }
        if (root->right && root->right->priority > root->priority) {
            root = rotate_left(root);
        }
    } else {
        return root; /* duplicate key — no-op */
    }
    update_metadata(root);
    return root;
}

/* Merge two key-disjoint subtrees (all left keys < all right keys), owning
 * both, restoring the max-heap on priorities. No allocation, cannot fail. */
static TreapNode *merge_nodes(TreapNode *left, TreapNode *right) {
    if (!left) {
        return right;
    }
    if (!right) {
        return left;
    }
    if (left->priority >= right->priority) {
        left->right = merge_nodes(left->right, right);
        update_metadata(left);
        return left;
    }
    right->left = merge_nodes(left, right->left);
    update_metadata(right);
    return right;
}

/* Delete `key` from subtree `root` (owned); returns the new root. No
 * allocation, cannot fail. */
static TreapNode *delete_rec(TreapNode *root, int key) {
    if (!root) {
        return NULL;
    }
    if (key < root->key) {
        root->left = delete_rec(root->left, key);
        update_metadata(root);
        return root;
    }
    if (key > root->key) {
        root->right = delete_rec(root->right, key);
        update_metadata(root);
        return root;
    }
    /* Equal — drop this node, merging its two children. */
    {
        TreapNode *left = root->left;
        TreapNode *right = root->right;
        free(root);
        return merge_nodes(left, right);
    }
}

/* Split subtree `node` (owned) by `key` into (<= key) and (> key). */
static void split_nodes(TreapNode *node, int key, TreapNode **left_out,
                        TreapNode **right_out) {
    if (!node) {
        *left_out = NULL;
        *right_out = NULL;
        return;
    }
    if (key < node->key) {
        TreapNode *l;
        TreapNode *r;
        split_nodes(node->left, key, &l, &r);
        node->left = r;
        update_metadata(node);
        *left_out = l;
        *right_out = node;
    } else { /* key >= node->key: this node belongs on the left */
        TreapNode *l;
        TreapNode *r;
        split_nodes(node->right, key, &l, &r);
        node->right = l;
        update_metadata(node);
        *left_out = node;
        *right_out = r;
    }
}

/* ---- public: construction / destruction ------------------------------- */

Treap *treap_empty(void) {
    Treap *t = malloc(sizeof *t);
    if (!t) {
        return NULL;
    }
    t->root = NULL;
    return t;
}

void treap_free(Treap *t) {
    if (!t) {
        return;
    }
    node_free(t->root);
    free(t);
}

/* ---- public: persistent updates --------------------------------------- */

Treap *treap_insert(const Treap *t, int key, const double *priority) {
    Treap *nt = malloc(sizeof *nt);
    TreapNode *cloned = NULL;
    double prio;
    int ok = 1;
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
    prio = priority ? *priority : next_priority();
    nt->root = insert_rec(cloned, key, prio, &ok);
    if (!ok) {
        node_free(nt->root);
        free(nt);
        return NULL;
    }
    return nt;
}

Treap *treap_delete(const Treap *t, int key) {
    Treap *nt = malloc(sizeof *nt);
    TreapNode *cloned = NULL;
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
    nt->root = delete_rec(cloned, key);
    return nt;
}

int treap_split(const Treap *t, int key, Treap **left_out, Treap **right_out) {
    Treap *lt = malloc(sizeof *lt);
    Treap *rt = malloc(sizeof *rt);
    TreapNode *cloned = NULL;
    TreapNode *l = NULL;
    TreapNode *r = NULL;
    if (!lt || !rt) {
        free(lt);
        free(rt);
        return 0;
    }
    if (t && t->root) {
        cloned = node_clone_deep(t->root);
        if (!cloned) {
            free(lt);
            free(rt);
            return 0;
        }
    }
    split_nodes(cloned, key, &l, &r);
    lt->root = l;
    rt->root = r;
    *left_out = lt;
    *right_out = rt;
    return 1;
}

Treap *treap_merge(const Treap *left, const Treap *right) {
    Treap *nt = malloc(sizeof *nt);
    TreapNode *lc = NULL;
    TreapNode *rc = NULL;
    if (!nt) {
        return NULL;
    }
    if (left && left->root) {
        lc = node_clone_deep(left->root);
        if (!lc) {
            free(nt);
            return NULL;
        }
    }
    if (right && right->root) {
        rc = node_clone_deep(right->root);
        if (!rc) {
            node_free(lc);
            free(nt);
            return NULL;
        }
    }
    nt->root = merge_nodes(lc, rc);
    return nt;
}

/* ---- public: queries -------------------------------------------------- */

const TreapNode *treap_search(const Treap *t, int key) {
    const TreapNode *cur = t ? t->root : NULL;
    while (cur) {
        if (key < cur->key) {
            cur = cur->left;
        } else if (key > cur->key) {
            cur = cur->right;
        } else {
            return cur;
        }
    }
    return NULL;
}

int treap_contains(const Treap *t, int key) {
    return treap_search(t, key) != NULL;
}

int treap_min_key(const Treap *t, int *out) {
    const TreapNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->left) {
        cur = cur->left;
    }
    *out = cur->key;
    return 1;
}

int treap_max_key(const Treap *t, int *out) {
    const TreapNode *cur = t ? t->root : NULL;
    if (!cur) {
        return 0;
    }
    while (cur->right) {
        cur = cur->right;
    }
    *out = cur->key;
    return 1;
}

int treap_predecessor(const Treap *t, int key, int *out) {
    const TreapNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (key <= cur->key) {
            cur = cur->left;
        } else {
            best = cur->key;
            found = 1;
            cur = cur->right;
        }
    }
    if (found) {
        *out = best;
    }
    return found;
}

int treap_successor(const Treap *t, int key, int *out) {
    const TreapNode *cur = t ? t->root : NULL;
    int found = 0;
    int best = 0;
    while (cur) {
        if (key >= cur->key) {
            cur = cur->right;
        } else {
            best = cur->key;
            found = 1;
            cur = cur->left;
        }
    }
    if (found) {
        *out = best;
    }
    return found;
}

int treap_kth_smallest(const Treap *t, size_t k, int *out) {
    const TreapNode *cur = t ? t->root : NULL;
    if (k == 0) {
        return 0;
    }
    while (cur) {
        size_t ls = node_size(cur->left);
        if (k == ls + 1) {
            *out = cur->key;
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

static void inorder_fill(const TreapNode *n, int *buf, size_t buf_len,
                         size_t *idx) {
    if (!n || *idx >= buf_len) {
        return;
    }
    inorder_fill(n->left, buf, buf_len, idx);
    if (*idx < buf_len) {
        buf[*idx] = n->key;
        (*idx)++;
    }
    inorder_fill(n->right, buf, buf_len, idx);
}

size_t treap_to_sorted_array(const Treap *t, int *buf, size_t buf_len) {
    size_t idx = 0;
    if (!buf || buf_len == 0) {
        return 0;
    }
    inorder_fill(t ? t->root : NULL, buf, buf_len, &idx);
    return idx;
}

size_t treap_size(const Treap *t) { return node_size(t ? t->root : NULL); }

static long node_height(const TreapNode *n) {
    long lh, rh;
    if (!n) {
        return -1;
    }
    lh = node_height(n->left);
    rh = node_height(n->right);
    return 1 + (lh > rh ? lh : rh);
}

long treap_height(const Treap *t) { return node_height(t ? t->root : NULL); }

/* Validate BST order, max-heap order, and cached sizes. `min`/`max` are
 * exclusive/inclusive key bounds (NULL = unbounded); `parent_prio` is the
 * enclosing node's priority (NULL at the root). Returns -1 on any violation,
 * else the subtree size. */
static long validate(const TreapNode *n, const int *min, const int *max,
                     const double *parent_prio) {
    long left, right;
    if (!n) {
        return 0;
    }
    if (min && n->key <= *min) {
        return -1;
    }
    if (max && n->key > *max) {
        return -1;
    }
    if (parent_prio && n->priority > *parent_prio) {
        return -1;
    }
    left = validate(n->left, min, &n->key, &n->priority);
    if (left < 0) {
        return -1;
    }
    right = validate(n->right, &n->key, max, &n->priority);
    if (right < 0) {
        return -1;
    }
    if (n->size != (size_t)(1 + left + right)) {
        return -1;
    }
    return 1 + left + right;
}

int treap_is_valid(const Treap *t) {
    return validate(t ? t->root : NULL, NULL, NULL, NULL) >= 0;
}
