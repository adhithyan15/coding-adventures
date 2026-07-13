# intel8008-simulator (C)

A **behavioral simulator for the Intel 8008** (1972) — the world's first 8-bit
microprocessor — in pure ISO C17. A faithful port of the Rust
[`intel8008-simulator`](../../rust/intel8008-simulator) crate. Completes the
repo's Intel 8008 trio alongside the ported
[`intel8008-encoder`](../intel8008-encoder) and
[`intel-8008-assembler`](../intel-8008-assembler).

## What it models

It executes 8008 machine code directly (no gate-level modelling): registers
A/B/C/D/E/H/L, the **M pseudo-register** (memory at `[H:L]`, low 6 bits of H),
four condition flags (carry / zero / sign / parity), a 16 KiB address space, and
the 8008's unique **8-level push-down call stack** where `stack[0]` is always the
live program counter. The full instruction set is covered — MOV, MVI, INR/DCR,
rotates, the eight ALU ops (register and immediate forms), conditional/
unconditional jumps and calls, RST, RET, IN/OUT, and HLT.

Each executed instruction produces an `I8008Trace` (address, raw bytes, a
disassembly mnemonic, before/after accumulator + flags, and any memory access).

## API

```c
#include "intel8008_simulator.h"

I8008Sim *s = i8008_new();
/* MVI B,1; MVI A,2; ADD B; HLT */
static const uint8_t prog[] = {0x06, 0x01, 0x3E, 0x02, 0x80, 0x76};
i8008_run(s, prog, sizeof prog, 100);   /* i8008_a(s) == 3 */
i8008_free(s);
```

- `i8008_new` / `i8008_free`, `i8008_reset`, `i8008_load_program`.
- `i8008_run` (load + step to HLT) / `i8008_step` (single instruction).
- Register/flag/state accessors, `i8008_set_input_port` / `_get_output_port`, and
  `i8008_trace_count` / `i8008_trace` for the recorded execution trace.

## Building

Builds through the shared [`iso-harness`](../iso-harness) engine under every ISO
C compiler on `PATH` with `-std=c17 -pedantic-errors -Wall -Wextra -Werror`:

```sh
sh BUILD          # POSIX: gcc and/or clang
```

Each compiler prints `N checks, 0 failed`.
