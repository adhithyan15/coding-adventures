/*
 * rope.c — implementation of the rope (see rope.h). A faithful port of the Rust
 * `rope` crate: a binary tree of leaf chunks with weighted internal nodes, and
 * value/move semantics realised in C as a consuming API.
 */
#include "rope.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy */

typedef enum { ROPE_LEAF, ROPE_INTERNAL } node_kind;

typedef struct rope_node {
    node_kind kind;
    /* leaf */
    char *chunk;
    size_t chunk_len;
    /* internal */
    size_t weight; /* total bytes in the left subtree */
    struct rope_node *left;
    struct rope_node *right;
} rope_node;

struct rope {
    rope_node *root; /* NULL when empty */
    size_t len;
};

/* ── node helpers ─────────────────────────────────────────────────────────── */
static void node_free(rope_node *n) {
    if (n == NULL) {
        return;
    }
    if (n->kind == ROPE_LEAF) {
        free(n->chunk);
    } else {
        node_free(n->left);
        node_free(n->right);
    }
    free(n);
}

static rope_node *leaf_new(const char *s, size_t len) {
    rope_node *n = (rope_node *)malloc(sizeof *n);
    if (n == NULL) {
        return NULL;
    }
    n->kind = ROPE_LEAF;
    n->chunk = (char *)malloc(len ? len : 1);
    if (n->chunk == NULL) {
        free(n);
        return NULL;
    }
    if (len) {
        memcpy(n->chunk, s, len);
    }
    n->chunk_len = len;
    n->weight = 0;
    n->left = NULL;
    n->right = NULL;
    return n;
}

static rope_node *internal_new(size_t weight, rope_node *left,
                               rope_node *right) {
    rope_node *n = (rope_node *)malloc(sizeof *n);
    if (n == NULL) {
        return NULL;
    }
    n->kind = ROPE_INTERNAL;
    n->chunk = NULL;
    n->chunk_len = 0;
    n->weight = weight;
    n->left = left;
    n->right = right;
    return n;
}

static void node_collect(const rope_node *n, char *out, size_t cap,
                         size_t *total) {
    if (n == NULL) {
        return;
    }
    if (n->kind == ROPE_LEAF) {
        size_t i;
        for (i = 0; i < n->chunk_len; i++) {
            if (out != NULL && *total < cap) {
                out[*total] = n->chunk[i];
            }
            (*total)++;
        }
    } else {
        node_collect(n->left, out, cap, total);
        node_collect(n->right, out, cap, total);
    }
}

static size_t node_depth(const rope_node *n) {
    size_t ld, rd;
    if (n == NULL || n->kind == ROPE_LEAF) {
        return 0;
    }
    ld = node_depth(n->left);
    rd = node_depth(n->right);
    return 1 + (ld > rd ? ld : rd);
}

static int node_balanced(const rope_node *n) {
    size_t ld, rd, diff;
    if (n == NULL || n->kind == ROPE_LEAF) {
        return 1;
    }
    ld = node_depth(n->left);
    rd = node_depth(n->right);
    diff = ld > rd ? ld - rd : rd - ld;
    return diff <= 1 && node_balanced(n->left) && node_balanced(n->right);
}

/* Descend by weight to fetch the byte at offset i (i < subtree length). */
static int node_byte_at(const rope_node *n, size_t i, char *out) {
    if (n == NULL) {
        return 0;
    }
    if (n->kind == ROPE_LEAF) {
        if (i < n->chunk_len) {
            *out = n->chunk[i];
            return 1;
        }
        return 0;
    }
    if (i < n->weight) {
        return node_byte_at(n->left, i, out);
    }
    return node_byte_at(n->right, i - n->weight, out);
}

/* ── construction ─────────────────────────────────────────────────────────── */
rope *rope_empty(void) {
    rope *r = (rope *)malloc(sizeof *r);
    if (r == NULL) {
        return NULL;
    }
    r->root = NULL;
    r->len = 0;
    return r;
}

rope *rope_from_string(const char *text, size_t len) {
    rope *r = rope_empty();
    if (r == NULL) {
        return NULL;
    }
    if (len == 0) {
        return r; /* empty rope */
    }
    r->root = leaf_new(text, len);
    if (r->root == NULL) {
        free(r);
        return NULL;
    }
    r->len = len;
    return r;
}

void rope_free(rope *r) {
    if (r == NULL) {
        return;
    }
    node_free(r->root);
    free(r);
}

size_t rope_len(const rope *r) { return r->len; }
int rope_is_empty(const rope *r) { return r->len == 0; }

/* ── concat (moves subtrees) ──────────────────────────────────────────────── */
rope *rope_concat(rope *left, rope *right) {
    rope *result;
    /* Empty operand → return the other unchanged (freeing the empty wrapper). */
    if (left->root == NULL) {
        free(left);
        return right;
    }
    if (right->root == NULL) {
        free(right);
        return left;
    }
    if (left->len > SIZE_MAX - right->len) {
        rope_free(left);
        rope_free(right);
        return NULL; /* combined length would overflow */
    }
    result = (rope *)malloc(sizeof *result);
    if (result == NULL) {
        rope_free(left);
        rope_free(right);
        return NULL;
    }
    result->root = internal_new(left->len, left->root, right->root);
    if (result->root == NULL) {
        rope_free(left);
        rope_free(right);
        free(result);
        return NULL;
    }
    result->len = left->len + right->len;
    /* The subtrees were moved into `result`; free only the wrappers. */
    free(left);
    free(right);
    return result;
}

/* ── stringify to a fresh buffer (helper for the rebuild ops) ─────────────── */
static char *rope_collect_alloc(const rope *r) {
    char *buf = (char *)malloc(r->len ? r->len : 1);
    size_t total = 0;
    if (buf == NULL) {
        return NULL;
    }
    node_collect(r->root, buf, r->len, &total);
    return buf;
}

/* ── edits (all rebuild from the flattened bytes) ─────────────────────────── */
int rope_split(rope *r, size_t i, rope **out_left, rope **out_right) {
    char *buf = rope_collect_alloc(r);
    size_t n = r->len;
    size_t at = i < n ? i : n;
    rope *l;
    rope *rt;
    *out_left = NULL;
    *out_right = NULL;
    if (buf == NULL) {
        rope_free(r);
        return 0;
    }
    l = rope_from_string(buf, at);
    rt = rope_from_string(buf + at, n - at);
    free(buf);
    rope_free(r);
    if (l == NULL || rt == NULL) {
        rope_free(l);
        rope_free(rt);
        return 0;
    }
    *out_left = l;
    *out_right = rt;
    return 1;
}

rope *rope_insert(rope *r, size_t i, const char *s, size_t slen) {
    rope *left = NULL;
    rope *right = NULL;
    rope *mid;
    rope *lm;
    if (!rope_split(r, i, &left, &right)) {
        return NULL; /* rope_split consumed r and freed any partial output */
    }
    mid = rope_from_string(s, slen);
    if (mid == NULL) {
        rope_free(left);
        rope_free(right);
        return NULL;
    }
    lm = rope_concat(left, mid); /* consumes left, mid */
    if (lm == NULL) {
        rope_free(right);
        return NULL;
    }
    return rope_concat(lm, right); /* consumes lm, right */
}

rope *rope_delete(rope *r, size_t start, size_t length) {
    char *buf = rope_collect_alloc(r);
    size_t n = r->len;
    size_t s = start < n ? start : n;
    size_t end;
    rope *left;
    rope *right;
    if (buf == NULL) {
        rope_free(r);
        return NULL;
    }
    /* end = min(s + length, n), computed without risking s+length overflow. */
    end = (length > n - s) ? n : s + length;
    left = rope_from_string(buf, s);
    right = rope_from_string(buf + end, n - end);
    free(buf);
    rope_free(r);
    if (left == NULL || right == NULL) {
        rope_free(left);
        rope_free(right);
        return NULL;
    }
    return rope_concat(left, right);
}

/* Build a balanced tree over buf[0..n) via recursive halving + concat. */
static rope *build_balanced(const char *buf, size_t n) {
    size_t mid;
    rope *left;
    rope *right;
    if (n == 0) {
        return rope_empty();
    }
    if (n <= 1) {
        return rope_from_string(buf, n);
    }
    mid = n / 2;
    left = build_balanced(buf, mid);
    right = build_balanced(buf + mid, n - mid);
    if (left == NULL || right == NULL) {
        rope_free(left);
        rope_free(right);
        return NULL;
    }
    return rope_concat(left, right);
}

rope *rope_rebalance(rope *r) {
    char *buf = rope_collect_alloc(r);
    size_t n = r->len;
    rope *result;
    if (buf == NULL) {
        rope_free(r);
        return NULL;
    }
    rope_free(r);
    result = build_balanced(buf, n);
    free(buf);
    return result;
}

/* ── reads ────────────────────────────────────────────────────────────────── */
size_t rope_to_string(const rope *r, char *out, size_t out_cap) {
    size_t total = 0;
    node_collect(r->root, out, out_cap, &total);
    return total;
}

int rope_index(const rope *r, size_t i, char *out_byte) {
    if (i >= r->len) {
        return 0;
    }
    return node_byte_at(r->root, i, out_byte);
}

size_t rope_substring(const rope *r, size_t start, size_t end, char *out,
                      size_t out_cap) {
    size_t n = r->len;
    size_t s = start < n ? start : n;
    size_t e = end < n ? end : n;
    size_t count, i;
    char *buf;
    if (s >= e) {
        return 0;
    }
    buf = rope_collect_alloc(r);
    if (buf == NULL) {
        return 0;
    }
    count = e - s;
    for (i = 0; i < count; i++) {
        if (out != NULL && i < out_cap) {
            out[i] = buf[s + i];
        }
    }
    free(buf);
    return count;
}

size_t rope_depth(const rope *r) { return node_depth(r->root); }
int rope_is_balanced(const rope *r) { return node_balanced(r->root); }
