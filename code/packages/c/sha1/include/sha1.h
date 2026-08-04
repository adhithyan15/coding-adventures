/*
 * sha1.h — the SHA-1 hash (FIPS 180-4), in pure ISO C17. A faithful port of the
 * Rust `sha1` crate.
 * ===========================================================================
 *
 * SHA-1 maps any byte string to a fixed 20-byte (160-bit) digest. It processes
 * the input in 512-bit blocks, maintaining five 32-bit words through 80 rounds.
 * (SHA-1 is broken for collision resistance and MUST NOT be used for security;
 * it remains useful for checksums, Git object IDs, and interop.) Output matches
 * the published FIPS test vectors.
 *
 *   • one-shot: sha1(data, len, out) / sha1_hex(data, len, out)
 *   • streaming: sha1_init → sha1_update (repeatedly) → sha1_final
 *
 * All buffers are fixed-size — no heap allocation, nothing to free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef SHA1_H
#define SHA1_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

#define SHA1_DIGEST_SIZE 20
#define SHA1_HEX_SIZE 41 /* 40 hex chars + NUL */

typedef struct {
    uint32_t state[5];
    uint64_t bit_length;
    uint8_t buffer[64];
    size_t buffer_len;
} sha1_ctx;

/* sha1_init — begin a new streaming hash. */
void sha1_init(sha1_ctx *ctx);

/* sha1_update — feed `len` bytes of `data` into the hash. */
void sha1_update(sha1_ctx *ctx, const void *data, size_t len);

/* sha1_final — finish and write the 20-byte digest to `out`. */
void sha1_final(sha1_ctx *ctx, uint8_t out[SHA1_DIGEST_SIZE]);

/* sha1 — one-shot: hash `len` bytes of `data` into the 20-byte `out`. */
void sha1(const void *data, size_t len, uint8_t out[SHA1_DIGEST_SIZE]);

/* sha1_hex — one-shot: hash `data`, write the lowercase hex digest (40 chars +
 * NUL) into `out`. */
void sha1_hex(const void *data, size_t len, char out[SHA1_HEX_SIZE]);

#endif /* SHA1_H */
