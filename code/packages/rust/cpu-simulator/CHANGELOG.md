# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `RegisterFile` -- configurable register count and bit width, with overflow masking
- `Memory` -- byte-addressable RAM with little-endian 32-bit word read/write
- `PipelineStage` enum (Fetch, Decode, Execute)
- `FetchResult`, `DecodeResult`, `ExecuteResult`, `PipelineTrace` structs
- `format_pipeline()` -- visual multi-column pipeline trace formatter
- `InstructionDecoder` and `InstructionExecutor` traits for ISA abstraction
- `CPU` struct with `step()`, `run()`, `load_program()`, and `state()` methods
- `CPUState` snapshot struct
- Comprehensive unit tests for all modules
- Literate programming style documentation throughout
