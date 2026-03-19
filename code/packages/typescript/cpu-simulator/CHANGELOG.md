# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `RegisterFile` class: configurable number of registers and bit width, read/write with bounds checking, bit-width masking, dump for inspection.
- `Memory` class: byte-addressable memory with readByte/writeByte, readWord/writeWord (little-endian), loadBytes for program loading, dump for debugging.
- `PipelineStage` enum: FETCH, DECODE, EXECUTE stages.
- `FetchResult`, `DecodeResult`, `ExecuteResult` interfaces for pipeline stage outputs.
- `PipelineTrace` interface capturing complete instruction execution history.
- `formatPipeline()` function for visual pipeline diagram output.
- `InstructionDecoder` and `InstructionExecutor` interfaces for ISA-agnostic design.
- `CPU` class: generic CPU with step() for single-instruction execution, run() for full program execution, loadProgram() for memory initialization, state snapshot via `state` property.
- `CPUState` interface for point-in-time CPU snapshots.
- Full test suite covering registers, memory, CPU pipeline, and state management.
- Knuth-style literate programming comments throughout all source files.

### Notes

- Ported from the Python cpu-simulator package.
- Depends on `@coding-adventures/arithmetic` package.
