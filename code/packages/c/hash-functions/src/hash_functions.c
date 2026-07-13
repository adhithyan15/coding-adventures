/*
 * hash_functions.c — implementation of the pure-ISO C hash functions.
 * ===================================================================
 *
 * See hash_functions.h for the overview. Each routine mirrors the Rust crate's
 * arithmetic exactly; unsigned wraparound in C provides the `wrapping_*`
 * semantics for free.
 */
#include "hash_functions.h"

#include <stdlib.h> /* calloc, free */
#include <string.h> /* strlen, memcpy */

/* ── Small bit helpers ─────────────────────────────────────────────────────*/

/* Rotate a 32-bit word left by `r` (0 < r < 32). */
static uint32_t rotl32(uint32_t x, unsigned r) {
    return (uint32_t)((x << r) | (x >> (32 - r)));
}
/* Rotate a 64-bit word left by `r` (0 < r < 64). */
static uint64_t rotl64(uint64_t x, unsigned r) {
    return (uint64_t)((x << r) | (x >> (64 - r)));
}
/* Population count (number of set bits) of a 64-bit word — Kernighan's method,
 * so no compiler builtins. Used by avalanche scoring (`count_ones`). */
static uint32_t popcount64(uint64_t x) {
    uint32_t n = 0;
    while (x != 0) {
        x &= x - 1;
        n++;
    }
    return n;
}

/* Read 4/8 bytes little-endian (the crate uses `u32/u64::from_le_bytes`). */
static uint32_t load_u32_le(const uint8_t *p) {
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) |
           ((uint32_t)p[3] << 24);
}
static uint64_t load_u64_le(const uint8_t *p) {
    return (uint64_t)p[0] | ((uint64_t)p[1] << 8) | ((uint64_t)p[2] << 16) |
           ((uint64_t)p[3] << 24) | ((uint64_t)p[4] << 32) |
           ((uint64_t)p[5] << 40) | ((uint64_t)p[6] << 48) |
           ((uint64_t)p[7] << 56);
}

/* ── FNV-1a ────────────────────────────────────────────────────────────────*/

uint32_t hf_fnv1a_32(const uint8_t *data, size_t len) {
    uint32_t hash = HF_FNV32_OFFSET_BASIS;
    for (size_t i = 0; i < len; i++) {
        hash ^= (uint32_t)data[i];
        hash *= HF_FNV32_PRIME; /* wraps mod 2^32 */
    }
    return hash;
}

uint64_t hf_fnv1a_64(const uint8_t *data, size_t len) {
    uint64_t hash = HF_FNV64_OFFSET_BASIS;
    for (size_t i = 0; i < len; i++) {
        hash ^= (uint64_t)data[i];
        hash *= HF_FNV64_PRIME; /* wraps mod 2^64 */
    }
    return hash;
}

/* ── DJB2 ──────────────────────────────────────────────────────────────────*/

uint64_t hf_djb2(const uint8_t *data, size_t len) {
    uint64_t hash = HF_DJB2_OFFSET_BASIS;
    for (size_t i = 0; i < len; i++)
        /* (hash << 5) + hash + byte == hash*33 + byte, all mod 2^64. */
        hash = (hash << 5) + hash + (uint64_t)data[i];
    return hash;
}

/* ── Polynomial rolling ────────────────────────────────────────────────────*/

/* Modular add: (a + b) mod m for a, b < m, without ever overflowing u64. If
 * a + b would wrap, `a >= m - b`, so `a - (m - b)` gives the reduced result. */
static uint64_t addmod(uint64_t a, uint64_t b, uint64_t m) {
    if (a >= m - b) return a - (m - b);
    return a + b;
}
/* Modular multiply: (a * b) mod m, exact for any m < 2^64. Binary (double-and-
 * add) using the overflow-safe `addmod`; this is what stands in for Rust's
 * `u128` intermediate. */
static uint64_t mulmod(uint64_t a, uint64_t b, uint64_t m) {
    uint64_t result = 0;
    a %= m;
    while (b != 0) {
        if (b & 1u) result = addmod(result, a, m);
        a = addmod(a, a, m); /* a = 2a mod m */
        b >>= 1;
    }
    return result;
}

uint64_t hf_polynomial_rolling_with_params(const uint8_t *data, size_t len,
                                           uint64_t base, uint64_t modulus) {
    uint64_t hash = 0;
    if (modulus == 0) return 0; /* Rust asserts modulus > 0. */
    for (size_t i = 0; i < len; i++)
        /* hash = (hash * base + byte) mod modulus */
        hash = addmod(mulmod(hash, base, modulus), (uint64_t)data[i] % modulus,
                      modulus);
    return hash;
}

uint64_t hf_polynomial_rolling(const uint8_t *data, size_t len) {
    return hf_polynomial_rolling_with_params(
        data, len, HF_POLYNOMIAL_ROLLING_DEFAULT_BASE,
        HF_POLYNOMIAL_ROLLING_DEFAULT_MODULUS);
}

/* ── Murmur3 (32-bit) ──────────────────────────────────────────────────────*/

#define MURMUR3_C1 ((uint32_t)0xCC9E2D51u)
#define MURMUR3_C2 ((uint32_t)0x1B873593u)

static uint32_t fmix32(uint32_t hash) {
    hash ^= hash >> 16;
    hash *= (uint32_t)0x85EBCA6Bu;
    hash ^= hash >> 13;
    hash *= (uint32_t)0xC2B2AE35u;
    hash ^= hash >> 16;
    return hash;
}

uint32_t hf_murmur3_32_with_seed(const uint8_t *data, size_t len,
                                 uint32_t seed) {
    uint32_t hash = seed;
    size_t nblocks = len / 4;

    for (size_t i = 0; i < nblocks; i++) {
        uint32_t k = load_u32_le(data + i * 4);
        k *= MURMUR3_C1;
        k = rotl32(k, 15);
        k *= MURMUR3_C2;

        hash ^= k;
        hash = rotl32(hash, 13);
        hash = hash * 5u + (uint32_t)0xE6546B64u;
    }

    /* Tail (0..3 leftover bytes). */
    size_t tail_len = len - nblocks * 4;
    if (tail_len != 0) {
        const uint8_t *tail = data + nblocks * 4;
        uint32_t k = 0;
        for (size_t i = 0; i < tail_len; i++)
            k ^= (uint32_t)tail[i] << (i * 8);
        k *= MURMUR3_C1;
        k = rotl32(k, 15);
        k *= MURMUR3_C2;
        hash ^= k;
    }

    hash ^= (uint32_t)len;
    return fmix32(hash);
}

uint32_t hf_murmur3_32(const uint8_t *data, size_t len) {
    return hf_murmur3_32_with_seed(data, len, 0);
}

/* ── SipHash-2-4 ───────────────────────────────────────────────────────────*/

#define SIPHASH_V0 ((uint64_t)0x736F6D6570736575ull)
#define SIPHASH_V1 ((uint64_t)0x646F72616E646F6Dull)
#define SIPHASH_V2 ((uint64_t)0x6C7967656E657261ull)
#define SIPHASH_V3 ((uint64_t)0x7465646279746573ull)

static void sipround(uint64_t *v0, uint64_t *v1, uint64_t *v2, uint64_t *v3) {
    *v0 += *v1;
    *v1 = rotl64(*v1, 13);
    *v1 ^= *v0;
    *v0 = rotl64(*v0, 32);

    *v2 += *v3;
    *v3 = rotl64(*v3, 16);
    *v3 ^= *v2;

    *v0 += *v3;
    *v3 = rotl64(*v3, 21);
    *v3 ^= *v0;

    *v2 += *v1;
    *v1 = rotl64(*v1, 17);
    *v1 ^= *v2;
    *v2 = rotl64(*v2, 32);
}

uint64_t hf_siphash_2_4(const uint8_t *data, size_t len, const uint8_t key[16]) {
    uint64_t k0 = load_u64_le(key);
    uint64_t k1 = load_u64_le(key + 8);

    uint64_t v0 = SIPHASH_V0 ^ k0;
    uint64_t v1 = SIPHASH_V1 ^ k1;
    uint64_t v2 = SIPHASH_V2 ^ k0;
    uint64_t v3 = SIPHASH_V3 ^ k1;

    size_t nblocks = len / 8;
    for (size_t i = 0; i < nblocks; i++) {
        uint64_t m = load_u64_le(data + i * 8);
        v3 ^= m;
        sipround(&v0, &v1, &v2, &v3);
        sipround(&v0, &v1, &v2, &v3);
        v0 ^= m;
    }

    /* Final block: the low byte holds len mod 256 in its top byte. */
    uint64_t last = ((uint64_t)len & 0xff) << 56;
    size_t tail_len = len - nblocks * 8;
    const uint8_t *tail = data + nblocks * 8;
    for (size_t i = 0; i < tail_len; i++)
        last |= (uint64_t)tail[i] << (i * 8);

    v3 ^= last;
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    v0 ^= last;

    v2 ^= 0xff;
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);
    sipround(&v0, &v1, &v2, &v3);

    return v0 ^ v1 ^ v2 ^ v3;
}

/* ── String helpers ────────────────────────────────────────────────────────*/

uint32_t hf_hash_str_fnv1a_32(const char *s) {
    return hf_fnv1a_32((const uint8_t *)s, strlen(s));
}
uint64_t hf_hash_str_siphash(const char *s, const uint8_t key[16]) {
    return hf_siphash_2_4((const uint8_t *)s, strlen(s), key);
}

/* ── HashFunction dispatch ─────────────────────────────────────────────────*/

HfHashFunction hf_new_fnv1a_32(void) {
    HfHashFunction h;
    memset(&h, 0, sizeof h);
    h.kind = HF_KIND_FNV1A_32;
    return h;
}
HfHashFunction hf_new_fnv1a_64(void) {
    HfHashFunction h;
    memset(&h, 0, sizeof h);
    h.kind = HF_KIND_FNV1A_64;
    return h;
}
HfHashFunction hf_new_djb2(void) {
    HfHashFunction h;
    memset(&h, 0, sizeof h);
    h.kind = HF_KIND_DJB2;
    return h;
}
HfHashFunction hf_new_polynomial_rolling_with(uint64_t base, uint64_t modulus) {
    HfHashFunction h;
    memset(&h, 0, sizeof h);
    h.kind = HF_KIND_POLYNOMIAL_ROLLING;
    h.base = base;
    h.modulus = modulus;
    return h;
}
HfHashFunction hf_new_polynomial_rolling(void) {
    return hf_new_polynomial_rolling_with(HF_POLYNOMIAL_ROLLING_DEFAULT_BASE,
                                          HF_POLYNOMIAL_ROLLING_DEFAULT_MODULUS);
}
HfHashFunction hf_new_murmur3_32_with_seed(uint32_t seed) {
    HfHashFunction h;
    memset(&h, 0, sizeof h);
    h.kind = HF_KIND_MURMUR3_32;
    h.seed = seed;
    return h;
}
HfHashFunction hf_new_murmur3_32(void) {
    return hf_new_murmur3_32_with_seed(0);
}
HfHashFunction hf_new_siphash_2_4(const uint8_t key[16]) {
    HfHashFunction h;
    memset(&h, 0, sizeof h);
    h.kind = HF_KIND_SIPHASH_2_4;
    memcpy(h.key, key, 16);
    return h;
}

uint64_t hf_hash(const HfHashFunction *hf, const uint8_t *data, size_t len) {
    switch (hf->kind) {
        case HF_KIND_FNV1A_32: return (uint64_t)hf_fnv1a_32(data, len);
        case HF_KIND_FNV1A_64: return hf_fnv1a_64(data, len);
        case HF_KIND_DJB2: return hf_djb2(data, len);
        case HF_KIND_POLYNOMIAL_ROLLING:
            return hf_polynomial_rolling_with_params(data, len, hf->base,
                                                     hf->modulus);
        case HF_KIND_MURMUR3_32:
            return (uint64_t)hf_murmur3_32_with_seed(data, len, hf->seed);
        case HF_KIND_SIPHASH_2_4: return hf_siphash_2_4(data, len, hf->key);
    }
    return 0; /* unreachable */
}

uint32_t hf_output_bits(const HfHashFunction *hf) {
    switch (hf->kind) {
        case HF_KIND_FNV1A_32:
        case HF_KIND_MURMUR3_32: return 32;
        case HF_KIND_FNV1A_64:
        case HF_KIND_DJB2:
        case HF_KIND_POLYNOMIAL_ROLLING:
        case HF_KIND_SIPHASH_2_4: return 64;
    }
    return 0; /* unreachable */
}

/* ── Analysis ──────────────────────────────────────────────────────────────*/

double hf_avalanche_score(HfHashCb hash, void *hash_ctx, uint32_t output_bits,
                          size_t sample_size, HfFillCb fill, void *fill_ctx) {
    uint64_t total_bit_flips = 0;
    uint64_t total_trials = 0;
    uint8_t input_bytes[8];

    if (sample_size == 0 || output_bits == 0 || output_bits > 64) return 0.0;

    for (size_t s = 0; s < sample_size; s++) {
        fill(input_bytes, sizeof input_bytes, fill_ctx);
        uint64_t h1 = hash(input_bytes, sizeof input_bytes, hash_ctx);

        for (size_t bit_pos = 0; bit_pos < sizeof(input_bytes) * 8; bit_pos++) {
            size_t byte_idx = bit_pos / 8;
            uint8_t bit_mask = (uint8_t)(1u << (bit_pos % 8));

            uint8_t flipped[8];
            memcpy(flipped, input_bytes, sizeof flipped);
            flipped[byte_idx] ^= bit_mask;
            uint64_t h2 = hash(flipped, sizeof flipped, hash_ctx);

            total_bit_flips += popcount64(h1 ^ h2);
            total_trials += output_bits;
        }
    }

    return (double)total_bit_flips / (double)total_trials;
}

double hf_distribution_test(HfHashCb hash, void *hash_ctx,
                            const HfInput *inputs, size_t n_inputs,
                            size_t num_buckets) {
    uint64_t *counts;
    uint64_t total = 0;
    double expected, chi2 = 0.0;

    if (num_buckets == 0 || n_inputs == 0) return -1.0;
    /* calloc does the checked multiply for us (guards size_t overflow). */
    counts = (uint64_t *)calloc(num_buckets, sizeof(uint64_t));
    if (counts == NULL) return -1.0;

    for (size_t i = 0; i < n_inputs; i++) {
        uint64_t h = hash(inputs[i].data, inputs[i].len, hash_ctx);
        counts[(size_t)(h % (uint64_t)num_buckets)]++;
    }

    for (size_t b = 0; b < num_buckets; b++) total += counts[b];
    if (total == 0) {
        free(counts);
        return -1.0;
    }

    expected = (double)total / (double)num_buckets;
    for (size_t b = 0; b < num_buckets; b++) {
        double observed = (double)counts[b];
        double delta = observed - expected;
        chi2 += delta * delta / expected;
    }
    free(counts);
    return chi2;
}
