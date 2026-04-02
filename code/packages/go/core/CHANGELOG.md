# Changelog

All notable changes to the `core` package will be documented in this file.

## [0.2.0] - 2026-04-02

### Changed
- Wrapped all public functions and methods with the Operations system (`StartNew`) for unified observability, capability enforcement, and telemetry tracing.
- `config.go`: `DefaultRegisterFileConfig()`, `DefaultCoreConfig()`, `SimpleConfig()`, `CortexA78LikeConfig()`, `DefaultMultiCoreConfig()` — wrapped with Operations.
- `decoder.go`: `NewMockDecoder()`, `MockDecoder.InstructionSize()`, `MockDecoder.Decode()`, `MockDecoder.Execute()`, all `Encode*()` helpers — wrapped with Operations.
- `interrupt_controller.go`: `NewInterruptController()`, `RaiseInterrupt()`, `Acknowledge()`, `PendingForCore()`, `PendingCount()`, `AcknowledgedCount()`, `Reset()` — wrapped with Operations.
- `memory_controller.go`: `NewMemoryController()`, `RequestRead()`, `RequestWrite()`, `Tick()`, `ReadWord()`, `WriteWord()`, `LoadProgram()`, `MemorySize()`, `PendingCount()` — wrapped with Operations.
- `register_file.go`: `NewRegisterFile()`, `Read()`, `Write()`, `Values()`, `Count()`, `Width()`, `Config()`, `Reset()`, `String()` — wrapped with Operations.
- `stats.go`: `CoreStats.IPC()`, `CoreStats.CPI()`, `CoreStats.String()` — wrapped with Operations.
- `core.go`: `NewCore()`, `LoadProgram()`, `Step()`, `Run()`, `Stats()`, `IsHalted()`, `ReadRegister()`, `WriteRegister()`, `RegisterFile()`, `MemoryController()`, `Cycle()`, `Config()`, `Pipeline()`, `Predictor()`, `CacheHierarchy()`, `EncodeProgram()` — wrapped with Operations.
- `multi_core.go`: `NewMultiCoreCPU()`, `LoadProgram()`, `Step()`, `Run()`, `Cores()`, `Stats()`, `InterruptController()`, `SharedMemoryController()`, `Cycle()`, `AllHalted()` — wrapped with Operations.
- Private callbacks and helpers remain unwrapped.

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
