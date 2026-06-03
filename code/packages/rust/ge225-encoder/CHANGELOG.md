# Changelog — ge225-encoder

## v0.1.0 — 2026-06-02 — initial carve-out from `iir-to-ge225` v0.9.0

Phase 1 of the historical-arch backend migration (see
`code/specs/HISTORICAL-ARCH-BACKEND-MIGRATION.md`).

### Added

- `LDA_OPCODE_NIBBLE` (= `0x1`), `STA_OPCODE_NIBBLE` (= `0x2`),
  `LD_OPCODE_NIBBLE` (= `0x3`), `ADD_OPCODE_NIBBLE` (= `0x4`),
  `SUB_OPCODE_NIBBLE` (= `0x5`), `BR_OPCODE_NIBBLE` (= `0x6`),
  `BNZ_OPCODE_NIBBLE` (= `0x7`), `BZ_OPCODE_NIBBLE` (= `0x8`),
  `JSR_OPCODE_NIBBLE` (= `0x9`), `RTS_OPCODE_NIBBLE` (= `0xA`),
  `BMI_OPCODE_NIBBLE` (= `0xB`).
- `HALT_WORD` (= `[0x00, 0x00, 0x00]`).
- `RTS_WORD` (= `[0x0A, 0x00, 0x00]`).
- `GP_REGISTER_COUNT` (= 16), `LDA_MAX_SIGNED`, `LDA_MIN_SIGNED`,
  `LDA_MAX_UNSIGNED`.
- `encode_lda`, `encode_sta`, `encode_ld`, `encode_add`,
  `encode_sub`, `encode_br`, `encode_bnz`, `encode_bz`,
  `encode_bmi`, `encode_jsr`.
- `decode_word` — round-trip decoder used by downstream
  simulators and tests.

### Source

Carved out of `iir-to-ge225` v0.9.0.  Byte sequences are unchanged;
the constants and `encode_*` helpers in `iir-to-ge225` are being
re-pointed at this crate in the same PR so there's a single source
of truth.

### Tests

20 unit tests pin every opcode nibble, every `encode_*` byte
sequence (including edge cases: zero, max-positive `i16`,
min-negative `i16`, max-unsigned `u16`, byte-boundary `0x00FF` /
`0x0100`, max-address `0xFFFF`), and the `decode_word` round-trip
against every `encode_*` helper.
