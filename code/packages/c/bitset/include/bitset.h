/*
 * bitset.h — a growable set of bits packed into 64-bit words, in pure ISO C17.
 * A faithful port of the Rust `bitset` crate.
 * ===========================================================================
 *
 * A bitset stores a sequence of bits (indexed from 0) packed 64 to a machine
 * word, so set/clear/test are single word operations and the bitwise set
 * operations (and/or/xor/not) run a word at a time.
 *
 *   bit index:   ... 5 4 3 2 1 0     word 0 holds bits 0..63,
 *   word 0:      0b...0 1 0 0 1 0     word 1 holds bits 64..127, and so on.
 *
 * `set` and `toggle` AUTO-GROW the bitset if the index is past the current
 * length (capacity doubles as needed); `clear` and `test` treat out-of-range
 * indices as unset. `len` is the logical bit count; `capacity` is how many bits
 * the current allocation could hold (a multiple of 64).
 *
 * The bitwise operations return a NEW bitset (written through an out-parameter,
 * which the caller must free). Bit 0 is the least-significant bit, so
 * from_binary_str("101") sets bits 0 and 2 (value 5) and to_binary_str prints
 * most-significant bit first.
 *
 * The bitset owns a heap allocation — pair every constructor with bitset_free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef BITSET_H
#define BITSET_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint64_t */

/* The bitset. Treat the fields as opaque; use the functions below. */
typedef struct {
    uint64_t *words;
    size_t nwords; /* allocated word count (capacity = nwords * 64) */
    size_t len;    /* logical bit count */
} bitset;

/* --- Construction (all return 1 on success, 0 on allocation failure) --- */

/* bitset_init — an all-zero bitset of `size` bits. */
int bitset_init(bitset *b, size_t size);

/* bitset_from_integer — a bitset holding the 128-bit value (`high` << 64 |
 * `low`). Zero yields an empty (len 0) bitset; otherwise len is 64 (high == 0)
 * or 128. */
int bitset_from_integer(bitset *b, uint64_t low, uint64_t high);

/* bitset_from_binary_str — parse a string of '0'/'1' (most-significant bit
 * first; the string length is the bit length). Returns 1 on success, 0 on
 * allocation failure, or -1 if `s` contains a non-binary character. */
int bitset_from_binary_str(bitset *b, const char *s);

/* bitset_free — release storage. Safe on a zeroed struct; idempotent. */
void bitset_free(bitset *b);

/* --- Single-bit operations --- */

/* bitset_set — set bit `i` to 1, growing the bitset if needed.
 * Returns 1, or 0 on allocation failure. */
int bitset_set(bitset *b, size_t i);

/* bitset_toggle — flip bit `i`, growing the bitset if needed. */
int bitset_toggle(bitset *b, size_t i);

/* bitset_clear — set bit `i` to 0 (no-op if i >= len; never grows). */
void bitset_clear(bitset *b, size_t i);

/* bitset_test — 1 if bit `i` is set, else 0 (0 for i >= len). */
int bitset_test(const bitset *b, size_t i);

/* --- Bitwise set operations (write a new bitset into *out) --- */

int bitset_and(const bitset *a, const bitset *b, bitset *out);
int bitset_or(const bitset *a, const bitset *b, bitset *out);
int bitset_xor(const bitset *a, const bitset *b, bitset *out);
int bitset_and_not(const bitset *a, const bitset *b, bitset *out); /* a & ~b */
int bitset_not(const bitset *a, bitset *out);                      /* size-preserving */

/* --- Queries --- */

size_t bitset_popcount(const bitset *b); /* number of set bits */
size_t bitset_len(const bitset *b);      /* logical bit count */
size_t bitset_capacity(const bitset *b); /* nwords * 64 */
int bitset_any(const bitset *b);         /* 1 if any bit set */
int bitset_all(const bitset *b);         /* 1 if all `len` bits set (true if len 0) */
int bitset_none(const bitset *b);        /* 1 if no bit set */
int bitset_is_empty(const bitset *b);    /* 1 if len == 0 */

/* bitset_to_integer — if no bit beyond index 63 is set, write the low 64 bits
 * to *out and return 1; otherwise return 0 (value does not fit in 64 bits). */
int bitset_to_integer(const bitset *b, uint64_t *out);

/* bitset_to_binary_str — write the most-significant-bit-first '0'/'1' string
 * (length == len) plus a NUL into `buf` (capacity `buf_size`). Returns the
 * number of characters written, or -1 if `buf` is too small. */
long bitset_to_binary_str(const bitset *b, char *buf, size_t buf_size);

#endif /* BITSET_H */
