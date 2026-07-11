/*
 * radix_tree.h — a radix tree (compressed trie / Patricia trie) for string-keyed
 * prefix search, in pure ISO C17. A faithful port of the Rust `radix-tree`
 * crate.
 * ===========================================================================
 *
 * A radix tree is a trie whose chains of single-child nodes are *compressed*
 * into one edge labelled with the whole shared substring. That keeps the tree
 * small while still supporting fast prefix queries:
 *
 *     insert "search", "searcher", "searching"  →
 *         (root) --"search"--> (end) --"er"----> (end)
 *                                    \--"ing"---> (end)
 *
 * Each node may mark the end of a key (with a value) and carries edges to
 * children, kept sorted by the first byte of the edge label so traversals emit
 * keys in order.
 *
 *   radix_insert / radix_search / radix_delete   — the map operations
 *   radix_starts_with / radix_longest_prefix_match — prefix queries
 *   radix_keys / radix_words_with_prefix         — ordered key enumeration
 *   radix_len / radix_node_count                  — introspection
 *
 * Keys are NUL-terminated byte strings; values are `long`. (The Rust crate is
 * generic over the value; C has no generics, so this port specialises to a
 * `long` value. The C++ sibling package is generic.) Byte-oriented: the crate
 * splits edges on Unicode chars, this port on bytes. Insert, search, and delete
 * are all byte-consistent, so the tree is a correct prefix map for any byte
 * string; results are identical to the crate for ASCII keys (only the internal
 * node layout of multi-byte keys can differ).
 *
 * Ownership: radix_new allocates; pair it with radix_free.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef RADIX_TREE_H
#define RADIX_TREE_H

#include <stddef.h> /* size_t */

typedef struct radix_tree radix_tree;

/* radix_new — a new empty tree. NULL on allocation failure. */
radix_tree *radix_new(void);

/* radix_free — release the tree and all its nodes. NULL-safe. */
void radix_free(radix_tree *tree);

/* radix_insert — insert or update `key` -> `value`. Returns 1 on success, 0 on
 * allocation failure (the tree is left unchanged and valid). */
int radix_insert(radix_tree *tree, const char *key, long value);

/* radix_search — look up `key`. On a hit writes the value to *out_value (which
 * may be NULL) and returns 1; returns 0 if absent. */
int radix_search(const radix_tree *tree, const char *key, long *out_value);

/* radix_contains — 1 if `key` is present, else 0. */
int radix_contains(const radix_tree *tree, const char *key);

/* radix_delete — remove `key`, pruning dead nodes and merging single-child
 * chains. Returns 1 if it was present, else 0. */
int radix_delete(radix_tree *tree, const char *key);

/* radix_starts_with — 1 if any key has `prefix` as a prefix (an empty prefix is
 * true iff the tree is non-empty). */
int radix_starts_with(const radix_tree *tree, const char *prefix);

/* radix_longest_prefix_match — find the longest stored key that is a prefix of
 * `key`. On a match writes it into `out` (up to `out_cap` bytes) and returns its
 * length; returns -1 if no stored key is a prefix of `key`. */
long radix_longest_prefix_match(const radix_tree *tree, const char *key,
                                char *out, size_t out_cap);

/* radix_len — number of keys stored. */
size_t radix_len(const radix_tree *tree);

/* radix_is_empty — 1 if no keys are stored. */
int radix_is_empty(const radix_tree *tree);

/* radix_node_count — total number of nodes (including the root). */
size_t radix_node_count(const radix_tree *tree);

/* Visitor callback for key enumeration: one key (a NUL-terminated string of
 * `len` bytes) plus the caller's `user` pointer. */
typedef void (*radix_key_fn)(const char *key, size_t len, void *user);

/* radix_keys — visit every key in ascending (byte) order. */
void radix_keys(const radix_tree *tree, radix_key_fn fn, void *user);

/* radix_words_with_prefix — visit every key having `prefix` as a prefix, in
 * ascending order. */
void radix_words_with_prefix(const radix_tree *tree, const char *prefix,
                             radix_key_fn fn, void *user);

#endif /* RADIX_TREE_H */
