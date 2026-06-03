# Changelog — intel4004-encoder

## v0.1.0 — 2026-06-03 — initial carve-out from iir-to-intel4004 v0.3.0

Phase 4 of the historical-arch backend migration (see
`code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`).  Same byte
values as the deprecated `iir-to-intel4004`; this crate just
moves the encoding tables to the proper architectural layer.

### Added

- `LDM_OPCODE` (0xD0), `LD_OPCODE` (0xA0), `XCH_OPCODE` (0xB0),
  `JUN_OPCODE` (0x40).
- `HALT_LOOP = [0x40, 0x00]` (`JUN 0x000`).
- `GP_REGISTER_COUNT` (= 16), `LDM_MAX` (= 15), `LDM_MIN_SIGNED`
  (= -8).
- `encode_ldm`, `encode_ld`, `encode_xch`, `encode_jun`.

### Tests

9 unit tests + 1 doctest pin every opcode and every `encode_*`
byte output (including edge cases: zero, max 4-bit, overflow
masking, max 12-bit JUN address).
