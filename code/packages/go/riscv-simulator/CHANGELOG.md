# Changelog

## [0.1.0] - Unreleased

### Added
- `RiscVDecoder` implementing RISC-V RV32I opcodes mapping.
- `RiscVExecutor` mapping decoded `cpu.DecodeResult` to arithmetic manipulations bridging the `cpu-simulator` generic execution loop.
- Enforcement of `x0` constant hardwiring to zero.
- Complete documentation adhering to literate programming standards.
- Helper testing encoders explicitly producing Little Endian binary sequences.
