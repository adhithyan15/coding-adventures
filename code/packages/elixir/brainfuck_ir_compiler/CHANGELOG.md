# Changelog — brainfuck_ir_compiler (Elixir)

## 0.1.0 — 2026-04-11

Initial release: Elixir port of the Go `brainfuck-ir-compiler` package.

### Added

- `BuildConfig` struct with `debug_config/0` and `release_config/0` presets.
  Flags: `insert_bounds_checks`, `insert_debug_locs`, `mask_byte_arithmetic`, `tape_size`.
- `CompileResult` struct wrapping `IrProgram` and `SourceMapChain`.
- `Compiler.compile/3` (and `BrainfuckIrCompiler.compile/3` delegate) — compiles
  a Brainfuck `ASTNode` tree into an `IrProgram` + `SourceMapChain`.
- Full Brainfuck command support: `>`, `<`, `+`, `-`, `.`, `,`, `[`, `]`.
- Debug-mode bounds checking: `CMP_GT/CMP_LT + BRANCH_NZ + __trap_oob` handler.
- Source map segments 1 (SourceToAst) and 2 (AstToIr) fully populated.
- Comprehensive ExUnit test suite ported from the Go `compiler_test.go`.
