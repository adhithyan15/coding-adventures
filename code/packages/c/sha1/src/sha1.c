/*
 * sha1.c — implementation of SHA-1 (FIPS 180-4). Standard algorithm; output
 * matches the Rust `sha1` crate and the published test vectors.
 */
#include "sha1.h"

/* Rotate a 32-bit word left by n (0 < n < 32). */
static uint32_t rotl(uint32_t x, unsigned n) {
    return ((x << n) | (x >> (32 - n))) & 0xffffffffu;
}

/* Compress one 64-byte block into the five-word state. */
static void sha1_transform(uint32_t state[5], const uint8_t block[64]) {
    uint32_t w[80];
    uint32_t a, b, c, d, e;
    unsigned i;

    for (i = 0; i < 16; i++) {
        w[i] = ((uint32_t)block[i * 4] << 24) |
               ((uint32_t)block[i * 4 + 1] << 16) |
               ((uint32_t)block[i * 4 + 2] << 8) |
               ((uint32_t)block[i * 4 + 3]);
    }
    for (i = 16; i < 80; i++) {
        w[i] = rotl(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
    }

    a = state[0]; b = state[1]; c = state[2]; d = state[3]; e = state[4];

    for (i = 0; i < 80; i++) {
        uint32_t f, k, tmp;
        if (i < 20) {
            f = (b & c) | (~b & d);
            k = 0x5a827999u;
        } else if (i < 40) {
            f = b ^ c ^ d;
            k = 0x6ed9eba1u;
        } else if (i < 60) {
            f = (b & c) | (b & d) | (c & d);
            k = 0x8f1bbcdcu;
        } else {
            f = b ^ c ^ d;
            k = 0xca62c1d6u;
        }
        tmp = (rotl(a, 5) + f + e + k + w[i]) & 0xffffffffu;
        e = d;
        d = c;
        c = rotl(b, 30);
        b = a;
        a = tmp;
    }

    state[0] = (state[0] + a) & 0xffffffffu;
    state[1] = (state[1] + b) & 0xffffffffu;
    state[2] = (state[2] + c) & 0xffffffffu;
    state[3] = (state[3] + d) & 0xffffffffu;
    state[4] = (state[4] + e) & 0xffffffffu;
}

void sha1_init(sha1_ctx *ctx) {
    ctx->state[0] = 0x67452301u;
    ctx->state[1] = 0xefcdab89u;
    ctx->state[2] = 0x98badcfeu;
    ctx->state[3] = 0x10325476u;
    ctx->state[4] = 0xc3d2e1f0u;
    ctx->bit_length = 0;
    ctx->buffer_len = 0;
}

void sha1_update(sha1_ctx *ctx, const void *data, size_t len) {
    const uint8_t *bytes = (const uint8_t *)data;
    size_t i;
    for (i = 0; i < len; i++) {
        ctx->buffer[ctx->buffer_len++] = bytes[i];
        if (ctx->buffer_len == 64) {
            sha1_transform(ctx->state, ctx->buffer);
            ctx->bit_length += 512;
            ctx->buffer_len = 0;
        }
    }
}

void sha1_final(sha1_ctx *ctx, uint8_t out[SHA1_DIGEST_SIZE]) {
    uint64_t total_bits = ctx->bit_length + (uint64_t)ctx->buffer_len * 8;
    size_t i;

    ctx->buffer[ctx->buffer_len++] = 0x80;
    if (ctx->buffer_len > 56) {
        while (ctx->buffer_len < 64) {
            ctx->buffer[ctx->buffer_len++] = 0;
        }
        sha1_transform(ctx->state, ctx->buffer);
        ctx->buffer_len = 0;
    }
    while (ctx->buffer_len < 56) {
        ctx->buffer[ctx->buffer_len++] = 0;
    }
    for (i = 0; i < 8; i++) {
        ctx->buffer[56 + i] = (uint8_t)(total_bits >> (56 - i * 8));
    }
    sha1_transform(ctx->state, ctx->buffer);

    for (i = 0; i < 5; i++) {
        out[i * 4] = (uint8_t)(ctx->state[i] >> 24);
        out[i * 4 + 1] = (uint8_t)(ctx->state[i] >> 16);
        out[i * 4 + 2] = (uint8_t)(ctx->state[i] >> 8);
        out[i * 4 + 3] = (uint8_t)(ctx->state[i]);
    }
}

void sha1(const void *data, size_t len, uint8_t out[SHA1_DIGEST_SIZE]) {
    sha1_ctx ctx;
    sha1_init(&ctx);
    sha1_update(&ctx, data, len);
    sha1_final(&ctx, out);
}

void sha1_hex(const void *data, size_t len, char out[SHA1_HEX_SIZE]) {
    static const char hex[] = "0123456789abcdef";
    uint8_t digest[SHA1_DIGEST_SIZE];
    size_t i;
    sha1(data, len, digest);
    for (i = 0; i < SHA1_DIGEST_SIZE; i++) {
        out[i * 2] = hex[digest[i] >> 4];
        out[i * 2 + 1] = hex[digest[i] & 0x0f];
    }
    out[SHA1_DIGEST_SIZE * 2] = '\0';
}
