/*
 * des.h — the DES block cipher (FIPS 46) and Triple DES (NIST SP 800-67), in
 * pure ISO C17. A faithful port of the Rust `des` crate.
 * ===========================================================================
 *
 * DES encrypts a 64-bit block under a 64-bit key (56 bits of key material + 8
 * parity bits) through a 16-round Feistel network. It is long retired for real
 * use — 56-bit keys are brute-forceable — but it is the archetypal block cipher
 * and its structure (initial/final permutations, an expansion permutation, eight
 * S-boxes, a key schedule of rotations and compression permutations) underpins
 * everything since.
 *
 *   des_expand_key            — derive the 16 round subkeys
 *   des_encrypt_block / des_decrypt_block — the raw 8-byte block cipher
 *   des_ecb_encrypt / des_ecb_decrypt     — ECB mode with PKCS#7 padding
 *   des_tdea_encrypt_block / des_tdea_decrypt_block — Triple DES (EDE)
 *
 * Output matches the FIPS 46 and NIST SP 800-20 known-answer test vectors.
 *
 * Security note: DES and 3DES are cryptographically broken for modern use (56-
 * and effective-112-bit keys; SWEET32 on the 64-bit block). This is a faithful,
 * historically faithful implementation for study and legacy interop, not a
 * recommendation.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef DES_H
#define DES_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* des_expand_key — derive the 16 round subkeys (each 48 bits = 6 bytes) from an
 * 8-byte key. Parity bits are ignored (dropped by PC-1). */
void des_expand_key(const uint8_t key[8], uint8_t subkeys[16][6]);

/* des_encrypt_block — encrypt one 8-byte block under `key` into `out` (which may
 * alias `block`). */
void des_encrypt_block(const uint8_t block[8], const uint8_t key[8],
                       uint8_t out[8]);

/* des_decrypt_block — decrypt one 8-byte block (encryption with the subkeys in
 * reverse order). */
void des_decrypt_block(const uint8_t block[8], const uint8_t key[8],
                       uint8_t out[8]);

/* des_ecb_encrypt — encrypt `len` bytes in ECB mode with PKCS#7 padding.
 * Returns a newly malloc'd buffer of `*out_len` bytes (a multiple of 8), or NULL
 * on allocation failure. Caller frees it. NOTE: ECB leaks plaintext patterns —
 * educational only. */
uint8_t *des_ecb_encrypt(const uint8_t *plaintext, size_t len,
                         const uint8_t key[8], size_t *out_len);

/* des_ecb_decrypt — decrypt an ECB ciphertext (a non-empty multiple of 8 bytes)
 * and strip PKCS#7 padding. On success sets *out (malloc'd, caller frees) and
 * *out_len and returns 1; returns 0 on a bad length, bad padding, or allocation
 * failure. */
int des_ecb_decrypt(const uint8_t *ciphertext, size_t len, const uint8_t key[8],
                    uint8_t **out, size_t *out_len);

/* des_tdea_encrypt_block — Triple DES EDE: C = E_k1(D_k2(E_k3(P))). */
void des_tdea_encrypt_block(const uint8_t block[8], const uint8_t k1[8],
                            const uint8_t k2[8], const uint8_t k3[8],
                            uint8_t out[8]);

/* des_tdea_decrypt_block — Triple DES DED: P = D_k3(E_k2(D_k1(C))). */
void des_tdea_decrypt_block(const uint8_t block[8], const uint8_t k1[8],
                            const uint8_t k2[8], const uint8_t k3[8],
                            uint8_t out[8]);

#endif /* DES_H */
