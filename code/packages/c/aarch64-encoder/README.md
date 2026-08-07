# aarch64-encoder (C)

An **AArch64 (ARM64) instruction encoder** in pure ISO C17. A faithful port of
the Rust [`aarch64-encoder`](../../rust/aarch64-encoder) crate: a stream-style
assembler that produces little-endian 32-bit instruction words for the AArch64
instruction set — the bottom of a CIR → native-code lowering.

## How it works

Each `a64_*` call emits one 4-byte instruction word (built from a base opcode
OR'd with 5-bit register fields and immediates, per ARM ARM DDI 0487). Branches
reference an `A64Label` bound to a later instruction; the PC-relative
displacement is patched at `a64_finish` time, which then produces the raw
`.text` byte stream.

## What it encodes

Moves (`movz`/`movk`/`mov_imm64`), integer arithmetic (`add`/`sub`/`mul`,
immediate forms, `sdiv`/`udiv`/`msub`), logical (`and`/`orr`/`eor`/`mvn`),
variable shifts, `neg`, compare, scaled loads/stores (incl. byte and double),
scalar double-precision FP (`fadd`/…/`fsqrt`) and int⇄real conversions, `stp`/
`ldp`, branches (`b`/`bl`/`b.cond`/`cbz`/`cbnz`/`blr`/`ret`), and misc
(`cset`/`nop`/`udf`/`svc`/`adrp` placeholder).

## Error model

The assembler carries a **sticky error** (like a builder): a method that
validates an immediate — or `a64_bind` on a re-bound label — latches the first
error, which `a64_finish` returns. Query it any time with `a64_error`. This
mirrors the Rust crate's `Result` at every fallible step.

## API

```c
#include "aarch64_encoder.h"

A64Assembler *a = a64_new();
a64_add(a, A64_X0, A64_X0, A64_X1);   /* add x0, x0, x1 */
a64_ret(a);
uint8_t *bytes; size_t len;
a64_finish(a, &bytes, &len);          /* 8 bytes */
free(bytes);
a64_free(a);
```

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
