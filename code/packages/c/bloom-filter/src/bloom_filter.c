/*
 * bloom_filter.c — implementation of the Bloom filter (see bloom_filter.h).
 * A faithful port of the Rust `bloom-filter` crate: same double-hashing scheme
 * (FNV-1a + djb2, finalised with fmix32) and the same optimal-m/k sizing.
 *
 * Note on hashing: the Rust crate hashes an element's `Debug` string; this port
 * hashes the raw element bytes you pass to add()/contains(). The filter is
 * self-consistent either way (add and contains use the identical hashing), so
 * the no-false-negatives guarantee holds — the two ports simply set different
 * bit patterns, which is fine since a Bloom filter has no cross-language wire
 * format to agree on.
 */
#include "bloom_filter.h"

#include <float.h>  /* DBL_MAX (to reject non-finite inputs without libm) */
#include <stdint.h> /* SIZE_MAX, uint32_t, uint64_t */
#include <stdlib.h> /* calloc, free */
#include <string.h> /* memset */

/* ── libm-free floating point helpers ─────────────────────────────────────── */
/* The strict multi-compiler harness does not link libm, so we carry our own
 * natural log and rounding for the sizing formulas. */

/* 2^53: the largest magnitude at which every integer is exactly representable
 * as a double. Beyond it a double→integer cast can misbehave, so we saturate. */
#define ISO_DBL_INT_LIMIT 9007199254740992.0

static double iso_ln2(void) { return 0.6931471805599453; }

/* Natural log for x > 0, via range reduction x = m·2^e (m in [1,2)) and the
 * fast-converging series ln(m) = 2·atanh(t), t = (m-1)/(m+1) in [0, 1/3]. */
static double iso_ln(double x) {
    int e = 0;
    double t, t2, term, sum;
    int k;
    /* Reject non-finite / out-of-domain input: x <= 0 and NaN both fail the
     * (x > 0.0) test; +inf is caught by (x > DBL_MAX). This keeps the halving
     * loop below finite (inf * 0.5 == inf would spin forever). */
    if (!(x > 0.0) || x > DBL_MAX) {
        return 0.0;
    }
    while (x >= 2.0) {
        x *= 0.5;
        e++;
    }
    while (x < 1.0) {
        x *= 2.0;
        e--;
    }
    t = (x - 1.0) / (x + 1.0);
    t2 = t * t;
    term = t;
    sum = 0.0;
    for (k = 1; k <= 25; k += 2) {
        sum += term / (double)k;
        term *= t2;
    }
    return 2.0 * sum + (double)e * iso_ln2();
}

/* Convert a non-negative double to size_t with the given rounding, saturating
 * safely instead of invoking undefined behaviour on out-of-range casts. */
static size_t d_to_size(double x, double bias) {
    if (x != x) {
        return 0; /* NaN → not a valid size (casting NaN to an int is UB) */
    }
    if (x <= 0.0) {
        return 0;
    }
    if (x >= ISO_DBL_INT_LIMIT) {
        return SIZE_MAX;
    }
    return (size_t)(unsigned long long)(x + bias);
}
static size_t d_ceil_size(double x) {
    /* ceil: truncate, then add one if we lost a fraction. */
    size_t t;
    if (x != x) {
        return 0; /* NaN guard (see d_to_size) */
    }
    if (x <= 0.0) {
        return 0;
    }
    if (x >= ISO_DBL_INT_LIMIT) {
        return SIZE_MAX;
    }
    t = (size_t)(unsigned long long)x;
    if ((double)t < x) {
        t += 1;
    }
    return t;
}

/* ── hashing (matches coding_adventures_hash_functions) ───────────────────── */
static uint32_t fnv1a_32(const uint8_t *d, size_t n) {
    uint32_t h = 0x811c9dc5u;
    size_t i;
    for (i = 0; i < n; i++) {
        h ^= d[i];
        h *= 0x01000193u;
    }
    return h;
}
static uint64_t djb2(const uint8_t *d, size_t n) {
    uint64_t h = 5381u;
    size_t i;
    for (i = 0; i < n; i++) {
        h = (h << 5) + h + d[i]; /* h*33 + byte, wrapping in 64 bits */
    }
    return h;
}
static uint32_t fmix32(uint32_t h) {
    h ^= h >> 16;
    h *= 0x85ebca6bu;
    h ^= h >> 13;
    h *= 0xc2b2ae35u;
    h ^= h >> 16;
    return h;
}

/* Derive the two base hashes for double hashing. */
static void hash_bases(const void *data, size_t len, uint32_t *h1,
                       uint32_t *h2) {
    const uint8_t *bytes = (const uint8_t *)data;
    uint64_t h2raw = djb2(bytes, len);
    uint32_t folded = (uint32_t)((h2raw ^ (h2raw >> 32)) & 0xffffffffu);
    *h1 = fmix32(fnv1a_32(bytes, len));
    *h2 = fmix32(folded) | 1u; /* force odd so it is coprime with 2^k moduli */
}

/* ── sizing helpers ───────────────────────────────────────────────────────── */
size_t bloom_optimal_m(size_t n, double p) {
    /* m = ceil(-n ln p / (ln 2)^2) */
    double ln2 = iso_ln2();
    double m = (-(double)n * iso_ln(p)) / (ln2 * ln2);
    return d_ceil_size(m);
}
size_t bloom_optimal_k(size_t m, size_t n) {
    /* k = max(1, round((m/n) ln 2)) */
    size_t k;
    if (n == 0) {
        return 1;
    }
    k = d_to_size(((double)m / (double)n) * iso_ln2(), 0.5);
    return k < 1 ? 1 : k;
}
size_t bloom_capacity_for_memory(size_t memory_bytes, double p) {
    /* n = floor(-(8·bytes) (ln 2)^2 / ln p) */
    double ln2 = iso_ln2();
    double m = (double)memory_bytes * 8.0;
    double n = (-m * (ln2 * ln2)) / iso_ln(p);
    return d_to_size(n, 0.0);
}

/* ── construction ─────────────────────────────────────────────────────────── */
static bloom_status from_parts(bloom_filter *bf, size_t bit_count,
                               size_t hash_count, size_t expected_items) {
    size_t byte_count;
    if (bit_count > SIZE_MAX - 7) {
        return BLOOM_ALLOC_FAILED; /* (bit_count + 7) would overflow */
    }
    byte_count = (bit_count + 7) / 8;
    if (byte_count == 0) {
        byte_count = 1; /* bit_count is >= 1 here, but be defensive */
    }
    bf->bits = (uint8_t *)calloc(byte_count, 1);
    if (bf->bits == NULL) {
        return BLOOM_ALLOC_FAILED;
    }
    bf->bit_count = bit_count;
    bf->hash_count = hash_count;
    bf->expected_items = expected_items;
    bf->byte_count = byte_count;
    bf->bits_set = 0;
    bf->items_added = 0;
    return BLOOM_OK;
}

bloom_status bloom_init(bloom_filter *bf, size_t expected_items,
                        double false_positive_rate) {
    size_t m, k;
    if (expected_items == 0) {
        return BLOOM_INVALID_EXPECTED_ITEMS;
    }
    if (!(false_positive_rate > 0.0 && false_positive_rate < 1.0)) {
        return BLOOM_INVALID_FALSE_POSITIVE_RATE;
    }
    m = bloom_optimal_m(expected_items, false_positive_rate);
    k = bloom_optimal_k(m, expected_items);
    return from_parts(bf, m, k, expected_items);
}

bloom_status bloom_init_params(bloom_filter *bf, size_t bit_count,
                               size_t hash_count) {
    if (bit_count == 0) {
        return BLOOM_INVALID_BIT_COUNT;
    }
    if (hash_count == 0) {
        return BLOOM_INVALID_HASH_COUNT;
    }
    return from_parts(bf, bit_count, hash_count, 0);
}

void bloom_free(bloom_filter *bf) {
    if (bf == NULL) {
        return;
    }
    free(bf->bits);
    memset(bf, 0, sizeof *bf);
}

/* ── membership ───────────────────────────────────────────────────────────── */
void bloom_add(bloom_filter *bf, const void *data, size_t len) {
    uint32_t h1, h2;
    size_t i;
    hash_bases(data, len, &h1, &h2);
    for (i = 0; i < bf->hash_count; i++) {
        uint64_t idx = ((uint64_t)h1 + (uint64_t)i * (uint64_t)h2) %
                       (uint64_t)bf->bit_count;
        size_t byte_idx = (size_t)(idx / 8);
        uint8_t mask = (uint8_t)(1u << (unsigned)(idx % 8));
        if ((bf->bits[byte_idx] & mask) == 0) {
            bf->bits[byte_idx] |= mask;
            bf->bits_set++;
        }
    }
    bf->items_added++;
}

int bloom_contains(const bloom_filter *bf, const void *data, size_t len) {
    uint32_t h1, h2;
    size_t i;
    hash_bases(data, len, &h1, &h2);
    for (i = 0; i < bf->hash_count; i++) {
        uint64_t idx = ((uint64_t)h1 + (uint64_t)i * (uint64_t)h2) %
                       (uint64_t)bf->bit_count;
        size_t byte_idx = (size_t)(idx / 8);
        uint8_t mask = (uint8_t)(1u << (unsigned)(idx % 8));
        if ((bf->bits[byte_idx] & mask) == 0) {
            return 0; /* a clear bit → definitely not present */
        }
    }
    return 1; /* all bits set → possibly present */
}

/* ── accessors ────────────────────────────────────────────────────────────── */
size_t bloom_bit_count(const bloom_filter *bf) { return bf->bit_count; }
size_t bloom_hash_count(const bloom_filter *bf) { return bf->hash_count; }
size_t bloom_bits_set(const bloom_filter *bf) { return bf->bits_set; }
size_t bloom_size_bytes(const bloom_filter *bf) { return bf->byte_count; }

double bloom_fill_ratio(const bloom_filter *bf) {
    if (bf->bit_count == 0) {
        return 0.0;
    }
    return (double)bf->bits_set / (double)bf->bit_count;
}

double bloom_estimated_false_positive_rate(const bloom_filter *bf) {
    double ratio, p;
    size_t i;
    if (bf->bits_set == 0) {
        return 0.0;
    }
    ratio = bloom_fill_ratio(bf);
    p = 1.0;
    for (i = 0; i < bf->hash_count; i++) {
        p *= ratio; /* ratio^k without libm's pow */
    }
    return p;
}

int bloom_is_over_capacity(const bloom_filter *bf) {
    if (bf->expected_items == 0) {
        return 0;
    }
    return bf->items_added > bf->expected_items ? 1 : 0;
}
