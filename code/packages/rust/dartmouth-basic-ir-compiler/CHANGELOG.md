# Changelog — dartmouth-basic-ir-compiler

## [0.1.0] — 2026-04-28

### Added

- Initial release: Dartmouth BASIC → IrProgram compiler in Rust.

#### Pipeline

Compilation pipeline:

1. **Parse** — `coding_adventures_dartmouth_basic_parser::parse_dartmouth_basic` lexes and parses BASIC source into a grammar AST.
2. **Compile** — `compile_basic_source` / `compile_basic_with_options` walks the AST and emits a target-independent `IrProgram`.

#### Public API

- `compile_basic_source(source)` — free-function shorthand with default options (GE-225 encoding, 32-bit integers).
- `compile_basic_with_options(source, char_encoding, int_bits)` — compile with explicit `CharEncoding` and integer bit width.
- `CharEncoding` — `Ge225` (6-bit typewriter codes) or `Ascii` (standard bytes).
- `CompileResult { program, var_regs }` — IR program and variable register map.
- `CompileError { message }` — describes which V1 feature was unsupported.
- `ascii_to_ge225(ch)` — converts an ASCII character to its GE-225 typewriter code.
- `scalar_reg(name)` — maps a BASIC variable name to its fixed virtual register index.

#### Virtual register layout

```
v0        — syscall argument
v1–v26    — scalar variables A–Z
v27–v286  — two-character variables A0–Z9
v287+     — expression temporaries
```

#### Supported V1 statements

REM, LET, PRINT (strings + numeric expressions), GOTO, IF…THEN (all 6 relational
operators), FOR…TO [STEP], NEXT, END, STOP.

#### compiler-ir extension

Added `IrOp::Mul` and `IrOp::Div` opcodes to the shared `compiler-ir` crate.
These are required for BASIC's `*`/`/` operators and for the unrolled decimal
digit-extraction routine used by PRINT of numeric expressions.

#### Design notes

- **Fixed register layout** — every BASIC variable gets a permanent virtual register.  GOTO-based control flow always reads/writes the correct register without liveness analysis.
- **Unrolled print-number** — `_emit_print_number` generates a compile-time-unrolled digit-extraction loop sized to `int_bits`.  `int_bits=20` for GE-225 (6 positions), `int_bits=32` default (10 positions).
- **`<=` / `>=` synthesised** — IR has no `CMP_LE`/`CMP_GE`; these are expressed as `NOT(CMP_GT)` / `NOT(CMP_LT)` using the two-instruction boolean-flip idiom `(v−1)&1`.
- **`CharEncoding`** — `Ge225` emits 6-bit typewriter codes (GE-225 backend); `Ascii` emits standard ASCII bytes (WASM/WASI `fd_write`).

#### Tests

43 tests (36 unit + 7 doc-tests), all passing:

- `_start` label emitted at program entry.
- `_line_N` label emitted for each BASIC line number.
- `var_regs` maps A→v1, B→v2, Z→v26.
- `scalar_reg` correctly maps single-letter and two-character names.
- `ascii_to_ge225` handles uppercase, lowercase, digits, unsupported chars.
- REM emits COMMENT instruction.
- LET emits LOAD_IMM + ADD_IMM.
- LET with `*` emits MUL; LET with `/` emits DIV.
- PRINT string emits correct number of SYSCALL instructions.
- Bare PRINT emits exactly one SYSCALL (carriage return only).
- PRINT number emits DIV and MUL for digit extraction.
- GOTO emits JUMP to `_line_N`.
- IF `<` emits CMP_LT + BRANCH_NZ.
- IF `<=` uses the NOT-bool idiom (AND_IMM present).
- IF `=` emits CMP_EQ; IF `<>` emits CMP_NE.
- FOR/NEXT emits CMP_GT, BRANCH_NZ, ADD (increment), JUMP (back-edge).
- FOR STEP 2 emits LOAD_IMM 2.
- NEXT without FOR returns CompileError.
- END and STOP emit HALT.
- GOSUB returns CompileError mentioning "GOSUB".
- `^` operator returns CompileError.
- `int_bits < 2` returns error.
- ASCII encoding uses newline (10) for carriage-return.
- Unary minus emits SUB.
- Multi-line program with IF loop compiles successfully.
