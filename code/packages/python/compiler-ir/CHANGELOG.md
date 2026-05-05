# Changelog

## [Unreleased]

### Added — VMCOND00 Phase 2: THROW opcode

Adds `IrOp.THROW` (66) implementing VMCOND00 Layer 2 — unwind exceptions — in
the compiler IR opcode set shared by the JVM, CLR, and BEAM backends.

- **`IrOp.THROW` (66)** — raise a condition value and unwind the call stack
  until a matching handler is found.  Operand layout:
  `THROW condition_reg`
  where `condition_reg` is a register holding the condition object (or tagged
  integer sentinel).  The opcode does not have a destination register — control
  exits the current instruction stream entirely.

  Backend lowering strategies (for reference):

  - **vm-core**: `handle_throw` walks the static `IIRFunction.exception_table`
    from innermost to outermost frame, matching `[from_ip, to_ip)` and
    `type_id`; jumps to `handler_ip` and writes the condition into `val_reg`.
  - **JVM**: lower to `athrow` after boxing the condition into a synthetic
    `LangCondition` class that extends `Throwable`; handler entries become
    JVM exception table rows.
  - **CLR**: lower to `throw` after boxing into a synthetic
    `LangCondition : Exception` type; handler entries become CIL `.try / catch`
    regions.

The total opcode count grows from 66 → 67.  All existing opcode IDs (0–65)
remain stable; serialized IR text files round-trip unchanged.

The corresponding interpreter-IR mnemonic (`"throw"` added to `THROW_OPS`) and
vm-core dispatch handler ship in the same PR.

**Spec reference:** VMCOND00 §3 Layer 2 — unwind exceptions.

---

### Added — VMCOND00 Phase 1: SYSCALL_CHECKED and BRANCH_ERR opcodes

Two new opcodes implementing the VMCOND00 Layer 1 result-value error protocol.
Languages that opt in to this layer can invoke host syscalls without trapping
and inspect the error code with a dedicated conditional branch.  The rest of
the IR is unchanged — programs that don't use these opcodes are unaffected.

- **`IrOp.SYSCALL_CHECKED` (64)** — Invoke a SYSCALL00-numbered host syscall
  without trapping on errors.  Operand layout:
  `SYSCALL_CHECKED n, arg_reg, val_dst, err_dst`
  - `n`       — SYSCALL00 canonical syscall number (immediate)
  - `arg_reg` — register holding the single argument
  - `val_dst` — register to receive the success value (0 on error)
  - `err_dst` — register to receive the error code: 0 ok, -1 EOF, <-1 negated errno

- **`IrOp.BRANCH_ERR` (65)** — Branch to a label when an error register is
  non-zero.  Operand layout: `BRANCH_ERR err_reg, label`.  Falls through when
  `err_reg == 0` (success); jumps when `err_reg != 0` (syscall failed).

The total opcode count grows from 64 → 66.  All existing opcode IDs (0–63)
remain stable; serialized IR text files round-trip unchanged.

These opcodes map to the **compiler IR** world (AOT compilation via
`ir-to-jvm-class-file`, `ir-to-cil-bytecode`, `ir-to-beam`).  The
interpreter IR (``interpreter-ir``) gets the corresponding ``syscall_checked``
and ``branch_err`` string mnemonics in the same PR.

**Spec reference:** VMCOND00 §3 Layer 1 — result values; SYSCALL00 §2.

---

### Added — TW03 Phase 3a heap-primitive ops

Eight new opcodes (55–62) introducing the cross-backend Lisp
heap-primitive interface.  See
[TW03-phase3-heap-primitives.md](../../../specs/TW03-phase3-heap-primitives.md)
for the multi-backend lowering plan.

- **`IrOp.MAKE_CONS` (55)** — allocate a cons cell.  Operand layout:
  `MAKE_CONS dst, head_reg, tail_reg`.
- **`IrOp.CAR` (56)** — read the head of a cons cell.
  `CAR dst, src`.
- **`IrOp.CDR` (57)** — read the tail of a cons cell.
  `CDR dst, src`.
- **`IrOp.IS_NULL` (58)** — sets `dst=1` if `src` is the nil sentinel,
  else 0.  Result feeds straight into `BRANCH_Z` / `BRANCH_NZ`.
- **`IrOp.IS_PAIR` (59)** — sets `dst=1` if `src` is a cons cell.
- **`IrOp.MAKE_SYMBOL` (60)** — intern a symbol named by an `IrLabel`
  (reuses the existing label round-trip path).  `MAKE_SYMBOL dst, name_label`.
- **`IrOp.IS_SYMBOL` (61)** — sets `dst=1` if `src` is a symbol.
- **`IrOp.LOAD_NIL` (62)** — store the nil sentinel into `dst`.

These are the **cross-backend interface** for TW03 Phase 3.
Per-backend lowering ships in subsequent phases (JVM03 / CLR03 /
BEAM03).  Adding new opcodes does not touch existing 0–54 IDs, so
older serialized IR text round-trips unchanged.

### Added — earlier (rolled into 0.5.0)

- **`IrOp.F64_SQRT` (49)** — unary 64-bit floating square root
  (`dst = sqrt(src)`) for frontends that need standard real math builtins.
- **`IrOp.F64_SIN` (50)**, **`IrOp.F64_COS` (51)**,
  **`IrOp.F64_ATAN` (52)**, **`IrOp.F64_LN` (53)**, and
  **`IrOp.F64_EXP` (54)** — unary 64-bit floating standard math operations
  for frontends with real numeric libraries.
- **`IrOp.F64_POW` (63)** — binary 64-bit floating power operation
  (`dst = pow(base, exponent)`) for frontends with real exponentiation.

## [0.4.0] — 2026-04-29

### Added — TW03 Phase 2 closure ops

- **``IrOp.MAKE_CLOSURE`` (47)** — construct a closure value
  capturing free variables from the enclosing scope.  Operand
  layout: ``MAKE_CLOSURE dst, fn_label, num_captured, capt0, capt1, ...``
- **``IrOp.APPLY_CLOSURE`` (48)** — invoke a closure value with
  zero or more arguments.  Operand layout:
  ``APPLY_CLOSURE dst, closure_reg, num_args, arg0, arg1, ...``

These ops are the **cross-backend interface** for TW03 Phase 2.
Lowering strategies differ per backend:

- JVM/CLR — closure becomes an object reference; per-lambda
  class with captured fields + ``apply`` method.  See
  ``code/specs/JVM02-phase2-multi-class-closure-lowering.md``.
- BEAM — emit ``make_fun2`` referencing a ``FunT`` chunk row.
- vm-core — delegate to the host-side ``make_closure`` /
  ``apply_closure`` builtins (already implemented in TW00).

Backends that don't yet support these ops should reject via
their pre-flight validator with a clear "TW03 Phase 2: closures
not yet implemented for this backend" message — never silently
miscompile.

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
