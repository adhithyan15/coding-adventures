# x25519 (C)

X25519 (Curve25519 ECDH, RFC 7748) in pure ISO C17. A faithful port of the Rust
`x25519` crate.

X25519 is the elliptic-curve Diffie-Hellman function on Curve25519: a
constant-time Montgomery-ladder scalar multiplication over GF(2²⁵⁵−19). It
underpins TLS 1.3, Signal, WireGuard, and SSH key exchange.

## API

```c
#include "x25519.h"

uint8_t alice_pub[32], shared[32];
x25519_base(alice_pub, alice_private);        /* derive a public key */
x25519(shared, alice_private, bob_public);    /* the shared secret */
```

Keys and coordinates are 32-byte little-endian arrays; results are written to a
caller-provided 32-byte buffer. `x25519` returns `-1` (where the Rust returns
`Err`) when the output is all zeros — the signal of a low-order point input,
which a Diffie-Hellman caller must reject.

Field elements use radix-2⁵¹ (five 51-bit limbs); the schoolbook multiply
accumulates 102-bit partial products. The Rust crate uses `u128` for that; this
port carries a **small, contained 128-bit emulation** (a 64×64→128 multiply plus
add/shift), so it needs no `__int128`. Verified against the RFC 7748 test
vectors, including the 1000-iteration stress test.

## Portability

Pure ISO C17 — no `__int128`, no extensions. Compiles clean under GCC, Clang, and
MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the
shared [`iso-harness`](../iso-harness).

## Development

```bash
sh BUILD
```
