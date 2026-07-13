# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `ge225-encoder` crate: an instruction encoder
  for the GE-225 (1959), the mainframe Dartmouth BASIC was designed on.
- `ge225_encode_lda` / `_sta` / `_ld` / `_add` / `_sub` (register ops masked to
  4 bits) and `_br` / `_bnz` / `_bz` / `_bmi` / `_jsr` (16-bit big-endian
  branches), each writing a 3-byte big-endian word; `ge225_decode_word` inverse.
- Opcode-nibble constants, `GE225_HALT_WORD` / `GE225_RTS_WORD`, and the
  `GE225_LDA_*` / `GE225_GP_REGISTER_COUNT` capacity constants.
- 23 checks mirroring the crate's doctests (LDA immediate, register masking,
  every branch, and the decode round-trip), run under every ISO C compiler via
  the shared `iso-harness`; also clean under ASan + UBSan.
