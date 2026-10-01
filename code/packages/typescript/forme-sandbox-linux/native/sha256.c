/* Package-local streaming SHA-256, derived from c/sha256 (FIPS 180-4). */
#include "sha256.h"

#include <string.h>

static const uint32_t round_constants[64] = {
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
    0x90befffau, 0xa4506cebu, 0xbef9a3f7u, 0xc67178f2u,
};

static uint32_t rotate_right(uint32_t value, unsigned count) {
    return (value >> count) | (value << (32 - count));
}

static void transform(uint32_t state[8], const uint8_t block[64]) {
    uint32_t words[64];
    for (unsigned index = 0; index < 16; index++) {
        words[index] = ((uint32_t)block[index * 4] << 24)
            | ((uint32_t)block[index * 4 + 1] << 16)
            | ((uint32_t)block[index * 4 + 2] << 8)
            | (uint32_t)block[index * 4 + 3];
    }
    for (unsigned index = 16; index < 64; index++) {
        uint32_t small0 = rotate_right(words[index - 15], 7)
            ^ rotate_right(words[index - 15], 18) ^ (words[index - 15] >> 3);
        uint32_t small1 = rotate_right(words[index - 2], 17)
            ^ rotate_right(words[index - 2], 19) ^ (words[index - 2] >> 10);
        words[index] = words[index - 16] + small0 + words[index - 7] + small1;
    }

    uint32_t a = state[0], b = state[1], c = state[2], d = state[3];
    uint32_t e = state[4], f = state[5], g = state[6], h = state[7];
    for (unsigned index = 0; index < 64; index++) {
        uint32_t big1 = rotate_right(e, 6) ^ rotate_right(e, 11) ^ rotate_right(e, 25);
        uint32_t choose = (e & f) ^ (~e & g);
        uint32_t first = h + big1 + choose + round_constants[index] + words[index];
        uint32_t big0 = rotate_right(a, 2) ^ rotate_right(a, 13) ^ rotate_right(a, 22);
        uint32_t majority = (a & b) ^ (a & c) ^ (b & c);
        uint32_t second = big0 + majority;
        h = g; g = f; f = e; e = d + first;
        d = c; c = b; b = a; a = first + second;
    }
    state[0] += a; state[1] += b; state[2] += c; state[3] += d;
    state[4] += e; state[5] += f; state[6] += g; state[7] += h;
}

void sha256_init(sha256_ctx *context) {
    static const uint32_t initial[8] = {
        0x6a09e667u, 0xbb67ae85u, 0x3c6ef372u, 0xa54ff53au,
        0x510e527fu, 0x9b05688cu, 0x1f83d9abu, 0x5be0cd19u,
    };
    memcpy(context->state, initial, sizeof(initial));
    context->bit_length = 0;
    context->buffer_len = 0;
}

void sha256_update(sha256_ctx *context, const void *data, size_t length) {
    const uint8_t *bytes = data;
    for (size_t index = 0; index < length; index++) {
        context->buffer[context->buffer_len++] = bytes[index];
        if (context->buffer_len == 64) {
            transform(context->state, context->buffer);
            context->bit_length += 512;
            context->buffer_len = 0;
        }
    }
}

void sha256_final(sha256_ctx *context, uint8_t output[SHA256_DIGEST_SIZE]) {
    uint64_t total_bits = context->bit_length + (uint64_t)context->buffer_len * 8;
    context->buffer[context->buffer_len++] = 0x80;
    if (context->buffer_len > 56) {
        while (context->buffer_len < 64) context->buffer[context->buffer_len++] = 0;
        transform(context->state, context->buffer);
        context->buffer_len = 0;
    }
    while (context->buffer_len < 56) context->buffer[context->buffer_len++] = 0;
    for (size_t index = 0; index < 8; index++) {
        context->buffer[56 + index] = (uint8_t)(total_bits >> (56 - index * 8));
    }
    transform(context->state, context->buffer);
    for (size_t index = 0; index < 8; index++) {
        output[index * 4] = (uint8_t)(context->state[index] >> 24);
        output[index * 4 + 1] = (uint8_t)(context->state[index] >> 16);
        output[index * 4 + 2] = (uint8_t)(context->state[index] >> 8);
        output[index * 4 + 3] = (uint8_t)context->state[index];
    }
}
