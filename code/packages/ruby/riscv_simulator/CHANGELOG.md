# Changelog

## [0.1.0] - 2026-03-18

### Added
- RiscVDecoder: decodes addi (I-type), add/sub (R-type), ecall
- RiscVExecutor: executes decoded RISC-V instructions with x0 hardwired to zero
- Assembler helpers: encode_addi, encode_add, encode_sub, encode_ecall, assemble
- RiscVSimulator: high-level wrapper combining CPU + RISC-V decoder/executor
