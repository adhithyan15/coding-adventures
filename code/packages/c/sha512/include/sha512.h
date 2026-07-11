/*
 * sha512.h — the SHA-512 hash (FIPS 180-4), in pure ISO C17. A faithful port of
 * the Rust `sha512` crate.
 * ===========================================================================
 *
 * SHA-512 maps any byte string to a fixed 64-byte (512-bit) digest. It is
 * structurally like SHA-256 but works in 64-bit words: eight of them, 1024-bit
 * (128-byte) blocks, 80 rounds, and a 128-bit length field. Output matches the
 * published FIPS test vectors.
 *
 *   • one-shot: sha512(data, len, out) / sha512_hex(data, len, out)
 *   • streaming: sha512_init → sha512_update (repeatedly) → sha512_final
 *
 * All buffers are fixed-size — no heap allocation, nothing to free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef SHA512_H
#define SHA512_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint64_t */

#define SHA512_DIGEST_SIZE 64
#define SHA512_HEX_SIZE 129 /* 128 hex chars + NUL */

typedef struct {
    uint64_t state[8];
    uint64_t length_low;  /* total message length in bits (low 64) */
    uint64_t length_high; /* high 64 bits of the 128-bit length */
    uint8_t buffer[128];
    size_t buffer_len;
} sha512_ctx;

/* sha512_init — begin a new streaming hash. */
void sha512_init(sha512_ctx *ctx);

/* sha512_update — feed `len` bytes of `data` into the hash. */
void sha512_update(sha512_ctx *ctx, const void *data, size_t len);

/* sha512_final — finish and write the 64-byte digest to `out`. */
void sha512_final(sha512_ctx *ctx, uint8_t out[SHA512_DIGEST_SIZE]);

/* sha512 — one-shot: hash `len` bytes of `data` into the 64-byte `out`. */
void sha512(const void *data, size_t len, uint8_t out[SHA512_DIGEST_SIZE]);

/* sha512_hex — one-shot: hash `data`, write the lowercase hex digest (128 chars
 * + NUL) into `out`. */
void sha512_hex(const void *data, size_t len, char out[SHA512_HEX_SIZE]);

#endif /* SHA512_H */
