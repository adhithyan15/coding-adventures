# Changelog

## 0.1.0 — 2026-03-19

Initial release. Faithful port of the Go `core` package (D05).

### Added

- `CoreConfig` — complete configuration for a processor core (pipeline, caches, registers, predictor, memory)
- `RegisterFileConfig` — register file configuration (count, width, zero register)
- `FPUnitConfig` — optional floating-point unit configuration
- `MultiCoreConfig` — multi-core processor configuration
- Preset configurations:
  - `simple_config` — MIPS R2000-like (5-stage, 4KB caches, static predictor)
  - `cortex_a78_like_config` — ARM Cortex-A78 approximation (13-stage, 64KB caches, 2-bit predictor)
  - `default_core_config` — minimal teaching core
  - `default_multi_core_config` — 2-core system
- `RegisterFile` — configurable register file with optional zero register (RISC-V convention)
- `MockDecoder` — simple ISA decoder supporting NOP, ADD, SUB, ADDI, LOAD, STORE, BRANCH, HALT
- `MemoryController` — serializes memory requests with configurable latency, supports async read/write
- `InterruptController` — routes interrupts to cores with acknowledge protocol
- `CoreStats` — aggregate statistics (IPC, CPI, cache stats, hazard counts, predictor accuracy)
- `Core` — complete processor core composing pipeline, predictor, hazard unit, cache hierarchy, register file, and clock
- `MultiCoreCPU` — multi-core processor with shared memory and interrupt controller
- Instruction encoding helpers: `encode_nop`, `encode_add`, `encode_sub`, `encode_addi`, `encode_load`, `encode_store`, `encode_branch`, `encode_halt`, `encode_program`
- Branch predictor factory (`create_branch_predictor`) supporting all predictor types
- Full test suite with 105 tests, 98%+ line coverage, 86%+ branch coverage
