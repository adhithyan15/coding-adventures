# Changelog

All notable changes to the `gpu-core` Go package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial Go port of the Python `gpu-core` package.
- `InstructionSet` and `ProcessingElement` interfaces (`protocols.go`).
- `ExecuteResult` struct with `NewExecuteResult()` constructor.
- `Opcode` enum with all 16 opcodes using `iota` (`opcodes.go`).
- `Instruction` struct with assembly-like `String()` formatting.
- Helper constructors for all 16 opcodes: `Fadd()`, `Fsub()`, `Fmul()`, `Ffma()`, `Fneg()`, `Fabs()`, `Load()`, `Store()`, `Mov()`, `Limm()`, `Beq()`, `Blt()`, `Bne()`, `Jmp()`, `Nop()`, `Halt()` (`helpers.go`).
- `FPRegisterFile` with configurable size (1-256) and format (`registers.go`).
- `LocalMemory` with byte-level and float-level access (`memory.go`).
- `GenericISA` implementing all 16 opcodes using `fp-arithmetic` (`generic_isa.go`).
- `GPUCore` with functional options pattern, fetch-execute loop, `Run()`, `Reset()` (`core.go`).
- `GPUCoreTrace` with `Format()` for educational display (`trace.go`).
- Comprehensive test suite covering all files plus end-to-end program tests.
- BUILD file for CI integration.
