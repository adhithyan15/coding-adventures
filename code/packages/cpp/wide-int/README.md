# wide-int (C++)

**Portable 128-bit integers**, built from two 64-bit halves — header-only, ISO
C++17, in namespace `ca::wide_int`. Part of the
[CCPP02](../../../specs/CCPP02-os-platform-lane.md) lane (bucket A: computable
from scratch, no OS, no extensions).

## Why

Rust has native `u128`/`i128`; C++ does not. GCC and Clang offer `__int128` as
an extension, but **MSVC has no 128-bit integer type**, and `__int128` is
rejected under `-pedantic-errors`. So a *portable* 128-bit integer must be
synthesised from two `std::uint64_t` halves using only standard 64-bit
arithmetic — which is what `ca::wide_int::u128` / `i128` do, with idiomatic C++
operators, compiling identically under GCC, Clang, and MSVC.

## What it provides

`u128` and `i128` (value = `hi * 2^64 + lo`) with the full operator set —
`+ - * / % & | ^ ~ << >>`, comparisons, `divmod`, the exact widening
`u128::mul_u64` (64×64→128), `to_string`/`to_hex`. Signed division truncates
toward zero; `i128::operator>>` is arithmetic (sign-extending). The core ops are
`constexpr`, so they can run at compile time.

```cpp
#include "wide_int.hpp"
namespace wi = ca::wide_int;

wi::u128 a(0xFFFFFFFFFFFFFFFFu);
wi::u128 sq = a * a;                  // 2^128 - 2^65 + 1, no overflow
std::string s = sq.to_string();       // "340282366920938463444927863358058659841"
constexpr wi::u128 c = wi::u128(0xFFFFFFFFu) * wi::u128(0x100000000u); // compile-time
```

## Building

```sh
sh BUILD          # POSIX: g++ and/or clang++, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified under ASan + UBSan; the
committed tests are pure-ISO (golden vectors + algebraic property sweeps +
`static_assert` constexpr checks), matching the C sibling that was cross-checked
against native `__int128` over 5M random operations.
