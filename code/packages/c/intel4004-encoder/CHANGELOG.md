# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `intel4004-encoder` crate: an instruction
  encoder for the Intel 4004 (1971), the world's first commercial microprocessor.
- `intel4004_encode_ldm` / `_ld` / `_xch` (single-byte, low nibble masked) and
  `intel4004_encode_jun` (2-byte, address masked to 12 bits); the
  `INTEL4004_HALT_LOOP` (`JUN 0x000`) constant.
- Opcode constants (`INTEL4004_LDM_OPCODE` / `_LD_OPCODE` / `_XCH_OPCODE` /
  `_JUN_OPCODE`) and the `INTEL4004_GP_REGISTER_COUNT` / `_LDM_MAX` /
  `_LDM_MIN_SIGNED` capacity constants.
- 17 checks mirroring the crate's doctests, run under every ISO C compiler via
  the shared `iso-harness`.
