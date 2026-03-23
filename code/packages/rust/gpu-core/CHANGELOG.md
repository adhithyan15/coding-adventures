# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial Rust port of the Python `gpu-core` package.
- `protocols` module: `ProcessingElement` and `InstructionSet` traits, `ExecuteResult` type with builder pattern.
- `registers` module: `FPRegisterFile` with configurable register count (1-256) and float format (FP32/FP16/BF16).
- `memory` module: `LocalMemory` byte-addressable scratchpad with FP-aware load/store.
- `opcodes` module: `Opcode` enum (16 opcodes), `Instruction` struct, helper constructors (`fadd`, `fsub`, `fmul`, `ffma`, `fneg`, `fabs`, `load`, `store`, `mov`, `limm`, `beq`, `blt`, `bne`, `jmp`, `nop`, `halt`).
- `generic_isa` module: `GenericISA` implementing all 16 opcodes with educational trace descriptions.
- `core` module: `GPUCore` with pluggable ISA, configurable registers/memory, fetch-execute loop, run with step limit.
- `trace` module: `GPUCoreTrace` with `format()` for educational display.
- Comprehensive unit tests in each module.
- Integration tests covering arithmetic, memory, control flow, loops, dot product, and edge cases.
- BUILD file for CI integration.
