# Changelog — @coding-adventures/compiler-ir

## [0.1.0] — 2026-04-11

### Added

- `IrOp` enum with 25 opcodes (LOAD_IMM through COMMENT), matching the Go implementation exactly
- `IrRegister`, `IrImmediate`, `IrLabel` operand types as TypeScript discriminated union members with `kind` tags
- `IrOperand` union type for heterogeneous operand lists
- `IrInstruction` interface with `opcode`, `operands`, and `id` fields
- `IrDataDecl` interface for `.data` / `.bss` segment declarations
- `IrProgram` class with `addInstruction()` and `addData()` methods, `version` defaulting to 1
- `IDGenerator` class with `next()` and `current()` methods; supports custom start value
- Factory functions `reg()`, `imm()`, `lbl()` for constructing operands
- `operandToString()` for canonical operand text representation
- `opToString()` and `parseOp()` for opcode name roundtrip
- `printIr(program)` — IrProgram → canonical text format
- `parseIr(text)` — canonical text → IrProgram (roundtrip inverse)
- Safety limits in parser: 1,000,000 max lines, 16 max operands per instruction, 65535 max register index
- Comprehensive test suite with >95% coverage

### Implementation notes

- TypeScript uses a discriminated union (`kind` field) instead of Go's sealed interface pattern
- The `IrOp` enum uses explicit numeric values (0–24) to match Go iota order exactly
- `parseIr` handles negative immediates correctly using `String(val)` comparison
