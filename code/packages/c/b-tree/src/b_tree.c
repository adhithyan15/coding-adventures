/*
 * b_tree.c — implementation of the B-tree (see b_tree.h). A faithful port of the
 * Rust `b-tree` crate's CLRS algorithm, specialised to long -> long.
 *
 * Each node stores its keys/values in fixed-capacity arrays of size 2t-1 and its
 * child pointers in an array of size 2t. Because insertion splits a child before
 * descending (so a node is never over-full) and deletion pre-fills before
 * descending, a node never exceeds these capacities — which makes the memory
 * management here straightforward and overflow-free.
 */
#include "b_tree.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memmove, memcpy */

typedef struct btree_node {
    long *keys;                   /* capacity 2t-1 */
    long *values;                 /* capacity 2t-1 */
    struct btree_node **children; /* capacity 2t (only used when !is_leaf) */
    size_t n;                     /* number of keys */
    int is_leaf;
} btree_node;

struct btree {
    btree_node *root;
    size_t t;
    size_t size;
};

/* ── node lifetime ────────────────────────────────────────────────────────── */
static btree_node *node_new(size_t t, int is_leaf) {
    btree_node *nd = (btree_node *)malloc(sizeof *nd);
    if (nd == NULL) {
        return NULL;
    }
    /* calloc does the checked multiply; 2t-1 / 2t cannot overflow because
     * btree_new clamps t (see below). */
    nd->keys = (long *)calloc(2 * t - 1, sizeof(long));
    nd->values = (long *)calloc(2 * t - 1, sizeof(long));
    nd->children = (btree_node **)calloc(2 * t, sizeof(btree_node *));
    if (nd->keys == NULL || nd->values == NULL || nd->children == NULL) {
        free(nd->keys);
        free(nd->values);
        free(nd->children);
        free(nd);
        return NULL;
    }
    nd->n = 0;
    nd->is_leaf = is_leaf;
    return nd;
}

/* Free just this node's arrays and struct (children are NOT recursed into). */
static void node_free_shallow(btree_node *nd) {
    free(nd->keys);
    free(nd->values);
    free(nd->children);
    free(nd);
}

/* Free this node and, recursively, its whole subtree. */
static void node_free(btree_node *nd) {
    if (nd == NULL) {
        return;
    }
    if (!nd->is_leaf) {
        size_t i;
        for (i = 0; i <= nd->n; i++) {
            node_free(nd->children[i]);
        }
    }
    node_free_shallow(nd);
}

/* ── search ───────────────────────────────────────────────────────────────── */
/* Binary search: sets *pos to the match index (found) or the descent child
 * index (not found). */
static int find_pos(const btree_node *nd, long key, size_t *pos) {
    size_t lo = 0, hi = nd->n;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (nd->keys[mid] == key) {
            *pos = mid;
            return 1;
        }
        if (nd->keys[mid] < key) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *pos = lo;
    return 0;
}

static int node_search(const btree_node *nd, long key, long *out) {
    size_t i;
    if (find_pos(nd, key, &i)) {
        if (out != NULL) {
            *out = nd->values[i];
        }
        return 1;
    }
    if (nd->is_leaf) {
        return 0;
    }
    return node_search(nd->children[i], key, out);
}

/* ── insertion ────────────────────────────────────────────────────────────── */
/* Insert (k, v) into a non-full node's key/value arrays at index i. */
static void node_insert_kv(btree_node *nd, size_t i, long k, long v) {
    memmove(&nd->keys[i + 1], &nd->keys[i], (nd->n - i) * sizeof(long));
    memmove(&nd->values[i + 1], &nd->values[i], (nd->n - i) * sizeof(long));
    nd->keys[i] = k;
    nd->values[i] = v;
    nd->n++;
}

/* Split parent->children[ci], which must be full (2t-1 keys). Allocates the new
 * right sibling first, so on failure nothing is mutated. Returns 0 on OOM. */
static int split_child(btree_node *parent, size_t ci, size_t t) {
    btree_node *full = parent->children[ci];
    size_t median = t - 1;                 /* key to promote */
    size_t rcount = full->n - 1 - median;  /* = t-1 keys go to the right node */
    btree_node *right = node_new(t, full->is_leaf);
    long mk, mv;
    if (right == NULL) {
        return 0;
    }
    memcpy(right->keys, &full->keys[median + 1], rcount * sizeof(long));
    memcpy(right->values, &full->values[median + 1], rcount * sizeof(long));
    right->n = rcount;
    if (!full->is_leaf) {
        memcpy(right->children, &full->children[median + 1],
               (rcount + 1) * sizeof(btree_node *));
    }
    mk = full->keys[median];
    mv = full->values[median];
    full->n = median; /* full keeps the left t-1 keys (and t children) */

    /* Insert the median and the new right child into the parent. */
    memmove(&parent->keys[ci + 1], &parent->keys[ci],
            (parent->n - ci) * sizeof(long));
    memmove(&parent->values[ci + 1], &parent->values[ci],
            (parent->n - ci) * sizeof(long));
    parent->keys[ci] = mk;
    parent->values[ci] = mv;
    /* parent currently has parent->n+1 children; open a slot at ci+1. */
    memmove(&parent->children[ci + 2], &parent->children[ci + 1],
            (parent->n - ci) * sizeof(btree_node *));
    parent->children[ci + 1] = right;
    parent->n++;
    return 1;
}

/* Insert into a node guaranteed non-full. Returns 0 on OOM; sets *grew. */
static int insert_non_full(btree_node *nd, long key, long value, size_t t,
                           int *grew) {
    size_t pos;
    size_t idx;
    if (find_pos(nd, key, &pos)) {
        nd->values[pos] = value; /* key exists → overwrite */
        *grew = 0;
        return 1;
    }
    if (nd->is_leaf) {
        node_insert_kv(nd, pos, key, value);
        *grew = 1;
        return 1;
    }
    idx = pos;
    if (nd->children[idx]->n == 2 * t - 1) { /* child full → split first */
        if (!split_child(nd, idx, t)) {
            return 0;
        }
        if (key == nd->keys[idx]) {
            nd->values[idx] = value;
            *grew = 0;
            return 1;
        }
        if (key > nd->keys[idx]) {
            idx++;
        }
    }
    return insert_non_full(nd->children[idx], key, value, t, grew);
}

int btree_insert(btree *tree, long key, long value) {
    size_t t = tree->t;
    int grew = 0;
    if (tree->root == NULL) {
        btree_node *leaf = node_new(t, 1);
        if (leaf == NULL) {
            return 0;
        }
        leaf->keys[0] = key;
        leaf->values[0] = value;
        leaf->n = 1;
        tree->root = leaf;
        tree->size++;
        return 1;
    }
    if (tree->root->n == 2 * t - 1) {
        /* Root full → grow height: new internal root over the old root. */
        btree_node *nr = node_new(t, 0);
        if (nr == NULL) {
            return 0;
        }
        nr->children[0] = tree->root;
        nr->n = 0;
        if (!split_child(nr, 0, t)) {
            nr->children[0] = NULL; /* leave the old root intact */
            node_free_shallow(nr);
            return 0;
        }
        tree->root = nr; /* split succeeded → the new root is a valid tree */
        if (!insert_non_full(nr, key, value, t, &grew)) {
            return 0;
        }
        if (grew) {
            tree->size++;
        }
        return 1;
    }
    if (!insert_non_full(tree->root, key, value, t, &grew)) {
        return 0;
    }
    if (grew) {
        tree->size++;
    }
    return 1;
}

/* ── deletion ─────────────────────────────────────────────────────────────── */
static void node_remove_kv(btree_node *nd, size_t i) {
    memmove(&nd->keys[i], &nd->keys[i + 1], (nd->n - i - 1) * sizeof(long));
    memmove(&nd->values[i], &nd->values[i + 1], (nd->n - i - 1) * sizeof(long));
    nd->n--;
}

static void predecessor(btree_node *nd, size_t idx, long *k, long *v) {
    btree_node *n = nd->children[idx];
    while (!n->is_leaf) {
        n = n->children[n->n];
    }
    *k = n->keys[n->n - 1];
    *v = n->values[n->n - 1];
}

static void successor(btree_node *nd, size_t idx, long *k, long *v) {
    btree_node *n = nd->children[idx + 1];
    while (!n->is_leaf) {
        n = n->children[0];
    }
    *k = n->keys[0];
    *v = n->values[0];
}

/* Merge children[idx] and children[idx+1] with keys[idx] as the separator. */
static void merge_children(btree_node *nd, size_t idx) {
    btree_node *right = nd->children[idx + 1];
    long sk = nd->keys[idx];
    long sv = nd->values[idx];
    btree_node *left;
    size_t lo;
    /* Remove the right child pointer and the separator key from `nd`. */
    memmove(&nd->children[idx + 1], &nd->children[idx + 2],
            (nd->n - idx - 1) * sizeof(btree_node *));
    memmove(&nd->keys[idx], &nd->keys[idx + 1],
            (nd->n - idx - 1) * sizeof(long));
    memmove(&nd->values[idx], &nd->values[idx + 1],
            (nd->n - idx - 1) * sizeof(long));
    nd->n--;
    /* Append separator + right's contents into the left child. */
    left = nd->children[idx];
    lo = left->n;
    left->keys[lo] = sk;
    left->values[lo] = sv;
    memcpy(&left->keys[lo + 1], right->keys, right->n * sizeof(long));
    memcpy(&left->values[lo + 1], right->values, right->n * sizeof(long));
    if (!left->is_leaf) {
        memcpy(&left->children[lo + 1], right->children,
               (right->n + 1) * sizeof(btree_node *));
    }
    left->n = lo + 1 + right->n;
    node_free_shallow(right); /* right's children were moved into left */
}

/* Borrow from the right sibling: rotate keys[idx] down into children[idx] and
 * pull children[idx+1]'s first key up. */
static void rotate_left(btree_node *nd, size_t idx) {
    btree_node *left = nd->children[idx];
    btree_node *right = nd->children[idx + 1];
    size_t li = left->n;
    long nk, nv;
    left->keys[li] = nd->keys[idx];
    left->values[li] = nd->values[idx];
    if (!left->is_leaf) {
        left->children[li + 1] = right->children[0];
        memmove(&right->children[0], &right->children[1],
                right->n * sizeof(btree_node *));
    }
    left->n++;
    nk = right->keys[0];
    nv = right->values[0];
    memmove(&right->keys[0], &right->keys[1], (right->n - 1) * sizeof(long));
    memmove(&right->values[0], &right->values[1], (right->n - 1) * sizeof(long));
    right->n--;
    nd->keys[idx] = nk;
    nd->values[idx] = nv;
}

/* Borrow from the left sibling: rotate keys[idx-1] down into children[idx] and
 * pull children[idx-1]'s last key up. */
static void rotate_right(btree_node *nd, size_t idx) {
    btree_node *right = nd->children[idx];
    btree_node *left = nd->children[idx - 1];
    memmove(&right->keys[1], &right->keys[0], right->n * sizeof(long));
    memmove(&right->values[1], &right->values[0], right->n * sizeof(long));
    if (!right->is_leaf) {
        memmove(&right->children[1], &right->children[0],
                (right->n + 1) * sizeof(btree_node *));
        right->children[0] = left->children[left->n]; /* left's last child */
    }
    right->keys[0] = nd->keys[idx - 1];
    right->values[0] = nd->values[idx - 1];
    right->n++;
    left->n--;
    nd->keys[idx - 1] = left->keys[left->n];
    nd->values[idx - 1] = left->values[left->n];
}

/* Ensure children[idx] has >= t keys before descending; returns the (possibly
 * shifted) child index to continue into. */
static size_t ensure_child_has_t_keys(btree_node *nd, size_t idx, size_t t) {
    int has_left, has_right;
    if (nd->children[idx]->n >= t) {
        return idx;
    }
    has_left = idx > 0;
    has_right = idx + 1 < nd->n + 1;
    if (has_left && nd->children[idx - 1]->n >= t) {
        rotate_right(nd, idx);
        return idx;
    }
    if (has_right && nd->children[idx + 1]->n >= t) {
        rotate_left(nd, idx);
        return idx;
    }
    if (has_left) {
        merge_children(nd, idx - 1);
        return idx - 1;
    }
    merge_children(nd, idx);
    return idx;
}

static int node_delete(btree_node *nd, long key, size_t t) {
    size_t i;
    if (find_pos(nd, key, &i)) {
        if (nd->is_leaf) {
            node_remove_kv(nd, i);
            return 1;
        }
        if (nd->children[i]->n >= t) {
            long pk, pv;
            predecessor(nd, i, &pk, &pv);
            nd->keys[i] = pk;
            nd->values[i] = pv;
            return node_delete(nd->children[i], pk, t);
        }
        if (nd->children[i + 1]->n >= t) {
            long sk, sv;
            successor(nd, i, &sk, &sv);
            nd->keys[i] = sk;
            nd->values[i] = sv;
            return node_delete(nd->children[i + 1], sk, t);
        }
        merge_children(nd, i);
        return node_delete(nd->children[i], key, t);
    }
    if (nd->is_leaf) {
        return 0;
    }
    {
        size_t ni = ensure_child_has_t_keys(nd, i, t);
        return node_delete(nd->children[ni], key, t);
    }
}

int btree_delete(btree *tree, long key) {
    int deleted;
    if (tree->root == NULL) {
        return 0;
    }
    deleted = node_delete(tree->root, key, tree->t);
    if (deleted) {
        tree->size--;
        if (tree->root->n == 0) {
            btree_node *old = tree->root;
            if (!old->is_leaf) {
                tree->root = old->children[0]; /* shrink height */
            } else {
                tree->root = NULL; /* tree emptied */
            }
            node_free_shallow(old);
        }
    }
    return deleted;
}

/* ── queries ──────────────────────────────────────────────────────────────── */
int btree_search(const btree *tree, long key, long *out_value) {
    if (tree->root == NULL) {
        return 0;
    }
    return node_search(tree->root, key, out_value);
}

int btree_contains(const btree *tree, long key) {
    return btree_search(tree, key, NULL);
}

static const btree_node *node_min(const btree_node *nd) {
    while (!nd->is_leaf) {
        nd = nd->children[0];
    }
    return nd;
}
static const btree_node *node_max(const btree_node *nd) {
    while (!nd->is_leaf) {
        nd = nd->children[nd->n];
    }
    return nd;
}

int btree_min_key(const btree *tree, long *out) {
    const btree_node *nd;
    if (tree->root == NULL) {
        return 0;
    }
    nd = node_min(tree->root);
    if (out != NULL) {
        *out = nd->keys[0];
    }
    return 1;
}

int btree_max_key(const btree *tree, long *out) {
    const btree_node *nd;
    if (tree->root == NULL) {
        return 0;
    }
    nd = node_max(tree->root);
    if (out != NULL) {
        *out = nd->keys[nd->n - 1];
    }
    return 1;
}

size_t btree_len(const btree *tree) { return tree->size; }
int btree_is_empty(const btree *tree) { return tree->size == 0; }

static size_t node_height(const btree_node *nd) {
    if (nd->is_leaf) {
        return 0;
    }
    return 1 + node_height(nd->children[0]);
}
size_t btree_height(const btree *tree) {
    if (tree->root == NULL) {
        return 0;
    }
    return node_height(tree->root);
}

/* ── traversals ───────────────────────────────────────────────────────────── */
static void node_inorder(const btree_node *nd, btree_visit_fn fn, void *user) {
    size_t i;
    if (nd->is_leaf) {
        for (i = 0; i < nd->n; i++) {
            fn(nd->keys[i], nd->values[i], user);
        }
    } else {
        for (i = 0; i < nd->n; i++) {
            node_inorder(nd->children[i], fn, user);
            fn(nd->keys[i], nd->values[i], user);
        }
        node_inorder(nd->children[nd->n], fn, user);
    }
}

void btree_inorder(const btree *tree, btree_visit_fn fn, void *user) {
    if (tree->root != NULL) {
        node_inorder(tree->root, fn, user);
    }
}

static void node_range(const btree_node *nd, long low, long high,
                       btree_visit_fn fn, void *user) {
    size_t i;
    if (nd->is_leaf) {
        for (i = 0; i < nd->n; i++) {
            if (nd->keys[i] >= low && nd->keys[i] <= high) {
                fn(nd->keys[i], nd->values[i], user);
            }
        }
        return;
    }
    for (i = 0; i < nd->n; i++) {
        if (nd->keys[i] > low) {
            node_range(nd->children[i], low, high, fn, user);
        }
        if (nd->keys[i] >= low && nd->keys[i] <= high) {
            fn(nd->keys[i], nd->values[i], user);
        }
        if (nd->keys[i] >= high) {
            return;
        }
    }
    node_range(nd->children[nd->n], low, high, fn, user);
}

void btree_range_query(const btree *tree, long low, long high,
                       btree_visit_fn fn, void *user) {
    if (tree->root != NULL) {
        node_range(tree->root, low, high, fn, user);
    }
}

/* ── validation ───────────────────────────────────────────────────────────── */
static int node_validate(const btree_node *nd, size_t t, int is_root,
                         size_t depth, size_t *out_leaf_depth) {
    size_t min_keys = is_root ? 1 : t - 1;
    size_t max_keys = 2 * t - 1;
    size_t i;
    if (nd->n != 0 && (nd->n < min_keys || nd->n > max_keys)) {
        *out_leaf_depth = depth;
        return 0;
    }
    for (i = 1; i < nd->n; i++) {
        if (nd->keys[i - 1] >= nd->keys[i]) {
            *out_leaf_depth = depth;
            return 0;
        }
    }
    if (nd->is_leaf) {
        *out_leaf_depth = depth;
        return 1;
    }
    {
        int have = 0;
        size_t ld = depth;
        for (i = 0; i <= nd->n; i++) {
            size_t cd;
            if (!node_validate(nd->children[i], t, 0, depth + 1, &cd)) {
                *out_leaf_depth = depth;
                return 0;
            }
            if (!have) {
                have = 1;
                ld = cd;
            } else if (ld != cd) {
                *out_leaf_depth = depth;
                return 0;
            }
        }
        *out_leaf_depth = ld;
        return 1;
    }
}

int btree_is_valid(const btree *tree) {
    size_t ld;
    if (tree->root == NULL || tree->root->n == 0) {
        return 1;
    }
    return node_validate(tree->root, tree->t, 1, 0, &ld);
}

/* ── tree lifetime ────────────────────────────────────────────────────────── */
btree *btree_new(size_t t) {
    btree *tree = (btree *)malloc(sizeof *tree);
    if (tree == NULL) {
        return NULL;
    }
    if (t < 2) {
        t = 2;
    }
    if (t > SIZE_MAX / 4) {
        t = SIZE_MAX / 4; /* keep 2t and 2t-1 from overflowing */
    }
    tree->root = NULL;
    tree->t = t;
    tree->size = 0;
    return tree;
}

void btree_free(btree *tree) {
    if (tree == NULL) {
        return;
    }
    node_free(tree->root);
    free(tree);
}
