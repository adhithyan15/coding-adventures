# Changelog

## 0.1.0 — 2026-03-19

### Added
- Initial release
- `ProcessingElement` protocol — generic interface for any accelerator PE
- `InstructionSet` protocol — pluggable instruction set architecture
- `GPUCore` — configurable processing element with fetch-execute loop
- `GenericISA` — educational instruction set with 16 opcodes
- `FPRegisterFile` — configurable FP register file (1-256 registers, FP32/FP16/BF16)
- `LocalMemory` — byte-addressable scratchpad with FP-aware load/store
- `GPUCoreTrace` — execution trace for educational visibility
- Helper constructors: `fadd`, `fmul`, `ffma`, `limm`, `halt`, etc.
