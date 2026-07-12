/*
 * argon2d.h — Argon2d, data-dependent memory-hard password hashing (RFC 9106),
 * in pure ISO C17. A faithful port of the Rust `argon2d` crate.
 * ===========================================================================
 *
 * Argon2 is the winner of the Password Hashing Competition. It fills a large
 * memory matrix (`memory_cost` KiB) with the BLAKE2b-derived compression
 * function, reading it back in a data-dependent order so that an attacker
 * cannot trade memory for speed. The *d* variant derives every reference index
 * from the previous block's first 64 bits — maximal GPU/ASIC resistance, but a
 * timing side channel, so Argon2d suits only threat models without side-channel
 * attackers (e.g. proof-of-work). Prefer Argon2id for password hashing.
 *
 *   H0        = BLAKE2b(params || pass || salt || key || ad)
 *   B[i][0/1] = H'(H0 || 0/1 || i)                         (first two columns)
 *   B[i][j]   = G(B[i][j-1], B[l'][z'])                    (fill; XOR after pass 0)
 *   tag       = H'(XOR of the last column across lanes)
 *
 * where G is the Argon2 compression, H' the variable-length BLAKE2b extender,
 * and (l', z') the data-dependent reference block. Built on the sibling
 * `blake2b` package.
 *
 * The tag is written into a caller-provided buffer of `tag_length` bytes.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef ARGON2D_H
#define ARGON2D_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t */

/* The only supported Argon2 version (v1.3). */
#define ARGON2D_VERSION 0x13u

typedef enum {
    ARGON2D_OK = 0,
    ARGON2D_PASSWORD_TOO_LONG,     /* password length exceeds 2^32-1 */
    ARGON2D_SALT_TOO_SHORT,        /* salt shorter than 8 bytes */
    ARGON2D_SALT_TOO_LONG,         /* salt length exceeds 2^32-1 */
    ARGON2D_KEY_TOO_LONG,          /* key length exceeds 2^32-1 */
    ARGON2D_AD_TOO_LONG,           /* associated data exceeds 2^32-1 */
    ARGON2D_TAG_TOO_SMALL,         /* tag_length < 4 */
    ARGON2D_INVALID_PARALLELISM,   /* parallelism not in [1, 2^24-1] */
    ARGON2D_MEMORY_TOO_SMALL,      /* memory_cost < 8*parallelism */
    ARGON2D_TIME_COST_ZERO,        /* time_cost < 1 */
    ARGON2D_UNSUPPORTED_VERSION,   /* version != 0x13 */
    ARGON2D_ALLOC_ERROR,           /* out of memory for the working matrix */
    ARGON2D_BAD_ARGS               /* NULL output buffer */
} Argon2dStatus;

/* Optional inputs. Any pointer may be NULL (with the matching length 0). If
 * `version` is 0 the default (0x13) is used; any other value must equal 0x13. */
typedef struct {
    const uint8_t *key;
    size_t key_len;
    const uint8_t *associated_data;
    size_t ad_len;
    uint32_t version;
} Argon2dOptions;

/* argon2d — compute the Argon2d tag. `out` must have room for `tag_length`
 * bytes. `opts` may be NULL (no key / no AD / default version). Returns
 * ARGON2D_OK on success, or a status describing the invalid parameter. */
Argon2dStatus argon2d(const uint8_t *password, size_t password_len,
                      const uint8_t *salt, size_t salt_len, uint32_t time_cost,
                      uint32_t memory_cost, uint32_t parallelism,
                      uint32_t tag_length, const Argon2dOptions *opts,
                      uint8_t *out);

#endif /* ARGON2D_H */
