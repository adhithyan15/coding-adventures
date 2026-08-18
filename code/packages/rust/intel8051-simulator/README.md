# intel8051-simulator

Pure-Rust behavioral simulator for the Intel 8051 (MCS-51, 1980) —
the most-manufactured CPU architecture in history (over 20 billion
units). Port of `code/packages/python/intel8051-simulator`
(spec [`07p-intel-8051-simulator.md`](../../../specs/07p-intel-8051-simulator.md)).

Fourth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## Why the 8051 was revolutionary

Before the 8051, an embedded controller needed a separate CPU chip, a
RAM chip, a ROM chip, and an I/O peripheral chip. The 8051 put all of
this on one package — CPU, 4 KB ROM, 128 B RAM, 32 I/O pins, a UART,
two 16-bit timers, and an interrupt controller — defining the
microcontroller as a product category. It is still manufactured today
by numerous licensees (Atmel/Microchip AT89, NXP 80C51, Silicon Labs
EFM8, and others).

## Harvard architecture — four independent memory spaces

Unlike this codebase's flat-memory historical arches (RISC-V, MIPS
R2000, Intel 8080, ARM1), the 8051 has genuinely separate address
spaces:

```text
code   (64 KiB) — program memory: fetched by PC, read-only via MOVC
iram  (256 B)   — internal RAM (0x00-0x7F) + SFRs (0x80-0xFF)
xdata  (64 KiB) — external data memory: read/write only via MOVX
```

`iram` additionally has a **bit-addressable region** (0x20-0x2F) and
**four switchable register banks** (0x00-0x1F) — see the crate-level
doc comment and `code/specs/07p-intel-8051-simulator.md` for the full
memory map.

## Module layout

| Module | Contents |
|--------|----------|
| `opcodes` | Memory sizes, SFR addresses, PSW bit masks, instruction opcode constants. |
| `encoding` | Pure `encode_*` helpers — what `intel8051-encoder` re-exports. |
| `decode` | Pure opcode → operand-length decoding (no CPU state). |
| `execute` | Instruction semantics — mutates simulator state. |
| `simulator` | The public `Intel8051Simulator` struct. |

## HALT convention

The real 8051 has no HALT instruction. This simulator reuses the
already-shipped Python reference's convention: opcode `0xA5`
(reserved/undefined on real silicon) is a HALT sentinel — executing it
sets `halted() == true` and stops the fetch-decode-execute loop. See
[`intel8051-backend`](../intel8051-backend)'s README/spec for why this
was kept over the alternative (self-jump detection).

## Quick start

```rust
use intel8051_simulator::Intel8051Simulator;

// MOV A, #42 ; HALT
let mut sim = Intel8051Simulator::new();
sim.load_program(&[0x74, 42, 0xA5], 0);
let result = sim.run_loaded_with_limit(100);
assert!(result.halted);
assert_eq!(sim.acc(), 42);
```

## What's ported vs. what's not

Every instruction group in `code/specs/07p-intel-8051-simulator.md`'s
instruction-set tables (data transfer, arithmetic, logic, bit
manipulation, jumps, CJNE/DJNZ, subroutines) is ported. Not modeled:
timers, the serial port, and the interrupt controller — this is a
**behavioral instruction-set simulator**, not a cycle-accurate or
peripheral-accurate one, matching the Python reference's documented
scope.
