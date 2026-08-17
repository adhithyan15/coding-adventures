# Changelog — arm1-encoder

## v0.1.0 — 2026-08-17 — initial carve-out

Second lane of the 9-architecture expansion following the pattern
documented in `HISTORICAL-ARCH-BACKEND-MIGRATION.md`.

### Added

- Re-exports of `encode_mov_imm`, `encode_halt`, `COND_AL` from
  `arm1_simulator`.
- Register-role constant: `R0` (0) — the ARM1 return-value register.
- `HALT_WORD = 0xEF12_3456` — the pseudo-halt `SWI #0x123456`
  (`AL`-conditioned) that `arm1-simulator::ARM1::execute_swi`
  intercepts to stop the fetch-decode-execute loop.  Unlike
  `armv7-backend`'s `BX_LR` (ARM1 predates the `BX`/link-register
  return convention entirely), this is a simulator-level pseudo-halt
  rather than a real subroutine return.

### Tests

7 unit tests pin every constant, the canonical
`MOV R0, #42 = 0xE3A0_002A` value the ARM1 e2e smoke test pins, and
both constants' little-endian byte layout.
