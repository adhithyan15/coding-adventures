# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-09-01

### Added

- Exact 64 KiB `functional::Armv7Simulator` with 16 32-bit registers, mirrored
  R15/PC, NZCV flags, halt state, and installed-program metadata
- Validated restore, origin-aware loading, checked little-endian memory and
  register access, typed failures, atomic steps, transactional bounded runs,
  and complete before/after traces and results
- MOV, ADD, SUB, CMP, AND, ORR, LDR, STR, B, BL, all sixteen conditions, the S
  bit, rotated immediates, and the custom HLT contract from Spec 07b
- Structured instruction encoders, five lifecycle suites, five direct Spec 07b
  suites, and 388 reproducible Python common-surface full-state vectors
- Strict formatting, Clippy, and rustdoc verification with 90.15% line coverage
  (714/792 lines)

### Fixed

- Unknown or unsupported instructions now fail closed in the checked lane
  instead of silently advancing
- The crate-level condition-bit prose no longer forms a broken rustdoc link

### Compatibility

- The original `ARMSimulator` API remains available unchanged for consumers

## [0.1.0] - 2026-03-19

### Added

- `ARMDecoder` -- decodes data processing instructions (MOV, ADD, SUB) with immediate rotate support
- `ARMExecutor` -- executes decoded instructions against registers
- `ARMSimulator` -- full simulation environment with 16 registers
- Encoding helpers: `encode_mov_imm`, `encode_add`, `encode_sub`, `encode_hlt`, `assemble`
- ARM immediate rotate decoding (4-bit rotate * 2 applied to 8-bit value)
- Comprehensive test suite including rotate edge cases
