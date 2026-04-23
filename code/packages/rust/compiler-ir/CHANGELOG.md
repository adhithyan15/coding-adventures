# Changelog — compiler-ir

## [0.1.0] — 2026-04-11

Initial release: Rust port of the `compiler-ir` Go package.

### Added

- `IrOp` enum with 25 opcodes across 7 categories (constants, memory, arithmetic,
  comparison, control flow, system, meta)
- `IrOp::Display` trait for canonical text names (`LOAD_IMM`, `BRANCH_Z`, etc.)
- `parse_op()` function for text-name → `IrOp` conversion (inverse of `Display`)
- `IrOperand` enum: `Register(usize)`, `Immediate(i64)`, `Label(String)`
- `IrOperand::Display` for canonical text (`v0`, `42`, `_start`)
- `IrInstruction` struct with opcode, operands, and unique monotonic ID
- `IrDataDecl` struct for named data segment declarations
- `IrProgram` struct with instructions, data, entry label, and version
- `IdGenerator` for monotonic instruction ID generation with `from_start()` support
- `print_ir()` — serializes `IrProgram` to canonical human-readable text
- `parse_ir()` — deserializes canonical text back to `IrProgram` (roundtrip)
- Safety limits in parser: max 1,000,000 lines, 16 operands/instruction, register index ≤ 65,535
- 54 unit tests + 11 doc tests (100% pass rate)
