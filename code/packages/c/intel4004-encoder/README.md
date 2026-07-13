# intel4004-encoder (C)

A **pure Intel 4004 instruction encoder** in ISO C17. A faithful port of the Rust
[`intel4004-encoder`](../../rust/intel4004-encoder) crate — the encoding tables
for the Intel 4004 (1971), the **world's first commercial microprocessor**. Part
of the repo's historical-CPU encoder family alongside
[`ibm704-encoder`](../ibm704-encoder) and [`ge225-encoder`](../ge225-encoder).

## ISA subset

| mnemonic | opcode | bytes | effect |
|----------|--------|-------|--------|
| `LDM n` | `0xD0 \| n` | 1 | ACC ← 4-bit immediate |
| `LD r`  | `0xA0 \| r` | 1 | ACC ← register r |
| `XCH r` | `0xB0 \| r` | 1 | ACC ↔ register r (the 4004's store) |
| `JUN a` | `0100 aaaa aaaaaaaa` | 2 | unconditional 12-bit ROM jump |

The 4004 has no formal `HLT`; `JUN 0x000` at ROM address 0 loops on itself,
which every 4004 simulator treats as halt (`INTEL4004_HALT_LOOP = {0x40, 0x00}`).

## API

```c
#include "intel4004_encoder.h"

uint8_t a = intel4004_encode_ldm(5);   /* 0xD5 */
uint8_t b = intel4004_encode_xch(3);   /* 0xB3 */
uint8_t jun[2];
intel4004_encode_jun(0xABC, jun);      /* {0x4A, 0xBC} */
```

- `intel4004_encode_ldm` / `_ld` / `_xch` — single-byte ops (low nibble masked).
- `intel4004_encode_jun(addr, out2)` — 2-byte jump (address masked to 12 bits).
- Constants: `INTEL4004_LDM_OPCODE` / `_LD_OPCODE` / `_XCH_OPCODE` / `_JUN_OPCODE`,
  `INTEL4004_HALT_LOOP`, and `INTEL4004_GP_REGISTER_COUNT` / `_LDM_MAX` /
  `_LDM_MIN_SIGNED`.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
