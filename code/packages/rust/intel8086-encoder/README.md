# Intel 8086 Encoder (Rust)

Pure-Rust instruction encoder for the Intel 8086 (1978) — this lane's
curated opcode subset only. Mirror of `mos6502-encoder`/`arm1-encoder`/
`riscv-encoder`. Ninth and final lane of the 9-architecture expansion
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

- Re-exports of the `encode_*` helpers from
  `intel8086_simulator::encoding` (the in-tree source of truth for this
  lane's curated opcode subset), so `intel8086-backend` can depend on a
  small, IR-agnostic surface without pulling in the full simulator's
  decode/execute machinery.
- `HALT_BYTE` (`0xF4`) — the `HLT` opcode byte every program
  `intel8086-backend` compiles ends with.
- `REG_AX` — the accumulator register index, re-exported from
  `intel8086_simulator::opcodes` so `intel8086-backend` doesn't need a
  direct dependency on the simulator crate just to name the register it
  targets.

No IR knowledge lives here — `intel8086-backend` maps CIR onto these
`encode_*` calls.

## Why `HLT`, not a pseudo-halt or repurposed opcode?

Unlike ARM1 (no real halt instruction — `arm1-backend` invented `SWI
#0x123456`) or MOS 6502 (`BRK` is technically a software-interrupt
opcode this repo's simulator stack *treats* as HALT by convention), the
Intel 8086 has a genuine, single-byte, no-operand hardware instruction
whose sole documented purpose is to stop the fetch-decode-execute loop:
`HLT`. This is the least-invented halt-related decision anywhere in the
9-architecture expansion — see `intel8086-backend`'s module doc and
`code/specs/intel8086-backend.md` for the full comparison against the
other eight lanes' halt conventions.

## Quick start

```rust
use intel8086_encoder::{encode_mov_reg_imm16, encode_hlt, assemble, HALT_BYTE, REG_AX};

// const_i64 v=42 lowered to `MOV AX,42` -- the first instruction
// intel8086-backend emits for the canonical IIR `const 42; ret` program.
let bytes = assemble(&[encode_mov_reg_imm16(REG_AX, 42), encode_hlt()]);
assert_eq!(bytes, vec![0xB8, 42, 0x00, 0xF4]);
assert_eq!(HALT_BYTE, 0xF4);
```
