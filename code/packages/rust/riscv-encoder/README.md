# riscv-encoder

Pure-Rust RV32I instruction encoder.  Mirror of
`ge225-encoder` / `intel4004-encoder` / `armv7-encoder` /
`intel8008-encoder`.

Phase 7 (the FINAL lane) of the historical-arch backend migration
— see [`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).

## What's inside

* Re-exports of `encode_addi`, `encode_jalr`, `encode_add`,
  `encode_sub`, … from `riscv-simulator::encoding` (the in-tree
  source of truth for RV32I bit layout).
* Register-index constants for the registers `riscv-backend` uses:
  `X0_ZERO`, `X1_RA`, `A0`, the `TEMP_REGISTERS` array, etc.
* Canonical word constants: `RET_WORD = 0x0000_8067` (i.e.
  `jalr x0, x1, 0` — the universal RV32I "return from function").

No IR knowledge.  `riscv-backend` is the consumer that maps CIR
ops onto encoder calls.
