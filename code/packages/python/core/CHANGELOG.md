# Changelog

All notable changes to the `coding-adventures-core` package.

## [0.1.0] - 2026-03-19

### Added
- `CoreConfig` dataclass with all tunable parameters for a processor core.
- `RegisterFileConfig` and `RegisterFile` with configurable width, count, and zero-register convention.
- `FPUnitConfig` for optional floating-point unit configuration.
- `ISADecoder` protocol for pluggable instruction set architectures.
- `MockDecoder` with NOP, ADD, SUB, ADDI, LOAD, STORE, BRANCH, HALT instructions.
- Instruction encoding helpers: `encode_nop`, `encode_add`, `encode_sub`, `encode_addi`, `encode_load`, `encode_store`, `encode_branch`, `encode_halt`, `encode_program`.
- `MemoryController` with synchronous and asynchronous read/write, little-endian word access.
- `InterruptController` with interrupt routing, acknowledgment, and reset.
- `Core` class composing pipeline, branch predictor, hazard unit, cache hierarchy, register file, memory controller, and clock.
- `CoreStats` aggregating statistics from all sub-components with IPC/CPI computation.
- `MultiCoreCPU` connecting multiple cores to shared memory and interrupt controller.
- Preset configurations: `simple_config()` (MIPS R2000-like), `cortex_a78_like_config()` (ARM Cortex-A78-like).
- `default_core_config()` and `default_multi_core_config()` for testing.
- `create_branch_predictor()` factory for all supported predictor types.
- Comprehensive test suite with >80% coverage.
