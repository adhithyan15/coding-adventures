# Changelog — brainfuck-ir-compiler (Python)

## [0.2.0] — 2026-04-20

### Changed

- **SYSCALL instruction now carries the arg register as `operands[1]`.**
  All three SYSCALL emissions (WRITE, READ, EXIT) now include the IR virtual
  register that holds the syscall argument as an explicit second operand:
  `SYSCALL 1, v4` instead of bare `SYSCALL 1`.  This makes the IR
  self-describing: backends no longer need out-of-band `syscall_arg_reg`
  configuration to know which register to read.  The register index (4) is
  unchanged — only the encoding in the IR instruction is new.

## [0.1.0] — 2026-04-12

### Added

- `BuildConfig` dataclass with flags: `insert_bounds_checks`,
  `insert_debug_locs`, `mask_byte_arithmetic`, `tape_size`.
- `debug_config()` preset — all safety checks enabled.
- `release_config()` preset — safety checks off, byte masking on.
- `compile_brainfuck(ast, filename, config)` — main entry point.
  Takes a Brainfuck `ASTNode`, a filename, and a `BuildConfig`.
  Returns a `CompileResult` with `IrProgram` + `SourceMapChain`.
- `CompileResult` dataclass holding `program` and `source_map`.
- Full command → IR mapping for all 8 Brainfuck commands:
  `>`, `<`, `+`, `-`, `.`, `,`, `[`, `]`.
- Prologue: `_start` label, `LOAD_ADDR v0, tape`, `LOAD_IMM v1, 0`.
- Epilogue: `HALT`. Debug mode adds `__trap_oob` handler.
- Bounds checking in debug mode: `CMP_GT/CMP_LT` + `BRANCH_NZ`.
- Optional byte masking: `AND_IMM v2, v2, 255` after INC/DEC.
- Loop compilation: `LABEL loop_N_start`, `BRANCH_Z`, `JUMP`, `LABEL loop_N_end`.
- Source map generation: `SourceToAst` and `AstToIr` segments.
- Full test suite with >80% coverage.
- Passes `ruff check` with no errors.
