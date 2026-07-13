# x86_64-encoder (C)

An **x86-64 (AMD64) instruction encoder** in pure ISO C17. A faithful port of
the Rust [`x86_64-encoder`](../../rust/x86_64-encoder) crate: a stream-style
assembler that produces little-endian x86-64 machine-code byte streams in
64-bit (long) mode — the bottom of a CIR → native-code lowering. A companion to
the ported [`aarch64-encoder`](../aarch64-encoder).

## How it works

Each `x64_*` call emits one logical instruction (1–15 bytes) as a REX prefix +
opcode(s) + ModR/M (+ SIB) + displacement/immediate, per the Intel SDM Vol. 2 /
AMD64 APM Vol. 3. Branches reference an `X64Label` bound to a later byte; the
rel32 displacement is patched at `x64_finish` time. Cross-function / runtime
references are recorded as external relocations (`x64_external_reloc_*`) for a
later packager. V1 "always-long-form" policy: branches use rel32, memory
operands use disp32.

## What it encodes

MOV family (reg/reg, imm32, imm64/movabs, mem load/store, `lea` RIP-relative),
integer arithmetic (`add`/`sub`/`imul`/`idiv`/`div`/`cqo`/`neg`, imm32 forms),
logical, shifts (by CL and imm8), compare + `setcc` + `movzx`, SSE2 scalar
double (`movsd`/`addsd`/…/`sqrtsd`) and int⇄real conversions, stack
(`push`/`pop`), control flow (`jmp`/`jcc`/`call`/`ret`), and misc
(`nop`/`int3`/`ud2`).

## Error model

The assembler carries a **sticky error** (`x64_error`): a re-bound label latches
it, and `x64_finish` surfaces an unbound-label or out-of-range branch. Mirrors
the Rust crate's `Result` at every fallible step.

## API

```c
#include "x86_64_encoder.h"

X64Assembler *a = x64_new();
x64_mov_r64_r64(a, X64_RAX, X64_RDI);   /* mov rax, rdi */
x64_add(a, X64_RAX, X64_RSI);           /* add rax, rsi */
x64_ret(a);
uint8_t *bytes; size_t len;
x64_finish(a, &bytes, &len);            /* 48 89 F8 48 01 F0 C3 */
free(bytes);
x64_free(a);
```

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
