# Changelog — armv7-encoder

## v0.1.0 — 2026-06-03 — initial carve-out

Phase 5 of the historical-arch backend migration.  Pure encoder
crate matching the structure of `ge225-encoder` /
`intel4004-encoder`.

### Added

- `BX_LR` (0xE12FFF1E), `BKPT` (0xE12FFF7F), `MOV_IMM_R0_BASE`
  (0xE3A00000), `MOV_REG_BASE` (0xE1A00000).
- `GP_REGISTER_COUNT` (= 12, the ABI-allocatable set r0..r11),
  `MOV_IMM_MAX` (= 255).
- `encode_mov_imm(rd, imm8)`, `encode_mov_reg(rd, rm)`.

### Tests

8 unit tests pin every constant and every `encode_*` byte output,
including the canonical `MOV r0, #42 = 0xE3A0_002A` value the
ARMv7 e2e smoke test pins.
