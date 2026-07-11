# sha256-c

C ABI wrapper for the `coding_adventures_sha256` crate. Compiles to a static
library (`libsha256_c.a`) and a dynamic library, exposing SHA-256 over a stable
C ABI for Swift, C, and C++ callers (see `swift/sha256-native`).

See `code/packages/swift/sha256-native/Sources/CSha256/include/sha256_c.h` for
the contract:

```c
void  sha256_c_digest(const uint8_t* data, size_t len, uint8_t* out32);
HASHER* sha256_c_hasher_new(void);
void    sha256_c_hasher_update(HASHER*, const uint8_t* data, size_t len);
void    sha256_c_hasher_digest(const HASHER*, uint8_t* out32);
HASHER* sha256_c_hasher_clone(const HASHER*);
void    sha256_c_hasher_free(HASHER*);
```

Digests are written into a caller-owned 32-byte buffer, so no allocation crosses
the boundary on the one-shot path; the streaming hasher is an opaque handle.
