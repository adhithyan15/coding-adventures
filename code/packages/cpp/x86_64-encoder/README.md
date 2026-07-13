# x86_64-encoder (C++)

An **x86-64 (AMD64) instruction encoder**, header-only, ISO C++17. A faithful
port of the Rust [`x86_64-encoder`](../../rust/x86_64-encoder) crate, in
namespace `ca::x86_64_encoder`: a stream-style `Assembler` that produces
little-endian x86-64 machine-code byte streams in 64-bit (long) mode. A
companion to the ported [`aarch64-encoder`](../aarch64-encoder).

## How it works

Each method emits one logical instruction (1–15 bytes) as a REX prefix +
opcode(s) + ModR/M (+ SIB) + displacement/immediate, per the Intel SDM Vol. 2 /
AMD64 APM Vol. 3. Branches reference a `LabelId` bound to a later byte; the
rel32 displacement is patched at `finish()`. Cross-function / runtime references
are recorded as `ExternalReloc`s. V1 "always-long-form": branches use rel32,
memory operands use disp32.

## What it encodes

MOV family (reg/reg, imm32, movabs imm64, mem load/store, RIP-relative `lea`),
integer arithmetic, logical, shifts, compare + `setcc` + `movzx`, SSE2 scalar
double + int⇄real conversions, stack, control flow (`jmp`/`jcc`/`call`/`ret`),
and misc. Where the Rust crate returns `Result`, this port throws `Error`
(carrying an `ErrorKind`).

## API

```cpp
#include "x86_64_encoder.hpp"
namespace x64 = ca::x86_64_encoder;

x64::Assembler a;
a.mov_r64_r64(x64::Reg::Rax, x64::Reg::Rdi);   // mov rax, rdi
a.add(x64::Reg::Rax, x64::Reg::Rsi);           // add rax, rsi
a.ret();
std::vector<std::uint8_t> bytes = a.finish();   // 48 89 F8 48 01 F0 C3
```

## Building

Builds through the shared [`iso-harness`](../../c/iso-harness) engine under every
ISO C++ compiler on `PATH` with `-std=c++17 -pedantic-errors -Wall -Wextra
-Werror`:

```sh
sh BUILD          # POSIX: g++ and/or clang++
```

Each compiler prints `N checks, 0 failed`.
