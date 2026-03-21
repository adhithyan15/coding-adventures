# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-21

### Added

- Initial Elixir port of the core package (from Go D05 implementation)
- `CodingAdventures.Core.Config` -- CoreConfig with presets (Simple, CortexA78Like, Default)
  - `RegisterFileConfig` -- register count, width, zero register convention
  - `FPUnitConfig` -- floating-point unit configuration (optional)
  - `MultiCoreConfig` -- multi-core processor configuration
- `CodingAdventures.Core.Decoder` -- ISADecoder behaviour (Elixir equivalent of Go interface)
- `CodingAdventures.Core.MockDecoder` -- test decoder with 8 instruction types (NOP, ADD, SUB, ADDI, LOAD, STORE, BRANCH, HALT)
  - Instruction encoding/decoding helpers
  - Sign-extended 12-bit immediates
  - `encode_program/1` for byte-level program encoding
- `CodingAdventures.Core.RegisterFile` -- configurable register file
  - Zero register convention (RISC-V/MIPS style)
  - Bit-width masking (8, 32, 64-bit support)
  - Defensive out-of-range handling
- `CodingAdventures.Core.MemoryController` -- memory access with latency simulation
  - Immediate word read/write (little-endian)
  - Async read/write requests with configurable latency
  - Program loading
- `CodingAdventures.Core.InterruptController` -- interrupt routing for multi-core
  - Raise, acknowledge, and route interrupts to cores
  - Default routing to core 0 for unspecified targets
- `CodingAdventures.Core.Core` -- complete processor core composition
  - Wires pipeline, register file, memory, and ISA decoder
  - Step-by-step and run-to-halt execution
  - Uses Agent for stateful callback integration with functional pipeline
- `CodingAdventures.Core.MultiCore` -- multi-core processor
  - Multiple independent cores with shared memory
  - Per-core statistics
  - Shared memory controller and interrupt controller
- `CodingAdventures.Core.Stats` -- aggregate performance statistics (IPC, CPI)
- Comprehensive ExUnit test suite covering all modules
