# Changelog

## 0.5.0 (2026-04-20)

### Added

- **`int_bits` parameter for `compile_basic()`.**  Controls how many decimal
  digit positions the `PRINT`-of-number unroller emits.  The rule is:
  `max_value = 2**(int_bits-1) - 1`, `digit_positions = len(str(max_value))`.
  Defaults to `32` (10 digits, covering the full 32-bit signed range used by
  the JVM and WASM backends).  Pass `int_bits=20` for the GE-225 backend (6
  digits, max value 524 287 — the GE-225's 20-bit signed word limit).

  **Why this matters**: every power-of-ten constant is emitted as a
  `LOAD_IMM`.  If the constant exceeds the target machine's signed word range,
  it is silently truncated, producing garbled digit extraction.  The 0.4.0 fix
  extended the power list to cover 32-bit integers but neglected the GE-225's
  narrower range.  This parameter makes the tradeoff explicit and enforced at
  the API boundary.

## 0.4.0 (2026-04-20)

### Fixed

- **`PRINT` of numbers ≥ 1,000,000 printed garbled output.**  The
  `_emit_print_number` function only extracted digits down to the
  hundred-thousands place (`100000, 10000, 1000, 100, 10`), so any value
  ≥ 1,000,000 had its leading digits mangled (e.g. `3628800` printed as
  `T28800`, where `T` = ASCII 84 = the result of treating the combined
  millions+hundred-thousands value 36 as a digit code).  The power list is
  extended to cover the full 32-bit signed integer range (max 2,147,483,647):
  `1_000_000_000, 100_000_000, 10_000_000, 1_000_000, 100_000, 10_000,
  1_000, 100, 10`.

## 0.3.0 (2026-04-20)

### Changed

- **SYSCALL instruction now carries the arg register as `operands[1]`.**
  All three SYSCALL emissions in the print pipeline (string char emit,
  loop-body digit emit, units-digit emit) now encode the virtual register
  holding the print argument as an explicit second operand: `SYSCALL 1, v0`
  instead of bare `SYSCALL 1`.  This makes the IR self-describing: WASM and
  JVM backends no longer need frontend-specific `syscall_arg_reg` config to
  know which register holds the character to print.

## 0.2.0 (2026-04-19)

### Added

- `PRINT expr` support: numeric variables, arithmetic expressions, and
  comma-separated mixed prints (`PRINT "LABEL", X`) now compile correctly.
  Each digit is extracted at runtime via an unrolled divide-and-modulo
  sequence; GE-225 digit codes 0–9 match the digit values exactly, so
  the quotient is loaded directly into v0 and dispatched via SYSCALL 1.
- Negative number printing: prints '-' (GE-225 code 0o33) then the
  absolute value; uses a CMP_LT + negate pattern.
- Leading-zero suppression: a single ADD trick (`r_dig + r_started == 0`)
  replaces a costlier AND-of-booleans for cleaner IR.

### Changed

- `_compile_print` restructured around `print_item` / `print_list` grammar
  nodes; now dispatches on STRING token vs. expression sub-tree.
- Two tests that previously expected `CompileError` for `PRINT variable`
  updated to verify the digit-extraction IR is emitted.

## 0.1.0 (2026-04-18)

Initial release of the Dartmouth BASIC IR compiler.

### Added

- `compile_basic(ast)` function: lowers a parsed Dartmouth BASIC AST to a target-independent `IrProgram`
- V1 statement support: `REM`, `LET`, `PRINT` (string literals), `GOTO`, `IF/THEN`, `FOR/NEXT`, `END`, `STOP`
- All six relational operators for `IF`: `<`, `>`, `=`, `<>`, `<=`, `>=`
- Expression compilation: addition, subtraction, multiplication, division, unary minus
- Fixed virtual register assignment: variables A–Z → v1–v26
- GE-225 typewriter code table for `PRINT` character encoding
- `CompileError` raised for V1-excluded features (GOSUB, DIM, INPUT, DEF FN, power operator)
- `CompileResult` dataclass with `program` and `var_regs`
