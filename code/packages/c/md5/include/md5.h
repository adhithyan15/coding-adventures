/*
 * md5.h — the MD5 hash (RFC 1321), in pure ISO C17. A faithful port of the Rust
 * `md5` crate.
 * ===========================================================================
 *
 * MD5 maps any byte string to a fixed 16-byte (128-bit) digest, processing
 * 512-bit blocks through 64 rounds over four 32-bit words. Unlike SHA, MD5 is
 * LITTLE-endian (both the message words and the output). (MD5 is broken for
 * collision resistance and MUST NOT be used for security; it remains useful for
 * checksums and interop.) Output matches the RFC 1321 test suite.
 *
 *   • one-shot: md5(data, len, out) / md5_hex(data, len, out)
 *   • streaming: md5_init → md5_update (repeatedly) → md5_final
 *
 * All buffers are fixed-size — no heap allocation, nothing to free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef MD5_H
#define MD5_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t, uint64_t */

#define MD5_DIGEST_SIZE 16
#define MD5_HEX_SIZE 33 /* 32 hex chars + NUL */

typedef struct {
    uint32_t state[4];
    uint64_t bit_length;
    uint8_t buffer[64];
    size_t buffer_len;
} md5_ctx;

/* md5_init — begin a new streaming hash. */
void md5_init(md5_ctx *ctx);

/* md5_update — feed `len` bytes of `data` into the hash. */
void md5_update(md5_ctx *ctx, const void *data, size_t len);

/* md5_final — finish and write the 16-byte digest to `out`. */
void md5_final(md5_ctx *ctx, uint8_t out[MD5_DIGEST_SIZE]);

/* md5 — one-shot: hash `len` bytes of `data` into the 16-byte `out`. */
void md5(const void *data, size_t len, uint8_t out[MD5_DIGEST_SIZE]);

/* md5_hex — one-shot: hash `data`, write the lowercase hex digest (32 chars +
 * NUL) into `out`. */
void md5_hex(const void *data, size_t len, char out[MD5_HEX_SIZE]);

#endif /* MD5_H */
