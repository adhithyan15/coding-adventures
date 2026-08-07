/*
 * sha256.c — implementation of SHA-256 (FIPS 180-4). Standard algorithm; the
 * output matches the Rust `sha256` crate and the published test vectors.
 */
#include "sha256.h"

#include <string.h> /* memcpy */

/* The first 32 bits of the fractional parts of the cube roots of the first 64
 * primes — the SHA-256 round constants. */
static const uint32_t K[64] = {
    0x428a2f98u, 0x71374491u, 0xb5c0fbcfu, 0xe9b5dba5u, 0x3956c25bu, 0x59f111f1u,
    0x923f82a4u, 0xab1c5ed5u, 0xd807aa98u, 0x12835b01u, 0x243185beu, 0x550c7dc3u,
    0x72be5d74u, 0x80deb1feu, 0x9bdc06a7u, 0xc19bf174u, 0xe49b69c1u, 0xefbe4786u,
    0x0fc19dc6u, 0x240ca1ccu, 0x2de92c6fu, 0x4a7484aau, 0x5cb0a9dcu, 0x76f988dau,
    0x983e5152u, 0xa831c66du, 0xb00327c8u, 0xbf597fc7u, 0xc6e00bf3u, 0xd5a79147u,
    0x06ca6351u, 0x14292967u, 0x27b70a85u, 0x2e1b2138u, 0x4d2c6dfcu, 0x53380d13u,
    0x650a7354u, 0x766a0abbu, 0x81c2c92eu, 0x92722c85u, 0xa2bfe8a1u, 0xa81a664bu,
    0xc24b8b70u, 0xc76c51a3u, 0xd192e819u, 0xd6990624u, 0xf40e3585u, 0x106aa070u,
    0x19a4c116u, 0x1e376c08u, 0x2748774cu, 0x34b0bcb5u, 0x391c0cb3u, 0x4ed8aa4au,
    0x5b9cca4fu, 0x682e6ff3u, 0x748f82eeu, 0x78a5636fu, 0x84c87814u, 0x8cc70208u,
    0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u};

/* Rotate a 32-bit word right by n (0 < n < 32). Masking keeps it 32-bit even if
 * uint32_t promotes to a wider int. */
static uint32_t rotr(uint32_t x, unsigned n) {
    return ((x >> n) | (x << (32 - n))) & 0xffffffffu;
}

/* Compress one 64-byte block into the eight-word state. */
static void sha256_transform(uint32_t state[8], const uint8_t block[64]) {
    uint32_t w[64];
    uint32_t a, b, c, d, e, f, g, h;
    unsigned i;

    /* Load the block as 16 big-endian words, then extend to 64. */
    for (i = 0; i < 16; i++) {
        w[i] = ((uint32_t)block[i * 4] << 24) |
               ((uint32_t)block[i * 4 + 1] << 16) |
               ((uint32_t)block[i * 4 + 2] << 8) |
               ((uint32_t)block[i * 4 + 3]);
    }
    for (i = 16; i < 64; i++) {
        uint32_t s0 = rotr(w[i - 15], 7) ^ rotr(w[i - 15], 18) ^ (w[i - 15] >> 3);
        uint32_t s1 = rotr(w[i - 2], 17) ^ rotr(w[i - 2], 19) ^ (w[i - 2] >> 10);
        w[i] = (w[i - 16] + s0 + w[i - 7] + s1) & 0xffffffffu;
    }

    a = state[0]; b = state[1]; c = state[2]; d = state[3];
    e = state[4]; f = state[5]; g = state[6]; h = state[7];

    for (i = 0; i < 64; i++) {
        uint32_t big_s1 = rotr(e, 6) ^ rotr(e, 11) ^ rotr(e, 25);
        uint32_t ch = (e & f) ^ (~e & g);
        uint32_t t1 = (h + big_s1 + ch + K[i] + w[i]) & 0xffffffffu;
        uint32_t big_s0 = rotr(a, 2) ^ rotr(a, 13) ^ rotr(a, 22);
        uint32_t maj = (a & b) ^ (a & c) ^ (b & c);
        uint32_t t2 = (big_s0 + maj) & 0xffffffffu;
        h = g; g = f; f = e;
        e = (d + t1) & 0xffffffffu;
        d = c; c = b; b = a;
        a = (t1 + t2) & 0xffffffffu;
    }

    state[0] = (state[0] + a) & 0xffffffffu;
    state[1] = (state[1] + b) & 0xffffffffu;
    state[2] = (state[2] + c) & 0xffffffffu;
    state[3] = (state[3] + d) & 0xffffffffu;
    state[4] = (state[4] + e) & 0xffffffffu;
    state[5] = (state[5] + f) & 0xffffffffu;
    state[6] = (state[6] + g) & 0xffffffffu;
    state[7] = (state[7] + h) & 0xffffffffu;
}

void sha256_init(sha256_ctx *ctx) {
    /* First 32 bits of the fractional parts of the square roots of the first 8
     * primes. */
    ctx->state[0] = 0x6a09e667u;
    ctx->state[1] = 0xbb67ae85u;
    ctx->state[2] = 0x3c6ef372u;
    ctx->state[3] = 0xa54ff53au;
    ctx->state[4] = 0x510e527fu;
    ctx->state[5] = 0x9b05688cu;
    ctx->state[6] = 0x1f83d9abu;
    ctx->state[7] = 0x5be0cd19u;
    ctx->bit_length = 0;
    ctx->buffer_len = 0;
}

void sha256_update(sha256_ctx *ctx, const void *data, size_t len) {
    const uint8_t *bytes = (const uint8_t *)data;
    size_t i;
    for (i = 0; i < len; i++) {
        ctx->buffer[ctx->buffer_len++] = bytes[i];
        if (ctx->buffer_len == 64) {
            sha256_transform(ctx->state, ctx->buffer);
            ctx->bit_length += 512;
            ctx->buffer_len = 0;
        }
    }
}

void sha256_final(sha256_ctx *ctx, uint8_t out[SHA256_DIGEST_SIZE]) {
    uint64_t total_bits = ctx->bit_length + (uint64_t)ctx->buffer_len * 8;
    size_t i;

    /* Append the 0x80 padding byte, then zeros until 8 bytes remain in a block,
     * then the 64-bit big-endian message length. */
    ctx->buffer[ctx->buffer_len++] = 0x80;
    if (ctx->buffer_len > 56) {
        while (ctx->buffer_len < 64) {
            ctx->buffer[ctx->buffer_len++] = 0;
        }
        sha256_transform(ctx->state, ctx->buffer);
        ctx->buffer_len = 0;
    }
    while (ctx->buffer_len < 56) {
        ctx->buffer[ctx->buffer_len++] = 0;
    }
    for (i = 0; i < 8; i++) {
        ctx->buffer[56 + i] = (uint8_t)(total_bits >> (56 - i * 8));
    }
    sha256_transform(ctx->state, ctx->buffer);

    /* Serialize the state as the big-endian digest. */
    for (i = 0; i < 8; i++) {
        out[i * 4] = (uint8_t)(ctx->state[i] >> 24);
        out[i * 4 + 1] = (uint8_t)(ctx->state[i] >> 16);
        out[i * 4 + 2] = (uint8_t)(ctx->state[i] >> 8);
        out[i * 4 + 3] = (uint8_t)(ctx->state[i]);
    }
}

void sha256(const void *data, size_t len, uint8_t out[SHA256_DIGEST_SIZE]) {
    sha256_ctx ctx;
    sha256_init(&ctx);
    sha256_update(&ctx, data, len);
    sha256_final(&ctx, out);
}

void sha256_hex(const void *data, size_t len, char out[SHA256_HEX_SIZE]) {
    static const char hex[] = "0123456789abcdef";
    uint8_t digest[SHA256_DIGEST_SIZE];
    size_t i;
    sha256(data, len, digest);
    for (i = 0; i < SHA256_DIGEST_SIZE; i++) {
        out[i * 2] = hex[digest[i] >> 4];
        out[i * 2 + 1] = hex[digest[i] & 0x0f];
    }
    out[SHA256_DIGEST_SIZE * 2] = '\0';
}
