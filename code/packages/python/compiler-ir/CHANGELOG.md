# Changelog

## [0.1.0] - 2026-04-11

### Added

- Initial Python port of the Go `compiler-ir` package
- `IrOp` IntEnum with 25 opcodes (LOAD_IMM through COMMENT)
- `IrRegister`, `IrImmediate`, `IrLabel` frozen dataclasses for operands
- `IrInstruction` dataclass with opcode, operands list, and unique ID
- `IrDataDecl` dataclass for static data segment declarations
- `IrProgram` dataclass with `add_instruction()` and `add_data()` methods
- `IDGenerator` class producing monotonic unique instruction IDs
- `print_ir()` function converting IrProgram to canonical text format
- `parse_ir()` function converting canonical text back to IrProgram
- `IrParseError` exception for malformed IR text
- `parse_op()` helper converting opcode name strings to IrOp values
- `NAME_TO_OP` and `OP_NAMES` dictionaries for name↔opcode conversion
- Comprehensive test suite (95%+ coverage)
