# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `intel8008-encoder` crate: an instruction
  encoder for the Intel 8008 (1972), the second-generation 8-bit Intel
  microprocessor.
- `intel8008_encode_mvi_a` (2-byte immediate load) and `intel8008_encode_jmp` /
  `intel8008_encode_cal` (3-byte, 14-bit address low byte first then high 6
  bits, masked); opcode constants `INTEL8008_HLT` / `_RET` / `_MVI_A` / `_JMP` /
  `_CAL` and the `INTEL8008_GP_REGISTER_COUNT` / `_MVI_MAX` capacity constants.
- 16 checks mirroring the crate's doctests plus address-masking cases, run under
  every ISO C compiler via the shared `iso-harness`.
