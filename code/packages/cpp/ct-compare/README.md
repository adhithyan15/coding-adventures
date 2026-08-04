# ct-compare (C++)

A pure ISO **C++17**, header-only library for **constant-time** byte comparison,
in namespace `ca::ct_compare`. A faithful port of the Rust `ct-compare` crate.

It compiles clean under **GCC, Clang, and MSVC** with `-std=c++17
-pedantic-errors -Wall -Wextra -Werror` (and `/std:c++17 /permissive- /W4 /WX`
on MSVC), via the shared [`iso-harness`](../../c/iso-harness/). Standard library
only.

## Why constant-time?

A naive `memcmp` — or an equality loop that returns as soon as it finds a
mismatch — runs for a different amount of time depending on **where** two values
first differ. When one of the values is a secret (a MAC/auth tag, a derived key,
a password hash), that timing is a side channel: an attacker who can measure it
can recover the secret one byte at a time.

These routines do the **same work for every byte** regardless of its value: they
fold all differences into an accumulator with no data-dependent branch, then
inspect the accumulator once at the end. An optimiser barrier (a read through a
`volatile` object — the pure-ISO stand-in for Rust's `core::hint::black_box`)
keeps the compiler from re-introducing an early exit.

## API

```cpp
#include "ct_compare.hpp"
namespace ct = ca::ct_compare;

std::vector<std::uint8_t> a = ..., b = ...;

bool ok  = ct::ct_eq(a, b);                    // same length AND bytes
auto out = ct::ct_select_bytes(a, b, choice);  // a if choice, else b (throws on len mismatch)
bool u   = ct::ct_eq_u64(x, y);                // constant-time 64-bit equality

std::array<std::uint8_t, 16> p, q;             // compile-time length
bool eq = ct::ct_eq_fixed(p, q);
```

- `ct_eq(a, b)` — compares the length first (length is public), then the bytes
  in constant time.
- `ct_eq_fixed<N>(a, b)` — over a `std::array<std::uint8_t, N>` (no length check).
- `ct_select_bytes(a, b, choice)` — returns a copy of `a` when `choice`, else
  `b`; throws `std::invalid_argument` if the inputs differ in length (the crate
  panics).
- `ct_eq_u64(a, b)` — branchless 64-bit equality.

## Building & testing

```sh
sh BUILD          # POSIX: compiles + runs the tests under every compiler found
```

Tests check equality, first/last-byte differences, the length-mismatch throw,
and — the key property — that **every single-bit flip at every byte position**
is detected, which a short-circuiting bug would miss.
