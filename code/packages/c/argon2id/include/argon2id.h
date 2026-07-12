/*
 * argon2id.h — Argon2id, hybrid memory-hard password hashing (RFC 9106), in pure
 * ISO C17. A faithful port of the Rust `argon2id` crate.
 * ===========================================================================
 *
 * Argon2 fills a large memory matrix (`memory_cost` KiB) with a BLAKE2b-derived
 * compression function, reading it back so that an attacker cannot trade memory
 * for speed. The *id* variant combines Argon2i and Argon2d: the first two slices
 * of the first pass use data-INDEPENDENT addressing (an address stream), and
 * everything after uses data-DEPENDENT addressing (the previous block). This
 * blends some side-channel resistance in the early passes with the GPU/ASIC
 * resistance of the data-dependent mode — the RECOMMENDED variant for password
 * hashing (RFC 9106 §4).
 *
 * The address stream (RFC 9106 §3.4.2) generates (J1, J2) pairs by running the
 * compression function twice over a counter block.
 *
 *   H0        = BLAKE2b(params || pass || salt || key || ad)
 *   B[i][0/1] = H'(H0 || 0/1 || i)
 *   B[i][j]   = G(B[i][j-1], B[l'][z'])   (z' data-independent then dependent)
 *   tag       = H'(XOR of the last column across lanes)
 *
 * Built on the sibling `blake2b` package. The tag is written into a
 * caller-provided buffer of `tag_length` bytes.
 *
 * Portability: pure ISO C17 — GCC, Clang, and MSVC with -pedantic-errors /
 * /permissive- and warnings-as-errors.
 */
#ifndef ARGON2ID_H
#define ARGON2ID_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t, uint32_t */

/* The only supported Argon2 version (v1.3). */
#define ARGON2ID_VERSION 0x13u

typedef enum {
    ARGON2ID_OK = 0,
    ARGON2ID_PASSWORD_TOO_LONG,   /* password length exceeds 2^32-1 */
    ARGON2ID_SALT_TOO_SHORT,      /* salt shorter than 8 bytes */
    ARGON2ID_SALT_TOO_LONG,       /* salt length exceeds 2^32-1 */
    ARGON2ID_KEY_TOO_LONG,        /* key length exceeds 2^32-1 */
    ARGON2ID_AD_TOO_LONG,         /* associated data exceeds 2^32-1 */
    ARGON2ID_TAG_TOO_SMALL,       /* tag_length < 4 */
    ARGON2ID_INVALID_PARALLELISM, /* parallelism not in [1, 2^24-1] */
    ARGON2ID_MEMORY_TOO_SMALL,    /* memory_cost < 8*parallelism */
    ARGON2ID_TIME_COST_ZERO,      /* time_cost < 1 */
    ARGON2ID_UNSUPPORTED_VERSION, /* version != 0x13 */
    ARGON2ID_ALLOC_ERROR,         /* out of memory for the working matrix */
    ARGON2ID_BAD_ARGS             /* NULL output buffer */
} Argon2idStatus;

/* Optional inputs. Any pointer may be NULL (with the matching length 0). If
 * `version` is 0 the default (0x13) is used; any other value must equal 0x13. */
typedef struct {
    const uint8_t *key;
    size_t key_len;
    const uint8_t *associated_data;
    size_t ad_len;
    uint32_t version;
} Argon2idOptions;

/* argon2id — compute the Argon2id tag. `out` must have room for `tag_length`
 * bytes. `opts` may be NULL (no key / no AD / default version). Returns
 * ARGON2ID_OK on success, or a status describing the invalid parameter. */
Argon2idStatus argon2id(const uint8_t *password, size_t password_len,
                      const uint8_t *salt, size_t salt_len, uint32_t time_cost,
                      uint32_t memory_cost, uint32_t parallelism,
                      uint32_t tag_length, const Argon2idOptions *opts,
                      uint8_t *out);

#endif /* ARGON2ID_H */
