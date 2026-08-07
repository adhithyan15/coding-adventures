/*
 * trie.h — a trie (prefix tree) mapping byte-string keys to int values, in pure
 * ISO C17. A faithful port of the Rust `trie` crate.
 * ===========================================================================
 *
 * A trie stores keys character by character down a tree: all keys sharing a
 * prefix share the path for that prefix, so lookups and prefix queries are
 * O(key length) and the keys come out in sorted order.
 *
 *   insert("cat"), insert("car"), insert("dog"):
 *
 *        (root)
 *        /    \
 *       c      d
 *       |      |
 *       a      o
 *      / \     |
 *     t   r    g*          '*' marks a node that ends a key
 *     *   *
 *
 * The Rust crate keys on Unicode `char`s; this C port keys on BYTES (each node
 * has one slot per possible byte value), so a UTF-8 string is stored by its
 * byte sequence. Values are `int`. Enumeration (all_words / keys / prefix
 * search) visits keys in ascending byte order, matching the crate's sorted
 * BTreeMap iteration.
 *
 * The trie owns heap storage — pair trie_init with trie_free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef TRIE_H
#define TRIE_H

#include <stddef.h> /* size_t */

typedef struct trie_node trie_node; /* opaque */

typedef struct {
    trie_node *root;
    size_t size; /* number of keys stored */
} trie;

/* trie_init — create an empty trie. Returns 1, or 0 on allocation failure. */
int trie_init(trie *t);

/* trie_free — release all nodes. Safe on a zeroed struct; idempotent. */
void trie_free(trie *t);

/* trie_insert — associate `value` with NUL-terminated `key` (overwriting any
 * previous value). Returns 1, or 0 on allocation failure. */
int trie_insert(trie *t, const char *key, int value);

/* trie_search — if `key` is present, write its value to *out and return 1;
 * otherwise return 0. */
int trie_search(const trie *t, const char *key, int *out);

/* trie_contains_key — 1 if `key` is present, else 0. */
int trie_contains_key(const trie *t, const char *key);

/* trie_delete — remove `key` (pruning now-unused nodes). Returns 1 if the key
 * was present (and removed), 0 if it was not. */
int trie_delete(trie *t, const char *key);

/* trie_starts_with — 1 if any stored key begins with `prefix` (an empty prefix
 * matches iff the trie is non-empty). */
int trie_starts_with(const trie *t, const char *prefix);

/* Callback for enumeration: receives each key and value, in ascending key
 * order. `ud` is the user pointer passed to the iterator. */
typedef void (*trie_visit_fn)(const char *key, int value, void *ud);

/* trie_foreach_prefix — call `visit` for every key that starts with `prefix`,
 * in sorted order. An empty prefix visits every key. Returns 1, or 0 on an
 * internal allocation failure (building the key buffers). */
int trie_foreach_prefix(const trie *t, const char *prefix, trie_visit_fn visit,
                        void *ud);

/* trie_foreach — call `visit` for every key (equivalent to prefix ""). */
int trie_foreach(const trie *t, trie_visit_fn visit, void *ud);

/* trie_longest_prefix_match — find the longest stored key that is a prefix of
 * `string`. On a match, copy that key into `out_key` (capacity `out_size`),
 * write its value to *out_value, and return 1. Returns 0 if no stored key is a
 * prefix of `string`, or -1 if `out_key` is too small. */
int trie_longest_prefix_match(const trie *t, const char *string, char *out_key,
                              size_t out_size, int *out_value);

/* trie_len — number of keys. trie_is_empty — 1 if empty else 0. */
size_t trie_len(const trie *t);
int trie_is_empty(const trie *t);

#endif /* TRIE_H */
