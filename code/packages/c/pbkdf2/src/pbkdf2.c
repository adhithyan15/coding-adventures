/*
 * pbkdf2.c — implementation of PBKDF2 (see pbkdf2.h). A faithful port of the
 * Rust `pbkdf2` crate: the same block loop, INT_32_BE(i) salt suffix, and
 * U-value XOR accumulation. The PRF is HMAC (the sibling `hmac` package) over
 * the given hash.
 */
#include "pbkdf2.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy */

#include "hmac.h"   /* hmac_compute */
#include "sha1.h"   /* sha1 */
#include "sha256.h" /* sha256 */
#include "sha512.h" /* sha512 */

/* The largest digest we handle (SHA-512 = 64 bytes); U/T scratch is stack-sized
 * to this, so no per-iteration allocation is needed. */
#define PBKDF2_MAX_HLEN 64u

Pbkdf2Status pbkdf2(pbkdf2_hash_fn hash, size_t h_len, size_t block_size,
                    const uint8_t *password, size_t password_len,
                    const uint8_t *salt, size_t salt_len, size_t iterations,
                    uint8_t *dk_out, size_t key_length,
                    int allow_empty_password) {
    uint8_t t[PBKDF2_MAX_HLEN];
    uint8_t prev[PBKDF2_MAX_HLEN];
    uint8_t next[PBKDF2_MAX_HLEN];
    uint8_t *seed;
    size_t seed_len;
    size_t num_blocks;
    size_t offset;
    uint32_t i;

    /* ---- argument validation (mirrors the Rust error variants) -------- */
    if (!hash || !dk_out) {
        return PBKDF2_BAD_ARGS;
    }
    if (h_len == 0 || h_len > PBKDF2_MAX_HLEN) {
        return PBKDF2_BAD_ARGS;
    }
    if (password_len == 0 && !allow_empty_password) {
        return PBKDF2_EMPTY_PASSWORD;
    }
    if (iterations == 0) {
        return PBKDF2_INVALID_ITERATIONS;
    }
    if (key_length == 0) {
        return PBKDF2_INVALID_KEY_LENGTH;
    }
    if (key_length > PBKDF2_MAX_KEY_LENGTH) {
        return PBKDF2_KEY_LENGTH_TOO_LARGE;
    }
    /* Guard the salt||counter concatenation against size_t overflow. */
    if (salt_len > (size_t)-1 - 4u) {
        return PBKDF2_BAD_ARGS;
    }

    /* ceil(key_length / h_len). key_length <= 2^20 and h_len >= 1, so this is
     * at most 2^20 — well within uint32_t. */
    num_blocks = (key_length + h_len - 1) / h_len;

    seed_len = salt_len + 4u;
    seed = malloc(seed_len);
    if (!seed) {
        return PBKDF2_PRF_ERROR;
    }
    if (salt_len > 0) {
        memcpy(seed, salt, salt_len);
    }

    offset = 0;
    for (i = 1; i <= (uint32_t)num_blocks; i++) {
        size_t j;
        size_t k;
        size_t take;

        /* Seed = Salt || INT_32_BE(i). */
        seed[salt_len + 0] = (uint8_t)((i >> 24) & 0xFF);
        seed[salt_len + 1] = (uint8_t)((i >> 16) & 0xFF);
        seed[salt_len + 2] = (uint8_t)((i >> 8) & 0xFF);
        seed[salt_len + 3] = (uint8_t)(i & 0xFF);

        /* U_1 = PRF(Password, Seed); T = U_1; prev = U_1. */
        if (!hmac_compute(hash, h_len, block_size, password, password_len, seed,
                          seed_len, t)) {
            free(seed);
            return PBKDF2_PRF_ERROR;
        }
        memcpy(prev, t, h_len);

        /* U_j = PRF(Password, U_{j-1}); T ^= U_j, for j = 2..iterations. */
        for (j = 1; j < iterations; j++) {
            if (!hmac_compute(hash, h_len, block_size, password, password_len,
                              prev, h_len, next)) {
                free(seed);
                return PBKDF2_PRF_ERROR;
            }
            for (k = 0; k < h_len; k++) {
                t[k] ^= next[k];
            }
            memcpy(prev, next, h_len);
        }

        /* Append the block, truncating the final one to key_length. */
        take = key_length - offset;
        if (take > h_len) {
            take = h_len;
        }
        memcpy(dk_out + offset, t, take);
        offset += take;
    }

    free(seed);
    return PBKDF2_OK;
}

Pbkdf2Status pbkdf2_hmac_sha1(const uint8_t *password, size_t password_len,
                              const uint8_t *salt, size_t salt_len,
                              size_t iterations, uint8_t *dk_out,
                              size_t key_length, int allow_empty_password) {
    return pbkdf2(sha1, 20, 64, password, password_len, salt, salt_len,
                  iterations, dk_out, key_length, allow_empty_password);
}

Pbkdf2Status pbkdf2_hmac_sha256(const uint8_t *password, size_t password_len,
                                const uint8_t *salt, size_t salt_len,
                                size_t iterations, uint8_t *dk_out,
                                size_t key_length, int allow_empty_password) {
    return pbkdf2(sha256, 32, 64, password, password_len, salt, salt_len,
                  iterations, dk_out, key_length, allow_empty_password);
}

Pbkdf2Status pbkdf2_hmac_sha512(const uint8_t *password, size_t password_len,
                                const uint8_t *salt, size_t salt_len,
                                size_t iterations, uint8_t *dk_out,
                                size_t key_length, int allow_empty_password) {
    return pbkdf2(sha512, 64, 128, password, password_len, salt, salt_len,
                  iterations, dk_out, key_length, allow_empty_password);
}
