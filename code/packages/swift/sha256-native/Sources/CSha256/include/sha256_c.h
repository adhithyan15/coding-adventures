/*
 * sha256_c.h — C declarations for the Rust sha256-c static library.
 *
 * The public contract between the Rust implementation (crate `sha256-c`) and
 * any C, C++, or Swift caller. Import via a module map to compute SHA-256.
 *
 * The 32-byte digest is written into a CALLER-OWNED buffer, so no allocation
 * crosses the boundary on the one-shot path. The streaming hasher is an opaque
 * handle you must free with sha256_c_hasher_free.
 */
#ifndef SHA256_C_H
#define SHA256_C_H

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Opaque streaming-hasher handle. */
typedef struct Sha256Hasher Sha256Hasher;

/* Write the 32-byte SHA-256 digest of data[0..len] into out (>= 32 bytes). */
void sha256_c_digest(const uint8_t *data, size_t len, uint8_t *out32);

/* Allocate a new streaming hasher. Free it with sha256_c_hasher_free. */
Sha256Hasher *sha256_c_hasher_new(void);

/* Feed data[0..len] into the hasher. */
void sha256_c_hasher_update(Sha256Hasher *h, const uint8_t *data, size_t len);

/* Write the current 32-byte digest into out32 (non-destructive). */
void sha256_c_hasher_digest(const Sha256Hasher *h, uint8_t *out32);

/* Return an independent copy of the hasher (free it separately). */
Sha256Hasher *sha256_c_hasher_clone(const Sha256Hasher *h);

/* Free a hasher handle. Passing NULL is a no-op. */
void sha256_c_hasher_free(Sha256Hasher *h);

#ifdef __cplusplus
}
#endif

#endif /* SHA256_C_H */
