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

/* ── Bounded draws (spec rng.md, "Rejection Sampling", v2) ─────────────────
 *
 * A uniform integer in [min, max], drawn from a stream of 32-bit outputs,
 * identically in every language of the repository. All three generators
 * share this logic; `draw` supplies the raw 32-bit stream.
 *
 * span = max - min + 1 lies between 1 and 2^64. Taking r % span of a raw draw
 * over-samples small offsets whenever the number of possible draws is not a
 * multiple of span; rejecting the `threshold` lowest draws removes exactly the
 * excess. Three regimes, chosen by span alone:
 *
 *     span <= 2^32    draw 32 bits,  threshold = (2^32 - span) mod span
 *     span <  2^64    draw 64 bits,  threshold = (2^64 - span) mod span
 *     span == 2^64    the whole int64 domain: every uint64 is a valid offset
 *
 * Example, span = 3: 2^32 = 3 * 1431655765 + 1, so threshold = 1. Rejecting
 * r = 0 leaves 2^32 - 1 draws, an exact multiple of 3.
 *
 * v1 took the threshold modulo 2^64 while drawing 32 bits. That was biased,
 * never reached more than 2^32 - 1 above min, and for spans such as 2^63 + 1
 * rejected every possible draw, so it never returned. Its whole-domain case
 * added a single 32-bit draw to min, reaching only 2^32 of the 2^64 values.
 *
 * Returns 0 and stores the value in *out, or returns -1 without drawing
 * anything when min > max. All arithmetic is unsigned, so it wraps instead of
 * overflowing. */
typedef uint32_t (*next_u32_fn)(void *);

static uint64_t draw64(void *g, next_u32_fn draw) {
    uint64_t hi = draw(g);
    uint64_t lo = draw(g);
    return (hi << 32) | lo;
}

static int range_from(void *g, next_u32_fn draw, int64_t min, int64_t max,
                      int64_t *out) {
    uint64_t span, threshold, r;
    if (min > max) {
        return -1;
    }
    span = ((uint64_t)max - (uint64_t)min) + 1u; /* 2^64 wraps to 0 */
    if (span == 0) {
        r = draw64(g, draw);
    } else if (span <= 0x100000000u) {
        threshold = (0x100000000u - span) % span;
        do {
            r = draw(g);
        } while (r < threshold);
        r %= span;
    } else {
        threshold = (0u - span) % span; /* (2^64 - span) mod span */
        do {
            r = draw64(g, draw);
        } while (r < threshold);
        r %= span;
    }
    /* min + r lies in [min, max]; converting that value back to int64_t is
     * exact because it is representable. */
    *out = (int64_t)((uint64_t)min + r);
    return 0;
}

/* The unchecked form keeps v1's contract: min > max returns min. */
static int64_t range_or_min(void *g, next_u32_fn draw, int64_t min,
                            int64_t max) {
    int64_t value = min;
    (void)range_from(g, draw, min, max, &value);
    return value;
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
int64_t rng_lcg_next_int_in_range(rng_lcg *g, int64_t min,
                                  int64_t max) {
    return range_or_min(g, lcg_draw, min, max);
}
int rng_lcg_try_int_in_range(rng_lcg *g, int64_t min, int64_t max,
                             int64_t *out) {
    return range_from(g, lcg_draw, min, max, out);
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
    return range_or_min(g, xs_draw, min, max);
}
int rng_xorshift64_try_int_in_range(rng_xorshift64 *g, int64_t min, int64_t max,
                                    int64_t *out) {
    return range_from(g, xs_draw, min, max, out);
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
int64_t rng_pcg32_next_int_in_range(rng_pcg32 *g, int64_t min,
                                    int64_t max) {
    return range_or_min(g, pcg_draw, min, max);
}
int rng_pcg32_try_int_in_range(rng_pcg32 *g, int64_t min, int64_t max,
                               int64_t *out) {
    return range_from(g, pcg_draw, min, max, out);
}
