/*
 * rng.h — deterministic pseudo-random number generators, in pure ISO C17. A
 * faithful port of the Rust `rng` crate.
 * ===========================================================================
 *
 * Three classic non-cryptographic PRNGs, each fully deterministic given a seed
 * (so results are reproducible):
 *
 *   rng_lcg        — a 64-bit Linear Congruential Generator (returns the high
 *                    32 bits, discarding the low-quality low bits)
 *   rng_xorshift64 — Marsaglia's Xorshift64 (three XOR-shifts, no multiply)
 *   rng_pcg32      — O'Neill's PCG32 (an LCG plus the XSH-RR output permutation)
 *
 * Each exposes the same interface: init, next_u32, next_u64 (two u32s),
 * next_float (a double in [0, 1)), and next_int_in_range (rejection-sampled to
 * avoid modulo bias).
 *
 * These are NOT cryptographically secure — do not use them for keys or nonces.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef RNG_H
#define RNG_H

#include <stdint.h> /* uint32_t, uint64_t, int64_t */

/* ── Lcg — 64-bit linear congruential generator ───────────────────────────── */
typedef struct {
    uint64_t state;
} rng_lcg;

void rng_lcg_init(rng_lcg *g, uint64_t seed);
uint32_t rng_lcg_next_u32(rng_lcg *g);
uint64_t rng_lcg_next_u64(rng_lcg *g);
double rng_lcg_next_float(rng_lcg *g); /* [0.0, 1.0) */
/* Uniform integer in [min, max] (rejection sampling). If min > max, returns
 * min (the Rust crate panics). */
int64_t rng_lcg_next_int_in_range(rng_lcg *g, int64_t min, int64_t max);

/* ── Xorshift64 — Marsaglia (2003) ────────────────────────────────────────── */
typedef struct {
    uint64_t state;
} rng_xorshift64;

/* Seed 0 is replaced with 1 (0 is the all-zeros fixed point). */
void rng_xorshift64_init(rng_xorshift64 *g, uint64_t seed);
uint32_t rng_xorshift64_next_u32(rng_xorshift64 *g);
uint64_t rng_xorshift64_next_u64(rng_xorshift64 *g);
double rng_xorshift64_next_float(rng_xorshift64 *g);
int64_t rng_xorshift64_next_int_in_range(rng_xorshift64 *g, int64_t min,
                                         int64_t max);

/* ── Pcg32 — O'Neill (2014), XSH-RR ───────────────────────────────────────── */
typedef struct {
    uint64_t state;
    uint64_t increment;
} rng_pcg32;

void rng_pcg32_init(rng_pcg32 *g, uint64_t seed);
uint32_t rng_pcg32_next_u32(rng_pcg32 *g);
uint64_t rng_pcg32_next_u64(rng_pcg32 *g);
double rng_pcg32_next_float(rng_pcg32 *g);
int64_t rng_pcg32_next_int_in_range(rng_pcg32 *g, int64_t min, int64_t max);

#endif /* RNG_H */
