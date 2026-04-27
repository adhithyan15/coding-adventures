# Changelog — CodingAdventures::CompilerIr

## [0.01] — 2026-04-11

### Added

- Initial Perl port of the Go `compiler-ir` package.
- `IrOp` — 25 opcode constants matching Go iota sequence (0..24), plus `op_name()` and `parse_op()` bidirectional lookup.
- `IrRegister` — virtual register operand (`v0`, `v1`, ...); blessed hashref with `index` field and `to_string()`.
- `IrImmediate` — literal integer operand; blessed hashref with `value` field and `to_string()`.
- `IrLabel` — named label operand; blessed hashref with `name` field and `to_string()`.
- `IrInstruction` — single IR instruction: `opcode`, `operands`, `id` fields.
- `IrDataDecl` — data segment declaration: `label`, `size`, `init` fields.
- `IrProgram` — complete program container with `add_instruction()` and `add_data()` methods.
- `IDGenerator` — monotonic unique ID counter with `new()`, `new_from($start)`, `next()`, `current()`.
- `Printer` — `print_ir($program)` function producing canonical IR text matching the Go printer exactly.
- `Parser` — `parse_ir($text)` function reconstructing IrProgram from canonical IR text.
- `CodingAdventures::CompilerIr` — top-level module loading all sub-modules and re-exporting `print_ir`/`parse_ir`.
- Comprehensive test suite in `t/compiler_ir.t` covering all types, printer, parser, and roundtrip.
