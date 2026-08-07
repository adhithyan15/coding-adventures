# intel8008-encoder (C)

A **pure Intel 8008 instruction encoder** in ISO C17. A faithful port of the
Rust [`intel8008-encoder`](../../rust/intel8008-encoder) crate — the encoding
tables for the Intel 8008 (1972), the second-generation 8-bit Intel
microprocessor. A companion to the ported [`intel4004-encoder`](../intel4004-encoder)
and [`intel-8008-assembler`](../intel-8008-assembler).

## ISA subset

| mnemonic | opcode | bytes | effect |
|----------|--------|-------|--------|
| `HLT` | `0x76` | 1 | halt (`01_110_110`) |
| `RET` | `0x07` | 1 | return from subroutine |
| `MVI A, n` | `0x3E nn` | 2 | ACC ← 8-bit immediate |
| `JMP addr` | `0x7C lo hi` | 3 | unconditional jump, 14-bit address |
| `CAL addr` | `0x7E lo hi` | 3 | call subroutine, 14-bit address |

The 3-byte instructions encode the 14-bit address low byte first, then the high
6 bits: `[opcode, addr & 0xFF, (addr >> 8) & 0x3F]`.

## API

```c
#include "intel8008_encoder.h"

uint8_t mvi[2];
intel8008_encode_mvi_a(42, mvi);   /* {0x3E, 0x2A} */
uint8_t jmp[3];
intel8008_encode_jmp(0x0100, jmp); /* {0x7C, 0x00, 0x01} */
```

- `intel8008_encode_mvi_a(n, out2)` / `intel8008_encode_jmp(addr, out3)` /
  `intel8008_encode_cal(addr, out3)` — write into caller buffers.
- Constants `INTEL8008_HLT` / `_RET` / `_MVI_A` / `_JMP` / `_CAL`, and
  `INTEL8008_GP_REGISTER_COUNT` / `_MVI_MAX`.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
