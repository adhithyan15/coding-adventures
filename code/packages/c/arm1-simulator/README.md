# arm1-simulator (C)

A **behavioral simulator for the ARM1** (1985) — the first ARM chip, designed by
Sophie Wilson and Steve Furber at Acorn — in pure ISO C17. A faithful port of
the Rust [`arm1-simulator`](../../rust/arm1-simulator) crate.

## What it models

The complete ARMv1 instruction set:

- 16 data-processing ops (AND/EOR/SUB/RSB/ADD/ADC/SBC/RSC/TST/TEQ/CMP/CMN/ORR/
  MOV/BIC/MVN) through the inline **barrel shifter** (LSL/LSR/ASR/ROR/RRX).
- Load/store (LDR/STR/LDRB/STRB, pre/post-indexed), block transfer (LDM/STM, all
  four stacking modes), branch (B/BL), and SWI.
- **Conditional execution on every instruction** (16 condition codes).
- 4 processor modes (USR/FIQ/IRQ/SVC) with banked registers, and ARMv1's
  distinctive shared PC + status register (R15).

Each executed instruction yields an `Arm1Trace` (before/after registers and
flags, plus recorded memory reads/writes and a disassembly mnemonic).

## API

```c
#include "arm1_simulator.h"

ARM1 *cpu = arm1_new(4096);
uint32_t prog[] = { arm1_encode_mov_imm(ARM1_COND_AL, 0, 42),
                    arm1_encode_halt() };
arm1_load_program_words(cpu, prog, 2, 0);
Arm1Trace traces[100];
size_t n = arm1_run(cpu, 100, traces, 100);   /* arm1_read_register(cpu, 0)==42 */
arm1_free(cpu);
```

- `arm1_new` / `arm1_free` / `arm1_reset`; register, flag, mode, and memory
  accessors; `arm1_step` (one instruction) / `arm1_run` (to halt or a cap).
- Pure functions `arm1_evaluate_condition`, `arm1_barrel_shift`,
  `arm1_decode_immediate`, `arm1_alu_execute`, `arm1_decode`,
  `arm1_disassemble`, and the `arm1_encode_*` helpers.

`Arm1Trace` is a plain value type: memory-access lists are fixed arrays (a block
transfer touches at most 16 registers) and the mnemonic is a bounded buffer.
Verified clean under ASan + UBSan, the macOS `leaks` tool (0 leaks), and a
50k-iteration random-program / random-instruction fuzz.

## Building

```sh
sh BUILD          # POSIX: gcc and/or clang, via the shared iso-harness
```

Each compiler prints `N checks, 0 failed`.
