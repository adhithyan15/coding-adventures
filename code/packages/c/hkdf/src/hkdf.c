/*
 * hkdf.c — HKDF extract/expand (RFC 5869), built on the sibling HMAC primitive.
 * Ported from the Rust `hkdf` crate.
 */
#include "hkdf.h"

#include <stdint.h> /* SIZE_MAX */
#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, memset */

#define HKDF_MAX_DIGEST 64 /* covers SHA-512; SHA-256 uses 32 */

hkdf_status hkdf_extract(hmac_hash_fn hash, size_t digest_size,
                         size_t block_size, const uint8_t *salt, size_t saltlen,
                         const uint8_t *ikm, size_t ikmlen, uint8_t *prk_out) {
    /* An absent salt is a string of digest_size zero bytes (RFC 5869 §2.2). */
    uint8_t zero_salt[HKDF_MAX_DIGEST];
    const uint8_t *effective_salt = salt;
    size_t effective_len = saltlen;
    if (saltlen == 0) {
        memset(zero_salt, 0, digest_size);
        effective_salt = zero_salt;
        effective_len = digest_size;
    }
    if (!hmac_compute(hash, digest_size, block_size, effective_salt,
                      effective_len, ikm, ikmlen, prk_out)) {
        return HKDF_ALLOC_FAILED;
    }
    return HKDF_OK;
}

hkdf_status hkdf_expand(hmac_hash_fn hash, size_t digest_size, size_t block_size,
                        const uint8_t *prk, size_t prklen, const uint8_t *info,
                        size_t infolen, uint8_t *out, size_t length) {
    size_t n, i, produced;
    uint8_t t_prev[HKDF_MAX_DIGEST];
    uint8_t t_cur[HKDF_MAX_DIGEST];
    size_t t_prev_len = 0; /* T(0) is empty */
    uint8_t *message;
    size_t message_cap;

    if (length == 0) {
        return HKDF_OUTPUT_TOO_SHORT;
    }
    /* length <= 255 * digest_size (guard the multiply). */
    if (digest_size == 0 || length > (size_t)255 * digest_size) {
        return HKDF_OUTPUT_TOO_LONG;
    }
    n = (length + digest_size - 1) / digest_size;

    /* Each HMAC message is T(i-1) || info || one counter byte. */
    if (infolen > SIZE_MAX - digest_size - 1) {
        return HKDF_ALLOC_FAILED;
    }
    message_cap = digest_size + infolen + 1;
    message = (uint8_t *)malloc(message_cap);
    if (message == NULL) {
        return HKDF_ALLOC_FAILED;
    }

    produced = 0;
    for (i = 1; i <= n; i++) {
        size_t msg_len = 0;
        size_t copy;
        memcpy(message, t_prev, t_prev_len);
        msg_len += t_prev_len;
        memcpy(message + msg_len, info, infolen);
        msg_len += infolen;
        message[msg_len++] = (uint8_t)i;

        if (!hmac_compute(hash, digest_size, block_size, prk, prklen, message,
                          msg_len, t_cur)) {
            free(message);
            return HKDF_ALLOC_FAILED;
        }

        /* Append T(i), truncating the final block to `length`. */
        copy = digest_size;
        if (produced + copy > length) {
            copy = length - produced;
        }
        memcpy(out + produced, t_cur, copy);
        produced += copy;

        memcpy(t_prev, t_cur, digest_size);
        t_prev_len = digest_size;
    }

    free(message);
    return HKDF_OK;
}

hkdf_status hkdf(hmac_hash_fn hash, size_t digest_size, size_t block_size,
                 const uint8_t *salt, size_t saltlen, const uint8_t *ikm,
                 size_t ikmlen, const uint8_t *info, size_t infolen,
                 uint8_t *out, size_t length) {
    uint8_t prk[HKDF_MAX_DIGEST];
    hkdf_status st = hkdf_extract(hash, digest_size, block_size, salt, saltlen,
                                  ikm, ikmlen, prk);
    if (st != HKDF_OK) {
        return st;
    }
    return hkdf_expand(hash, digest_size, block_size, prk, digest_size, info,
                       infolen, out, length);
}
