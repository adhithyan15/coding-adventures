/*
 * argon2d.c — implementation of Argon2d (see argon2d.h). A faithful port of the
 * Rust `argon2d` crate: the same G compression, permutation P, H' extender,
 * index_alpha reference mapping, and data-dependent segment fill.
 */
#include "argon2d.h"

#include <stdlib.h> /* calloc, malloc, free */
#include <string.h> /* memcpy */

#include "blake2b.h" /* blake2b (one-shot, digest_size 1..64) */

#define MASK32 0xFFFFFFFFull
#define BLOCK_SIZE 1024u
#define BLOCK_WORDS 128u /* BLOCK_SIZE / 8 */
#define SYNC_POINTS 4u
#define TYPE_D 0u

/* ---- compression: G, permutation P, compress ------------------------- */

static uint64_t rotr64(uint64_t x, unsigned n) {
    return (x >> n) | (x << (64 - n));
}

/* Argon2 G mixer over four words of v (BLAKE2b round + a 2*lo*lo term). */
static void g_b(uint64_t *v, size_t a, size_t b, size_t c, size_t d) {
    uint64_t va = v[a], vb = v[b], vc = v[c], vd = v[d];
    va = va + vb + 2ull * (va & MASK32) * (vb & MASK32);
    vd = rotr64(vd ^ va, 32);
    vc = vc + vd + 2ull * (vc & MASK32) * (vd & MASK32);
    vb = rotr64(vb ^ vc, 24);
    va = va + vb + 2ull * (va & MASK32) * (vb & MASK32);
    vd = rotr64(vd ^ va, 16);
    vc = vc + vd + 2ull * (vc & MASK32) * (vd & MASK32);
    vb = rotr64(vb ^ vc, 63);
    v[a] = va;
    v[b] = vb;
    v[c] = vc;
    v[d] = vd;
}

/* Permutation P over 16 words: 4 column then 4 diagonal G-rounds. */
static void permutation_p(uint64_t *v) {
    g_b(v, 0, 4, 8, 12);
    g_b(v, 1, 5, 9, 13);
    g_b(v, 2, 6, 10, 14);
    g_b(v, 3, 7, 11, 15);
    g_b(v, 0, 5, 10, 15);
    g_b(v, 1, 6, 11, 12);
    g_b(v, 2, 7, 8, 13);
    g_b(v, 3, 4, 9, 14);
}

/* Compression G(x, y): R = X XOR Y; row-P then column-P on R; out = R XOR that.
 * out must be distinct from x and y (all BLOCK_WORDS words). */
static void compress(const uint64_t *x, const uint64_t *y, uint64_t *out) {
    uint64_t r[BLOCK_WORDS];
    uint64_t q[BLOCK_WORDS];
    uint64_t col[16];
    size_t i;
    size_t c;
    size_t row;

    for (i = 0; i < BLOCK_WORDS; i++) {
        r[i] = x[i] ^ y[i];
        q[i] = r[i];
    }
    /* Row rounds: eight independent 16-word rows. */
    for (i = 0; i < 8; i++) {
        permutation_p(q + i * 16);
    }
    /* Column rounds: gather two words per row across a column, permute, scatter. */
    for (c = 0; c < 8; c++) {
        for (row = 0; row < 8; row++) {
            col[2 * row] = q[row * 16 + 2 * c];
            col[2 * row + 1] = q[row * 16 + 2 * c + 1];
        }
        permutation_p(col);
        for (row = 0; row < 8; row++) {
            q[row * 16 + 2 * c] = col[2 * row];
            q[row * 16 + 2 * c + 1] = col[2 * row + 1];
        }
    }
    for (i = 0; i < BLOCK_WORDS; i++) {
        out[i] = r[i] ^ q[i];
    }
}

/* ---- byte/word helpers ------------------------------------------------ */

static void store_le32(uint8_t *p, uint32_t n) {
    p[0] = (uint8_t)(n & 0xFF);
    p[1] = (uint8_t)((n >> 8) & 0xFF);
    p[2] = (uint8_t)((n >> 16) & 0xFF);
    p[3] = (uint8_t)((n >> 24) & 0xFF);
}

static uint64_t load_le64(const uint8_t *p) {
    return (uint64_t)p[0] | ((uint64_t)p[1] << 8) | ((uint64_t)p[2] << 16) |
           ((uint64_t)p[3] << 24) | ((uint64_t)p[4] << 32) |
           ((uint64_t)p[5] << 40) | ((uint64_t)p[6] << 48) |
           ((uint64_t)p[7] << 56);
}

static void store_le64(uint8_t *p, uint64_t w) {
    int i;
    for (i = 0; i < 8; i++) {
        p[i] = (uint8_t)((w >> (8 * i)) & 0xFF);
    }
}

static void bytes_to_block(const uint8_t *data, uint64_t *block) {
    size_t i;
    for (i = 0; i < BLOCK_WORDS; i++) {
        block[i] = load_le64(data + i * 8);
    }
}

static void block_to_bytes(const uint64_t *block, uint8_t *out) {
    size_t i;
    for (i = 0; i < BLOCK_WORDS; i++) {
        store_le64(out + i * 8, block[i]);
    }
}

/* ---- H' variable-length BLAKE2b extender (RFC 9106 §3.3) -------------- */

/* Write `t` bytes of H'(x) into `out`. `x` is `x_len` bytes. Returns 1, or 0 on
 * an internal BLAKE2b failure (implausible for valid sizes). */
static int blake2b_long(uint32_t t, const uint8_t *x, size_t x_len,
                        uint8_t *out) {
    uint8_t *input;
    size_t in_len;
    uint8_t v[64];
    uint32_t r;
    uint32_t final_size;
    uint32_t k;
    size_t off;

    if (t == 0) {
        return 0;
    }
    if (x_len > (size_t)-1 - 4u) {
        return 0;
    }
    in_len = 4u + x_len;
    input = malloc(in_len);
    if (!input) {
        return 0;
    }
    store_le32(input, t);
    if (x_len > 0) {
        memcpy(input + 4, x, x_len);
    }

    if (t <= 64) {
        int ok = blake2b(input, in_len, t, out);
        free(input);
        return ok;
    }

    /* r = ceil(t/32) - 2, then 32 bytes per block plus a final chunk. */
    r = (t + 31u) / 32u - 2u;
    if (!blake2b(input, in_len, 64, v)) {
        free(input);
        return 0;
    }
    free(input);
    memcpy(out, v, 32);
    off = 32;
    for (k = 1; k < r; k++) {
        if (!blake2b(v, 64, 64, v)) {
            return 0;
        }
        memcpy(out + off, v, 32);
        off += 32;
    }
    final_size = t - 32u * r;
    if (!blake2b(v, 64, final_size, out + off)) {
        return 0;
    }
    return 1;
}

/* ---- reference index mapping (RFC 9106 §3.4) -------------------------- */

static size_t index_alpha(uint64_t j1, size_t r, size_t sl, size_t c,
                          int same_lane, size_t q, size_t sl_len) {
    size_t w;
    size_t start;
    uint64_t x;
    uint64_t y;
    int64_t rel;
    int64_t res;

    if (r == 0 && sl == 0) {
        w = c - 1;
        start = 0;
    } else if (r == 0) {
        if (same_lane) {
            w = sl * sl_len + c - 1;
        } else if (c == 0) {
            w = sl * sl_len - 1;
        } else {
            w = sl * sl_len;
        }
        start = 0;
    } else {
        if (same_lane) {
            w = q - sl_len + c - 1;
        } else if (c == 0) {
            w = q - sl_len - 1;
        } else {
            w = q - sl_len;
        }
        start = ((sl + 1) * sl_len) % q;
    }

    x = (j1 * j1) >> 32;                  /* uint64 wraps (defined) */
    y = ((uint64_t)w * x) >> 32;
    rel = (int64_t)w - 1 - (int64_t)y;
    res = (int64_t)start + rel;
    /* rem_euclid(res, q). */
    res = res % (int64_t)q;
    if (res < 0) {
        res += (int64_t)q;
    }
    return (size_t)res;
}

/* ---- segment fill (data-dependent) ------------------------------------ */

/* memory is a flat [p*q][BLOCK_WORDS] matrix; block(lane, col) points into it. */
static uint64_t *block_at(uint64_t *memory, size_t lane, size_t col, size_t q) {
    return memory + (lane * q + col) * BLOCK_WORDS;
}

static void fill_segment(uint64_t *memory, size_t r, size_t lane, size_t sl,
                         size_t q, size_t sl_len, size_t p) {
    size_t starting_c = (r == 0 && sl == 0) ? 2 : 0;
    size_t i;
    uint64_t newb[BLOCK_WORDS];

    for (i = starting_c; i < sl_len; i++) {
        size_t col = sl * sl_len + i;
        size_t prev_col = (col == 0) ? (q - 1) : (col - 1);
        const uint64_t *prev_block = block_at(memory, lane, prev_col, q);
        uint64_t pseudo_rand = prev_block[0];
        uint64_t j1 = pseudo_rand & MASK32;
        uint64_t j2 = (pseudo_rand >> 32) & MASK32;
        size_t l_prime = (r == 0 && sl == 0) ? lane : (size_t)(j2 % (uint64_t)p);
        size_t z_prime =
            index_alpha(j1, r, sl, i, l_prime == lane, q, sl_len);
        const uint64_t *ref_block = block_at(memory, l_prime, z_prime, q);
        uint64_t *dst = block_at(memory, lane, col, q);
        size_t k;

        compress(prev_block, ref_block, newb);
        if (r == 0) {
            memcpy(dst, newb, BLOCK_SIZE);
        } else {
            for (k = 0; k < BLOCK_WORDS; k++) {
                dst[k] ^= newb[k];
            }
        }
    }
}

/* ---- validation ------------------------------------------------------- */

static Argon2dStatus validate(size_t password_len, size_t salt_len,
                              uint32_t time_cost, uint32_t memory_cost,
                              uint32_t parallelism, uint32_t tag_length,
                              size_t key_len, size_t ad_len, uint32_t version) {
    if ((uint64_t)password_len > 0xFFFFFFFFull) {
        return ARGON2D_PASSWORD_TOO_LONG;
    }
    if (salt_len < 8) {
        return ARGON2D_SALT_TOO_SHORT;
    }
    if ((uint64_t)salt_len > 0xFFFFFFFFull) {
        return ARGON2D_SALT_TOO_LONG;
    }
    if ((uint64_t)key_len > 0xFFFFFFFFull) {
        return ARGON2D_KEY_TOO_LONG;
    }
    if ((uint64_t)ad_len > 0xFFFFFFFFull) {
        return ARGON2D_AD_TOO_LONG;
    }
    if (tag_length < 4) {
        return ARGON2D_TAG_TOO_SMALL;
    }
    if (parallelism < 1 || parallelism > 0xFFFFFF) {
        return ARGON2D_INVALID_PARALLELISM;
    }
    if (memory_cost < 8 * parallelism) {
        return ARGON2D_MEMORY_TOO_SMALL;
    }
    if (time_cost < 1) {
        return ARGON2D_TIME_COST_ZERO;
    }
    if (version != ARGON2D_VERSION) {
        return ARGON2D_UNSUPPORTED_VERSION;
    }
    return ARGON2D_OK;
}

/* ---- public entry point ----------------------------------------------- */

Argon2dStatus argon2d(const uint8_t *password, size_t password_len,
                      const uint8_t *salt, size_t salt_len, uint32_t time_cost,
                      uint32_t memory_cost, uint32_t parallelism,
                      uint32_t tag_length, const Argon2dOptions *opts,
                      uint8_t *out) {
    const uint8_t *key = NULL;
    size_t key_len = 0;
    const uint8_t *ad = NULL;
    size_t ad_len = 0;
    uint32_t version = ARGON2D_VERSION;
    Argon2dStatus rc;
    size_t p;
    size_t t;
    size_t segment_length;
    size_t m_prime;
    size_t q;
    size_t sl_len;
    uint8_t *h0_in;
    size_t h0_in_len;
    size_t off;
    uint8_t h0[64];
    uint64_t *memory;
    uint8_t *blockbuf;
    uint64_t final_block[BLOCK_WORDS];
    size_t i;
    size_t r;
    size_t sl;
    size_t lane;
    size_t k;

    if (!out) {
        return ARGON2D_BAD_ARGS;
    }
    if (opts) {
        key = opts->key;
        key_len = opts->key_len;
        ad = opts->associated_data;
        ad_len = opts->ad_len;
        if (opts->version != 0) {
            version = opts->version;
        }
    }

    rc = validate(password_len, salt_len, time_cost, memory_cost, parallelism,
                  tag_length, key_len, ad_len, version);
    if (rc != ARGON2D_OK) {
        return rc;
    }

    p = parallelism;
    t = time_cost;
    segment_length = memory_cost / (SYNC_POINTS * parallelism);
    m_prime = segment_length * SYNC_POINTS * p;
    q = m_prime / p;
    sl_len = segment_length;

    /* H0 = BLAKE2b(params || pass || salt || key || ad). */
    h0_in_len = 4u * 6u + 4u + password_len + 4u + salt_len + 4u + key_len +
                4u + ad_len;
    h0_in = malloc(h0_in_len ? h0_in_len : 1);
    if (!h0_in) {
        return ARGON2D_ALLOC_ERROR;
    }
    off = 0;
    store_le32(h0_in + off, (uint32_t)p);
    off += 4;
    store_le32(h0_in + off, tag_length);
    off += 4;
    store_le32(h0_in + off, memory_cost);
    off += 4;
    store_le32(h0_in + off, (uint32_t)t);
    off += 4;
    store_le32(h0_in + off, version);
    off += 4;
    store_le32(h0_in + off, TYPE_D);
    off += 4;
    store_le32(h0_in + off, (uint32_t)password_len);
    off += 4;
    if (password_len > 0) {
        memcpy(h0_in + off, password, password_len);
    }
    off += password_len;
    store_le32(h0_in + off, (uint32_t)salt_len);
    off += 4;
    memcpy(h0_in + off, salt, salt_len);
    off += salt_len;
    store_le32(h0_in + off, (uint32_t)key_len);
    off += 4;
    if (key_len > 0) {
        memcpy(h0_in + off, key, key_len);
    }
    off += key_len;
    store_le32(h0_in + off, (uint32_t)ad_len);
    off += 4;
    if (ad_len > 0) {
        memcpy(h0_in + off, ad, ad_len);
    }
    off += ad_len;

    if (!blake2b(h0_in, h0_in_len, 64, h0)) {
        free(h0_in);
        return ARGON2D_ALLOC_ERROR;
    }
    free(h0_in);

    /* Allocate the working matrix p*q blocks of BLOCK_WORDS u64 (checked). */
    if (m_prime == 0 || m_prime > ((size_t)-1) / BLOCK_WORDS) {
        return ARGON2D_ALLOC_ERROR;
    }
    memory = calloc(m_prime * BLOCK_WORDS, sizeof(uint64_t));
    blockbuf = malloc(BLOCK_SIZE);
    if (!memory || !blockbuf) {
        free(memory);
        free(blockbuf);
        return ARGON2D_ALLOC_ERROR;
    }

    /* First two columns of each lane: H'(H0 || {0,1} || lane). */
    for (i = 0; i < p; i++) {
        uint8_t in0[64 + 8];
        int col;
        for (col = 0; col < 2; col++) {
            memcpy(in0, h0, 64);
            store_le32(in0 + 64, (uint32_t)col);
            store_le32(in0 + 68, (uint32_t)i);
            if (!blake2b_long(BLOCK_SIZE, in0, 72, blockbuf)) {
                free(memory);
                free(blockbuf);
                return ARGON2D_ALLOC_ERROR;
            }
            bytes_to_block(blockbuf, block_at(memory, i, (size_t)col, q));
        }
    }

    /* t passes over 4 segments over p lanes. */
    for (r = 0; r < t; r++) {
        for (sl = 0; sl < SYNC_POINTS; sl++) {
            for (lane = 0; lane < p; lane++) {
                fill_segment(memory, r, lane, sl, q, sl_len, p);
            }
        }
    }

    /* Final block = XOR of the last column across lanes. */
    memcpy(final_block, block_at(memory, 0, q - 1, q), BLOCK_SIZE);
    for (lane = 1; lane < p; lane++) {
        const uint64_t *lb = block_at(memory, lane, q - 1, q);
        for (k = 0; k < BLOCK_WORDS; k++) {
            final_block[k] ^= lb[k];
        }
    }

    block_to_bytes(final_block, blockbuf);
    rc = blake2b_long(tag_length, blockbuf, BLOCK_SIZE, out)
             ? ARGON2D_OK
             : ARGON2D_ALLOC_ERROR;
    free(memory);
    free(blockbuf);
    return rc;
}
