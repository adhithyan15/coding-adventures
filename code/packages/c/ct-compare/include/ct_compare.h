/*
 * ct_compare.h — constant-time byte comparison, in pure ISO C17. A faithful port
 * of the Rust `ct-compare` crate.
 * ===========================================================================
 *
 * A naive `memcmp` (or an `==` loop that returns early on the first mismatch)
 * takes a different amount of time depending on WHERE two values first differ.
 * For secrets — MAC/auth tags, derived keys, password hashes — that timing is a
 * side channel an attacker can exploit to recover the secret byte by byte.
 *
 * These routines instead do the SAME work for every byte regardless of its
 * value: they fold all differences into an accumulator with no data-dependent
 * branch, then check the accumulator once at the end.
 *
 *   ct_eq        — equal-length? and equal bytes? (length is treated as public)
 *   ct_eq_fixed  — equal bytes over a known length (no length check)
 *   ct_select_bytes — branchless select between two equal-length buffers
 *   ct_eq_u64    — constant-time equality of two 64-bit values
 *
 * The Rust crate uses `core::hint::black_box` as an optimiser barrier so the
 * loop is not folded back into an early-exit; the pure-ISO equivalent here is a
 * read through a `volatile` object.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef CT_COMPARE_H
#define CT_COMPARE_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint64_t */

/* ct_eq — 1 iff `a` (alen bytes) and `b` (blen bytes) have the same length AND
 * the same bytes; else 0. The length comparison is early (length is public); the
 * byte comparison is constant-time. */
int ct_eq(const uint8_t *a, size_t alen, const uint8_t *b, size_t blen);

/* ct_eq_fixed — 1 iff the `n` bytes of `a` and `b` are equal (no length check;
 * both must be at least `n` bytes). Constant-time over the `n` bytes. */
int ct_eq_fixed(const uint8_t *a, const uint8_t *b, size_t n);

/* ct_select_bytes — branchless select: writes a copy of `a` if `choice` is
 * non-zero, else a copy of `b`, into `out` (all three buffers are `n` bytes). No
 * instruction branches on `choice`. */
void ct_select_bytes(const uint8_t *a, const uint8_t *b, int choice, size_t n,
                     uint8_t *out);

/* ct_eq_u64 — 1 iff a == b, computed without a data-dependent branch. */
int ct_eq_u64(uint64_t a, uint64_t b);

#endif /* CT_COMPARE_H */
