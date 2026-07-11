/*
 * hmac.c — the HMAC construction (RFC 2104), hash-agnostic. Ported from the Rust
 * `hmac` crate's generic `hmac` function.
 */
#include "hmac.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, memset */

#define HMAC_IPAD 0x36
#define HMAC_OPAD 0x5c

int hmac_compute(hmac_hash_fn hash, size_t digest_size, size_t block_size,
                 const uint8_t *key, size_t keylen, const uint8_t *msg,
                 size_t msglen, uint8_t *out) {
    uint8_t *k0;         /* the normalized, block-sized key */
    uint8_t *inner_in;   /* (K0 ^ ipad) || message */
    uint8_t *outer_in;   /* (K0 ^ opad) || inner-digest */
    size_t i;
    int ok = 0;

    /* Guard the concatenation size against overflow. */
    if (msglen > SIZE_MAX - block_size ||
        digest_size > SIZE_MAX - block_size) {
        return 0;
    }

    k0 = (uint8_t *)calloc(block_size, 1); /* zero-padded by calloc */
    inner_in = (uint8_t *)malloc(block_size + msglen);
    outer_in = (uint8_t *)malloc(block_size + digest_size);
    if (k0 == NULL || inner_in == NULL || outer_in == NULL) {
        goto done;
    }

    /* Normalize the key: hash it if longer than the block, else copy; the rest
     * of k0 stays zero. */
    if (keylen > block_size) {
        hash(key, keylen, k0); /* writes digest_size <= block_size bytes */
    } else {
        memcpy(k0, key, keylen);
    }

    /* inner = H( (K0 ^ ipad) || message ) */
    for (i = 0; i < block_size; i++) {
        inner_in[i] = (uint8_t)(k0[i] ^ HMAC_IPAD);
    }
    memcpy(inner_in + block_size, msg, msglen);
    hash(inner_in, block_size + msglen, out); /* stash inner digest in out */

    /* HMAC = H( (K0 ^ opad) || inner ) */
    for (i = 0; i < block_size; i++) {
        outer_in[i] = (uint8_t)(k0[i] ^ HMAC_OPAD);
    }
    memcpy(outer_in + block_size, out, digest_size);
    hash(outer_in, block_size + digest_size, out);
    ok = 1;

done:
    free(k0);
    free(inner_in);
    free(outer_in);
    return ok;
}

int hmac_verify(const uint8_t *a, const uint8_t *b, size_t len) {
    uint8_t diff = 0;
    size_t i;
    for (i = 0; i < len; i++) {
        diff = (uint8_t)(diff | (a[i] ^ b[i]));
    }
    return diff == 0 ? 1 : 0;
}
