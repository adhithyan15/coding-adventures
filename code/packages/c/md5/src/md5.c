/*
 * md5.c — implementation of MD5 (RFC 1321). Standard algorithm; output matches
 * the Rust `md5` crate and the RFC 1321 test suite.
 */
#include "md5.h"

/* Per-round left-rotation amounts. */
static const unsigned S[64] = {
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
    5, 9,  14, 20, 5, 9,  14, 20, 5, 9,  14, 20, 5, 9,  14, 20,
    4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21};

/* K[i] = floor(2^32 * abs(sin(i + 1))) — the MD5 round constants. */
static const uint32_t K[64] = {
    0xd76aa478u, 0xe8c7b756u, 0x242070dbu, 0xc1bdceeeu, 0xf57c0fafu, 0x4787c62au,
    0xa8304613u, 0xfd469501u, 0x698098d8u, 0x8b44f7afu, 0xffff5bb1u, 0x895cd7beu,
    0x6b901122u, 0xfd987193u, 0xa679438eu, 0x49b40821u, 0xf61e2562u, 0xc040b340u,
    0x265e5a51u, 0xe9b6c7aau, 0xd62f105du, 0x02441453u, 0xd8a1e681u, 0xe7d3fbc8u,
    0x21e1cde6u, 0xc33707d6u, 0xf4d50d87u, 0x455a14edu, 0xa9e3e905u, 0xfcefa3f8u,
    0x676f02d9u, 0x8d2a4c8au, 0xfffa3942u, 0x8771f681u, 0x6d9d6122u, 0xfde5380cu,
    0xa4beea44u, 0x4bdecfa9u, 0xf6bb4b60u, 0xbebfbc70u, 0x289b7ec6u, 0xeaa127fau,
    0xd4ef3085u, 0x04881d05u, 0xd9d4d039u, 0xe6db99e5u, 0x1fa27cf8u, 0xc4ac5665u,
    0xf4292244u, 0x432aff97u, 0xab9423a7u, 0xfc93a039u, 0x655b59c3u, 0x8f0ccc92u,
    0xffeff47du, 0x85845dd1u, 0x6fa87e4fu, 0xfe2ce6e0u, 0xa3014314u, 0x4e0811a1u,
    0xf7537e82u, 0xbd3af235u, 0x2ad7d2bbu, 0xeb86d391u};

static uint32_t rotl(uint32_t x, unsigned n) {
    return ((x << n) | (x >> (32 - n))) & 0xffffffffu;
}

/* Compress one 64-byte block. Message words are read LITTLE-endian. */
static void md5_transform(uint32_t state[4], const uint8_t block[64]) {
    uint32_t m[16];
    uint32_t a, b, c, d;
    unsigned i;

    for (i = 0; i < 16; i++) {
        m[i] = ((uint32_t)block[i * 4]) |
               ((uint32_t)block[i * 4 + 1] << 8) |
               ((uint32_t)block[i * 4 + 2] << 16) |
               ((uint32_t)block[i * 4 + 3] << 24);
    }

    a = state[0]; b = state[1]; c = state[2]; d = state[3];

    for (i = 0; i < 64; i++) {
        uint32_t f;
        unsigned g;
        if (i < 16) {
            f = (b & c) | (~b & d);
            g = i;
        } else if (i < 32) {
            f = (d & b) | (~d & c);
            g = (5 * i + 1) % 16;
        } else if (i < 48) {
            f = b ^ c ^ d;
            g = (3 * i + 5) % 16;
        } else {
            f = c ^ (b | ~d);
            g = (7 * i) % 16;
        }
        f = (f + a + K[i] + m[g]) & 0xffffffffu;
        a = d;
        d = c;
        c = b;
        b = (b + rotl(f, S[i])) & 0xffffffffu;
    }

    state[0] = (state[0] + a) & 0xffffffffu;
    state[1] = (state[1] + b) & 0xffffffffu;
    state[2] = (state[2] + c) & 0xffffffffu;
    state[3] = (state[3] + d) & 0xffffffffu;
}

void md5_init(md5_ctx *ctx) {
    ctx->state[0] = 0x67452301u;
    ctx->state[1] = 0xefcdab89u;
    ctx->state[2] = 0x98badcfeu;
    ctx->state[3] = 0x10325476u;
    ctx->bit_length = 0;
    ctx->buffer_len = 0;
}

void md5_update(md5_ctx *ctx, const void *data, size_t len) {
    const uint8_t *bytes = (const uint8_t *)data;
    size_t i;
    for (i = 0; i < len; i++) {
        ctx->buffer[ctx->buffer_len++] = bytes[i];
        if (ctx->buffer_len == 64) {
            md5_transform(ctx->state, ctx->buffer);
            ctx->bit_length += 512;
            ctx->buffer_len = 0;
        }
    }
}

void md5_final(md5_ctx *ctx, uint8_t out[MD5_DIGEST_SIZE]) {
    uint64_t total_bits = ctx->bit_length + (uint64_t)ctx->buffer_len * 8;
    size_t i;

    ctx->buffer[ctx->buffer_len++] = 0x80;
    if (ctx->buffer_len > 56) {
        while (ctx->buffer_len < 64) {
            ctx->buffer[ctx->buffer_len++] = 0;
        }
        md5_transform(ctx->state, ctx->buffer);
        ctx->buffer_len = 0;
    }
    while (ctx->buffer_len < 56) {
        ctx->buffer[ctx->buffer_len++] = 0;
    }
    /* Message length as a 64-bit LITTLE-endian value. */
    for (i = 0; i < 8; i++) {
        ctx->buffer[56 + i] = (uint8_t)(total_bits >> (i * 8));
    }
    md5_transform(ctx->state, ctx->buffer);

    /* Output the state as little-endian bytes. */
    for (i = 0; i < 4; i++) {
        out[i * 4] = (uint8_t)(ctx->state[i]);
        out[i * 4 + 1] = (uint8_t)(ctx->state[i] >> 8);
        out[i * 4 + 2] = (uint8_t)(ctx->state[i] >> 16);
        out[i * 4 + 3] = (uint8_t)(ctx->state[i] >> 24);
    }
}

void md5(const void *data, size_t len, uint8_t out[MD5_DIGEST_SIZE]) {
    md5_ctx ctx;
    md5_init(&ctx);
    md5_update(&ctx, data, len);
    md5_final(&ctx, out);
}

void md5_hex(const void *data, size_t len, char out[MD5_HEX_SIZE]) {
    static const char hex[] = "0123456789abcdef";
    uint8_t digest[MD5_DIGEST_SIZE];
    size_t i;
    md5(data, len, digest);
    for (i = 0; i < MD5_DIGEST_SIZE; i++) {
        out[i * 2] = hex[digest[i] >> 4];
        out[i * 2 + 1] = hex[digest[i] & 0x0f];
    }
    out[MD5_DIGEST_SIZE * 2] = '\0';
}
