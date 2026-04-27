# Changelog — CodingAdventures::BrainfuckIrCompiler

## [0.01] — 2026-04-11

### Added

- Initial Perl port of the Go `brainfuck-ir-compiler` package.
- `BuildConfig` — composable build flags: `insert_bounds_checks`, `insert_debug_locs`, `mask_byte_arithmetic`, `tape_size`; plus `debug_config()` and `release_config()` class-method presets.
- `Compiler` — `compile($ast, $filename, $config)` function that:
  - Validates the AST root node type and tape size
  - Emits a tape data declaration (`.data tape N 0`)
  - Emits the prologue (`_start:`, `LOAD_ADDR v0, tape`, `LOAD_IMM v1, 0`, plus debug registers)
  - Walks the Perl Brainfuck AST (hashref-based, types: `program`, `instruction`, `loop`, `command`)
  - Emits correct IR sequences for all 6 commands (`>`, `<`, `+`, `-`, `.`, `,`)
  - Emits loop control flow (`loop_N_start:`, `LOAD_BYTE`, `BRANCH_Z`, body, `JUMP`, `loop_N_end:`)
  - Emits bounds-check sequences in debug mode (`CMP_GT`/`CMP_LT` + `BRANCH_NZ __trap_oob`)
  - Emits the epilogue (`HALT`, plus `__trap_oob` handler in debug mode)
  - Populates `SourceToAst` (Segment 1) and `AstToIr` (Segment 2) of the source map chain
  - Returns `{ program => IrProgram, source_map => SourceMapChain }`
- `CodingAdventures::BrainfuckIrCompiler` — top-level module loading sub-modules and re-exporting `compile`.
- Comprehensive test suite in `t/brainfuck_ir_compiler.t` mirroring the Go test suite.
