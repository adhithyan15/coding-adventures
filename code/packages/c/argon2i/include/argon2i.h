/*
 * argon2i.h — Argon2i, data-independent memory-hard password hashing (RFC 9106),
 * in pure ISO C17. A faithful port of the Rust `argon2i` crate.
 * ===========================================================================
 *
 * Argon2 fills a large memory matrix (`memory_cost` KiB) with a BLAKE2b-derived
 * compression function, reading it back so that an attacker cannot trade memory
 * for speed. The *i* variant picks each reference block from a deterministic
 * pseudo-random stream that does NOT depend on the password or memory contents —
 * a constant memory-access pattern that defeats side-channel observers, at the
 * cost of being the easiest variant to parallelise. Prefer Argon2id for password
 * hashing.
 *
 * The addressing stream (RFC 9106 §3.4.2) generates (J1, J2) pairs by running
 * the compression function twice over a counter block; the reference block index
 * therefore never depends on secret data.
 *
 *   H0        = BLAKE2b(params || pass || salt || key || ad)
 *   B[i][0/1] = H'(H0 || 0/1 || i)
 *   B[i][j]   = G(B[i][j-1], B[l'][z'])   (z' from the address stream)
 *   tag       = H'(XOR of the last column across lanes)
 *
 * Built on the sibling `blake2b` package. The tag is written into a
 * caller-provided buffer of `tag_length` bytes.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef ARGON2I_H
#define ARGON2I_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t */

/* The only supported Argon2 version (v1.3). */
#define ARGON2I_VERSION 0x13u

typedef enum {
    ARGON2I_OK = 0,
    ARGON2I_PASSWORD_TOO_LONG,   /* password length exceeds 2^32-1 */
    ARGON2I_SALT_TOO_SHORT,      /* salt shorter than 8 bytes */
    ARGON2I_SALT_TOO_LONG,       /* salt length exceeds 2^32-1 */
    ARGON2I_KEY_TOO_LONG,        /* key length exceeds 2^32-1 */
    ARGON2I_AD_TOO_LONG,         /* associated data exceeds 2^32-1 */
    ARGON2I_TAG_TOO_SMALL,       /* tag_length < 4 */
    ARGON2I_INVALID_PARALLELISM, /* parallelism not in [1, 2^24-1] */
    ARGON2I_MEMORY_TOO_SMALL,    /* memory_cost < 8*parallelism */
    ARGON2I_TIME_COST_ZERO,      /* time_cost < 1 */
    ARGON2I_UNSUPPORTED_VERSION, /* version != 0x13 */
    ARGON2I_ALLOC_ERROR,         /* out of memory for the working matrix */
    ARGON2I_BAD_ARGS             /* NULL output buffer */
} Argon2iStatus;

/* Optional inputs. Any pointer may be NULL (with the matching length 0). If
 * `version` is 0 the default (0x13) is used; any other value must equal 0x13. */
typedef struct {
    const uint8_t *key;
    size_t key_len;
    const uint8_t *associated_data;
    size_t ad_len;
    uint32_t version;
} Argon2iOptions;

/* argon2i — compute the Argon2i tag. `out` must have room for `tag_length`
 * bytes. `opts` may be NULL (no key / no AD / default version). Returns
 * ARGON2I_OK on success, or a status describing the invalid parameter. */
Argon2iStatus argon2i(const uint8_t *password, size_t password_len,
                      const uint8_t *salt, size_t salt_len, uint32_t time_cost,
                      uint32_t memory_cost, uint32_t parallelism,
                      uint32_t tag_length, const Argon2iOptions *opts,
                      uint8_t *out);

#endif /* ARGON2I_H */
