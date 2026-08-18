# intel8051-encoder

Pure-Rust Intel 8051 (MCS-51) instruction encoder. Mirror of
`arm1-encoder` / `mips-r2000-encoder` / `intel8008-encoder`.

Fourth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* Re-exports of `encode_mov_a_imm`, `encode_mov_rn_imm`, `encode_halt`
  from `intel8051_simulator::encoding` (the in-tree source of truth
  for 8051 bit-level encoding).
* Opcode constants: `MOV_A_IMM` (`0x74`), `HALT_OPCODE` (`0xA5` — the
  pseudo-halt `intel8051-backend` lowers `ret_*` to; see its
  crate-level doc comment for why the reserved-opcode sentinel was
  kept over self-jump detection).
* Capacity constant: `IMM8_MAX` (255).

No IR knowledge. `intel8051-backend` is the consumer that maps CIR ops
onto encoder calls.
