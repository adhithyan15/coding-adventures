# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `armv7-encoder` crate: a pure ARMv7-A
  (A32) instruction encoder.
- Canonical word constants (`ARMV7_BX_LR`, `ARMV7_BKPT`, `ARMV7_MOV_IMM_R0_BASE`,
  `ARMV7_MOV_REG_BASE`) and capacity constants (`ARMV7_GP_REGISTER_COUNT`,
  `ARMV7_MOV_IMM_MAX`) as macros.
- `armv7_encode_mov_imm` (MOV Rd, #imm8) and `armv7_encode_mov_reg` (MOV Rd, Rm)
  — branch-free bit-packing, register indices masked to 4 bits.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) assert exact ARM A32
  machine words (the Rust crate's doc vectors plus canonical encodings derived
  from the ARM ARM field layout).
