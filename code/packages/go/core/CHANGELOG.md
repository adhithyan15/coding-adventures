# Changelog

All notable changes to the `core` package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- `CoreConfig` with full configuration for pipeline, branch predictor, hazard detection, register file, FP unit, and cache hierarchy.
- `Core` struct that composes pipeline (D04), branch predictor (D02), hazard detection (D03), cache hierarchy (D01), register file, clock, and memory controller into a working processor core.
- `Core.Step()` for cycle-by-cycle execution.
- `Core.Run()` for running until halt or max cycles.
- `Core.LoadProgram()` for loading machine code into memory.
- `Core.Stats()` for aggregate statistics from all sub-components.
- `ISADecoder` interface for pluggable instruction set architectures.
- `MockDecoder` supporting NOP, ADD, SUB, ADDI, LOAD, STORE, BRANCH, HALT for testing.
- Instruction encoding helpers: `EncodeNOP`, `EncodeADD`, `EncodeSUB`, `EncodeADDI`, `EncodeLOAD`, `EncodeSTORE`, `EncodeBRANCH`, `EncodeHALT`.
- `EncodeProgram()` for converting instruction sequences to byte slices.
- `RegisterFile` with configurable count, width, and zero-register convention.
- `MemoryController` with async request processing and configurable latency.
- `InterruptController` for routing interrupts to cores in multi-core systems.
- `MultiCoreCPU` connecting multiple cores to shared memory and optional L3 cache.
- `CoreStats` aggregating pipeline, predictor, cache, and hazard statistics.
- Preset configurations: `SimpleConfig()`, `CortexA78LikeConfig()`.
- `DefaultCoreConfig()` and `DefaultMultiCoreConfig()` for quick setup.
- Comprehensive test suite with 91%+ coverage.
