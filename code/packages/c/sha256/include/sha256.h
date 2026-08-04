/*
 * sha256.h — the SHA-256 cryptographic hash (FIPS 180-4), in pure ISO C17. A
 * faithful port of the Rust `sha256` crate.
 * ===========================================================================
 *
 * SHA-256 maps any byte string to a fixed 32-byte (256-bit) digest. It processes
 * the input in 512-bit blocks, maintaining eight 32-bit working words; each
 * block runs 64 rounds mixing a message schedule with round constants. This is
 * the standard algorithm — any correct implementation produces identical output,
 * so the tests pin it to the published FIPS test vectors.
 *
 * Two ways to use it:
 *   • one-shot: sha256(data, len, out) / sha256_hex(data, len, out)
 *   • streaming: sha256_init → sha256_update (repeatedly) → sha256_final
 *
 * Everything uses fixed-size buffers — there is no heap allocation, so nothing
 * to free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef SHA256_H
#define SHA256_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

/* The digest is 32 bytes; the hex string is 64 chars + a NUL. */
#define SHA256_DIGEST_SIZE 32
#define SHA256_HEX_SIZE 65

/* Streaming context. Treat the fields as opaque; drive it with the functions
 * below. */
typedef struct {
    uint32_t state[8];   /* the eight working hash words */
    uint64_t bit_length; /* total message length in bits */
    uint8_t buffer[64];  /* partial 512-bit block */
    size_t buffer_len;   /* bytes currently in `buffer` */
} sha256_ctx;

/* sha256_init — begin a new streaming hash. */
void sha256_init(sha256_ctx *ctx);

/* sha256_update — feed `len` bytes of `data` into the hash. May be called any
 * number of times before sha256_final. */
void sha256_update(sha256_ctx *ctx, const void *data, size_t len);

/* sha256_final — finish the hash and write the 32-byte digest to `out`. The
 * context must not be reused afterward (re-init it first). */
void sha256_final(sha256_ctx *ctx, uint8_t out[SHA256_DIGEST_SIZE]);

/* sha256 — one-shot: hash `len` bytes of `data` into the 32-byte `out`. */
void sha256(const void *data, size_t len, uint8_t out[SHA256_DIGEST_SIZE]);

/* sha256_hex — one-shot: hash `data` and write the lowercase hex digest (64
 * chars + NUL) into `out`. */
void sha256_hex(const void *data, size_t len, char out[SHA256_HEX_SIZE]);

#endif /* SHA256_H */
