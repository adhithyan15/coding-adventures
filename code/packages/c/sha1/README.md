# sha1 (C)

The **SHA-1** hash (FIPS 180-4), in pure ISO C17. A faithful port of the Rust
`sha1` crate. One-shot and streaming; output verified against the published FIPS
test vectors.

> ⚠️ SHA-1 is broken for collision resistance — do not use it for security. It
> remains useful for checksums, Git object IDs, and interop.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "sha1.h"

char hex[SHA1_HEX_SIZE];
sha1_hex("abc", 3, hex);        /* "a9993e36...9cd0d89d" */

uint8_t digest[SHA1_DIGEST_SIZE];
sha1_ctx ctx;
sha1_init(&ctx);
sha1_update(&ctx, "ab", 2);
sha1_update(&ctx, "c", 1);
sha1_final(&ctx, digest);
```

Fixed-size buffers — no heap allocation, nothing to free.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/sha1`. See also the [C++ port](../../cpp/sha1/README.md).
