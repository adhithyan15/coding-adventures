# gf256 (C++)

A pure ISO **C++17**, header-only implementation of **Galois Field GF(2⁸)**
arithmetic — the finite field of 256 elements behind Reed-Solomon codes, QR
codes, and AES. A faithful port of the Rust `gf256` crate, in namespace
`ca::gf256`.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX` on
MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## API

```cpp
#include "gf256.hpp"
namespace gf = ca::gf256;

// Default Reed-Solomon field (0x11D), via log/antilog tables:
gf::multiply(3, 7);              // 9
gf::inverse(a);                  // a * inverse(a) == 1
gf::divide(9, 3);                // 7

// A field parameterised by any polynomial — e.g. AES (0x11B):
gf::Field aes(0x11B);
aes.multiply(0x57, 0x83);        // 0xC1 (FIPS-197 example)
aes.inverse(0x53);               // 0xCA
```

`add` / `subtract` / `multiply` / `divide` / `power` / `inverse` exist both as
free functions (the 0x11D field) and as `ca::gf256::Field` methods (any
polynomial). Division by zero and inverse of zero return `0`.

## Implementation notes

- The default-field log/antilog tables are built once via a **function-local
  static** (thread-safe in C++11, unlike the C sibling's lazy flag).
- The parameterised `Field` uses table-free Russian-peasant multiplication,
  correct for any irreducible polynomial.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests pin known values in both fields and verify `a·inverse(a)=1` and
`divide(multiply(a,b),b)=a` over full sweeps.

## Where it fits

A foundational primitive for the `code/packages/cpp` set: an AES port would use
`ca::gf256::Field(0x11B)`, and Reed-Solomon codes the default 0x11D field.
