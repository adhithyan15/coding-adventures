# blake2b (C)

The **BLAKE2b** hash (RFC 7693), in pure ISO C17. A faithful port of the Rust
`blake2b` crate. Digest size up to 64 bytes, optional keying (MAC), 16-byte salt
and personalization. One-shot and streaming; output verified against the
published RFC 7693 test vectors.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "blake2b.h"

char hex[2 * BLAKE2B_MAX_DIGEST + 1];
blake2b_hex("abc", 3, 64, hex);       /* BLAKE2b-512 */

uint8_t digest[BLAKE2B_MAX_DIGEST];
blake2b_ctx ctx;
blake2b_init(&ctx, 32, key, key_len, salt16, personal16); /* keyed/salted */
blake2b_update(&ctx, "ab", 2);
blake2b_update(&ctx, "c", 1);
blake2b_final(&ctx, digest);
```

Fixed-size buffers — no heap allocation, nothing to free. Pass NULL for an unused
key/salt/personal.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/blake2b`. See also the [C++ port](../../cpp/blake2b/README.md).
