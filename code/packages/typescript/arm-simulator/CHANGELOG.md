# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript port from Python implementation
- `ARMDecoder` class implementing `InstructionDecoder` for ARM data processing instructions
- `ARMExecutor` class implementing `InstructionExecutor` for MOV, ADD, SUB, HLT
- `ARMSimulator` high-level wrapper combining decoder, executor, and CPU
- Assembler helper functions: `encodeMovImm`, `encodeAdd`, `encodeSub`, `encodeHlt`, `assemble`
- Instruction encoding constants: `COND_AL`, `OPCODE_MOV`, `OPCODE_ADD`, `OPCODE_SUB`, `HLT_INSTRUCTION`
- Full test suite ported from Python and Go implementations
- Knuth-style literate programming comments explaining ARM architecture, encoding, and design decisions
