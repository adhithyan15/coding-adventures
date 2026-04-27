# Changelog — nib-ir-compiler

All notable changes to this package are documented here.
This project follows [Semantic Versioning](https://semver.org/).

## [0.1.0] — 2026-04-12

### Added

- Initial release: `compile_nib(typed_ast, config?) -> CompileResult`.
- `BuildConfig` dataclass with `insert_debug_comments` flag.
- `debug_config()` / `release_config()` factory functions.
- `CompileResult` dataclass with `program: IrProgram` and
  `source_map: SourceMapChain | None`.
- Fixed virtual register allocation: `v0 = zero, v1 = scratch, v2+ = named vars`.
- Support for all Nib v1 statements:
  - `let NAME: type = expr` — allocates a register, compiles initializer.
  - `NAME = expr` — assignment with register copy.
  - `return expr` — moves result to v1, emits RET.
  - `for i: type in start..end { body }` — loop with LOAD_IMM, CMP_LT,
    BRANCH_Z, body, ADD_IMM, JUMP, labels.
  - `if cond { then } else { else }` — BRANCH_Z, JUMP, labels for both branches.
  - Expression statements (e.g., bare function calls).
- Support for all Nib v1 expressions:
  - `INT_LIT`, `HEX_LIT` — LOAD_IMM.
  - `true`, `false` — LOAD_IMM 1 / 0.
  - Variable references — return the variable's dedicated register.
  - Function calls — compile args to v2/v3/..., emit CALL _fn_NAME.
  - `+%` (WRAP_ADD) — ADD + AND_IMM mask (15 for u4, 255 for u8/bcd).
  - `-` — SUB.
  - `+` — ADD (no mask; bare addition).
  - `==`, `!=`, `<`, `>`, `<=`, `>=` — CMP_EQ/CMP_NE/CMP_LT/CMP_GT
    (with operand-swap for `<=` and `>=`).
  - `&&` — AND.
  - `||` — ADD + CMP_NE (logical OR via sum != 0).
  - `!` — CMP_EQ with zero register (logical NOT).
  - `&` — AND (bitwise AND).
  - `~` — LOAD_IMM mask + SUB (bitwise complement).
- Program prologue: `LABEL _start`, `LOAD_IMM v0, 0`, optional `CALL _fn_main`, `HALT`.
- Function compilation: `LABEL _fn_NAME`, body, trailing `RET`.
- Calling convention: arguments in v2/v3/..., return value in v1.
- `static` declarations: `IrDataDecl(label, size, init)` with type-appropriate
  sizes (u4/bcd/bool → 1 byte, u8 → 2 bytes).
- `const` declarations: no IR emitted (inlined at use sites by type checker).
- 50+ unit tests covering all constructs, operator types, and edge cases.

### Implementation Notes

- Compiler uses a two-phase approach: static data collection, then function
  compilation. This mirrors the two-pass type checker pattern.
- Source map is `None` in v1 (full AST-to-IR mapping is a future feature).
- The `_extract_const_int` helper safely falls back to 0 for unknown expressions.
- For-loop bounds are extracted as constants (the type checker enforces this).
- BCD `+%` emits an additional COMMENT in debug mode signalling the backend
  should emit DAA after ADD.
