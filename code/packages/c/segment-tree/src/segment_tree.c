/*
 * segment_tree.c — implementation of the int segment tree. Ported from the Rust
 * `segment-tree` crate; the 1-indexed 4n layout and the recursive build/query/
 * update all match it.
 */
#include "segment_tree.h"

#include <limits.h> /* INT_MAX, INT_MIN — identities for min/max */
#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */

/* Built-in combine operations for the convenience builders. */
static int op_sum(int a, int b) { return a + b; }
static int op_min(int a, int b) { return a < b ? a : b; }
static int op_max(int a, int b) { return a > b ? a : b; }

/* build — recursively fill node `node`, which covers elements [left, right]. */
static void build(int *tree, const int *values, size_t node, size_t left,
                  size_t right, segment_tree_combine combine) {
    size_t mid;
    if (left == right) {
        tree[node] = values[left];
        return;
    }
    mid = (left + right) / 2;
    build(tree, values, node * 2, left, mid, combine);
    build(tree, values, node * 2 + 1, mid + 1, right, combine);
    tree[node] = combine(tree[node * 2], tree[node * 2 + 1]);
}

/* query — combine over the intersection of [left, right] and [ql, qr].
 * Returns `identity` for a node disjoint from the query range. */
static int query(const int *tree, size_t node, size_t left, size_t right,
                 size_t ql, size_t qr, segment_tree_combine combine,
                 int identity) {
    size_t mid;
    int l, r;
    if (right < ql || left > qr) {
        return identity; /* node fully outside the query range */
    }
    if (ql <= left && right <= qr) {
        return tree[node]; /* node fully inside */
    }
    mid = (left + right) / 2;
    l = query(tree, node * 2, left, mid, ql, qr, combine, identity);
    r = query(tree, node * 2 + 1, mid + 1, right, ql, qr, combine, identity);
    return combine(l, r);
}

/* update_node — set element `index` to `value` and refresh the ancestors. */
static void update_node(int *tree, size_t node, size_t left, size_t right,
                        size_t index, int value, segment_tree_combine combine) {
    size_t mid;
    if (left == right) {
        tree[node] = value;
        return;
    }
    mid = (left + right) / 2;
    if (index <= mid) {
        update_node(tree, node * 2, left, mid, index, value, combine);
    } else {
        update_node(tree, node * 2 + 1, mid + 1, right, index, value, combine);
    }
    tree[node] = combine(tree[node * 2], tree[node * 2 + 1]);
}

int segment_tree_init(segment_tree *t, const int *values, size_t n,
                      segment_tree_combine combine, int identity) {
    size_t node_count;
    size_t i;
    t->n = n;
    t->combine = combine;
    t->identity = identity;

    if (n == 0) {
        /* A single identity slot keeps the struct valid and queries safe. */
        t->tree = (int *)malloc(sizeof(int));
        if (t->tree == NULL) {
            return 0;
        }
        t->tree[0] = identity;
        return 1;
    }
    /* 4n+4 nodes always suffice for the recursive layout. Guard 4*n+4 against
     * size_t overflow, then use calloc so the node_count * sizeof(int) byte
     * multiply is ALSO overflow-checked (calloc returns NULL on overflow). */
    if (n > (SIZE_MAX - 4) / 4) {
        t->tree = NULL;
        t->n = 0;
        return 0;
    }
    node_count = 4 * n + 4;
    t->tree = (int *)calloc(node_count, sizeof(int));
    if (t->tree == NULL) {
        t->n = 0;
        return 0;
    }
    for (i = 0; i < node_count; i++) {
        t->tree[i] = identity;
    }
    build(t->tree, values, 1, 0, n - 1, combine);
    return 1;
}

int segment_tree_init_sum(segment_tree *t, const int *values, size_t n) {
    return segment_tree_init(t, values, n, op_sum, 0);
}

int segment_tree_init_min(segment_tree *t, const int *values, size_t n) {
    return segment_tree_init(t, values, n, op_min, INT_MAX);
}

int segment_tree_init_max(segment_tree *t, const int *values, size_t n) {
    return segment_tree_init(t, values, n, op_max, INT_MIN);
}

void segment_tree_free(segment_tree *t) {
    free(t->tree);
    t->tree = NULL;
    t->n = 0;
}

int segment_tree_query(const segment_tree *t, size_t left, size_t right) {
    if (t->n == 0 || left > right || right >= t->n) {
        return t->identity; /* empty or out-of-range → neutral, never OOB */
    }
    return query(t->tree, 1, 0, t->n - 1, left, right, t->combine, t->identity);
}

void segment_tree_update(segment_tree *t, size_t index, int value) {
    if (t->n == 0 || index >= t->n) {
        return; /* ignore out-of-range updates */
    }
    update_node(t->tree, 1, 0, t->n - 1, index, value, t->combine);
}

size_t segment_tree_len(const segment_tree *t) { return t->n; }

int segment_tree_is_empty(const segment_tree *t) { return t->n == 0 ? 1 : 0; }
