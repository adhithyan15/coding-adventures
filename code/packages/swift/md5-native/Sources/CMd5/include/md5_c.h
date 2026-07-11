/*
 * md5_c.h — C declarations for the Rust md5-c static library.
 * The 16-byte digest is written into a caller-owned buffer. The streaming
 * hasher is an opaque handle freed with md5_c_hasher_free. MD5 is broken —
 * checksum use only.
 */
#ifndef MD5_C_H
#define MD5_C_H
#include <stdint.h>
#include <stddef.h>
#ifdef __cplusplus
extern "C" {
#endif
typedef struct Md5Digest Md5Digest;
void md5_c_digest(const uint8_t *data, size_t len, uint8_t *out16);
Md5Digest *md5_c_hasher_new(void);
void md5_c_hasher_update(Md5Digest *h, const uint8_t *data, size_t len);
void md5_c_hasher_digest(const Md5Digest *h, uint8_t *out16);
Md5Digest *md5_c_hasher_clone(const Md5Digest *h);
void md5_c_hasher_free(Md5Digest *h);
#ifdef __cplusplus
}
#endif
#endif /* MD5_C_H */
