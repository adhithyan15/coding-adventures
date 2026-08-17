# Changelog — m68k-encoder

## [0.1.0] - 2026-08-17

### Added

- Thin re-export shim over `m68k_simulator::encoding`:
  `encode_move_l_imm_to_dn`, `encode_moveq`, `encode_trap15`,
  `encode_nop`, `encode_rts`, `assemble`.
- `D0` — the register-role constant `m68k-backend` writes `const_*`
  results into (the 68000's conventional scratch/return-value register,
  matching `arm1-encoder::R0`/`mips_r2000_encoder::V0`'s role).
- `HALT_BYTES` — the 2-byte `TRAP #15` HALT sentinel encoding
  (`[0x4E, 0x4F]`), big-endian (the 68000's native byte order — no
  endianness flip needed the way `arm1_encoder::HALT_WORD` needs one for
  ARM1's little-endian words).
- 5 unit tests + 1 doctest pinning the canonical `MOVE.L #42, D0`
  encoding and the halt-byte constant's consistency with `encode_trap15`.

Eighth lane of the 9-architecture expansion following the pattern
documented in
[`HISTORICAL-ARCH-BACKEND-MIGRATION.md`](../../../specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md).
