# hkdf (C)

**HKDF** — the HMAC-based key derivation function (RFC 5869), in pure ISO C17. A
faithful port of the Rust `hkdf` crate. Hash-agnostic (built on the sibling
`hmac`): `extract` then `expand`.

Compiles and runs under **GCC, Clang, and MSVC** with strict ISO-conformance
flags, via the shared [`iso-harness`](../iso-harness/README.md).

## Usage

```c
#include "hkdf.h"
#include "sha256.h"

uint8_t okm[42];
hkdf(sha256, /*digest*/32, /*block*/64,
     salt, saltlen, ikm, ikmlen, info, infolen, okm, 42);

/* or the two steps separately */
uint8_t prk[32];
hkdf_extract(sha256, 32, 64, salt, saltlen, ikm, ikmlen, prk);
hkdf_expand (sha256, 32, 64, prk, 32, info, infolen, okm, 42);
```

An empty salt is treated as `digest_size` zero bytes. `expand`/`hkdf` return
`HKDF_OUTPUT_TOO_SHORT` (length 0) or `HKDF_OUTPUT_TOO_LONG`
(length > 255·digest).

## Development

```bash
sh BUILD   # compile + run the tests under every C compiler present (strict ISO)
```

Ports `code/packages/rust/hkdf`. Verified against the **RFC 5869** HKDF-SHA256
vectors (the sibling `hmac` + `sha256` packages supply the primitives). See also
the [C++ port](../../cpp/hkdf/README.md).
