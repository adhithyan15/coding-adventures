# m68k-encoder

Pure-Rust Motorola 68000 instruction encoder. Re-exports the canonical
`encode_move_l_imm_to_dn`/`encode_trap15` (and a few more) helpers from
[`m68k-simulator`](../m68k-simulator) — the in-tree source of truth for
M68K bit-level encoding — and adds the `D0`/`HALT_BYTES` constants
[`m68k-backend`](../m68k-backend) uses to lower CIR to M68K machine code
bytes. No IR knowledge lives here; see `m68k-backend` for the CIR → bytes
lowering.

Eighth lane of the [9-architecture expansion](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md),
mirroring [`mos6502-encoder`](../mos6502-encoder) / [`arm1-encoder`](../arm1-encoder)
in shape.

## Why re-export instead of duplicating the encoding logic?

`m68k-simulator` already has the opword-field packing logic (it needs it
for `encoding.rs`, used by its own tests). Re-exporting from there means
any future opcode-encoding fix in the simulator propagates to encoder
consumers automatically, instead of maintaining two copies of the same
bit-packing arithmetic that could silently drift apart.

## Quick start

```rust
use m68k_encoder::{assemble, encode_move_l_imm_to_dn, encode_trap15, D0, HALT_BYTES};

// const_i64 v=42 lowered to `MOVE.L #42, D0` -- the first instruction
// m68k-backend emits for the canonical IIR `const 42; ret` program.
let bytes = assemble(&[encode_move_l_imm_to_dn(D0, 42), encode_trap15()]);
assert_eq!(bytes, vec![0x20, 0x3C, 0x00, 0x00, 0x00, 0x2A, 0x4E, 0x4F]);
assert_eq!(&bytes[6..8], &HALT_BYTES);
```
