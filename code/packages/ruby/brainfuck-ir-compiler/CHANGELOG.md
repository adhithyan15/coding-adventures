# Changelog

## [0.1.0] - 2026-04-11

### Added

- `BuildConfig` struct with `debug_config` and `release_config` class methods —
  exact port of Go's BuildConfig with insert_bounds_checks, insert_debug_locs,
  mask_byte_arithmetic, and tape_size fields
- `CompileResult` struct (program, source_map) holding the two compiler outputs
- `BrainfuckIrCompiler.compile(ast, filename, config) → CompileResult`
  public entry point
- Internal `Compiler` class with fixed register allocation:
  - v0=tape_base, v1=tape_ptr, v2=temp, v3=temp2, v4=sys_arg, v5=max_ptr, v6=zero
- Prologue: LABEL _start, LOAD_ADDR v0 tape, LOAD_IMM v1 0, plus debug
  bounds-check register setup
- Epilogue: HALT, plus __trap_oob handler in debug builds
- Command compilation for all 6 BF commands (>, <, +, -, ., ,)
- `emit_cell_mutation/1` for +/- with optional byte masking (AND_IMM 255)
- `emit_bounds_check_right/0` and `emit_bounds_check_left/0` for debug builds
- `compile_loop/1` emitting LABEL/LOAD_BYTE/BRANCH_Z/body/JUMP/LABEL
- Source map chain population: SourceToAst and AstToIr filled for every
  command and loop construct
- Comprehensive minitest suite (38 tests) mirroring the Go test suite
