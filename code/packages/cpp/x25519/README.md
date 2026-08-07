# x25519 (C++)

X25519 (Curve25519 ECDH, RFC 7748) in pure ISO C++17, header-only, in namespace
`ca::x25519`. A faithful port of the Rust `x25519` crate.

A constant-time Montgomery-ladder scalar multiplication over GF(2²⁵⁵−19) — the
elliptic-curve Diffie-Hellman used by TLS 1.3, Signal, WireGuard, and SSH.

## API

```cpp
#include "x25519.hpp"
using ca::x25519::Key;   // std::array<uint8_t, 32>

auto alice_pub = ca::x25519::x25519_base(alice_private);   // std::optional<Key>
auto shared    = ca::x25519::x25519(alice_private, *bob_public);
```

`x25519` returns `std::optional<Key>`, `std::nullopt` (where the Rust returns
`Err`) when the output is all zeros — a low-order point input a Diffie-Hellman
caller must reject.

Since pure ISO C++17 has no `__int128` (it would be rejected under
`-pedantic-errors` / `/permissive-`), this port carries the same small contained
128-bit emulation as the C sibling for the radix-2⁵¹ field multiply. Verified
against the RFC 7748 test vectors, including the 1000-iteration stress test.

## Portability

Pure ISO C++17 — no `__int128`, standard library only. Compiles clean under GCC,
Clang, and MSVC with `-pedantic-errors` / `/permissive-` and warnings-as-errors,
via the shared [`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
