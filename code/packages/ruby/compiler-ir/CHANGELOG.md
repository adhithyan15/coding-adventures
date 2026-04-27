# Changelog

## [0.1.0] - 2026-04-11

### Added

- `IrOp` module with 25 opcode constants (LOAD_IMM=0 .. COMMENT=24),
  `op_name/1` and `parse_op/1` helpers — exact port of Go `opcodes.go`
- `IrRegister`, `IrImmediate`, `IrLabel` frozen value objects (Data.define)
  with `to_s` producing "v0", "42", and "label_name" respectively
- `IrInstruction` Struct (opcode, operands, id)
- `IrDataDecl` Struct (label, size, init) for `.data` segment declarations
- `IrProgram` class (instructions, data, entry_label, version=1) with
  `add_instruction` and `add_data` mutation methods
- `IDGenerator` class with `next` and `current` — monotonic ID counter
  for source map chain linkage
- `IrPrinter.print(program) → String` — canonical text format printer
  matching Go's output byte-for-byte (11-char opcode column, "; #N" IDs)
- `IrParser.parse(text) → IrProgram` — text format parser with safety
  limits (MAX_LINES=1_000_000, MAX_OPERANDS=16, MAX_REGISTER=65535)
- Full print/parse roundtrip: `parse(print(prog))` produces structurally
  equal programs
- Comprehensive minitest suite (opcodes, types, printer, parser, roundtrip)
