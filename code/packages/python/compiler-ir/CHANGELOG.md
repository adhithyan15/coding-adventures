# Changelog

## [0.3.0] - 2026-04-20

### Added

- **`IrOp.OR` (27)** — register-register bitwise OR (`dst = lhs | rhs`).
  Required by the Oct language (`a | b`) and the Intel 8008 `ORA r` instruction.
  Backends that do not support OR should reject programs via their pre-flight
  validator.

- **`IrOp.OR_IMM` (28)** — register-immediate bitwise OR (`dst = src | imm`).
  Useful for setting specific bits without a scratch register.

- **`IrOp.XOR` (29)** — register-register bitwise XOR (`dst = lhs ^ rhs`).
  Required by the Oct language (`a ^ b`) and the Intel 8008 `XRA r` instruction.
  Also useful for zero-testing (XOR a register with itself clears it and sets Z).

- **`IrOp.XOR_IMM` (30)** — register-immediate bitwise XOR (`dst = src ^ imm`).
  The canonical NOT-a-byte idiom on 8-bit targets is `XOR_IMM dst, src, 0xFF`
  (flip all 8 bits).  On the Intel 8008, the backend lowers this to `XRI 0xFF`.

- **`IrOp.NOT` (31)** — bitwise complement (`dst = ~src`).
  Flips every bit in `src`.  On targets without a dedicated NOT instruction
  (e.g. Intel 8008), the backend lowers this to `XOR_IMM dst, src, word_mask`
  where `word_mask` is the all-ones value for the target word width.

- `TestBitwiseOpcodes` test class (22 tests) covering integer values,
  name round-trips, print/parse round-trips, operand shapes, NAME_TO_OP /
  OP_NAMES membership, distinctness, and collision-freedom.

### Changed

- `test_total_opcode_count` updated: 27 → 32 total opcodes.
- `test_opcode_integer_values` extended with assertions for all five new opcodes.
- `test_all_opcodes_roundtrip` extended with operand fixtures for all five
  new opcodes.
- Opcode Groups docstring in `opcodes.py` updated to list the new `Bitwise`
  group alongside the existing categories.

## [0.2.0] - 2026-04-13

### Added

- **`IrOp.MUL` (25)** — register-register multiplication (`dst = lhs * rhs`,
  signed integer; result is the low word of the product on fixed-width targets).
  Added for Dartmouth BASIC multiplication expressions.

- **`IrOp.DIV` (26)** — register-register integer division (`dst = lhs / rhs`,
  truncates toward zero).  Division by zero is a runtime error; backends are
  responsible for detection or documentation.  Added for Dartmouth BASIC integer
  division.

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
