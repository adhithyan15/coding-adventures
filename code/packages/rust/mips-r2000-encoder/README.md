# mips-r2000-encoder

Pure-Rust MIPS R2000 instruction encoder.  Mirror of `riscv-encoder` /
`armv7-encoder` / `intel8008-encoder`.

First lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* Re-exports of `encode_addiu`, `encode_jr`, `assemble` from
  `mips-r2000-simulator::encoding` (the in-tree source of truth for
  MIPS R2000 bit layout).
* Register-role constants: `ZERO`, `V0`, `V1`, `A0`, `SP`, `RA`, the
  `TEMP_REGISTERS` pool.
* Canonical word constant: `RET_WORD = 0x03E0_0008` (i.e. `JR $ra` — the
  universal MIPS R2000 "return from function").

No IR knowledge.  `mips-r2000-backend` is the consumer that maps CIR ops
onto encoder calls.
