# Changelog — mos6502-encoder

## [0.1.0] - 2026-08-17

### Added

- Re-exports `encode_lda_imm`/`encode_ldx_imm`/`encode_ldy_imm`/
  `encode_sta_zp`/`encode_adc_imm`/`encode_sbc_imm`/`encode_clc`/
  `encode_sec`/`encode_nop`/`encode_brk`/`assemble` from
  `mos6502_simulator::encoding`.
- `HALT_BYTE` (`0x00`) — the `BRK` halt sentinel, matching
  `mos6502-simulator`'s pre-existing, Python-original-derived convention
  (not a new choice invented for this lane; see the crate's module doc
  for the full rationale and the KIL/JAM / self-jump alternatives it
  deliberately does *not* use).
- 5 unit tests + 1 doctest pinning the canonical `LDA #42; BRK` byte
  sequence `mos6502-backend` emits for the IIR `const 42; ret` program.

Fifth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
