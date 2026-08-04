/*
 * rng.c — implementation of the three PRNGs (see rng.h). A faithful port of the
 * Rust `rng` crate; all arithmetic is unsigned (wrapping) 32/64-bit, so no
 * 128-bit type is needed.
 */
#include "rng.h"

/* Shared LCG constants (the PCG / Numerical-Recipes 64-bit multiplier). */
static const uint64_t LCG_MULTIPLIER = 6364136223846793005ULL;
static const uint64_t LCG_INCREMENT = 1442695040888963407ULL;
#define FLOAT_DIV 4294967296.0 /* 2^32 */

/* Rejection-sampled uniform in [min, max]; `draw` supplies the raw 32-bit
 * stream (all three generators share this logic). */
typedef uint32_t (*next_u32_fn)(void *);
static int64_t range_from(void *g, next_u32_fn draw, int64_t min, int64_t max) {
    uint64_t range_size, threshold;
    if (min > max) {
        return min; /* the Rust crate asserts min <= max */
    }
    range_size = ((uint64_t)max - (uint64_t)min) + 1ULL; /* wrapping arithmetic */
    if (range_size == 0) {
        /* Full 64-bit range (min=INT64_MIN, max=INT64_MAX): the Rust crate would
         * divide by zero. Return a defined value instead. */
        return (int64_t)((uint64_t)min + draw(g));
    }
    threshold = (0ULL - range_size) % range_size;
    for (;;) {
        uint64_t r = draw(g);
        if (r >= threshold) {
            return (int64_t)((uint64_t)min + (r % range_size));
        }
    }
}

/* ── Lcg ──────────────────────────────────────────────────────────────────── */
void rng_lcg_init(rng_lcg *g, uint64_t seed) { g->state = seed; }

uint32_t rng_lcg_next_u32(rng_lcg *g) {
    g->state = g->state * LCG_MULTIPLIER + LCG_INCREMENT; /* unsigned wraps */
    return (uint32_t)(g->state >> 32);                    /* high 32 bits */
}
uint64_t rng_lcg_next_u64(rng_lcg *g) {
    uint64_t hi = rng_lcg_next_u32(g);
    uint64_t lo = rng_lcg_next_u32(g);
    return (hi << 32) | lo;
}
double rng_lcg_next_float(rng_lcg *g) {
    return (double)rng_lcg_next_u32(g) / FLOAT_DIV;
}
static uint32_t lcg_draw(void *g) { return rng_lcg_next_u32((rng_lcg *)g); }
int64_t rng_lcg_next_int_in_range(rng_lcg *g, int64_t min, int64_t max) {
    return range_from(g, lcg_draw, min, max);
}

/* ── Xorshift64 ───────────────────────────────────────────────────────────── */
void rng_xorshift64_init(rng_xorshift64 *g, uint64_t seed) {
    g->state = (seed == 0) ? 1ULL : seed;
}
uint32_t rng_xorshift64_next_u32(rng_xorshift64 *g) {
    uint64_t x = g->state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    g->state = x;
    return (uint32_t)x;
}
uint64_t rng_xorshift64_next_u64(rng_xorshift64 *g) {
    uint64_t hi = rng_xorshift64_next_u32(g);
    uint64_t lo = rng_xorshift64_next_u32(g);
    return (hi << 32) | lo;
}
double rng_xorshift64_next_float(rng_xorshift64 *g) {
    return (double)rng_xorshift64_next_u32(g) / FLOAT_DIV;
}
static uint32_t xs_draw(void *g) {
    return rng_xorshift64_next_u32((rng_xorshift64 *)g);
}
int64_t rng_xorshift64_next_int_in_range(rng_xorshift64 *g, int64_t min,
                                         int64_t max) {
    return range_from(g, xs_draw, min, max);
}

/* ── Pcg32 ────────────────────────────────────────────────────────────────── */
void rng_pcg32_init(rng_pcg32 *g, uint64_t seed) {
    g->increment = LCG_INCREMENT | 1ULL; /* must be odd */
    g->state = 0;
    g->state = g->state * LCG_MULTIPLIER + g->increment; /* advance from zero */
    g->state = g->state + seed;                          /* mix seed in */
    g->state = g->state * LCG_MULTIPLIER + g->increment; /* scatter seed bits */
}
uint32_t rng_pcg32_next_u32(rng_pcg32 *g) {
    uint64_t old_state = g->state;
    uint32_t xorshifted, rot;
    g->state = old_state * LCG_MULTIPLIER + g->increment;
    /* XSH-RR output permutation. */
    xorshifted = (uint32_t)(((old_state >> 18) ^ old_state) >> 27);
    rot = (uint32_t)(old_state >> 59); /* 0..31 */
    /* rotate-right by rot; the ((32-rot) & 31) form is well-defined at rot 0. */
    return (uint32_t)((xorshifted >> rot) | (xorshifted << ((32 - rot) & 31)));
}
uint64_t rng_pcg32_next_u64(rng_pcg32 *g) {
    uint64_t hi = rng_pcg32_next_u32(g);
    uint64_t lo = rng_pcg32_next_u32(g);
    return (hi << 32) | lo;
}
double rng_pcg32_next_float(rng_pcg32 *g) {
    return (double)rng_pcg32_next_u32(g) / FLOAT_DIV;
}
static uint32_t pcg_draw(void *g) { return rng_pcg32_next_u32((rng_pcg32 *)g); }
int64_t rng_pcg32_next_int_in_range(rng_pcg32 *g, int64_t min, int64_t max) {
    return range_from(g, pcg_draw, min, max);
}
