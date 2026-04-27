# Changelog — brainfuck-ir-compiler

## [0.1.0] — 2026-04-11

Initial release: Rust port of the `brainfuck-ir-compiler` Go package.

### Added

- `BuildConfig` struct with composable compilation flags:
  - `insert_bounds_checks` — tape pointer range checks (traps to `__trap_oob`)
  - `insert_debug_locs` — source location marker `COMMENT` instructions
  - `mask_byte_arithmetic` — `AND_IMM v, v, 255` after every cell mutation
  - `tape_size` — configurable tape length (default 30,000)
- `debug_config()` preset: all checks enabled
- `release_config()` preset: bounds checks and debug locs disabled, byte masking enabled
- `compile(ast, filename, config)` — main entry point producing `CompileResult`
- `CompileResult` with `program: IrProgram` and `source_map: SourceMapChain`
- Prologue emission: `_start` label, tape base address load, tape pointer init,
  and optional bounds-check register setup
- Epilogue emission: `HALT` instruction, optional `__trap_oob` subroutine
- Command compilation for all 8 Brainfuck commands: `>`, `<`, `+`, `-`, `.`, `,`, `[`, `]`
- Loop compilation with `loop_N_start`/`loop_N_end` label scheme (N = 0-based counter)
- Bounds check emission for `>` (`CMP_GT` + `BRANCH_NZ`) and `<` (`CMP_LT` + `BRANCH_NZ`)
- Source map population: `SourceToAst` (per command and loop) + `AstToIr` (per command)
- 32 unit tests + 5 doc tests (100% pass rate)

### Bug fixes (during initial implementation)

- Fixed `CompileResult` missing `#[derive(Debug)]` — required by `Result::unwrap_err`
- Fixed wrong import path for `parse_brainfuck`: `brainfuck::parser::parse_brainfuck`
  (not `brainfuck::parse_brainfuck` which is not re-exported at crate root)
- Removed unused `print_ir`/`parse_ir` imports from module-level `use` statement
  (they are used only in tests and imported there directly)
