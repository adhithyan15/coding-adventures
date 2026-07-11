/*
 * ct_compare.c — implementation of constant-time comparison (see ct_compare.h).
 * A faithful port of the Rust `ct-compare` crate; the crate's `black_box`
 * optimiser barrier is realised here as a read through a `volatile` object,
 * which is the portable, pure-ISO way to stop the compiler folding the loop
 * back into an early-exit.
 */
#include "ct_compare.h"

/* Optimiser barriers: force the value to be materialised (the volatile read may
 * not be elided), preventing early-exit reintroduction. */
static uint8_t barrier_u8(uint8_t x) {
    volatile uint8_t v = x;
    return v;
}
static uint64_t barrier_u64(uint64_t x) {
    volatile uint64_t v = x;
    return v;
}

int ct_eq(const uint8_t *a, size_t alen, const uint8_t *b, size_t blen) {
    uint8_t acc = 0;
    size_t i;
    if (alen != blen) {
        return 0; /* length is public — this early return is intentional */
    }
    for (i = 0; i < alen; i++) {
        acc |= (uint8_t)(a[i] ^ b[i]);
    }
    return barrier_u8(acc) == 0 ? 1 : 0;
}

int ct_eq_fixed(const uint8_t *a, const uint8_t *b, size_t n) {
    uint8_t acc = 0;
    size_t i;
    for (i = 0; i < n; i++) {
        acc |= (uint8_t)(a[i] ^ b[i]);
    }
    return barrier_u8(acc) == 0 ? 1 : 0;
}

void ct_select_bytes(const uint8_t *a, const uint8_t *b, int choice, size_t n,
                     uint8_t *out) {
    /* mask = 0xFF when choice != 0, else 0x00 — with no branch. */
    uint8_t mask = (uint8_t)(0u - (unsigned)(choice != 0));
    size_t i;
    for (i = 0; i < n; i++) {
        /* b ^ ((a ^ b) & mask): a when mask=0xFF, b when mask=0x00. */
        out[i] = (uint8_t)(b[i] ^ ((uint8_t)(a[i] ^ b[i]) & mask));
    }
}

int ct_eq_u64(uint64_t a, uint64_t b) {
    uint64_t diff = a ^ b;
    /* Fold every bit of diff into the top bit: 0 iff diff == 0. */
    uint64_t folded = (diff | (0u - diff)) >> 63;
    return barrier_u64(folded) == 0 ? 1 : 0;
}
