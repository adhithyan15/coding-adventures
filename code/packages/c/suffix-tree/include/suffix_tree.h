/*
 * suffix_tree.h — a suffix index over a string, in pure ISO C17. A faithful port
 * of the Rust `suffix-tree` crate (DT15).
 * ===========================================================================
 *
 * The reference crate keeps a deliberately simple structure — a root whose
 * children are one leaf per suffix start — and answers substring queries with
 * direct string scans over the stored text. This port mirrors that exactly:
 * `suffix_tree` owns a copy of the text, and the query functions operate on it.
 *
 *   suffix_tree_search           — every start position where a pattern occurs
 *   suffix_tree_count_occurrences — how many times a pattern occurs
 *   suffix_tree_longest_repeated_substring — longest substring that repeats
 *   suffix_tree_node_count       — 1 (root) + one leaf per character
 *   suffix_tree_suffix           — borrow suffix i (text[i..])
 *
 * Plus two free functions that need no tree:
 *   suffix_longest_common_substring — LCS of two strings (dynamic programming)
 *
 * The crate counts Unicode scalar values ("chars"); this port works on bytes,
 * so results match for any ASCII / single-byte text (which is the usual case
 * for a C string API). Positions and lengths are byte offsets.
 *
 * Ownership: suffix_tree_build allocates; pair it with suffix_tree_free.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef SUFFIX_TREE_H
#define SUFFIX_TREE_H

#include <stddef.h> /* size_t */

typedef struct suffix_tree suffix_tree;

/* suffix_tree_build — build a tree over a copy of `text` (`len` bytes). Returns
 * NULL on allocation failure. */
suffix_tree *suffix_tree_build(const char *text, size_t len);

/* suffix_tree_free — release the tree. NULL-safe. */
void suffix_tree_free(suffix_tree *tree);

/* suffix_tree_text_len — length in bytes of the stored text. */
size_t suffix_tree_text_len(const suffix_tree *tree);

/* suffix_tree_node_count — 1 (root) + one leaf per byte of text. */
size_t suffix_tree_node_count(const suffix_tree *tree);

/* suffix_tree_search — find every start offset where `pattern` (`plen` bytes)
 * occurs in the text. Writes the first `min(count, out_cap)` offsets into `out`
 * (which may be NULL when out_cap is 0) and returns the TOTAL number of
 * occurrences. An empty pattern matches at every position 0..=text_len
 * (text_len + 1 of them), mirroring the Rust crate. */
size_t suffix_tree_search(const suffix_tree *tree, const char *pattern,
                          size_t plen, size_t *out, size_t out_cap);

/* suffix_tree_count_occurrences — number of times `pattern` occurs. */
size_t suffix_tree_count_occurrences(const suffix_tree *tree,
                                     const char *pattern, size_t plen);

/* suffix_tree_longest_repeated_substring — write the longest substring that
 * occurs at least twice into `out` (up to `out_cap` bytes) and return its full
 * length (which may exceed out_cap, in which case the write was truncated). */
size_t suffix_tree_longest_repeated_substring(const suffix_tree *tree, char *out,
                                              size_t out_cap);

/* suffix_tree_suffix — borrow suffix `i` (the substring text[i..]). On success
 * (i < text_len) sets *ptr to a pointer into the stored text and *suffix_len to
 * text_len - i, and returns 1. Returns 0 if i is out of range. */
int suffix_tree_suffix(const suffix_tree *tree, size_t i, const char **ptr,
                       size_t *suffix_len);

/* suffix_longest_common_substring — write the longest substring common to `a`
 * (`alen` bytes) and `b` (`blen` bytes) into `out` (up to `out_cap` bytes) and
 * return its full length. Returns 0 (empty) if either input is empty or on an
 * internal allocation failure. */
size_t suffix_longest_common_substring(const char *a, size_t alen, const char *b,
                                       size_t blen, char *out, size_t out_cap);

#endif /* SUFFIX_TREE_H */
