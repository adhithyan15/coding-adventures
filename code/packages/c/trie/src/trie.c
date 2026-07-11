/*
 * trie.c — implementation of the byte-keyed int trie. Ported from the Rust
 * `trie` crate; insert/search/delete and the sorted enumeration all match its
 * semantics (children visited in ascending key order).
 */
#include "trie.h"

#include <stdlib.h> /* malloc, calloc, realloc, free */
#include <string.h> /* strlen, memcpy */

#define TRIE_RADIX 256 /* one child slot per byte value */

struct trie_node {
    trie_node *children[TRIE_RADIX];
    int value;
    int is_end; /* 1 if a key ends here */
};

static trie_node *node_new(void) {
    /* calloc zeroes the child pointers, value, and is_end. */
    return (trie_node *)calloc(1, sizeof(trie_node));
}

static void node_free(trie_node *n) {
    int c;
    if (n == NULL) {
        return;
    }
    for (c = 0; c < TRIE_RADIX; c++) {
        node_free(n->children[c]);
    }
    free(n);
}

int trie_init(trie *t) {
    t->root = node_new();
    t->size = 0;
    return t->root != NULL ? 1 : 0;
}

void trie_free(trie *t) {
    node_free(t->root);
    t->root = NULL;
    t->size = 0;
}

/* find_node — walk the key and return the node it ends at, or NULL if the path
 * does not exist. */
static const trie_node *find_node(const trie_node *node, const char *key) {
    size_t i;
    for (i = 0; key[i] != '\0'; i++) {
        unsigned char ch = (unsigned char)key[i];
        node = node->children[ch];
        if (node == NULL) {
            return NULL;
        }
    }
    return node;
}

int trie_insert(trie *t, const char *key, int value) {
    trie_node *node = t->root;
    size_t i;
    for (i = 0; key[i] != '\0'; i++) {
        unsigned char ch = (unsigned char)key[i];
        if (node->children[ch] == NULL) {
            node->children[ch] = node_new();
            if (node->children[ch] == NULL) {
                return 0; /* allocation failure; partial path is harmless */
            }
        }
        node = node->children[ch];
    }
    if (!node->is_end) {
        t->size++;
    }
    node->is_end = 1;
    node->value = value;
    return 1;
}

int trie_search(const trie *t, const char *key, int *out) {
    const trie_node *node = find_node(t->root, key);
    if (node != NULL && node->is_end) {
        *out = node->value;
        return 1;
    }
    return 0;
}

int trie_contains_key(const trie *t, const char *key) {
    const trie_node *node = find_node(t->root, key);
    return (node != NULL && node->is_end) ? 1 : 0;
}

/* has_children — 1 if `n` has at least one child. */
static int has_children(const trie_node *n) {
    int c;
    for (c = 0; c < TRIE_RADIX; c++) {
        if (n->children[c] != NULL) {
            return 1;
        }
    }
    return 0;
}

/* delete_recursive — clear the end marker at the key's terminal node, then prune
 * nodes that become useless (no children and not a key end) on the way back up.
 * Returns 1 if the caller should free the child link it holds to `node`. */
static int delete_recursive(trie_node *node, const char *key, size_t depth) {
    if (key[depth] == '\0') {
        node->is_end = 0; /* unmark; caller already checked the key exists */
    } else {
        unsigned char ch = (unsigned char)key[depth];
        trie_node *child = node->children[ch];
        if (child != NULL && delete_recursive(child, key, depth + 1)) {
            node_free(child);
            node->children[ch] = NULL;
        }
    }
    /* This node may be pruned if it no longer ends a key and has no children. */
    return (!node->is_end && !has_children(node)) ? 1 : 0;
}

int trie_delete(trie *t, const char *key) {
    if (!trie_contains_key(t, key)) {
        return 0;
    }
    /* The root itself is never freed, so ignore its prune signal. */
    (void)delete_recursive(t->root, key, 0);
    t->size--;
    return 1;
}

int trie_starts_with(const trie *t, const char *prefix) {
    if (prefix[0] == '\0') {
        return t->size > 0 ? 1 : 0;
    }
    return find_node(t->root, prefix) != NULL ? 1 : 0;
}

/* --- enumeration ---------------------------------------------------------- */

/* A growable byte buffer used to build keys during a DFS. */
typedef struct {
    char *data;
    size_t len;
    size_t cap;
    int failed; /* set on allocation failure */
} keybuf;

static void keybuf_push(keybuf *kb, char ch) {
    if (kb->failed) {
        return;
    }
    if (kb->len + 1 > kb->cap) {
        size_t new_cap = kb->cap == 0 ? 16 : kb->cap * 2;
        char *grown = (char *)realloc(kb->data, new_cap);
        if (grown == NULL) {
            kb->failed = 1;
            return;
        }
        kb->data = grown;
        kb->cap = new_cap;
    }
    kb->data[kb->len++] = ch;
}

/* Depth-first collect in ascending byte order, invoking `visit` at each key end.
 * The key buffer is NUL-terminated before each callback. */
static void collect(const trie_node *node, keybuf *kb, trie_visit_fn visit,
                    void *ud) {
    int c;
    if (kb->failed) {
        return;
    }
    if (node->is_end) {
        keybuf_push(kb, '\0'); /* terminate for the callback */
        if (!kb->failed) {
            visit(kb->data, node->value, ud);
            kb->len--; /* pop the NUL */
        }
    }
    for (c = 0; c < TRIE_RADIX; c++) {
        if (node->children[c] != NULL) {
            keybuf_push(kb, (char)c);
            collect(node->children[c], kb, visit, ud);
            if (kb->failed) {
                return;
            }
            kb->len--; /* pop the character */
        }
    }
}

int trie_foreach_prefix(const trie *t, const char *prefix, trie_visit_fn visit,
                        void *ud) {
    const trie_node *node;
    keybuf kb;
    size_t i;

    node = (prefix[0] == '\0') ? t->root : find_node(t->root, prefix);
    if (node == NULL) {
        return 1; /* no keys with this prefix — nothing to visit */
    }
    kb.data = NULL;
    kb.len = 0;
    kb.cap = 0;
    kb.failed = 0;
    /* Seed the buffer with the prefix so collected keys are absolute. */
    for (i = 0; prefix[i] != '\0'; i++) {
        keybuf_push(&kb, prefix[i]);
    }
    collect(node, &kb, visit, ud);
    free(kb.data);
    return kb.failed ? 0 : 1;
}

int trie_foreach(const trie *t, trie_visit_fn visit, void *ud) {
    return trie_foreach_prefix(t, "", visit, ud);
}

int trie_longest_prefix_match(const trie *t, const char *string, char *out_key,
                              size_t out_size, int *out_value) {
    const trie_node *node = t->root;
    size_t i;
    size_t best_len = 0;
    int best_value = 0;
    int found = 0;

    if (node->is_end) {
        best_len = 0;
        best_value = node->value;
        found = 1;
    }
    for (i = 0; string[i] != '\0'; i++) {
        unsigned char ch = (unsigned char)string[i];
        node = node->children[ch];
        if (node == NULL) {
            break;
        }
        if (node->is_end) {
            best_len = i + 1;
            best_value = node->value;
            found = 1;
        }
    }
    if (!found) {
        return 0;
    }
    if (out_size <= best_len) {
        return -1; /* need best_len chars + NUL */
    }
    memcpy(out_key, string, best_len);
    out_key[best_len] = '\0';
    *out_value = best_value;
    return 1;
}

size_t trie_len(const trie *t) { return t->size; }

int trie_is_empty(const trie *t) { return t->size == 0 ? 1 : 0; }
