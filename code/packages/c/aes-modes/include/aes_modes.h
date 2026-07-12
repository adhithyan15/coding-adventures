/*
 * aes_modes.h — AES modes of operation (ECB, CBC, CTR, GCM) with PKCS#7
 * padding, in pure ISO C17. A faithful port of the Rust `aes-modes` crate.
 * ===========================================================================
 *
 * AES is a 128-bit (16-byte) block cipher; a *mode of operation* chains block
 * calls to encrypt arbitrary-length messages. This package builds on the raw
 * block cipher from the sibling `aes` package.
 *
 *   ECB — each block independently. INSECURE (identical blocks leak); here for
 *         teaching only. PKCS#7 padded.
 *   CBC — C[i] = E(P[i] XOR C[i-1]); needs a 16-byte IV. PKCS#7 padded.
 *   CTR — stream cipher: keystream = E(nonce||counter); XOR into data. 12-byte
 *         nonce, 32-bit big-endian counter from 1. No padding; enc == dec.
 *   GCM — CTR encryption + a GHASH authentication tag over AAD and ciphertext
 *         (AEAD). 12-byte IV. Decryption verifies the tag before returning.
 *
 * GHASH multiplies in GF(2^128) with the reducing polynomial x^128+x^7+x^2+x+1
 * (done byte-wise — no 128-bit integers needed).
 *
 * OWNERSHIP. Variable-length outputs are returned in a malloc'd buffer via an
 * out-pointer; the caller frees it with free(). Every function returns an
 * AesmStatus.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef AES_MODES_H
#define AES_MODES_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

typedef enum {
    AESM_OK = 0,
    AESM_BAD_KEY_LENGTH,        /* AES key length not 16/24/32 */
    AESM_BAD_IV_LENGTH,         /* IV not the required length */
    AESM_BAD_NONCE_LENGTH,      /* CTR nonce not 12 bytes */
    AESM_BAD_CIPHERTEXT_LENGTH, /* ECB/CBC input not a positive multiple of 16 */
    AESM_BAD_PADDING,           /* invalid PKCS#7 padding on decrypt */
    AESM_AUTH_FAILED,           /* GCM tag mismatch */
    AESM_BAD_ARGS,              /* NULL out-pointer, or a length overflow */
    AESM_ALLOC_ERROR            /* out of memory */
} AesmStatus;

/* ---- PKCS#7 padding (exposed, as in the Rust crate) ------------------- */

/* aesm_pkcs7_pad — append 1..16 bytes so the length is a multiple of 16. */
AesmStatus aesm_pkcs7_pad(const uint8_t *data, size_t len, uint8_t **out,
                          size_t *out_len);
/* aesm_pkcs7_unpad — strip valid PKCS#7 padding, else AESM_BAD_PADDING. */
AesmStatus aesm_pkcs7_unpad(const uint8_t *data, size_t len, uint8_t **out,
                            size_t *out_len);

/* ---- ECB (INSECURE — educational) ------------------------------------- */

AesmStatus aesm_ecb_encrypt(const uint8_t *plaintext, size_t pt_len,
                            const uint8_t *key, size_t key_len, uint8_t **out,
                            size_t *out_len);
AesmStatus aesm_ecb_decrypt(const uint8_t *ciphertext, size_t ct_len,
                            const uint8_t *key, size_t key_len, uint8_t **out,
                            size_t *out_len);

/* ---- CBC (16-byte IV) ------------------------------------------------- */

AesmStatus aesm_cbc_encrypt(const uint8_t *plaintext, size_t pt_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, uint8_t **out,
                            size_t *out_len);
AesmStatus aesm_cbc_decrypt(const uint8_t *ciphertext, size_t ct_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, uint8_t **out,
                            size_t *out_len);

/* ---- CTR (12-byte nonce; enc == dec) ---------------------------------- */

AesmStatus aesm_ctr_encrypt(const uint8_t *input, size_t in_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *nonce, size_t nonce_len,
                            uint8_t **out, size_t *out_len);
/* aesm_ctr_decrypt — identical to aesm_ctr_encrypt (stream cipher). */
AesmStatus aesm_ctr_decrypt(const uint8_t *input, size_t in_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *nonce, size_t nonce_len,
                            uint8_t **out, size_t *out_len);

/* ---- GCM (12-byte IV; AEAD) ------------------------------------------- */

/* aesm_gcm_encrypt — write the ciphertext (malloc'd) and the 16-byte tag. */
AesmStatus aesm_gcm_encrypt(const uint8_t *plaintext, size_t pt_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, const uint8_t *aad,
                            size_t aad_len, uint8_t **out_ct, size_t *out_ct_len,
                            uint8_t tag[16]);
/* aesm_gcm_decrypt — verify `tag`, then write the plaintext (malloc'd).
 * Returns AESM_AUTH_FAILED (and writes nothing) on a tag mismatch. */
AesmStatus aesm_gcm_decrypt(const uint8_t *ciphertext, size_t ct_len,
                            const uint8_t *key, size_t key_len,
                            const uint8_t *iv, size_t iv_len, const uint8_t *aad,
                            size_t aad_len, const uint8_t tag[16],
                            uint8_t **out_pt, size_t *out_pt_len);

#endif /* AES_MODES_H */
