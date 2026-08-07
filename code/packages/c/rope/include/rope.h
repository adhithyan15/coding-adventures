/*
 * rope.h — a rope (a balanced binary tree of string chunks), in pure ISO C17. A
 * faithful port of the Rust `rope` crate (DT16).
 * ===========================================================================
 *
 * A rope stores a long string as a binary tree whose leaves hold chunks and
 * whose internal nodes carry a `weight` (the length of everything in the left
 * subtree). That makes concatenation O(1) and splitting/indexing cheap, without
 * copying the whole string on every edit.
 *
 * The Rust crate has *value/move* semantics: operations like `concat` and
 * `insert` take their rope arguments BY VALUE and return a new rope. This C port
 * mirrors that with a "consuming" API — every operation below that takes a
 * `rope *` argument TAKES OWNERSHIP of it: after the call the argument pointer is
 * dead (do not use or free it), and the returned rope owns all the memory. This
 * is how the port stays allocation-frugal (subtrees are moved, not copied) while
 * matching the crate's semantics.
 *
 *   rope_from_string / rope_empty        — construct
 *   rope_concat                          — O(1) join (moves both subtrees)
 *   rope_split / rope_insert / rope_delete / rope_rebalance — edits
 *   rope_to_string / rope_index / rope_substring — read
 *   rope_len / rope_depth / rope_is_balanced     — measure
 *
 * The crate counts Unicode scalar values; this port works on bytes, so results
 * match for ASCII / single-byte text. Indices and lengths are byte offsets.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef ROPE_H
#define ROPE_H

#include <stddef.h> /* size_t */

typedef struct rope rope;

/* rope_empty — a new empty rope. NULL on allocation failure. */
rope *rope_empty(void);

/* rope_from_string — a new rope holding a copy of `text` (`len` bytes). An empty
 * string yields an empty rope. NULL on allocation failure. */
rope *rope_from_string(const char *text, size_t len);

/* rope_free — release a rope and its whole tree. NULL-safe. */
void rope_free(rope *r);

/* rope_len — number of bytes stored. */
size_t rope_len(const rope *r);

/* rope_is_empty — 1 if the rope holds no bytes. */
int rope_is_empty(const rope *r);

/* rope_concat — join `left` and `right` into one rope in O(1). CONSUMES both
 * arguments. Returns the new rope, or NULL on allocation failure (in which case
 * both arguments have still been freed). */
rope *rope_concat(rope *left, rope *right);

/* rope_split — split `rope` at byte `i` into `*out_left` (bytes 0..i) and
 * `*out_right` (bytes i..end); `i` is clamped to the length. CONSUMES `r`.
 * Returns 1 on success; on allocation failure returns 0 and sets both outputs to
 * NULL. */
int rope_split(rope *r, size_t i, rope **out_left, rope **out_right);

/* rope_insert — insert `s` (`slen` bytes) at byte offset `i`. CONSUMES `r`.
 * Returns the new rope, or NULL on allocation failure. */
rope *rope_insert(rope *r, size_t i, const char *s, size_t slen);

/* rope_delete — remove `length` bytes starting at byte `start` (both clamped).
 * CONSUMES `r`. Returns the new rope, or NULL on allocation failure. */
rope *rope_delete(rope *r, size_t start, size_t length);

/* rope_rebalance — rebuild `r` as a balanced tree over the same bytes. CONSUMES
 * `r`. Returns the new rope, or NULL on allocation failure. */
rope *rope_rebalance(rope *r);

/* rope_to_string — write the rope's bytes into `out` (up to `out_cap`) and
 * return the full length (which may exceed out_cap). `out` may be NULL when
 * out_cap is 0. Does NOT consume `r`. */
size_t rope_to_string(const rope *r, char *out, size_t out_cap);

/* rope_index — the byte at offset `i`. On success (i < len) writes it to
 * *out_byte and returns 1; returns 0 if out of range. Does NOT consume `r`. */
int rope_index(const rope *r, size_t i, char *out_byte);

/* rope_substring — write bytes [start, end) (both clamped to the length) into
 * `out` (up to `out_cap`) and return the number of bytes in the range. Does NOT
 * consume `r`. */
size_t rope_substring(const rope *r, size_t start, size_t end, char *out,
                      size_t out_cap);

/* rope_depth — height of the tree (0 for empty or a single leaf). */
size_t rope_depth(const rope *r);

/* rope_is_balanced — 1 if every internal node's subtree heights differ by <= 1. */
int rope_is_balanced(const rope *r);

#endif /* ROPE_H */
