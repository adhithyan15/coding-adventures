# sha256 (C)

The **SHA-256** cryptographic hash (FIPS 180-4), in pure ISO C17. A faithful
port of the Rust `sha256` crate. One-shot and streaming; output verified against
the published FIPS test vectors.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "sha256.h"

/* one-shot */
char hex[SHA256_HEX_SIZE];
sha256_hex("abc", 3, hex);   /* "ba7816bf...20015ad" */

uint8_t digest[SHA256_DIGEST_SIZE];
sha256("abc", 3, digest);    /* 32 raw bytes */

/* streaming */
sha256_ctx ctx;
sha256_init(&ctx);
sha256_update(&ctx, "ab", 2);
sha256_update(&ctx, "c", 1);
sha256_final(&ctx, digest);
```

All buffers are fixed-size — there is no heap allocation and nothing to free.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/sha256`. Standard algorithm; identical output to the
crate. See also the [C++ port](../../cpp/sha256/README.md).
