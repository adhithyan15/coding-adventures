/*
 * pbt_prng.c -- SplitMix64, rejection sampling and FNV-1a (VER05 section 2).
 *
 * SPLITMIX64
 * ----------
 * A 64-bit counter walked by the "golden gamma" and scrambled by a finalizer:
 *
 *     state += 0x9e3779b97f4a7c15
 *     z = (state ^ (state >> 30)) * 0xbf58476d1ce4e5b9
 *     z = (z     ^ (z     >> 27)) * 0x94d049bb133111eb
 *     out = z ^ (z >> 31)
 *
 * `uint64_t` arithmetic wraps modulo 2^64 by definition in C, so no masking.
 * Seed 0 gives 0xe220a8397b1dcdaf first (pinned by a test).
 *
 * UNBIASED BOUNDED DRAWS
 * ----------------------
 * `x % n` over a 64-bit word is biased unless n divides 2^64: the first
 * (2^64 mod n) residues are hit once more than the rest.  Rejection sampling
 * throws away the low `t = 2^64 mod n` words so the remaining count is an exact
 * multiple of n.  Mathematically t = (2^64 - n) mod n, and in C that is
 *
 *     (0 - n) % n          // unsigned: 0 - n wraps to 2^64 - n
 *
 * which is CORRECT here (it is not in Python, which has no wraparound).  The
 * same expression through a SIGNED type is wrong for n > 2^63, which is why a
 * test pins n = 2^63 + 1.
 */
#include <string.h>

#include "pbt.h"
#include "pbt_hooks.h"

void pbt_rng_init(pbt_rng *r, uint64_t seed)
{
    r->state = pbt_hooks.seed_map != NULL ? pbt_hooks.seed_map(seed) : seed;
}

uint64_t pbt_rng_next(pbt_rng *r)
{
    uint64_t z;
    r->state += 0x9e3779b97f4a7c15u;
    z = r->state;
    z = (z ^ (z >> 30)) * 0xbf58476d1ce4e5b9u;
    z = (z ^ (z >> 27)) * 0x94d049bb133111ebu;
    return z ^ (z >> 31);
}

uint64_t pbt_rng_bounded(pbt_rng *r, uint64_t n)
{
    uint64_t threshold;
    if (n == 0) {
        return 0;
    }
    threshold = pbt_hooks.threshold != NULL ? pbt_hooks.threshold(n) : ((uint64_t)0 - n) % n;
    for (;;) {
        uint64_t x = pbt_rng_next(r);
        if (x >= threshold) {
            return x % n;
        }
    }
}

int pbt_rng_bounded_big(pbt_rng *r, const pbt_big *span, pbt_big *out)
{
    pbt_big spanm1;
    pbt_big one;
    pbt_big t;
    pbt_big raw;
    pbt_big full;
    uint32_t bl;
    uint32_t k;
    uint32_t i;

    if (pbt_big_sign(span) <= 0 || pbt_big_bitlen(span) > PBT_BIG_MAX_BITS) {
        pbt_big_zero(out);
        return 1;
    }
    if (span->n <= 2) {
        pbt_big_set_u64(out, pbt_rng_bounded(r, pbt_big_low_u64(span)));
        return 0;
    }
    if (span->n == 3 && span->d[2] == 1 && span->d[1] == 0 && span->d[0] == 0) {
        pbt_big_set_u64(out, pbt_rng_next(r)); /* exactly 2^64: any word is uniform */
        return 0;
    }

    /* k = max(2, ceil(bitlen(span-1) / 64)) words per attempt. */
    pbt_big_set_u64(&one, 1);
    (void)pbt_big_sub(&spanm1, span, &one);
    bl = pbt_big_bitlen(&spanm1);
    k = (bl + 63u) / 64u;
    if (k < 2) {
        k = 2;
    }
    /* t = (2^(64k) - span) mod span */
    if (pbt_big_pow2(&full, 64u * k) || pbt_big_sub(&t, &full, span)) {
        pbt_big_zero(out);
        return 1;
    }
    (void)pbt_big_divmod_abs(NULL, &t, &t, span);

    for (;;) {
        pbt_big_zero(&raw);
        for (i = 0; i < k; i++) {
            /* The first word drawn is the most significant. */
            uint64_t w = pbt_rng_next(r);
            uint32_t at = 2u * (k - 1u - i);
            raw.d[at] = (uint32_t)(w & 0xffffffffu);
            raw.d[at + 1] = (uint32_t)(w >> 32);
        }
        raw.n = 2u * k;
        while (raw.n > 0 && raw.d[raw.n - 1] == 0) {
            raw.n--;
        }
        if (pbt_big_cmp_abs(&raw, &t) >= 0) {
            break;
        }
    }
    (void)pbt_big_divmod_abs(NULL, out, &raw, span);
    return 0;
}

uint64_t pbt_fnv1a64(const void *data, size_t len)
{
    const unsigned char *p = (const unsigned char *)data;
    uint64_t h = 0xcbf29ce484222325u;
    size_t i;
    for (i = 0; i < len; i++) {
        h ^= p[i];
        h *= 0x100000001b3u;
    }
    return h;
}

uint64_t pbt_sub_seed(uint64_t run_seed, const char *property_id)
{
    pbt_rng r;
    r.state = run_seed ^ pbt_fnv1a64(property_id, strlen(property_id));
    return pbt_rng_next(&r);
}
