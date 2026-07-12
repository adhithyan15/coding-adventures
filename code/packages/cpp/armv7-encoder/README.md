# armv7-encoder (C++)

A pure ARMv7-A (A32) instruction encoder in pure ISO C++17, header-only, in
namespace `ca::armv7`. A faithful port of the Rust `armv7-encoder` crate.

Canonical instruction-word constants plus typed `encode_*` helpers that return
the exact 32-bit machine word. All constants and helpers are `constexpr`.

## API

```cpp
#include "armv7_encoder.hpp"
namespace a7 = ca::armv7;

static_assert(a7::encode_mov_imm(0, 42) == 0xE3A0002A);  // MOV r0, #42
a7::encode_mov_reg(0, 1);                                 // MOV r0, r1 = 0xE1A00001
a7::BX_LR;                                                // 0xE12FFF1E
```

Every value is an exact ARM A32 encoding; register indices are masked to 4 bits
(out-of-range is the caller's responsibility, matching the Rust crate).

## Portability

Pure ISO C++17 — standard library only. Compiles clean under GCC, Clang, and MSVC
with `-pedantic-errors` / `/permissive-` and warnings-as-errors, via the shared
[`iso-harness`](../../c/iso-harness).

## Development

```bash
sh BUILD
```
