# intel4004-simulator (C)

A **behavioral simulator for the Intel 4004** (1971) — the world's first
commercial single-chip microprocessor — in pure ISO C17. A faithful port of the
Rust [`intel4004-simulator`](../../rust/intel4004-simulator) crate. Pairs with
the ported [`intel4004-encoder`](../intel4004-encoder), mirroring the repo's
Intel 8008 trio.

## What it models

The 4004 is natively **4-bit**: every value is a nibble (0-15) and all
arithmetic masks to 4 bits. It is an **accumulator machine** — operations funnel
through a single accumulator. This port executes 4004 machine code directly:

- 16 general registers (8 pairs), the accumulator, and the carry flag.
- Byte-addressable **ROM** (program memory, 12-bit address space).
- **Data RAM**: 4 banks × 4 registers × 16 characters (nibbles), plus 4 status
  nibbles per register, and a 4-bit output-port latch per bank.
- A **3-level hardware call stack** (nesting a 4th call wraps, silently losing
  the oldest return address — exactly as the chip did).
- The ROM I/O port.

All 46 instructions are covered — JCN/JUN/JMS/BBL/ISZ control flow, FIM/FIN/SRC
register-pair and RAM addressing, the ADD/SUB/INC accumulator ops (with the
4004's inverted-carry subtraction convention), rotates, BCD `DAA`, and the
one-hot keyboard decode `KBP`. Each executed instruction yields an `I4004Trace`
(address, raw bytes, disassembly mnemonic, and before/after accumulator + carry).

## API

```c
#include "intel4004_simulator.h"

I4004Sim *s = i4004_new(4096);
/* LDM 1; XCH R0; LDM 2; ADD R0; XCH R1; HLT  ->  R1 = 3 */
uint8_t prog[] = {i4004_encode_ldm(1), i4004_encode_xch(0),
                  i4004_encode_ldm(2), i4004_encode_add(0),
                  i4004_encode_xch(1), i4004_encode_hlt()};
i4004_run(s, prog, sizeof prog, 100);   /* i4004_register(s, 1) == 3 */
i4004_free(s);
```

- `i4004_new` / `i4004_free`, `i4004_reset`, `i4004_load_program`.
- `i4004_run` (reset + load + step to HLT) / `i4004_step` (single instruction;
  returns 0 if already halted, faithful to the Rust `step()` precondition).
- State accessors (accumulator, carry, registers, RAM/status/output, banks,
  stack) and `i4004_trace_count` / `i4004_trace` for the recorded trace.
- `i4004_encode_*` helpers build machine code; two-byte forms return the first
  byte and write the second through an out-parameter.

The growable trace buffer guards `size_t` overflow, and every ROM read is
bounds-checked (a runaway program counter reads `0x00`/NOP rather than out of
bounds). Verified clean under ASan + UBSan, the macOS `leaks` tool (0 leaks),
and an all-opcodes fuzz sweep across ROM sizes.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
