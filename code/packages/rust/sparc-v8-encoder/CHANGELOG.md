# Changelog — sparc-v8-encoder

## v0.1.0 — 2026-08-17 — initial carve-out

Sixth lane of the 9-architecture expansion following the pattern
documented in `HISTORICAL-ARCH-BACKEND-MIGRATION.md`.

### Added

- Re-exports of `encode_add_imm`, `encode_ta`, `assemble` from
  `sparc_v8_simulator`.
- Register-role constants: `G0` (0, hardwired zero) and `O0` (8, the
  SPARC calling-convention return-value register).
- `HALT_WORD = 0x91D0_2000` — `ta 0` (trap always, software trap #0),
  the HALT sentinel `sparc-v8-simulator` intercepts to stop the
  fetch-decode-execute loop.  Matches the Python original
  (`sparc_v8_simulator.state.HALT_WORD`).

### Tests

6 unit tests pin every constant, the canonical
`ADD %g0, 42, %o0 = 0x9000_202A` value the SPARC V8 e2e smoke test
pins, and both constants' big-endian byte layout.
