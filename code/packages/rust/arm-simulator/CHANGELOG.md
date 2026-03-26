# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `ARMDecoder` -- decodes data processing instructions (MOV, ADD, SUB) with immediate rotate support
- `ARMExecutor` -- executes decoded instructions against registers
- `ARMSimulator` -- full simulation environment with 16 registers
- Encoding helpers: `encode_mov_imm`, `encode_add`, `encode_sub`, `encode_hlt`, `assemble`
- ARM immediate rotate decoding (4-bit rotate * 2 applied to 8-bit value)
- Comprehensive test suite including rotate edge cases
