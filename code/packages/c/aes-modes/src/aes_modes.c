/*
 * aes_modes.c — implementation of the AES modes (see aes_modes.h). A faithful
 * port of the Rust `aes-modes` crate: the same ECB/CBC/CTR/GCM logic, PKCS#7
 * padding, and byte-wise GF(2^128) GHASH. The raw block cipher comes from the
 * sibling `aes` package.
 */
#include "aes_modes.h"

#include <stdlib.h> /* malloc, free */
#include <string.h> /* memcpy, memset */

#include "aes.h" /* aes_encrypt_block, aes_decrypt_block */

#define BLOCK_SIZE 16u

/* ---- small helpers ---------------------------------------------------- */

/* Encrypt one block, translating a bad key length to AESM_BAD_KEY_LENGTH. */
static AesmStatus enc_block(const uint8_t in[16], const uint8_t *key,
                           size_t key_len, uint8_t out[16]) {
    return aes_encrypt_block(in, key, key_len, out) ? AESM_OK
                                                    : AESM_BAD_KEY_LENGTH;
}

static AesmStatus dec_block(const uint8_t in[16], const uint8_t *key,
                           size_t key_len, uint8_t out[16]) {
    return aes_decrypt_block(in, key, key_len, out) ? AESM_OK
                                                    : AESM_BAD_KEY_LENGTH;
}

/* ---- PKCS#7 ----------------------------------------------------------- */

AesmStatus aesm_pkcs7_pad(const uint8_t *data, size_t len, uint8_t **out,
                          size_t *out_len) {
    size_t pad_len = BLOCK_SIZE - (len % BLOCK_SIZE);
    size_t total;
    uint8_t *buf;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    if (len > (size_t)-1 - pad_len) {
        return AESM_BAD_ARGS; /* length overflow */
    }
    total = len + pad_len;
    buf = malloc(total);
    if (!buf) {
        return AESM_ALLOC_ERROR;
    }
    if (len > 0) {
        memcpy(buf, data, len);
    }
    memset(buf + len, (int)(uint8_t)pad_len, pad_len);
    *out = buf;
    *out_len = total;
    return AESM_OK;
}

AesmStatus aesm_pkcs7_unpad(const uint8_t *data, size_t len, uint8_t **out,
                            size_t *out_len) {
    size_t pad_len;
    size_t i;
    uint8_t diff = 0;
    uint8_t *buf;
    size_t result_len;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    if (len == 0 || len % BLOCK_SIZE != 0) {
        return AESM_BAD_CIPHERTEXT_LENGTH;
    }
    pad_len = data[len - 1];
    if (pad_len < 1 || pad_len > BLOCK_SIZE) {
        return AESM_BAD_PADDING;
    }
    /* Constant-time-ish check: OR all differences so the loop is data-independent. */
    for (i = len - pad_len; i < len; i++) {
        diff |= (uint8_t)(data[i] ^ (uint8_t)pad_len);
    }
    if (diff != 0) {
        return AESM_BAD_PADDING;
    }
    result_len = len - pad_len;
    buf = malloc(result_len ? result_len : 1);
    if (!buf) {
        return AESM_ALLOC_ERROR;
    }
    if (result_len > 0) {
        memcpy(buf, data, result_len);
    }
    *out = buf;
    *out_len = result_len;
    return AESM_OK;
}

/* ---- ECB -------------------------------------------------------------- */

AesmStatus aesm_ecb_encrypt(const uint8_t *plaintext, size_t pt_len,
                            const uint8_t *key, size_t key_len, uint8_t **out,
                            size_t *out_len) {
    uint8_t *padded;
    size_t padded_len;
    uint8_t *buf;
    size_t off;
    AesmStatus rc;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    rc = aesm_pkcs7_pad(plaintext, pt_len, &padded, &padded_len);
    if (rc != AESM_OK) {
        return rc;
    }
    buf = malloc(padded_len);
    if (!buf) {
        free(padded);
        return AESM_ALLOC_ERROR;
    }
    for (off = 0; off < padded_len; off += BLOCK_SIZE) {
        rc = enc_block(padded + off, key, key_len, buf + off);
        if (rc != AESM_OK) {
            free(padded);
            free(buf);
            return rc;
        }
    }
    free(padded);
    *out = buf;
    *out_len = padded_len;
    return AESM_OK;
}

AesmStatus aesm_ecb_decrypt(const uint8_t *ciphertext, size_t ct_len,
                            const uint8_t *key, size_t key_len, uint8_t **out,
                            size_t *out_len) {
    uint8_t *plain;
    size_t off;
    AesmStatus rc;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    if (ct_len == 0 || ct_len % BLOCK_SIZE != 0) {
        return AESM_BAD_CIPHERTEXT_LENGTH;
    }
    plain = malloc(ct_len);
    if (!plain) {
        return AESM_ALLOC_ERROR;
    }
    for (off = 0; off < ct_len; off += BLOCK_SIZE) {
        rc = dec_block(ciphertext + off, key, key_len, plain + off);
        if (rc != AESM_OK) {
            free(plain);
            return rc;
        }
    }
    rc = aesm_pkcs7_unpad(plain, ct_len, out, out_len);
    free(plain);
    return rc;
}

/* ---- CBC -------------------------------------------------------------- */

AesmStatus aesm_cbc_encrypt(const uint8_t *plaintext, size_t pt_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, uint8_t **out,
                            size_t *out_len) {
    uint8_t *padded;
    size_t padded_len;
    uint8_t *buf;
    uint8_t prev[16];
    size_t off;
    size_t k;
    AesmStatus rc;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    if (iv_len != BLOCK_SIZE) {
        return AESM_BAD_IV_LENGTH;
    }
    rc = aesm_pkcs7_pad(plaintext, pt_len, &padded, &padded_len);
    if (rc != AESM_OK) {
        return rc;
    }
    buf = malloc(padded_len);
    if (!buf) {
        free(padded);
        return AESM_ALLOC_ERROR;
    }
    memcpy(prev, iv, BLOCK_SIZE);
    for (off = 0; off < padded_len; off += BLOCK_SIZE) {
        uint8_t xored[16];
        for (k = 0; k < BLOCK_SIZE; k++) {
            xored[k] = (uint8_t)(padded[off + k] ^ prev[k]);
        }
        rc = enc_block(xored, key, key_len, buf + off);
        if (rc != AESM_OK) {
            free(padded);
            free(buf);
            return rc;
        }
        memcpy(prev, buf + off, BLOCK_SIZE); /* prev = this ciphertext block */
    }
    free(padded);
    *out = buf;
    *out_len = padded_len;
    return AESM_OK;
}

AesmStatus aesm_cbc_decrypt(const uint8_t *ciphertext, size_t ct_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, uint8_t **out,
                            size_t *out_len) {
    uint8_t *plain;
    uint8_t prev[16];
    size_t off;
    size_t k;
    AesmStatus rc;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    if (iv_len != BLOCK_SIZE) {
        return AESM_BAD_IV_LENGTH;
    }
    if (ct_len == 0 || ct_len % BLOCK_SIZE != 0) {
        return AESM_BAD_CIPHERTEXT_LENGTH;
    }
    plain = malloc(ct_len);
    if (!plain) {
        return AESM_ALLOC_ERROR;
    }
    memcpy(prev, iv, BLOCK_SIZE);
    for (off = 0; off < ct_len; off += BLOCK_SIZE) {
        uint8_t dec[16];
        rc = dec_block(ciphertext + off, key, key_len, dec);
        if (rc != AESM_OK) {
            free(plain);
            return rc;
        }
        for (k = 0; k < BLOCK_SIZE; k++) {
            plain[off + k] = (uint8_t)(dec[k] ^ prev[k]);
        }
        memcpy(prev, ciphertext + off, BLOCK_SIZE); /* prev = this CT block */
    }
    rc = aesm_pkcs7_unpad(plain, ct_len, out, out_len);
    free(plain);
    return rc;
}

/* ---- CTR -------------------------------------------------------------- */

/* Build [nonce(12) || counter(4, big-endian)]. */
static void build_counter_block(const uint8_t *nonce, uint32_t counter,
                                uint8_t out[16]) {
    memcpy(out, nonce, 12);
    out[12] = (uint8_t)((counter >> 24) & 0xFF);
    out[13] = (uint8_t)((counter >> 16) & 0xFF);
    out[14] = (uint8_t)((counter >> 8) & 0xFF);
    out[15] = (uint8_t)(counter & 0xFF);
}

AesmStatus aesm_ctr_encrypt(const uint8_t *input, size_t in_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *nonce, size_t nonce_len,
                            uint8_t **out, size_t *out_len) {
    uint8_t *buf;
    uint32_t counter = 1;
    size_t off;
    AesmStatus rc;
    if (!out || !out_len) {
        return AESM_BAD_ARGS;
    }
    if (nonce_len != 12) {
        return AESM_BAD_NONCE_LENGTH;
    }
    buf = malloc(in_len ? in_len : 1);
    if (!buf) {
        return AESM_ALLOC_ERROR;
    }
    for (off = 0; off < in_len; off += BLOCK_SIZE) {
        uint8_t counter_block[16];
        uint8_t keystream[16];
        size_t n = in_len - off;
        size_t k;
        if (n > BLOCK_SIZE) {
            n = BLOCK_SIZE;
        }
        build_counter_block(nonce, counter, counter_block);
        rc = enc_block(counter_block, key, key_len, keystream);
        if (rc != AESM_OK) {
            free(buf);
            return rc;
        }
        for (k = 0; k < n; k++) {
            buf[off + k] = (uint8_t)(input[off + k] ^ keystream[k]);
        }
        counter = counter + 1; /* uint32 wraps (well-defined) */
    }
    *out = buf;
    *out_len = in_len;
    return AESM_OK;
}

AesmStatus aesm_ctr_decrypt(const uint8_t *input, size_t in_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *nonce, size_t nonce_len,
                            uint8_t **out, size_t *out_len) {
    return aesm_ctr_encrypt(input, in_len, key, key_len, nonce, nonce_len, out,
                            out_len);
}

/* ---- GCM -------------------------------------------------------------- */

/* GF(2^128) multiply with the GCM reducing polynomial (high byte 0xE1). */
static void gf128_mul(const uint8_t x[16], const uint8_t y[16],
                      uint8_t out[16]) {
    uint8_t z[16];
    uint8_t v[16];
    int i;
    memset(z, 0, 16);
    memcpy(v, y, 16);
    for (i = 0; i < 128; i++) {
        int byte_idx = i / 8;
        int bit_idx = 7 - (i % 8);
        int j;
        uint8_t carry;
        if ((x[byte_idx] >> bit_idx) & 1) {
            for (j = 0; j < 16; j++) {
                z[j] ^= v[j];
            }
        }
        carry = (uint8_t)(v[15] & 1);
        for (j = 15; j >= 1; j--) {
            v[j] = (uint8_t)((v[j] >> 1) | ((v[j - 1] & 1) << 7));
        }
        v[0] >>= 1;
        if (carry) {
            v[0] ^= 0xe1;
        }
    }
    memcpy(out, z, 16);
}

/* GHASH over AAD and ciphertext, ending with the bit-length block. */
static void ghash(const uint8_t h[16], const uint8_t *aad, size_t aad_len,
                  const uint8_t *ct, size_t ct_len, uint8_t out[16]) {
    uint8_t y[16];
    uint8_t block[16];
    size_t off;
    int j;
    uint64_t aad_bits;
    uint64_t ct_bits;
    memset(y, 0, 16);

    for (off = 0; off < aad_len; off += BLOCK_SIZE) {
        size_t n = aad_len - off;
        if (n > BLOCK_SIZE) {
            n = BLOCK_SIZE;
        }
        memset(block, 0, 16);
        memcpy(block, aad + off, n);
        for (j = 0; j < 16; j++) {
            block[j] ^= y[j];
        }
        gf128_mul(block, h, y);
    }
    for (off = 0; off < ct_len; off += BLOCK_SIZE) {
        size_t n = ct_len - off;
        if (n > BLOCK_SIZE) {
            n = BLOCK_SIZE;
        }
        memset(block, 0, 16);
        memcpy(block, ct + off, n);
        for (j = 0; j < 16; j++) {
            block[j] ^= y[j];
        }
        gf128_mul(block, h, y);
    }

    aad_bits = (uint64_t)aad_len * 8u;
    ct_bits = (uint64_t)ct_len * 8u;
    for (j = 0; j < 8; j++) {
        block[j] = (uint8_t)((aad_bits >> (56 - 8 * j)) & 0xFF);
        block[8 + j] = (uint8_t)((ct_bits >> (56 - 8 * j)) & 0xFF);
    }
    for (j = 0; j < 16; j++) {
        block[j] ^= y[j];
    }
    gf128_mul(block, h, out);
}

/* Increment the 32-bit big-endian counter in the last 4 bytes. */
static void increment_counter(uint8_t block[16]) {
    int i;
    for (i = 15; i >= 12; i--) {
        block[i] = (uint8_t)(block[i] + 1);
        if (block[i] != 0) {
            break;
        }
    }
}

/* CTR-style keystream over `in` using an evolving counter starting at J0
 * (incremented before each block). Writes `in_len` bytes to `out`. */
static AesmStatus gcm_ctr(const uint8_t j0[16], const uint8_t *in,
                         size_t in_len, const uint8_t *key, size_t key_len,
                         uint8_t *out) {
    uint8_t counter[16];
    size_t off;
    AesmStatus rc;
    memcpy(counter, j0, 16);
    for (off = 0; off < in_len; off += BLOCK_SIZE) {
        uint8_t keystream[16];
        size_t n = in_len - off;
        size_t k;
        if (n > BLOCK_SIZE) {
            n = BLOCK_SIZE;
        }
        increment_counter(counter);
        rc = enc_block(counter, key, key_len, keystream);
        if (rc != AESM_OK) {
            return rc;
        }
        for (k = 0; k < n; k++) {
            out[off + k] = (uint8_t)(in[off + k] ^ keystream[k]);
        }
    }
    return AESM_OK;
}

/* Compute the 16-byte GCM tag = GHASH(H, AAD, CT) XOR E(J0). */
static AesmStatus gcm_tag(const uint8_t h[16], const uint8_t j0[16],
                         const uint8_t *aad, size_t aad_len, const uint8_t *ct,
                         size_t ct_len, const uint8_t *key, size_t key_len,
                         uint8_t tag[16]) {
    uint8_t gh[16];
    uint8_t enc_j0[16];
    int i;
    AesmStatus rc;
    ghash(h, aad, aad_len, ct, ct_len, gh);
    rc = enc_block(j0, key, key_len, enc_j0);
    if (rc != AESM_OK) {
        return rc;
    }
    for (i = 0; i < 16; i++) {
        tag[i] = (uint8_t)(gh[i] ^ enc_j0[i]);
    }
    return AESM_OK;
}

AesmStatus aesm_gcm_encrypt(const uint8_t *plaintext, size_t pt_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, const uint8_t *aad,
                            size_t aad_len, uint8_t **out_ct, size_t *out_ct_len,
                            uint8_t tag[16]) {
    uint8_t zero[16];
    uint8_t h[16];
    uint8_t j0[16];
    uint8_t *ct;
    AesmStatus rc;
    if (!out_ct || !out_ct_len || !tag) {
        return AESM_BAD_ARGS;
    }
    if (iv_len != 12) {
        return AESM_BAD_IV_LENGTH;
    }
    memset(zero, 0, 16);
    rc = enc_block(zero, key, key_len, h); /* H = E(0^128) */
    if (rc != AESM_OK) {
        return rc;
    }
    memset(j0, 0, 16); /* J0 = IV || 0x00000001 */
    memcpy(j0, iv, 12);
    j0[15] = 1;

    ct = malloc(pt_len ? pt_len : 1);
    if (!ct) {
        return AESM_ALLOC_ERROR;
    }
    rc = gcm_ctr(j0, plaintext, pt_len, key, key_len, ct);
    if (rc != AESM_OK) {
        free(ct);
        return rc;
    }
    rc = gcm_tag(h, j0, aad, aad_len, ct, pt_len, key, key_len, tag);
    if (rc != AESM_OK) {
        free(ct);
        return rc;
    }
    *out_ct = ct;
    *out_ct_len = pt_len;
    return AESM_OK;
}

AesmStatus aesm_gcm_decrypt(const uint8_t *ciphertext, size_t ct_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, const uint8_t *aad,
                            size_t aad_len, const uint8_t tag[16],
                            uint8_t **out_pt, size_t *out_pt_len) {
    uint8_t zero[16];
    uint8_t h[16];
    uint8_t j0[16];
    uint8_t expected[16];
    uint8_t *pt;
    uint8_t diff = 0;
    int i;
    AesmStatus rc;
    if (!out_pt || !out_pt_len || !tag) {
        return AESM_BAD_ARGS;
    }
    if (iv_len != 12) {
        return AESM_BAD_IV_LENGTH;
    }
    memset(zero, 0, 16);
    rc = enc_block(zero, key, key_len, h);
    if (rc != AESM_OK) {
        return rc;
    }
    memset(j0, 0, 16);
    memcpy(j0, iv, 12);
    j0[15] = 1;

    /* Verify the tag BEFORE decrypting (constant-time compare). */
    rc = gcm_tag(h, j0, aad, aad_len, ciphertext, ct_len, key, key_len,
                 expected);
    if (rc != AESM_OK) {
        return rc;
    }
    for (i = 0; i < 16; i++) {
        diff |= (uint8_t)(expected[i] ^ tag[i]);
    }
    if (diff != 0) {
        return AESM_AUTH_FAILED;
    }

    pt = malloc(ct_len ? ct_len : 1);
    if (!pt) {
        return AESM_ALLOC_ERROR;
    }
    rc = gcm_ctr(j0, ciphertext, ct_len, key, key_len, pt);
    if (rc != AESM_OK) {
        free(pt);
        return rc;
    }
    *out_pt = pt;
    *out_pt_len = ct_len;
    return AESM_OK;
}
