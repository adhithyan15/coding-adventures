# Changelog — compiler-ir

## [0.2.0] — 2026-05-11

### Added — Bitwise OR, XOR, NOT opcodes

- `IrOp::Or` — register-register bitwise OR (`OR v3, v1, v2 → v3 = v1 | v2`).
- `IrOp::OrImm` — register-immediate bitwise OR (`OR_IMM v2, v2, 1 → v2 = v2 | 1`).
- `IrOp::Xor` — register-register bitwise XOR (`XOR v3, v1, v2 → v3 = v1 ^ v2`).
- `IrOp::XorImm` — register-immediate bitwise XOR (`XOR_IMM v2, v2, 1 → v2 = v2 ^ 1`).
- `IrOp::Not` — bitwise NOT / one's complement (`NOT v1, v2 → v1 = ~v2`).
  Equivalent to `XOR v1, v2, -1`; backends must produce a bitwise complement
  (not a logical negation).

All five ops added to `Display` and `parse_op()`.

Opcode count: 27 → 32.  Regression test updated to `test_opcode_count_is_32`.

---

## [0.1.1] — 2026-04-28

### Added

- `IrOp::Mul` — register-register signed multiplication (`MUL v3, v1, v2 → v3 = v1 * v2`).
- `IrOp::Div` — register-register signed integer division (`DIV v3, v1, v2 → v3 = v1 / v2`, truncates toward zero).
- Both opcodes added to `Display` (`"MUL"`, `"DIV"`) and `parse_op()`.

These opcodes are required by the Dartmouth BASIC IR compiler for the `*`/`/`
operators and for the unrolled decimal digit-extraction routine used by
`PRINT` of numeric expressions.

---

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
