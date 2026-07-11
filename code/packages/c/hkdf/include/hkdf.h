/*
 * hkdf.h — HKDF, the HMAC-based key derivation function (RFC 5869), in pure ISO
 * C17. A faithful port of the Rust `hkdf` crate.
 * ===========================================================================
 *
 * HKDF turns input keying material (IKM) into any number of pseudorandom output
 * bytes, in two steps:
 *
 *     extract:  PRK = HMAC-Hash(salt, IKM)            (salt defaults to zeros)
 *     expand:   OKM = T(1) || T(2) || …               (truncated to `length`)
 *               T(i) = HMAC-Hash(PRK, T(i-1) || info || i)
 *
 * It is hash-AGNOSTIC: pass a one-shot hash and its block/digest sizes (as with
 * the sibling `hmac`). The tests use SHA-256 and the RFC 5869 vectors.
 *
 * Portability: pure ISO C17. Compiles clean under GCC, Clang, and MSVC with
 * -pedantic-errors / /permissive- and warnings-as-errors. No extensions.
 */
#ifndef HKDF_H
#define HKDF_H

#include <stddef.h> /* size_t */
#include <stdint.h> /* uint8_t */

#include "hmac.h" /* hmac_hash_fn, hmac_compute — the extract/expand primitive */

/* Status codes. HKDF_OK is 0; every error is negative. */
typedef enum {
    HKDF_OK = 0,
    HKDF_OUTPUT_TOO_SHORT = -1, /* length == 0 */
    HKDF_OUTPUT_TOO_LONG = -2,  /* length > 255 * digest_size */
    HKDF_ALLOC_FAILED = -3
} hkdf_status;

/* hkdf_extract — PRK = HMAC-Hash(salt, IKM), written to `prk_out`
 * (`digest_size` bytes). An empty salt (saltlen == 0) is treated as a string of
 * `digest_size` zero bytes, per RFC 5869. Returns HKDF_OK or HKDF_ALLOC_FAILED. */
hkdf_status hkdf_extract(hmac_hash_fn hash, size_t digest_size,
                         size_t block_size, const uint8_t *salt, size_t saltlen,
                         const uint8_t *ikm, size_t ikmlen, uint8_t *prk_out);

/* hkdf_expand — expand `prk` to `length` output bytes in `out`, mixing in the
 * optional `info` context. Returns HKDF_OK, HKDF_OUTPUT_TOO_SHORT (length 0),
 * HKDF_OUTPUT_TOO_LONG (length > 255*digest_size), or HKDF_ALLOC_FAILED. */
hkdf_status hkdf_expand(hmac_hash_fn hash, size_t digest_size, size_t block_size,
                        const uint8_t *prk, size_t prklen, const uint8_t *info,
                        size_t infolen, uint8_t *out, size_t length);

/* hkdf — the full extract-then-expand, writing `length` bytes to `out`. */
hkdf_status hkdf(hmac_hash_fn hash, size_t digest_size, size_t block_size,
                 const uint8_t *salt, size_t saltlen, const uint8_t *ikm,
                 size_t ikmlen, const uint8_t *info, size_t infolen,
                 uint8_t *out, size_t length);

#endif /* HKDF_H */
