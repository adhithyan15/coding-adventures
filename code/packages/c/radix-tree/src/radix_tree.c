/*
 * radix_tree.c — implementation of the radix tree (see radix_tree.h). A faithful
 * port of the Rust `radix-tree` crate, specialised to a `long` value.
 *
 * A node keeps its outgoing edges in an array sorted by the first byte of each
 * edge label, so lookups binary-search and traversals emit keys in order. Insert
 * splits an edge when a key diverges partway along its label; delete prunes dead
 * nodes and merges a node that is left with a single child (path compression).
 */
#include "radix_tree.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, realloc, free */
#include <string.h> /* memcpy, memmove, strlen */

typedef struct radix_node radix_node;

typedef struct {
    char *label; /* owned; not NUL-terminated (length carried separately) */
    size_t label_len;
    radix_node *child;
} radix_edge;

struct radix_node {
    int is_end;
    long value;         /* valid when is_end */
    radix_edge *edges;  /* sorted by (unsigned char)label[0] */
    size_t n_edges;
    size_t cap_edges;
};

struct radix_tree {
    radix_node *root;
    size_t size;
};

/* ── small helpers ────────────────────────────────────────────────────────── */
static size_t common_prefix_len(const char *a, size_t alen, const char *b,
                                size_t blen) {
    size_t n = alen < blen ? alen : blen;
    size_t i = 0;
    while (i < n && a[i] == b[i]) {
        i++;
    }
    return i;
}

static char *dup_bytes(const char *src, size_t len) {
    char *p = (char *)malloc(len ? len : 1);
    if (p == NULL) {
        return NULL;
    }
    if (len) {
        memcpy(p, src, len);
    }
    return p;
}

static radix_node *node_new(void) {
    radix_node *nd = (radix_node *)malloc(sizeof *nd);
    if (nd == NULL) {
        return NULL;
    }
    nd->is_end = 0;
    nd->value = 0;
    nd->edges = NULL;
    nd->n_edges = 0;
    nd->cap_edges = 0;
    return nd;
}

static void node_free(radix_node *nd) {
    size_t i;
    if (nd == NULL) {
        return;
    }
    for (i = 0; i < nd->n_edges; i++) {
        free(nd->edges[i].label);
        node_free(nd->edges[i].child);
    }
    free(nd->edges);
    free(nd);
}

/* Binary search the edge whose label starts with `first`; sets *idx to the match
 * or the sorted insertion point, and returns 1 if found. */
static int edge_find(const radix_node *nd, unsigned char first, size_t *idx) {
    size_t lo = 0, hi = nd->n_edges;
    while (lo < hi) {
        size_t mid = lo + (hi - lo) / 2;
        unsigned char m = (unsigned char)nd->edges[mid].label[0];
        if (m == first) {
            *idx = mid;
            return 1;
        }
        if (m < first) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    *idx = lo;
    return 0;
}

static int node_reserve(radix_node *nd, size_t need) {
    size_t nc;
    radix_edge *ne;
    if (nd->cap_edges >= need) {
        return 1;
    }
    nc = nd->cap_edges ? nd->cap_edges * 2 : 4;
    while (nc < need) {
        if (nc > SIZE_MAX / 2) {
            return 0;
        }
        nc *= 2;
    }
    ne = (radix_edge *)realloc(nd->edges, nc * sizeof(radix_edge));
    if (ne == NULL) {
        return 0;
    }
    nd->edges = ne;
    nd->cap_edges = nc;
    return 1;
}

/* Insert an already-owned (label, child) edge at sorted index `idx`. The array
 * must already have room (caller reserved it) — returns 0 only if growth fails
 * when it had to grow. */
static int edge_insert_at(radix_node *nd, size_t idx, char *label,
                          size_t label_len, radix_node *child) {
    if (!node_reserve(nd, nd->n_edges + 1)) {
        return 0;
    }
    memmove(&nd->edges[idx + 1], &nd->edges[idx],
            (nd->n_edges - idx) * sizeof(radix_edge));
    nd->edges[idx].label = label;
    nd->edges[idx].label_len = label_len;
    nd->edges[idx].child = child;
    nd->n_edges++;
    return 1;
}

/* ── insertion ────────────────────────────────────────────────────────────── */
static int insert_recursive(radix_node *node, const char *key, size_t klen,
                            long value, int *added) {
    unsigned char first;
    size_t idx;
    radix_edge *edge;
    size_t common;

    if (klen == 0) {
        *added = !node->is_end;
        node->is_end = 1;
        node->value = value;
        return 1;
    }
    first = (unsigned char)key[0];
    if (!edge_find(node, first, &idx)) {
        /* No edge for this byte → add a leaf edge labelled with the whole key. */
        char *lbl = dup_bytes(key, klen);
        radix_node *leaf = node_new();
        if (lbl == NULL || leaf == NULL) {
            free(lbl);
            node_free(leaf);
            return 0;
        }
        leaf->is_end = 1;
        leaf->value = value;
        if (!edge_insert_at(node, idx, lbl, klen, leaf)) {
            free(lbl);
            node_free(leaf);
            return 0;
        }
        *added = 1;
        return 1;
    }
    edge = &node->edges[idx];
    common = common_prefix_len(key, klen, edge->label, edge->label_len);
    if (common == edge->label_len) {
        /* The whole label matches → descend, consuming it. */
        return insert_recursive(edge->child, key + common, klen - common, value,
                                added);
    }
    /* The key diverges partway along the label → split the edge. Allocate every
     * new piece FIRST so an out-of-memory leaves the existing edge intact. */
    {
        size_t label_rest_len = edge->label_len - common;
        size_t key_rest_len = klen - common;
        char *label_rest = dup_bytes(edge->label + common, label_rest_len);
        radix_node *split_node = node_new();
        char *key_rest = NULL;
        radix_node *leaf = NULL;
        int fail = (label_rest == NULL || split_node == NULL);
        if (!fail && !node_reserve(split_node, key_rest_len ? 2 : 1)) {
            fail = 1;
        }
        if (!fail && key_rest_len) {
            key_rest = dup_bytes(key + common, key_rest_len);
            leaf = node_new();
            if (key_rest == NULL || leaf == NULL) {
                fail = 1;
            } else {
                leaf->is_end = 1;
                leaf->value = value;
            }
        }
        if (fail) {
            free(label_rest);
            free(key_rest);
            node_free(split_node); /* has no edges yet → frees only itself */
            node_free(leaf);
            return 0;
        }
        /* Wire it up — from here nothing can fail (split_node is reserved). The
         * old child moves under the label's remainder inside the split node. */
        edge_insert_at(split_node, 0, label_rest, label_rest_len, edge->child);
        if (key_rest_len == 0) {
            split_node->is_end = 1;
            split_node->value = value;
        } else {
            size_t p;
            edge_find(split_node, (unsigned char)key_rest[0], &p);
            edge_insert_at(split_node, p, key_rest, key_rest_len, leaf);
        }
        edge->label_len = common; /* truncate to the shared prefix (keep buffer) */
        edge->child = split_node;
        *added = 1;
        return 1;
    }
}

int radix_insert(radix_tree *tree, const char *key, long value) {
    size_t klen = strlen(key);
    int added = 0;
    if (!insert_recursive(tree->root, key, klen, value, &added)) {
        return 0;
    }
    if (added) {
        tree->size++;
    }
    return 1;
}

/* ── search / prefix queries ──────────────────────────────────────────────── */
/* Descend consuming `key`; returns the terminal node, or NULL if a label fails
 * to match fully. */
static const radix_node *descend(const radix_node *node, const char *key,
                                 size_t klen) {
    const char *k = key;
    size_t rem = klen;
    while (rem > 0) {
        size_t idx;
        const radix_edge *e;
        size_t common;
        if (!edge_find(node, (unsigned char)k[0], &idx)) {
            return NULL;
        }
        e = &node->edges[idx];
        common = common_prefix_len(k, rem, e->label, e->label_len);
        if (common < e->label_len) {
            return NULL;
        }
        k += common;
        rem -= common;
        node = e->child;
    }
    return node;
}

int radix_search(const radix_tree *tree, const char *key, long *out_value) {
    const radix_node *n = descend(tree->root, key, strlen(key));
    if (n != NULL && n->is_end) {
        if (out_value != NULL) {
            *out_value = n->value;
        }
        return 1;
    }
    return 0;
}

int radix_contains(const radix_tree *tree, const char *key) {
    const radix_node *n = descend(tree->root, key, strlen(key));
    return n != NULL && n->is_end;
}

int radix_starts_with(const radix_tree *tree, const char *prefix) {
    size_t plen = strlen(prefix);
    const radix_node *node = tree->root;
    const char *k = prefix;
    size_t rem = plen;
    if (plen == 0) {
        return tree->size > 0;
    }
    while (rem > 0) {
        size_t idx;
        const radix_edge *e;
        size_t common;
        if (!edge_find(node, (unsigned char)k[0], &idx)) {
            return 0;
        }
        e = &node->edges[idx];
        common = common_prefix_len(k, rem, e->label, e->label_len);
        if (common == rem) {
            return 1;
        }
        if (common < e->label_len) {
            return 0;
        }
        k += common;
        rem -= common;
        node = e->child;
    }
    return node->is_end || node->n_edges > 0;
}

long radix_longest_prefix_match(const radix_tree *tree, const char *key,
                                char *out, size_t out_cap) {
    size_t klen = strlen(key);
    const radix_node *node = tree->root;
    const char *k = key;
    size_t rem = klen;
    size_t consumed = 0;
    long best = node->is_end ? 0 : -1;
    while (rem > 0) {
        size_t idx;
        const radix_edge *e;
        size_t common;
        if (!edge_find(node, (unsigned char)k[0], &idx)) {
            break;
        }
        e = &node->edges[idx];
        common = common_prefix_len(k, rem, e->label, e->label_len);
        if (common < e->label_len) {
            break;
        }
        consumed += common;
        k += common;
        rem -= common;
        node = e->child;
        if (node->is_end) {
            best = (long)consumed;
        }
    }
    if (best < 0) {
        return -1;
    }
    {
        size_t w = (size_t)best < out_cap ? (size_t)best : out_cap;
        if (out != NULL && w > 0) {
            memcpy(out, key, w);
        }
    }
    return best;
}

/* ── deletion ─────────────────────────────────────────────────────────────── */
static void delete_recursive(radix_node *node, const char *key, size_t klen,
                             int *deleted, int *mergeable) {
    unsigned char first;
    size_t idx;
    radix_edge *edge;
    size_t common;
    int child_deleted = 0, child_mergeable = 0;

    if (klen == 0) {
        if (!node->is_end) {
            *deleted = 0;
            *mergeable = 0;
            return;
        }
        node->is_end = 0;
        *deleted = 1;
        *mergeable = node->n_edges == 1; /* is_end now false */
        return;
    }
    first = (unsigned char)key[0];
    if (!edge_find(node, first, &idx)) {
        *deleted = 0;
        *mergeable = 0;
        return;
    }
    edge = &node->edges[idx];
    common = common_prefix_len(key, klen, edge->label, edge->label_len);
    if (common < edge->label_len) {
        *deleted = 0;
        *mergeable = 0;
        return;
    }
    delete_recursive(edge->child, key + common, klen - common, &child_deleted,
                     &child_mergeable);
    if (!child_deleted) {
        *deleted = 0;
        *mergeable = 0;
        return;
    }
    if (child_mergeable) {
        /* The child now has exactly one edge → fold it up into this edge. */
        radix_node *child = edge->child;
        radix_edge *grand = &child->edges[0];
        size_t mlen = edge->label_len + grand->label_len;
        char *merged = (char *)malloc(mlen ? mlen : 1);
        if (merged != NULL) {
            memcpy(merged, edge->label, edge->label_len);
            memcpy(merged + edge->label_len, grand->label, grand->label_len);
            free(edge->label);
            edge->label = merged;
            edge->label_len = mlen;
            edge->child = grand->child; /* adopt the grandchild subtree */
            free(grand->label);
            free(child->edges);
            free(child);
        }
        /* If the merge allocation failed we simply keep the extra node: the tree
         * is still a correct map, just not maximally compressed. */
    } else if (!edge->child->is_end && edge->child->n_edges == 0) {
        /* The child became a dead end → prune it. */
        radix_node *child = edge->child;
        char *lbl = edge->label;
        memmove(&node->edges[idx], &node->edges[idx + 1],
                (node->n_edges - idx - 1) * sizeof(radix_edge));
        node->n_edges--;
        free(lbl);
        node_free(child);
    }
    *deleted = 1;
    *mergeable = !node->is_end && node->n_edges == 1;
}

int radix_delete(radix_tree *tree, const char *key) {
    int deleted = 0, mergeable = 0;
    delete_recursive(tree->root, key, strlen(key), &deleted, &mergeable);
    if (deleted) {
        tree->size--;
    }
    return deleted;
}

/* ── enumeration ──────────────────────────────────────────────────────────── */
typedef struct {
    char *data;
    size_t len;
    size_t cap;
    int ok;
} strbuf;

static void sb_init(strbuf *sb) {
    sb->data = NULL;
    sb->len = 0;
    sb->cap = 0;
    sb->ok = 1;
}
static void sb_free(strbuf *sb) { free(sb->data); }

static void sb_append(strbuf *sb, const char *s, size_t n) {
    if (!sb->ok) {
        return;
    }
    if (sb->cap < sb->len + n + 1) {
        size_t nc = sb->cap ? sb->cap * 2 : 16;
        char *nd;
        while (nc < sb->len + n + 1) {
            if (nc > SIZE_MAX / 2) {
                sb->ok = 0;
                return;
            }
            nc *= 2;
        }
        nd = (char *)realloc(sb->data, nc);
        if (nd == NULL) {
            sb->ok = 0;
            return;
        }
        sb->data = nd;
        sb->cap = nc;
    }
    memcpy(sb->data + sb->len, s, n);
    sb->len += n;
}

static void collect(const radix_node *node, strbuf *sb, radix_key_fn fn,
                    void *user) {
    size_t i;
    if (node->is_end && sb->ok) {
        if (sb->cap < sb->len + 1) {
            sb_append(sb, "", 0); /* ensure room for the NUL */
        }
        if (sb->ok) {
            sb->data[sb->len] = '\0';
            fn(sb->data, sb->len, user);
        }
    }
    for (i = 0; i < node->n_edges; i++) {
        size_t old = sb->len;
        sb_append(sb, node->edges[i].label, node->edges[i].label_len);
        collect(node->edges[i].child, sb, fn, user);
        sb->len = old; /* pop the label back off */
    }
}

void radix_keys(const radix_tree *tree, radix_key_fn fn, void *user) {
    strbuf sb;
    sb_init(&sb);
    collect(tree->root, &sb, fn, user);
    sb_free(&sb);
}

void radix_words_with_prefix(const radix_tree *tree, const char *prefix,
                             radix_key_fn fn, void *user) {
    size_t plen = strlen(prefix);
    const radix_node *node = tree->root;
    const char *k = prefix;
    size_t rem = plen;
    strbuf path;
    sb_init(&path);

    if (plen == 0) {
        collect(node, &path, fn, user);
        sb_free(&path);
        return;
    }
    while (rem > 0) {
        size_t idx;
        const radix_edge *e;
        size_t common;
        if (!edge_find(node, (unsigned char)k[0], &idx)) {
            sb_free(&path);
            return; /* no such prefix */
        }
        e = &node->edges[idx];
        common = common_prefix_len(k, rem, e->label, e->label_len);
        if (common == rem) {
            if (common == e->label_len) {
                sb_append(&path, e->label, e->label_len);
                node = e->child;
                rem = 0; /* fall through to collect below */
            } else {
                /* Prefix ends mid-label → collect the whole subtree under it. */
                sb_append(&path, e->label, e->label_len);
                collect(e->child, &path, fn, user);
                sb_free(&path);
                return;
            }
        } else if (common < e->label_len) {
            sb_free(&path);
            return;
        } else {
            sb_append(&path, e->label, e->label_len);
            k += common;
            rem -= common;
            node = e->child;
        }
    }
    collect(node, &path, fn, user);
    sb_free(&path);
}

/* ── introspection ────────────────────────────────────────────────────────── */
size_t radix_len(const radix_tree *tree) { return tree->size; }
int radix_is_empty(const radix_tree *tree) { return tree->size == 0; }

static size_t count_nodes(const radix_node *nd) {
    size_t total = 1;
    size_t i;
    for (i = 0; i < nd->n_edges; i++) {
        total += count_nodes(nd->edges[i].child);
    }
    return total;
}
size_t radix_node_count(const radix_tree *tree) {
    return count_nodes(tree->root);
}

/* ── tree lifetime ────────────────────────────────────────────────────────── */
radix_tree *radix_new(void) {
    radix_tree *tree = (radix_tree *)malloc(sizeof *tree);
    if (tree == NULL) {
        return NULL;
    }
    tree->root = node_new();
    if (tree->root == NULL) {
        free(tree);
        return NULL;
    }
    tree->size = 0;
    return tree;
}

void radix_free(radix_tree *tree) {
    if (tree == NULL) {
        return;
    }
    node_free(tree->root);
    free(tree);
}
