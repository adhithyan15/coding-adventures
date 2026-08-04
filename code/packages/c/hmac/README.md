# hmac (C)

**HMAC** — keyed-hash message authentication (RFC 2104), in pure ISO C17. A
faithful port of the Rust `hmac` crate's generic construction. Hash-agnostic:
pass any one-shot hash plus its block/digest sizes.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "hmac.h"
#include "sha256.h"   /* or any hash with a one-shot `hash(data, len, out)` */

uint8_t mac[32];
hmac_compute(sha256, /*digest*/32, /*block*/64, key, keylen, msg, msglen, mac);

/* constant-time comparison (no early-out, avoids timing leaks) */
int ok = hmac_verify(mac, expected, 32);
```

`hmac_compute` returns 0 only on allocation failure. `hmac_verify` compares in
constant time.

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/hmac`. Verified against the **RFC 4231 HMAC-SHA256**
vectors (the sibling `sha256` package supplies the hash in the tests). See also
the [C++ port](../../cpp/hmac/README.md).
