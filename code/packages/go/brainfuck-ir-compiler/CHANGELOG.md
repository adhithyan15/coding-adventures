# Changelog

All notable changes to the `brainfuck-ir-compiler` package will be documented
in this file.

## [0.1.0] — 2026-04-12

### Added

- `Compile()` function translating Brainfuck AST → IR program with source map
- `BuildConfig` with composable flags: bounds checks, debug locations,
  byte masking, configurable tape size
- `DebugConfig()` and `ReleaseConfig()` preset constructors
- Compilation of all 8 Brainfuck commands (>, <, +, -, ., ,, [, ])
- Fixed register allocation: v0 (tape base), v1 (pointer), v2-v6 (temps)
- Source map generation: SourceToAst and AstToIr segments
- Debug mode: out-of-bounds trap handler (__trap_oob) with CMP_GT/CMP_LT
  checks before every pointer move
- Prologue/epilogue generation with HALT and optional trap handlers
- Loop compilation with unique labels (loop_N_start, loop_N_end)
- Nested loop support to arbitrary depth
- IR printer integration: compiled programs round-trip through print/parse
- 28 tests covering all commands, loops, bounds checks, source maps,
  custom configurations, and error cases
