/*
 * blake2b.c — implementation of BLAKE2b (RFC 7693). Standard algorithm; output
 * matches the Rust `blake2b` crate and the published test vectors.
 */
#include "blake2b.h"

#include <string.h> /* memcpy, memset */

static const uint64_t IV[8] = {
    0x6a09e667f3bcc908u, 0xbb67ae8584caa73bu, 0x3c6ef372fe94f82bu,
    0xa54ff53a5f1d36f1u, 0x510e527fade682d1u, 0x9b05688c2b3e6c1fu,
    0x1f83d9abfb41bd6bu, 0x5be0cd19137e2179u};

static const uint8_t SIGMA[12][16] = {
    {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
    {14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3},
    {11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4},
    {7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8},
    {9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13},
    {2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9},
    {12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11},
    {13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10},
    {6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5},
    {10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0},
    /* Rounds 10 and 11 reuse SIGMA[0] and SIGMA[1] (i % 10). */
    {0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15},
    {14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3}};

static uint64_t rotr(uint64_t x, unsigned n) {
    return (x >> n) | (x << (64 - n));
}

#define G(a, b, c, d, x, y)                                                    \
    do {                                                                       \
        v[a] = v[a] + v[b] + (x);                                              \
        v[d] = rotr(v[d] ^ v[a], 32);                                          \
        v[c] = v[c] + v[d];                                                    \
        v[b] = rotr(v[b] ^ v[c], 24);                                          \
        v[a] = v[a] + v[b] + (y);                                              \
        v[d] = rotr(v[d] ^ v[a], 16);                                          \
        v[c] = v[c] + v[d];                                                    \
        v[b] = rotr(v[b] ^ v[c], 63);                                          \
    } while (0)

static void compress(uint64_t h[8], const uint8_t block[128], uint64_t t_low,
                     uint64_t t_high, int is_final) {
    uint64_t m[16];
    uint64_t v[16];
    unsigned i;

    for (i = 0; i < 16; i++) {
        m[i] = ((uint64_t)block[i * 8]) | ((uint64_t)block[i * 8 + 1] << 8) |
               ((uint64_t)block[i * 8 + 2] << 16) |
               ((uint64_t)block[i * 8 + 3] << 24) |
               ((uint64_t)block[i * 8 + 4] << 32) |
               ((uint64_t)block[i * 8 + 5] << 40) |
               ((uint64_t)block[i * 8 + 6] << 48) |
               ((uint64_t)block[i * 8 + 7] << 56);
    }
    for (i = 0; i < 8; i++) {
        v[i] = h[i];
        v[i + 8] = IV[i];
    }
    v[12] ^= t_low;
    v[13] ^= t_high;
    if (is_final) {
        v[14] ^= 0xffffffffffffffffu;
    }
    for (i = 0; i < 12; i++) {
        const uint8_t *s = SIGMA[i];
        G(0, 4, 8, 12, m[s[0]], m[s[1]]);
        G(1, 5, 9, 13, m[s[2]], m[s[3]]);
        G(2, 6, 10, 14, m[s[4]], m[s[5]]);
        G(3, 7, 11, 15, m[s[6]], m[s[7]]);
        G(0, 5, 10, 15, m[s[8]], m[s[9]]);
        G(1, 6, 11, 12, m[s[10]], m[s[11]]);
        G(2, 7, 8, 13, m[s[12]], m[s[13]]);
        G(3, 4, 9, 14, m[s[14]], m[s[15]]);
    }
    for (i = 0; i < 8; i++) {
        h[i] ^= v[i] ^ v[i + 8];
    }
}

int blake2b_init(blake2b_ctx *ctx, size_t digest_size, const uint8_t *key,
                 size_t key_len, const uint8_t *salt, const uint8_t *personal) {
    uint8_t p[64];
    unsigned i;

    if (digest_size < 1 || digest_size > BLAKE2B_MAX_DIGEST ||
        key_len > BLAKE2B_MAX_KEY) {
        return 0;
    }

    /* Parameter block: digest size, key length, fanout=1, depth=1, then the
     * optional salt (offset 32) and personalization (offset 48). */
    memset(p, 0, sizeof p);
    p[0] = (uint8_t)digest_size;
    p[1] = (uint8_t)key_len;
    p[2] = 1;
    p[3] = 1;
    if (salt != NULL) {
        memcpy(p + 32, salt, 16);
    }
    if (personal != NULL) {
        memcpy(p + 48, personal, 16);
    }
    for (i = 0; i < 8; i++) {
        uint64_t pw = ((uint64_t)p[i * 8]) | ((uint64_t)p[i * 8 + 1] << 8) |
                      ((uint64_t)p[i * 8 + 2] << 16) |
                      ((uint64_t)p[i * 8 + 3] << 24) |
                      ((uint64_t)p[i * 8 + 4] << 32) |
                      ((uint64_t)p[i * 8 + 5] << 40) |
                      ((uint64_t)p[i * 8 + 6] << 48) |
                      ((uint64_t)p[i * 8 + 7] << 56);
        ctx->state[i] = IV[i] ^ pw;
    }
    ctx->count_low = 0;
    ctx->count_high = 0;
    ctx->digest_size = digest_size;

    /* Keyed mode: the first block is the key, zero-padded to 128 bytes. */
    if (key_len > 0) {
        memset(ctx->buffer, 0, sizeof ctx->buffer);
        memcpy(ctx->buffer, key, key_len);
        ctx->buffer_len = BLAKE2B_BLOCK_SIZE;
    } else {
        ctx->buffer_len = 0;
    }
    return 1;
}

/* Add `n` to the 128-bit byte counter. */
static void count_add(blake2b_ctx *ctx, uint64_t n) {
    uint64_t prev = ctx->count_low;
    ctx->count_low += n;
    if (ctx->count_low < prev) {
        ctx->count_high++;
    }
}

void blake2b_update(blake2b_ctx *ctx, const void *data, size_t len) {
    const uint8_t *bytes = (const uint8_t *)data;
    while (len > 0) {
        size_t take;
        if (ctx->buffer_len == BLAKE2B_BLOCK_SIZE) {
            /* Buffer is full and more input follows → compress it (non-final). */
            count_add(ctx, BLAKE2B_BLOCK_SIZE);
            compress(ctx->state, ctx->buffer, ctx->count_low, ctx->count_high, 0);
            ctx->buffer_len = 0;
        }
        take = BLAKE2B_BLOCK_SIZE - ctx->buffer_len;
        if (take > len) {
            take = len;
        }
        memcpy(ctx->buffer + ctx->buffer_len, bytes, take);
        ctx->buffer_len += take;
        bytes += take;
        len -= take;
    }
}

void blake2b_final(blake2b_ctx *ctx, uint8_t *out) {
    uint8_t full[64];
    uint64_t t_low, t_high;
    unsigned i;

    /* Total length = compressed blocks + the pending buffer. */
    t_low = ctx->count_low;
    t_high = ctx->count_high;
    {
        uint64_t prev = t_low;
        t_low += ctx->buffer_len;
        if (t_low < prev) {
            t_high++;
        }
    }
    memset(ctx->buffer + ctx->buffer_len, 0,
           BLAKE2B_BLOCK_SIZE - ctx->buffer_len);
    compress(ctx->state, ctx->buffer, t_low, t_high, 1);

    for (i = 0; i < 8; i++) {
        full[i * 8] = (uint8_t)(ctx->state[i]);
        full[i * 8 + 1] = (uint8_t)(ctx->state[i] >> 8);
        full[i * 8 + 2] = (uint8_t)(ctx->state[i] >> 16);
        full[i * 8 + 3] = (uint8_t)(ctx->state[i] >> 24);
        full[i * 8 + 4] = (uint8_t)(ctx->state[i] >> 32);
        full[i * 8 + 5] = (uint8_t)(ctx->state[i] >> 40);
        full[i * 8 + 6] = (uint8_t)(ctx->state[i] >> 48);
        full[i * 8 + 7] = (uint8_t)(ctx->state[i] >> 56);
    }
    memcpy(out, full, ctx->digest_size);
}

int blake2b(const void *data, size_t len, size_t digest_size, uint8_t *out) {
    blake2b_ctx ctx;
    if (!blake2b_init(&ctx, digest_size, NULL, 0, NULL, NULL)) {
        return 0;
    }
    blake2b_update(&ctx, data, len);
    blake2b_final(&ctx, out);
    return 1;
}

int blake2b_hex(const void *data, size_t len, size_t digest_size, char *out) {
    static const char hex[] = "0123456789abcdef";
    uint8_t digest[BLAKE2B_MAX_DIGEST];
    size_t i;
    if (!blake2b(data, len, digest_size, digest)) {
        return 0;
    }
    for (i = 0; i < digest_size; i++) {
        out[i * 2] = hex[digest[i] >> 4];
        out[i * 2 + 1] = hex[digest[i] & 0x0f];
    }
    out[digest_size * 2] = '\0';
    return 1;
}
