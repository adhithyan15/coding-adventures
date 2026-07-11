/*
 * b_plus_tree.c — implementation of the B+ tree (see b_plus_tree.h). A faithful
 * port of the Rust `b-plus-tree` crate, specialised to long -> long.
 *
 * Node arrays are fixed-capacity, sized by the degree t:
 *   - leaf:     keys[2t], values[2t], plus a `next` chain pointer
 *   - internal: keys[2t], children[2t+1]
 * A node reaches at most 2t keys (transiently, at the moment it splits), so
 * these capacities are never exceeded.
 *
 * Insert propagates splits bottom-up. To stay allocation-safe, each level that
 * is full pre-allocates its split node BEFORE mutating anything, so an
 * out-of-memory condition leaves the whole tree unchanged and valid.
 */
#include "b_plus_tree.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* memmove, memcpy */

typedef struct bpt_node {
    int is_leaf;
    size_t n;                     /* key count */
    long *keys;                   /* capacity 2t */
    long *values;                 /* leaf: capacity 2t; internal: NULL */
    struct bpt_node *next;        /* leaf: next leaf in key order; else NULL */
    struct bpt_node **children;   /* internal: capacity 2t+1; leaf: NULL */
} bpt_node;

struct bpt {
    bpt_node *root;       /* always non-NULL (starts as an empty leaf) */
    bpt_node *first_leaf; /* leftmost leaf, for O(1) full-scan start */
    size_t t;
    size_t size;
};

/* Result of an insertion bubbling up from a subtree. */
typedef struct {
    int did_split;
    long sep_key;
    bpt_node *right_child;
    int grew;
} insert_result;

/* ── node lifetime ────────────────────────────────────────────────────────── */
static bpt_node *node_new_leaf(size_t t) {
    bpt_node *nd = (bpt_node *)malloc(sizeof *nd);
    if (nd == NULL) {
        return NULL;
    }
    nd->keys = (long *)calloc(2 * t, sizeof(long));
    nd->values = (long *)calloc(2 * t, sizeof(long));
    nd->children = NULL;
    if (nd->keys == NULL || nd->values == NULL) {
        free(nd->keys);
        free(nd->values);
        free(nd);
        return NULL;
    }
    nd->is_leaf = 1;
    nd->n = 0;
    nd->next = NULL;
    return nd;
}

static bpt_node *node_new_internal(size_t t) {
    bpt_node *nd = (bpt_node *)malloc(sizeof *nd);
    if (nd == NULL) {
        return NULL;
    }
    nd->keys = (long *)calloc(2 * t, sizeof(long));
    nd->children = (bpt_node **)calloc(2 * t + 1, sizeof(bpt_node *));
    nd->values = NULL;
    if (nd->keys == NULL || nd->children == NULL) {
        free(nd->keys);
        free(nd->children);
        free(nd);
        return NULL;
    }
    nd->is_leaf = 0;
    nd->n = 0;
    nd->next = NULL;
    return nd;
}

/* Free a node's own arrays and struct; do NOT recurse into children or `next`. */
static void node_free_shallow(bpt_node *nd) {
    free(nd->keys);
    free(nd->values);
    free(nd->children);
    free(nd);
}

/* Free a node and its whole subtree (children only — never the `next` chain,
 * which is owned through the ownership tree, not the leaf list). */
static void node_free(bpt_node *nd) {
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

/* ── array searches ───────────────────────────────────────────────────────── */
/* Exact match: sets *pos and returns 1 if found. */
static int find_exact(const long *keys, size_t n, long key, size_t *pos) {
    size_t lo = 0, hi = n;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (keys[mid] == key) {
            *pos = mid;
            return 1;
        }
        if (keys[mid] < key) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *pos = lo;
    return 0;
}

/* Number of keys <= key (partition point for descent). */
static size_t count_le(const long *keys, size_t n, long key) {
    size_t lo = 0, hi = n;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (keys[mid] <= key) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    return lo;
}

/* Number of keys < key (insertion point for a separator). */
static size_t count_lt(const long *keys, size_t n, long key) {
    size_t lo = 0, hi = n;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        if (keys[mid] < key) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    return lo;
}

static size_t child_index(const bpt_node *nd, long key) {
    size_t ci = count_le(nd->keys, nd->n, key);
    return ci > nd->n ? nd->n : ci; /* clamp to the last child */
}

static bpt_node *find_leaf(bpt_node *node, long key) {
    while (!node->is_leaf) {
        node = node->children[child_index(node, key)];
    }
    return node;
}

static bpt_node *leftmost_leaf(bpt_node *node) {
    while (!node->is_leaf) {
        node = node->children[0];
    }
    return node;
}

/* ── insertion ────────────────────────────────────────────────────────────── */
static int insert_node(bpt_node *node, long key, long value, size_t t,
                       insert_result *out);

static int insert_leaf(bpt_node *leaf, long key, long value, size_t t,
                       insert_result *out) {
    size_t pos;
    bpt_node *right = NULL;
    if (find_exact(leaf->keys, leaf->n, key, &pos)) {
        leaf->values[pos] = value; /* overwrite */
        out->did_split = 0;
        out->grew = 0;
        return 1;
    }
    /* A full leaf (2t-1 keys) will overflow to 2t and split: pre-allocate. */
    if (leaf->n == 2 * t - 1) {
        right = node_new_leaf(t);
        if (right == NULL) {
            return 0; /* leaf untouched */
        }
    }
    memmove(&leaf->keys[pos + 1], &leaf->keys[pos], (leaf->n - pos) * sizeof(long));
    memmove(&leaf->values[pos + 1], &leaf->values[pos],
            (leaf->n - pos) * sizeof(long));
    leaf->keys[pos] = key;
    leaf->values[pos] = value;
    leaf->n++;
    if (right != NULL) {
        size_t rc = leaf->n - t; /* = t */
        memcpy(right->keys, &leaf->keys[t], rc * sizeof(long));
        memcpy(right->values, &leaf->values[t], rc * sizeof(long));
        right->n = rc;
        leaf->n = t;
        right->next = leaf->next; /* splice into the leaf chain */
        leaf->next = right;
        out->did_split = 1;
        out->sep_key = right->keys[0]; /* separator = right's first key (a copy) */
        out->right_child = right;
        out->grew = 1;
        return 1;
    }
    out->did_split = 0;
    out->grew = 1;
    return 1;
}

static int insert_internal(bpt_node *node, long key, long value, size_t t,
                           insert_result *out) {
    bpt_node *rint = NULL;
    size_t ci;
    insert_result child;
    if (node->n == 2 * t - 1) { /* a full node may split after this insert */
        rint = node_new_internal(t);
        if (rint == NULL) {
            return 0;
        }
    }
    ci = child_index(node, key);
    if (!insert_node(node->children[ci], key, value, t, &child)) {
        node_free(rint);
        return 0;
    }
    if (!child.did_split) {
        node_free(rint); /* not needed */
        out->did_split = 0;
        out->grew = child.grew;
        return 1;
    }
    /* Insert the child's separator + new right child into this node. */
    {
        size_t pos = count_lt(node->keys, node->n, child.sep_key);
        memmove(&node->keys[pos + 1], &node->keys[pos],
                (node->n - pos) * sizeof(long));
        node->keys[pos] = child.sep_key;
        memmove(&node->children[pos + 2], &node->children[pos + 1],
                (node->n - pos) * sizeof(bpt_node *));
        node->children[pos + 1] = child.right_child;
        node->n++;
    }
    if (node->n >= 2 * t) { /* split this internal node (rint pre-allocated) */
        size_t mid = t - 1;
        long promote = node->keys[mid];
        size_t rk = node->n - (mid + 1);  /* right keys */
        size_t rch = node->n - mid;       /* right children */
        memcpy(rint->keys, &node->keys[mid + 1], rk * sizeof(long));
        memcpy(rint->children, &node->children[mid + 1], rch * sizeof(bpt_node *));
        rint->n = rk;
        node->n = mid; /* left keeps keys[0..mid-1] and children[0..mid] */
        out->did_split = 1;
        out->sep_key = promote;
        out->right_child = rint;
        out->grew = child.grew;
        return 1;
    }
    node_free(rint); /* NULL here (node was not full) — defensive */
    out->did_split = 0;
    out->grew = child.grew;
    return 1;
}

static int insert_node(bpt_node *node, long key, long value, size_t t,
                       insert_result *out) {
    if (node->is_leaf) {
        return insert_leaf(node, key, value, t, out);
    }
    return insert_internal(node, key, value, t, out);
}

int bpt_insert(bpt *tree, long key, long value) {
    size_t t = tree->t;
    insert_result r;
    bpt_node *new_root = NULL;
    if (tree->root->n == 2 * t - 1) { /* root may split → pre-allocate new root */
        new_root = node_new_internal(t);
        if (new_root == NULL) {
            return 0;
        }
    }
    if (!insert_node(tree->root, key, value, t, &r)) {
        node_free(new_root);
        return 0;
    }
    if (r.did_split) {
        new_root->keys[0] = r.sep_key;
        new_root->children[0] = tree->root;
        new_root->children[1] = r.right_child;
        new_root->n = 1;
        tree->root = new_root;
    } else {
        node_free(new_root); /* unused */
    }
    if (r.grew) {
        tree->size++;
    }
    tree->first_leaf = leftmost_leaf(tree->root);
    return 1;
}

/* ── deletion (allocation-free) ───────────────────────────────────────────── */
static int leftmost_key(const bpt_node *nd, long *out) {
    while (!nd->is_leaf) {
        nd = nd->children[0];
    }
    if (nd->n == 0) {
        return 0;
    }
    *out = nd->keys[0];
    return 1;
}

static void maybe_update_separator(bpt_node *node, size_t ci) {
    if (ci > 0 && ci <= node->n) {
        long lk;
        if (leftmost_key(node->children[ci], &lk)) {
            node->keys[ci - 1] = lk;
        }
    }
}

static void borrow_from_left(bpt_node *node, size_t ci) {
    bpt_node *left = node->children[ci - 1];
    bpt_node *right = node->children[ci];
    if (right->is_leaf) {
        long bk = left->keys[left->n - 1];
        long bv = left->values[left->n - 1];
        left->n--;
        memmove(&right->keys[1], &right->keys[0], right->n * sizeof(long));
        memmove(&right->values[1], &right->values[0], right->n * sizeof(long));
        right->keys[0] = bk;
        right->values[0] = bv;
        right->n++;
        node->keys[ci - 1] = right->keys[0];
    } else {
        long sep = node->keys[ci - 1];
        bpt_node *llc = left->children[left->n]; /* left's last child */
        long llk = left->keys[left->n - 1];
        left->n--;
        node->keys[ci - 1] = llk;
        memmove(&right->keys[1], &right->keys[0], right->n * sizeof(long));
        right->keys[0] = sep;
        memmove(&right->children[1], &right->children[0],
                (right->n + 1) * sizeof(bpt_node *));
        right->children[0] = llc;
        right->n++;
    }
}

static void borrow_from_right(bpt_node *node, size_t ci) {
    bpt_node *left = node->children[ci];
    bpt_node *right = node->children[ci + 1];
    if (left->is_leaf) {
        long bk = right->keys[0];
        long bv = right->values[0];
        memmove(&right->keys[0], &right->keys[1], (right->n - 1) * sizeof(long));
        memmove(&right->values[0], &right->values[1],
                (right->n - 1) * sizeof(long));
        right->n--;
        left->keys[left->n] = bk;
        left->values[left->n] = bv;
        left->n++;
        node->keys[ci] = right->keys[0];
    } else {
        long sep = node->keys[ci];
        long rfk = right->keys[0];
        bpt_node *rfc = right->children[0];
        memmove(&right->keys[0], &right->keys[1], (right->n - 1) * sizeof(long));
        memmove(&right->children[0], &right->children[1],
                right->n * sizeof(bpt_node *));
        right->n--;
        node->keys[ci] = rfk;
        left->keys[left->n] = sep;
        left->children[left->n + 1] = rfc;
        left->n++;
    }
}

/* Merge the right node into the left node; the two are children[l] and
 * children[l+1], with keys[l] as the separator. Removes them from `node`. */
static void merge_pair(bpt_node *node, size_t l) {
    bpt_node *left = node->children[l];
    bpt_node *right = node->children[l + 1];
    long sep = node->keys[l];
    /* Remove the right child pointer and the separator from the parent. */
    memmove(&node->children[l + 1], &node->children[l + 2],
            (node->n - l - 1) * sizeof(bpt_node *));
    memmove(&node->keys[l], &node->keys[l + 1], (node->n - l - 1) * sizeof(long));
    node->n--;
    if (left->is_leaf) {
        memcpy(&left->keys[left->n], right->keys, right->n * sizeof(long));
        memcpy(&left->values[left->n], right->values, right->n * sizeof(long));
        left->n += right->n;
        left->next = right->next; /* unlink `right` from the leaf chain */
    } else {
        left->keys[left->n] = sep;
        memcpy(&left->keys[left->n + 1], right->keys, right->n * sizeof(long));
        memcpy(&left->children[left->n + 1], right->children,
               (right->n + 1) * sizeof(bpt_node *));
        left->n += 1 + right->n;
    }
    node_free_shallow(right); /* right's children/next were moved into left */
}

static void fix_underfull(bpt_node *node, size_t ci, size_t t) {
    size_t n_children = node->n + 1;
    if (ci > 0 && node->children[ci - 1]->n >= t) {
        borrow_from_left(node, ci);
        return;
    }
    if (ci + 1 < n_children && node->children[ci + 1]->n >= t) {
        borrow_from_right(node, ci);
        return;
    }
    if (ci > 0) {
        merge_pair(node, ci - 1); /* merge left sibling with children[ci] */
    } else {
        merge_pair(node, ci); /* merge children[ci] with right sibling */
    }
}

static int delete_node(bpt_node *node, long key, size_t t, int is_root,
                       int *underfull) {
    size_t min_keys = is_root ? 0 : t - 1;
    if (node->is_leaf) {
        size_t i;
        if (!find_exact(node->keys, node->n, key, &i)) {
            return 0;
        }
        memmove(&node->keys[i], &node->keys[i + 1],
                (node->n - i - 1) * sizeof(long));
        memmove(&node->values[i], &node->values[i + 1],
                (node->n - i - 1) * sizeof(long));
        node->n--;
        *underfull = node->n < min_keys;
        return 1;
    }
    {
        size_t ci = child_index(node, key);
        int child_underfull = 0;
        if (!delete_node(node->children[ci], key, t, 0, &child_underfull)) {
            return 0;
        }
        if (child_underfull) {
            fix_underfull(node, ci, t);
        } else {
            maybe_update_separator(node, ci);
        }
        *underfull = node->n < min_keys;
        return 1;
    }
}

int bpt_delete(bpt *tree, long key) {
    int underfull = 0;
    if (!delete_node(tree->root, key, tree->t, 1, &underfull)) {
        return 0;
    }
    if (!tree->root->is_leaf && tree->root->n == 0) {
        bpt_node *old = tree->root;
        tree->root = old->children[0];
        node_free_shallow(old);
    }
    tree->size--;
    tree->first_leaf = leftmost_leaf(tree->root);
    return 1;
}

/* ── queries ──────────────────────────────────────────────────────────────── */
int bpt_search(const bpt *tree, long key, long *out_value) {
    bpt_node *leaf = find_leaf(tree->root, key);
    size_t i;
    if (find_exact(leaf->keys, leaf->n, key, &i)) {
        if (out_value != NULL) {
            *out_value = leaf->values[i];
        }
        return 1;
    }
    return 0;
}

int bpt_contains(const bpt *tree, long key) {
    return bpt_search(tree, key, NULL);
}

int bpt_min_key(const bpt *tree, long *out) {
    if (tree->size == 0) {
        return 0;
    }
    if (out != NULL) {
        *out = tree->first_leaf->keys[0];
    }
    return 1;
}

int bpt_max_key(const bpt *tree, long *out) {
    const bpt_node *nd = tree->root;
    if (tree->size == 0) {
        return 0;
    }
    while (!nd->is_leaf) {
        nd = nd->children[nd->n];
    }
    if (out != NULL) {
        *out = nd->keys[nd->n - 1];
    }
    return 1;
}

size_t bpt_len(const bpt *tree) { return tree->size; }
int bpt_is_empty(const bpt *tree) { return tree->size == 0; }

size_t bpt_height(const bpt *tree) {
    const bpt_node *nd = tree->root;
    size_t h = 0;
    while (!nd->is_leaf) {
        h++;
        nd = nd->children[0];
    }
    return h;
}

/* ── scans over the leaf chain ────────────────────────────────────────────── */
void bpt_full_scan(const bpt *tree, bpt_visit_fn fn, void *user) {
    const bpt_node *cur = tree->first_leaf;
    while (cur != NULL) {
        size_t i;
        for (i = 0; i < cur->n; i++) {
            fn(cur->keys[i], cur->values[i], user);
        }
        cur = cur->next;
    }
}

void bpt_range_scan(const bpt *tree, long low, long high, bpt_visit_fn fn,
                    void *user) {
    const bpt_node *cur = find_leaf(tree->root, low);
    int emitted = 0;
    while (cur != NULL) {
        int done = 1;
        size_t i;
        for (i = 0; i < cur->n; i++) {
            if (cur->keys[i] > high) {
                break;
            }
            if (cur->keys[i] >= low) {
                fn(cur->keys[i], cur->values[i], user);
                done = 0;
                emitted = 1;
            }
        }
        if (done && emitted) {
            break;
        }
        cur = cur->next;
    }
}

/* ── validation ───────────────────────────────────────────────────────────── */
static int validate_node(const bpt_node *nd, size_t t, int is_root,
                         size_t depth, size_t *out_leaf_depth, size_t *out_count) {
    size_t min_keys = is_root ? 0 : t - 1;
    size_t max_keys = 2 * t - 1;
    size_t i;
    if (nd->n > max_keys || (!is_root && nd->n < min_keys)) {
        return 0;
    }
    for (i = 1; i < nd->n; i++) {
        if (nd->keys[i - 1] >= nd->keys[i]) {
            return 0;
        }
    }
    if (nd->is_leaf) {
        *out_leaf_depth = depth;
        *out_count = nd->n;
        return 1;
    }
    {
        int have = 0;
        size_t ld = depth;
        size_t total = 0;
        for (i = 0; i <= nd->n; i++) {
            size_t cd, cc;
            if (!validate_node(nd->children[i], t, 0, depth + 1, &cd, &cc)) {
                return 0;
            }
            if (!have) {
                have = 1;
                ld = cd;
            } else if (ld != cd) {
                return 0;
            }
            total += cc;
        }
        *out_leaf_depth = ld;
        *out_count = total;
        return 1;
    }
}

int bpt_is_valid(const bpt *tree) {
    size_t leaf_depth = 0, count = 0;
    const bpt_node *cur;
    size_t list_count = 0;
    int have_prev = 0;
    long prev = 0;
    if (!validate_node(tree->root, tree->t, 1, 0, &leaf_depth, &count)) {
        return 0;
    }
    if (count != tree->size) {
        return 0;
    }
    /* The leaf chain must list every key exactly once, strictly ascending. */
    cur = tree->first_leaf;
    while (cur != NULL) {
        size_t i;
        for (i = 0; i < cur->n; i++) {
            if (have_prev && prev >= cur->keys[i]) {
                return 0;
            }
            prev = cur->keys[i];
            have_prev = 1;
            list_count++;
        }
        cur = cur->next;
    }
    return list_count == tree->size;
}

/* ── tree lifetime ────────────────────────────────────────────────────────── */
bpt *bpt_new(size_t t) {
    bpt *tree = (bpt *)malloc(sizeof *tree);
    if (tree == NULL) {
        return NULL;
    }
    if (t < 2) {
        t = 2;
    }
    if (t > SIZE_MAX / 4) {
        t = SIZE_MAX / 4; /* keep 2t and 2t+1 from overflowing */
    }
    tree->root = node_new_leaf(t);
    if (tree->root == NULL) {
        free(tree);
        return NULL;
    }
    tree->first_leaf = tree->root;
    tree->t = t;
    tree->size = 0;
    return tree;
}

void bpt_free(bpt *tree) {
    if (tree == NULL) {
        return;
    }
    node_free(tree->root);
    free(tree);
}
