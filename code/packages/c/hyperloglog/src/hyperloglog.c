/*
 * hyperloglog.c — approximate cardinality estimation (implementation).
 * ===========================================================================
 *
 * A faithful C port of the Rust `hyperloglog` crate. The estimation pipeline is
 * the textbook HyperLogLog with the same corrections the Rust (and its Python
 * reference) use:
 *
 *   add:   h = fmix64(fnv1a_64(bytes));   // hash + strong avalanche
 *          bucket = top `precision` bits of h
 *          rho    = 1 + (leading zeros in the remaining bits)
 *          registers[bucket] = max(registers[bucket], rho)
 *
 *   count: raw = alpha * m^2 / Σ 2^(-register)      // harmonic-mean core
 *          if raw is small, use linear counting m*ln(m/zeros)
 *          if raw is large (near 2^32), apply the large-range correction
 *
 * The hash comes from `c/hash-functions` (hf_fnv1a_64) and every transcendental
 * (ln / sqrt / log2 / ceil / round) from `c/float-math` (fm_*), so nothing here
 * links a math library — the lane's no-libm rule holds.
 */
#include "hyperloglog/hyperloglog.h"

#include "float_math.h"     /* fm_log, fm_sqrt, fm_log2, fm_ceil, fm_round */
#include "hash_functions.h" /* hf_fnv1a_64 */

#include <stdlib.h> /* malloc, calloc, free */
#include <string.h> /* strlen */

struct hll {
    uint8_t precision;
    uint8_t *registers;
    size_t nregisters;
};

/* ------------------------------------------------------------------------- *
 * Bit / hash helpers
 * ------------------------------------------------------------------------- */

/*
 * fmix64 — the MurmurHash3 finalizer. Spreads the FNV-1a output so that even the
 * high bits (which pick the bucket) and low bits (which drive rho) are well
 * mixed. The multiplies wrap mod 2^64, which is exactly C's unsigned overflow.
 */
static uint64_t fmix64(uint64_t k) {
    k ^= k >> 33;
    k *= (uint64_t)0xFF51AFD7ED558CCDULL;
    k ^= k >> 33;
    k *= (uint64_t)0xC4CEB9FE1A85EC53ULL;
    k ^= k >> 33;
    return k;
}

/* Count leading zero bits of a NON-ZERO 64-bit value. Hand-rolled rather than a
 * compiler builtin so it stays pure ISO (portable to MSVC too). */
static uint32_t clz64_nonzero(uint64_t v) {
    uint32_t n = 0;
    while ((v & (uint64_t)0x8000000000000000ULL) == 0) {
        n++;
        v <<= 1;
    }
    return n;
}

/*
 * count_leading_zeros — leading zeros of `value` when it is read as a `bit_width`-
 * bit number (Rust's helper). Zero has `bit_width` leading zeros; otherwise take
 * the 64-bit leading-zero count and drop the (64 - bit_width) high padding bits.
 */
static uint32_t count_leading_zeros(uint64_t value, uint8_t bit_width) {
    uint32_t leading;
    if (value == 0) {
        return (uint32_t)bit_width;
    }
    leading = clz64_nonzero(value);
    /* saturating_sub: leading is at most 64 - bit_width for a bit_width-bit value,
     * but guard anyway so the subtraction can never wrap. */
    if (leading < (uint32_t)(64 - bit_width)) {
        return 0;
    }
    return leading - (uint32_t)(64 - bit_width);
}

/* The bias-correction constant alpha for a register count (Rust's table). */
static double alpha_for_registers(size_t registers) {
    switch (registers) {
    case 16:
        return 0.673;
    case 32:
        return 0.697;
    case 64:
        return 0.709;
    default:
        return 0.7213 / (1.0 + 1.079 / (double)registers);
    }
}

/* ------------------------------------------------------------------------- *
 * Lifecycle
 * ------------------------------------------------------------------------- */

hll_status hll_create(uint8_t precision, hll **out) {
    hll *h;
    size_t register_count;
    if (!out) {
        return HLL_ERR_INVALID;
    }
    if (precision < HLL_MIN_PRECISION || precision > HLL_MAX_PRECISION) {
        return HLL_ERR_INVALID_PRECISION;
    }
    h = (hll *)malloc(sizeof(*h));
    if (!h) {
        return HLL_ERR_NOMEM;
    }
    register_count = (size_t)1 << precision; /* precision <= 16 → no overflow */
    h->registers = (uint8_t *)calloc(register_count, sizeof(uint8_t));
    if (!h->registers) {
        free(h);
        return HLL_ERR_NOMEM;
    }
    h->precision = precision;
    h->nregisters = register_count;
    *out = h;
    return HLL_OK;
}

hll_status hll_create_default(hll **out) {
    return hll_create(HLL_DEFAULT_PRECISION, out);
}

void hll_destroy(hll *h) {
    if (!h) {
        return;
    }
    free(h->registers);
    free(h);
}

/* ------------------------------------------------------------------------- *
 * Observation
 * ------------------------------------------------------------------------- */

hll_status hll_add_bytes(hll *h, const unsigned char *bytes, size_t len) {
    uint64_t hash;
    uint32_t precision;
    uint32_t remaining_bits;
    uint64_t remaining;
    size_t bucket;
    uint32_t rho;
    if (!h || (!bytes && len > 0)) {
        return HLL_ERR_INVALID;
    }
    hash = fmix64(hf_fnv1a_64(bytes, len));

    precision = (uint32_t)h->precision;
    bucket = (size_t)(hash >> (64 - precision));
    remaining_bits = 64 - precision;
    /* precision >= 4 keeps remaining_bits <= 60, so the shift below is defined;
     * the == 64 branch mirrors the Rust guard for completeness. */
    if (remaining_bits == 64) {
        remaining = hash;
    } else {
        remaining = hash & (((uint64_t)1 << remaining_bits) - 1);
    }
    rho = count_leading_zeros(remaining, (uint8_t)remaining_bits) + 1;

    if (rho > (uint32_t)h->registers[bucket]) {
        h->registers[bucket] = (uint8_t)rho;
    }
    return HLL_OK;
}

hll_status hll_add_str(hll *h, const char *s) {
    if (!h || !s) {
        return HLL_ERR_INVALID;
    }
    return hll_add_bytes(h, (const unsigned char *)s, strlen(s));
}

/* ------------------------------------------------------------------------- *
 * Estimation
 * ------------------------------------------------------------------------- */

size_t hll_count(const hll *h) {
    double m;
    double z_sum = 0.0;
    double alpha;
    double estimate;
    double two_32;
    size_t i;
    if (!h) {
        return 0;
    }
    m = (double)h->nregisters;

    /* Σ 2^(-register). Each register is a small non-negative integer, so 2^(-r)
     * is 1 / 2^r computed exactly by an integer shift (no pow needed). */
    for (i = 0; i < h->nregisters; i++) {
        uint8_t r = h->registers[i];
        if (r == 0) {
            z_sum += 1.0;
        } else if (r < 64) {
            z_sum += 1.0 / (double)((uint64_t)1 << r);
        }
        /* r >= 64 is unreachable (rho <= 61), and would add a negligible term. */
    }

    alpha = alpha_for_registers(h->nregisters);
    estimate = alpha * m * m / z_sum;

    /* Small-cardinality regime: linear counting over the empty registers. */
    if (estimate <= 2.5 * m) {
        size_t zeros = 0;
        for (i = 0; i < h->nregisters; i++) {
            if (h->registers[i] == 0) {
                zeros++;
            }
        }
        if (zeros > 0) {
            estimate = m * fm_log(m / (double)zeros);
        }
    }

    /* Large-cardinality regime near 2^32: undo the hash-space saturation. */
    two_32 = (double)((uint64_t)1 << 32);
    if (estimate > two_32 / 30.0) {
        double ratio = 1.0 - estimate / two_32;
        if (ratio > 0.0) {
            estimate = -two_32 * fm_log(ratio);
        }
    }

    estimate = fm_round(estimate);
    if (estimate < 0.0) {
        estimate = 0.0;
    }
    /* Saturate rather than let a double at/above the size_t range hit an
     * out-of-range cast (UB in C). Written as `!(estimate < LIMIT)` so it also
     * rejects the boundary value and any NaN: on a 64-bit target `(double)SIZE_MAX`
     * rounds UP to 2^64, so a plain `>` would let estimate == 2^64 slip through to
     * the UB cast. Matters mainly where size_t is 32-bit (the large-range estimate
     * can approach 2^32). Rust's `as usize` saturates identically. */
    if (!(estimate < (double)((size_t)-1))) {
        return (size_t)-1;
    }
    return (size_t)estimate;
}

int hll_is_empty(const hll *h) {
    return hll_count(h) == 0 ? 1 : 0;
}

/* ------------------------------------------------------------------------- *
 * Merge
 * ------------------------------------------------------------------------- */

hll_status hll_merge(const hll *a, const hll *b, hll **out) {
    hll *merged;
    hll_status st;
    size_t i;
    if (!a || !b || !out) {
        return HLL_ERR_INVALID;
    }
    if (a->precision != b->precision) {
        return HLL_ERR_PRECISION_MISMATCH;
    }
    st = hll_create(a->precision, &merged);
    if (st != HLL_OK) {
        return st;
    }
    /* Register-wise maximum: the union of the two register states. */
    for (i = 0; i < merged->nregisters; i++) {
        uint8_t va = a->registers[i];
        uint8_t vb = b->registers[i];
        merged->registers[i] = (va > vb) ? va : vb;
    }
    *out = merged;
    return HLL_OK;
}

/* ------------------------------------------------------------------------- *
 * Accessors & static helpers
 * ------------------------------------------------------------------------- */

uint8_t hll_precision(const hll *h) {
    return h ? h->precision : (uint8_t)0;
}

size_t hll_num_registers(const hll *h) {
    return h ? h->nregisters : 0;
}

/*
 * Clamp a caller-supplied precision into the valid [MIN, MAX] range. The public
 * free functions below take a raw precision (mirroring the Rust free functions),
 * but an out-of-range value would make `1 << precision` a shift past the width of
 * size_t — undefined behaviour — so we clamp defensively rather than trust it.
 */
static uint8_t hll__clamp_precision(uint8_t precision) {
    if (precision < HLL_MIN_PRECISION) {
        return (uint8_t)HLL_MIN_PRECISION;
    }
    if (precision > HLL_MAX_PRECISION) {
        return (uint8_t)HLL_MAX_PRECISION;
    }
    return precision;
}

double hll_error_rate_for_precision(uint8_t precision) {
    double m = (double)((size_t)1 << hll__clamp_precision(precision));
    return 1.04 / fm_sqrt(m);
}

double hll_error_rate(const hll *h) {
    if (!h) {
        return 0.0;
    }
    return hll_error_rate_for_precision(h->precision);
}

size_t hll_memory_bytes(uint8_t precision) {
    size_t m = (size_t)1 << hll__clamp_precision(precision);
    return (m * 6) / 8;
}

uint8_t hll_optimal_precision(double desired_error) {
    double min_m;
    double p;
    /* Non-positive / NaN targets can't be met; the Rust cast-and-clamp lands on
     * the max precision, so short-circuit there and never feed a bad value to
     * the double->uint8_t cast below. */
    if (!(desired_error > 0.0)) {
        return (uint8_t)HLL_MAX_PRECISION;
    }
    min_m = 1.04 / desired_error;
    min_m = min_m * min_m; /* (1.04 / desired_error)^2 */
    p = fm_ceil(fm_log2(min_m));
    /* Clamp in double space BEFORE the cast (this also tames +inf from a tiny
     * target). */
    if (!(p >= (double)HLL_MIN_PRECISION)) { /* covers NaN / below-min */
        p = (double)HLL_MIN_PRECISION;
    } else if (p > (double)HLL_MAX_PRECISION) {
        p = (double)HLL_MAX_PRECISION;
    }
    return (uint8_t)p;
}
