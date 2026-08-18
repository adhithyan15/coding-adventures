# Changelog — mips-r2000-encoder

## v0.1.0 — 2026-08-17 — initial carve-out

First lane of the 9-architecture expansion following the pattern
documented in `HISTORICAL-ARCH-BACKEND-MIGRATION.md`.

### Added

- Re-exports of `encode_addiu`, `encode_jr`, `assemble` from
  `mips-r2000-simulator::encoding`.
- Register-role constants: `ZERO` (0), `V0` (2), `V1` (3), `A0` (4),
  `SP` (29), `RA` (31), `TEMP_REGISTERS` (`[8..15]`, `$t0..$t7`).
- `RET_WORD = 0x03E0_0008` — `JR $ra`, the canonical MIPS R2000
  return-from-function.  Unlike RISC-V's `jalr` (which carries an
  immediate), `JR` has no immediate field, so this is a single fixed
  word rather than a function call result.

### Tests

8 unit tests pin every constant and the canonical
`ADDIU $v0, $zero, 42 = 0x2402_002A` value the MIPS R2000 e2e smoke
test pins, plus big-endian assembly.
