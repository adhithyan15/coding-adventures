#ifndef FORME_SANDBOX_SHA256_H
#define FORME_SANDBOX_SHA256_H

#include <stddef.h>
#include <stdint.h>

#define SHA256_DIGEST_SIZE 32

typedef struct {
    uint32_t state[8];
    uint64_t bit_length;
    uint8_t buffer[64];
    size_t buffer_len;
} sha256_ctx;

void sha256_init(sha256_ctx *context);
void sha256_update(sha256_ctx *context, const void *data, size_t length);
void sha256_final(sha256_ctx *context, uint8_t output[SHA256_DIGEST_SIZE]);

#endif
