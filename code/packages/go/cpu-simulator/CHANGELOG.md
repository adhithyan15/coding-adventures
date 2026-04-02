# Changelog

## [0.2.0] - 2026-04-02

### Changed
- Wrapped all public functions and methods with the Operations system (`StartNew`) for unified observability, capability enforcement, and telemetry tracing.
- `NewCPU()`, `CPU.State()`, `CPU.LoadProgram()`, `CPU.Step()`, `CPU.Run()` — wrapped with Operations.
- `NewMemory()`, `Memory.ReadByte()`, `Memory.WriteByte()`, `Memory.ReadWord()`, `Memory.WriteWord()`, `Memory.LoadBytes()`, `Memory.Dump()` — wrapped with Operations.
- `NewRegisterFile()`, `RegisterFile.Read()`, `RegisterFile.Write()`, `RegisterFile.Dump()` — wrapped with Operations.
- `PipelineTrace.FormatPipeline()` — wrapped with Operations.
- `NewSparseMemory()`, `SparseMemory.ReadByte()`, `SparseMemory.WriteByte()`, `SparseMemory.ReadWord()`, `SparseMemory.WriteWord()`, `SparseMemory.LoadBytes()`, `SparseMemory.Dump()`, `SparseMemory.RegionCount()` — wrapped with Operations.
- Private methods (`checkAddress`, `findRegion`) remain unwrapped.

## [0.1.0] - Unreleased

### Added
- Created `cpu-simulator` generic architecture model.
- `Memory` simulated as byte-addressable array with Little Endian conventions.
- `RegisterFile` simulated with dynamic bit widths.
- Defined generic fetch-decode-execute `InstructionDecoder` and `InstructionExecutor` interfaces.
- `PipelineTrace` debugging formatting added.
