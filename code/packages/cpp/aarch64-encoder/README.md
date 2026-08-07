# aarch64-encoder (C++)

An **AArch64 (ARM64) instruction encoder**, header-only, ISO C++17. A faithful
port of the Rust [`aarch64-encoder`](../../rust/aarch64-encoder) crate, in
namespace `ca::aarch64_encoder`: a stream-style `Assembler` that produces
little-endian 32-bit instruction words for the AArch64 instruction set.

## How it works

Each method emits one 4-byte instruction word (a base opcode OR'd with 5-bit
register fields and immediates, per ARM ARM DDI 0487). Branches reference a
`LabelId` bound to a later instruction; the PC-relative displacement is patched
at `finish()` time, which returns the raw `.text` byte stream.

## What it encodes

Moves, integer arithmetic (register + immediate, `sdiv`/`udiv`/`msub`), logical,
variable shifts, `neg`, compare, scaled loads/stores (byte + double), scalar
double-precision FP and int⇄real conversions, `stp`/`ldp`, branches
(`b`/`bl`/`b.cond`/`cbz`/`cbnz`/`blr`/`ret`), and misc (`cset`/`nop`/`udf`/`svc`/
`adrp` placeholder). Where the Rust crate returns `Result`, this port throws
`Error` (carrying an `ErrorKind`).

## API

```cpp
#include "aarch64_encoder.hpp"
namespace a64 = ca::aarch64_encoder;

a64::Assembler a;
a.add(a64::Reg::X0, a64::Reg::X0, a64::Reg::X1);   // add x0, x0, x1
a.ret();
std::vector<std::uint8_t> bytes = a.finish();       // 8 bytes
```

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
