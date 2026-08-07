/*
 * aes.h — the AES block cipher (FIPS 197), in pure ISO C17. A faithful port of
 * the Rust `aes` crate.
 * ===========================================================================
 *
 * AES (the Advanced Encryption Standard) encrypts a 128-bit block under a 128-,
 * 192-, or 256-bit key. Each round applies four steps to a 4x4 byte "state":
 *   SubBytes    — a non-linear byte substitution (the S-box)
 *   ShiftRows   — cyclically shift each row
 *   MixColumns  — mix each column via GF(2^8) matrix multiplication
 *   AddRoundKey — XOR in the round key
 * with 10, 12, or 14 rounds for the three key sizes (the last round omits
 * MixColumns).
 *
 * The S-box is built from the multiplicative inverse in GF(2^8) (AES polynomial
 * 0x11B) plus an affine transform — computed here via the sibling `gf256`
 * package, exactly as the Rust crate uses its `gf256::Field`.
 *
 *   aes_encrypt_block / aes_decrypt_block — the raw 16-byte block cipher
 *   aes_expand_key                        — the key schedule (round keys)
 *   aes_sbox / aes_inv_sbox               — the S-box tables
 *
 * Output matches the FIPS 197 known-answer test vectors (Appendices B and C).
 *
 * Note: the lazy S-box build is single-threaded (the Rust crate uses OnceLock;
 * pure ISO C has no portable one-time-init primitive).
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors. No extensions.
 */
#ifndef AES_H
#define AES_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* aes_encrypt_block — encrypt one 16-byte `block` under `key` (16, 24, or 32
 * bytes) into `out`. Returns 1 on success, 0 if the key length is invalid. */
int aes_encrypt_block(const uint8_t block[16], const uint8_t *key,
                      size_t key_len, uint8_t out[16]);

/* aes_decrypt_block — decrypt one 16-byte `block`. Returns 1, or 0 on a bad key
 * length. */
int aes_decrypt_block(const uint8_t block[16], const uint8_t *key,
                      size_t key_len, uint8_t out[16]);

/* aes_expand_key — derive the round keys. Writes (Nr+1) round keys (each a 4x4
 * state) into `round_keys` (capacity 15, enough for AES-256's 15 keys) and the
 * round count Nr into *nr_out. Returns 1, or 0 on a bad key length. */
int aes_expand_key(const uint8_t *key, size_t key_len,
                   uint8_t round_keys[15][4][4], int *nr_out);

/* aes_sbox / aes_inv_sbox — the 256-byte S-box and its inverse (built on first
 * use). Handy for inspection and testing. */
const uint8_t *aes_sbox(void);
const uint8_t *aes_inv_sbox(void);

#endif /* AES_H */
