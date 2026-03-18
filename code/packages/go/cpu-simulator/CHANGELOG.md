# Changelog

## [0.1.0] - Unreleased

### Added
- Created `cpu-simulator` generic architecture model.
- `Memory` simulated as byte-addressable array with Little Endian conventions.
- `RegisterFile` simulated with dynamic bit widths.
- Defined generic fetch-decode-execute `InstructionDecoder` and `InstructionExecutor` interfaces.
- `PipelineTrace` debugging formatting added.
