# arm1-encoder

Pure-Rust ARM1 (ARMv1) instruction encoder.  Mirror of
`mips-r2000-encoder` / `armv7-encoder` / `intel8008-encoder`.

Second lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* Re-exports of `encode_mov_imm`, `encode_halt`, `COND_AL` from
  `arm1_simulator` (the in-tree source of truth for ARM1 bit layout).
* Register-role constant: `R0` — the value ARM1's `MOV R0, #imm`
  writes to and `read_register(0)` reads back.
* Canonical word constant: `HALT_WORD = 0xEF12_3456` (i.e.
  `SWI #0x123456`, `AL`-conditioned — the pseudo-halt
  `arm1-simulator` intercepts to stop execution; see the crate-level
  doc comment in `src/lib.rs` for why ARM1 uses a pseudo-halt rather
  than `BX LR`).

No IR knowledge.  `arm1-backend` is the consumer that maps CIR ops
onto encoder calls.
