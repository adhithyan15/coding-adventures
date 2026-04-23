# Changelog — compiler_ir (Elixir)

## 0.1.0 — 2026-04-11

Initial release: Elixir port of the Go `compiler-ir` package.

### Added

- `IrOp` module with all 25 opcode atoms (`:load_imm`, `:branch_nz`, etc.)
  and `to_string/1` / `parse/1` for roundtrip-safe text conversion.
- `IrRegister` struct — virtual register operand (`%IrRegister{index: 0}` → `"v0"`).
- `IrImmediate` struct — integer immediate operand.
- `IrLabel` struct — named label operand.
- `IrInstruction` struct — opcode + operands + unique ID.
- `IrDataDecl` struct — data segment declaration (`.data label size init`).
- `IrProgram` struct with `new/1`, `add_instruction/2`, `add_data/2`.
- `IDGenerator` struct with `new/0`, `new_from/1`, `next/1`, `current/1`.
- `Printer` module — renders `IrProgram` to canonical `.ir` text.
- `Parser` module — parses `.ir` text back to `IrProgram` (roundtrip-safe).
- Comprehensive ExUnit test suite covering all modules.
