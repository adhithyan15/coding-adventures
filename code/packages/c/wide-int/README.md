# wide-int (C)

**Portable 128-bit integers**, built from two 64-bit halves — pure ISO C17.
Part of the [CCPP02](../../../specs/CCPP02-os-platform-lane.md) lane (bucket A:
computable from scratch, no OS, no extensions).

## Why

Rust has native `u128`/`i128`; C does not. GCC and Clang offer `__int128` as an
*extension*, but **MSVC has no 128-bit integer type at all**, and `__int128` is
rejected under `-pedantic-errors`. So a *portable* 128-bit integer must be
synthesised from two `uint64_t` halves using only standard 64-bit arithmetic —
which is exactly what this library does. It is the substrate the campaign's
`u128`-using crates build on, and it compiles identically under GCC, Clang, and
MSVC.

## What it provides

`wi_u128` and `wi_i128` (value = `hi * 2^64 + lo`), with:

- **arithmetic** — add / sub / mul (mod 2^128), the exact widening `wi_mul_u64`
  (64×64→128), and division-with-remainder (`wi_u128_divmod` / `wi_i128_divmod`,
  binary long division; signed truncates toward zero);
- **bitwise / shifts** — and / or / xor / not, logical `shl`/`shr`, arithmetic
  `sar`; every shift is total (n ≥ 128 → 0, no undefined shifts);
- **comparison** — signed and unsigned three-way compare;
- **formatting** — decimal and zero-padded hex.

```c
#include "wide_int.h"

wi_u128 a = wi_u128_from_u64(0xFFFFFFFFFFFFFFFFu);
wi_u128 sq = wi_u128_mul(a, a);      /* 2^128 - 2^65 + 1, no overflow */
char buf[40];
wi_u128_to_dec(sq, buf);             /* "340282366920938463444927863358058659841" */
```

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`. Verified under ASan + UBSan, and
cross-checked against native `__int128` over 5M random operations (unsigned and
signed) as a local correctness oracle — the committed tests stay pure-ISO
(golden vectors + algebraic property sweeps: `a+b-b == a`, `q*d+r == n` with
`r < d`, etc.), since `__int128` cannot appear under `-pedantic-errors`.
