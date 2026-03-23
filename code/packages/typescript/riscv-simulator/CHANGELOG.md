# Changelog

All notable changes to the riscv-simulator package will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2026-03-19

### Added
- TypeScript port of the Python RISC-V RV32I simulator
- RiscVDecoder: decodes R-type, I-type, and system instructions
- RiscVExecutor: executes addi, add, sub, and ecall instructions
- RiscVSimulator: high-level wrapper combining CPU + RISC-V ISA
- Assembler helper functions: encodeAddi, encodeAdd, encodeEcall, assemble
- Full literate programming documentation with instruction encoding diagrams
- Comprehensive test suite ported from Python
