# `mos6502-encoder`

Pure-Rust MOS 6502 instruction encoder. No IR knowledge — re-exports the
canonical `encode_*` helpers from
[`mos6502-simulator`](../mos6502-simulator) (the in-tree source of truth
for MOS 6502 encoding) and adds the `HALT_BYTE` constant
[`mos6502-backend`](../mos6502-backend) uses to lower CIR to MOS 6502
machine code bytes.

Mirror of `mips-r2000-encoder` / `arm1-encoder` / `armv7-encoder` /
`intel8008-encoder` in shape. Fifth lane of the 9-architecture expansion
— see
[`code/specs/mos6502-encoder.md`](../../../specs/mos6502-encoder.md) and
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## Why `BRK`, not KIL/JAM or a self-jump loop?

`mos6502-simulator` already has a pre-existing, documented halt
convention ported directly from the Python original: `BRK` (opcode
`0x00`) sets `halted = true`. That predates this lane — see the crate's
module doc for the full rationale, and
`code/packages/python/mos6502-simulator/src/mos6502_simulator/
simulator.py`'s own module docstring for the original statement of the
convention.

## Usage

```rust
use mos6502_encoder::{encode_lda_imm, encode_brk, assemble, HALT_BYTE};

let bytes = assemble(&[encode_lda_imm(42), encode_brk()]);
assert_eq!(bytes, vec![0xA9, 42, 0x00]);
assert_eq!(HALT_BYTE, 0x00);
```
