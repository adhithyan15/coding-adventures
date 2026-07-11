/*
 * chacha20_poly1305.h — the ChaCha20 stream cipher and the ChaCha20-Poly1305
 * AEAD (RFC 8439), in pure ISO C17. A faithful port of the Rust
 * `chacha20-poly1305` crate.
 * ===========================================================================
 *
 * ChaCha20 is a fast stream cipher: it turns a 256-bit key + 96-bit nonce +
 * 32-bit block counter into a keystream that is XORed with the data. Poly1305 is
 * a one-time authenticator producing a 16-byte tag. Together they form an AEAD
 * (Authenticated Encryption with Associated Data): encryption plus a tag that
 * detects any tampering with the ciphertext or the associated data.
 *
 * Poly1305 needs 130-bit modular arithmetic; since ISO C has no 128-bit integer,
 * this port uses the well-known "poly1305-donna" representation (five 26-bit
 * limbs, products taken in uint64_t). Output matches the RFC 8439 test vectors.
 *
 *   chacha20_encrypt(...)      — raw stream cipher (also decrypts: XOR again)
 *   poly1305_mac(...)          — one-time authenticator
 *   aead_encrypt / aead_decrypt — the full ChaCha20-Poly1305 AEAD
 *
 * Buffers are caller-provided and fixed where possible; the AEAD helpers write
 * ciphertext/plaintext into a caller buffer the same length as the input.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef CHACHA20_POLY1305_H
#define CHACHA20_POLY1305_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t */

/* chacha20_encrypt — XOR `len` bytes of `input` with the ChaCha20 keystream for
 * `key` (32 bytes), `nonce` (12 bytes), starting block `counter`, into `output`
 * (which may alias `input`). Decryption is the same call on the ciphertext. */
void chacha20_encrypt(const uint8_t *input, size_t len, const uint8_t key[32],
                      const uint8_t nonce[12], uint32_t counter,
                      uint8_t *output);

/* poly1305_mac — compute the 16-byte one-time authenticator of `message` under
 * the 32-byte one-time `key`, into `tag`. */
void poly1305_mac(const uint8_t *message, size_t len, const uint8_t key[32],
                  uint8_t tag[16]);

/* aead_encrypt — ChaCha20-Poly1305 AEAD (RFC 8439 §2.8). Encrypts `plaintext`
 * into `ciphertext` (same length) and writes the 16-byte `tag`. `aad` may be
 * NULL when `aad_len` is 0. Returns 1, or 0 on an internal allocation failure. */
int aead_encrypt(const uint8_t *plaintext, size_t plaintext_len,
                 const uint8_t key[32], const uint8_t nonce[12],
                 const uint8_t *aad, size_t aad_len, uint8_t *ciphertext,
                 uint8_t tag[16]);

/* aead_decrypt — verify `tag` (constant-time) and decrypt `ciphertext` into
 * `plaintext` (same length). Returns 1 on success, 0 if the tag is invalid
 * (plaintext is still written but must be discarded) or on allocation failure. */
int aead_decrypt(const uint8_t *ciphertext, size_t ciphertext_len,
                 const uint8_t key[32], const uint8_t nonce[12],
                 const uint8_t *aad, size_t aad_len, const uint8_t tag[16],
                 uint8_t *plaintext);

#endif /* CHACHA20_POLY1305_H */
