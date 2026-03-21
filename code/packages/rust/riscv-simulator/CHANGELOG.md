# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `RiscVDecoder` -- decodes I-type (addi), R-type (add, sub), and system (ecall) instructions
- `RiscVExecutor` -- executes decoded instructions against registers and memory
- `RiscVSimulator` -- full simulation environment wrapping the generic CPU
- Encoding helpers: `encode_addi`, `encode_add`, `encode_sub`, `encode_ecall`, `assemble`
- x0 hardwired-to-zero enforcement in all write paths
- Sign extension for 12-bit immediate values
- Comprehensive test suite covering normal operation, edge cases, and unknown instructions
