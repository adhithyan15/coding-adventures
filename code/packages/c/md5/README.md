# md5 (C)

The **MD5** hash (RFC 1321), in pure ISO C17. A faithful port of the Rust `md5`
crate. One-shot and streaming; output verified against the RFC 1321 test suite.

> ⚠️ MD5 is broken for collision resistance — do not use it for security. It
> remains useful for checksums and interop.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "md5.h"

char hex[MD5_HEX_SIZE];
md5_hex("abc", 3, hex);         /* "900150983cd24fb0d6963f7d28e17f72" */

uint8_t digest[MD5_DIGEST_SIZE];
md5_ctx ctx;
md5_init(&ctx);
md5_update(&ctx, "ab", 2);
md5_update(&ctx, "c", 1);
md5_final(&ctx, digest);
```

Fixed-size buffers — no heap allocation, nothing to free. MD5 is little-endian
(both message words and output).

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/md5`. See also the [C++ port](../../cpp/md5/README.md).
