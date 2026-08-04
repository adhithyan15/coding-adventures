# zeroize (C)

**Secure in-memory wiping for secrets**, in pure ISO C17 — a faithful port of
the Rust `zeroize` crate.

## The problem it solves

When you clear a secret (a key, password, token) by writing zeros, an optimizing
compiler is allowed to **delete the clear**: nothing reads the buffer
afterward, so the store is "dead". The secret then lingers in RAM, exposed to a
later swap-to-disk, core dump, or use-after-free elsewhere.

The fix is a **`volatile` store** — the C standard classifies a volatile access
as *observable behavior* the implementation may not optimize away.
`zeroize_bytes` writes zeros through a `volatile unsigned char *`, the canonical
portable secure-zero.

> Verified: at `-O3`, a plain `memset` into a soon-dead buffer is compiled to
> **zero instructions** (eliminated), while this library's volatile loop emits
> the full run of store-zero instructions — the clear provably happens.

## API

```c
#include "zeroize.h"

uint8_t key[32] = { /* ... secret ... */ };
zeroize_bytes(key, sizeof key);        /* wiped, not elided */

uint64_t token = 0xDEADBEEFCAFEF00D;
zeroize_u64(&token);                   /* typed convenience */

/* A growable buffer that scrubs its full capacity (like Rust's Vec impl): */
ZrBytes b;
zr_bytes_init(&b);
zr_bytes_extend(&b, secret, secret_len);
zr_bytes_zeroize(&b);                  /* scrub all capacity, len = 0 */
zr_bytes_free(&b);
```

## Divergence from the Rust crate

Rust pairs `write_volatile` with a `compiler_fence`; this C port relies on
volatile stores alone (the load-bearing defense against dead-store elimination),
omitting an explicit fence for maximal MSVC portability. Rust's `Zeroizing<T>`
RAII wrapper and `Option` impl are language features with no C analogue (they are
provided by the **C++** port); in C, call `zeroize_bytes` before `free`. 128-bit
integers are omitted (pure ISO C has none) — wipe those via
`zeroize_bytes(&x, sizeof x)`.

## Building

```sh
sh BUILD    # builds & runs the tests under every C compiler present
```

Pure ISO C17. Builds clean under GCC, Clang, and MSVC with `-pedantic-errors` /
`/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../iso-harness); the test suite also runs clean under
AddressSanitizer + UndefinedBehaviorSanitizer.
