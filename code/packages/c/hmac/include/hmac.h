/*
 * hmac.h — HMAC (keyed-hash message authentication, RFC 2104), in pure ISO C17.
 * A faithful port of the Rust `hmac` crate's generic construction.
 * ===========================================================================
 *
 * HMAC turns any cryptographic hash H (with block size B and digest size L)
 * into a keyed authenticator:
 *
 *     K0    = H(key) if len(key) > B, else key, right-padded with zeros to B
 *     HMAC  = H( (K0 XOR opad) || H( (K0 XOR ipad) || message ) )
 *
 * with ipad = 0x36 repeated and opad = 0x5c repeated. This port is
 * hash-AGNOSTIC: you pass a one-shot hash function plus its block and digest
 * sizes, so it works with SHA-256 (B=64, L=32), SHA-1, MD5, SHA-512 (B=128,
 * L=64), etc. The tests instantiate it with the sibling `sha256` package and the
 * published RFC 4231 HMAC-SHA256 vectors.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef HMAC_H
#define HMAC_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

/* A one-shot hash: hash `len` bytes of `data` into `out` (digest_size bytes). */
typedef void (*hmac_hash_fn)(const void *data, size_t len, uint8_t *out);

/* hmac_compute — write the `digest_size`-byte HMAC of `msg` under `key` (using
 * hash `hash` with the given block/digest sizes) into `out`.
 * Returns 1 on success, or 0 on allocation failure (or an implausible size). */
int hmac_compute(hmac_hash_fn hash, size_t digest_size, size_t block_size,
                 const uint8_t *key, size_t keylen, const uint8_t *msg,
                 size_t msglen, uint8_t *out);

/* hmac_verify — constant-time equality of two byte buffers of length `len`
 * (does not short-circuit on the first differing byte, to avoid timing leaks).
 * Returns 1 if equal, else 0. */
int hmac_verify(const uint8_t *a, const uint8_t *b, size_t len);

#endif /* HMAC_H */
