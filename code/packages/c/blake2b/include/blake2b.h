/*
 * blake2b.h — the BLAKE2b hash (RFC 7693), in pure ISO C17. A faithful port of
 * the Rust `blake2b` crate.
 * ===========================================================================
 *
 * BLAKE2b is a fast cryptographic hash producing a digest of any size up to 64
 * bytes. It supports optional keying (turning it into a MAC), plus a 16-byte
 * salt and 16-byte personalization for domain separation — all folded into a
 * parameter block that seeds the state. It works in 64-bit words, 128-byte
 * blocks, and 12 rounds of the G mixing function. Output matches the published
 * RFC 7693 test vectors.
 *
 *   • one-shot: blake2b(data, len, digest_size, out) / blake2b_hex(...)
 *   • streaming: blake2b_init → blake2b_update → blake2b_final
 *
 * All buffers are fixed-size — no heap allocation, nothing to free.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef BLAKE2B_H
#define BLAKE2B_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint64_t */

#define BLAKE2B_BLOCK_SIZE 128
#define BLAKE2B_MAX_DIGEST 64
#define BLAKE2B_MAX_KEY 64

typedef struct {
    uint64_t state[8];
    uint8_t buffer[128];
    size_t buffer_len;
    uint64_t count_low;  /* bytes compressed in non-final blocks (low 64) */
    uint64_t count_high; /* high 64 of the 128-bit counter */
    size_t digest_size;
} blake2b_ctx;

/* blake2b_init — start a hash with digest size `digest_size` (1..64), an
 * optional key (`key`/`key_len`, or NULL/0 for unkeyed; up to 64 bytes), and an
 * optional 16-byte `salt` and 16-byte `personal` (each NULL to omit).
 * Returns 1 on success, or 0 if a parameter is out of range. */
int blake2b_init(blake2b_ctx *ctx, size_t digest_size, const uint8_t *key,
                 size_t key_len, const uint8_t *salt, const uint8_t *personal);

/* blake2b_update — feed `len` bytes of `data` into the hash. */
void blake2b_update(blake2b_ctx *ctx, const void *data, size_t len);

/* blake2b_final — finish and write `digest_size` bytes to `out`. */
void blake2b_final(blake2b_ctx *ctx, uint8_t *out);

/* blake2b — one-shot unkeyed hash of `len` bytes into `out` (`digest_size`
 * bytes). Returns 1, or 0 if `digest_size` is out of range. */
int blake2b(const void *data, size_t len, size_t digest_size, uint8_t *out);

/* blake2b_hex — one-shot unkeyed hash, lowercase hex into `out` (which must hold
 * 2*digest_size + 1 chars). Returns 1, or 0 on a bad digest size. */
int blake2b_hex(const void *data, size_t len, size_t digest_size, char *out);

#endif /* BLAKE2B_H */
