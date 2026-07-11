# sha512 (C)

The **SHA-512** hash (FIPS 180-4), in pure ISO C17. A faithful port of the Rust
`sha512` crate. One-shot and streaming; output verified against the published
FIPS test vectors.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "sha512.h"

char hex[SHA512_HEX_SIZE];
sha512_hex("abc", 3, hex);        /* "ddaf35a1...a54ca49f" (128 hex chars) */

uint8_t digest[SHA512_DIGEST_SIZE];
sha512_ctx ctx;
sha512_init(&ctx);
sha512_update(&ctx, "ab", 2);
sha512_update(&ctx, "c", 1);
sha512_final(&ctx, digest);
```

Fixed-size buffers — no heap allocation, nothing to free.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/sha512`. See also the [C++ port](../../cpp/sha512/README.md).
