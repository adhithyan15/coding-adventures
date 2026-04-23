# Changelog

All notable changes to `coding-adventures-dartmouth-basic-ge225-compiler` will be documented here.

## 0.2.0 (2026-04-20)

### Fixed

- **`PRINT` of numbers ≥ 10 000 produced garbled output on the GE-225.**
  The IR compiler's `_emit_print_number` was updated in v0.5.0 of
  `dartmouth-basic-ir-compiler` to extend the digit-extraction power list to
  cover the full 32-bit signed range (10 digit positions, largest power
  1 000 000 000).  The GE-225, however, is a 20-bit machine: its maximum
  signed value is 2^19 − 1 = 524 287, and `LOAD_IMM 1_000_000_000` silently
  truncates to a meaningless 20-bit residue.  The runner now calls
  `compile_basic(ast, int_bits=20)`, which limits the power list to
  `[100 000, 10 000, 1 000, 100, 10]` — all five values fit comfortably inside
  a GE-225 word — and correctly covers every representable positive integer.

## 0.1.0 (2026-04-19)

### Added

- Initial release of the full Dartmouth BASIC → GE-225 compiled pipeline.
- `run_basic(source)` — single public entry point that chains all four pipeline
  stages: parse → IR compile → GE-225 backend → simulate.
- `RunResult` dataclass with `output`, `var_values`, `steps`, and `halt_address`.
- `BasicError` exception wrapping parse, compile, codegen, and runtime failures.
- Support for the complete V1 BASIC subset: `LET`, `PRINT` (strings and
  numerics), `FOR`/`NEXT`, `IF`/`THEN`, `GOTO`, `REM`, `STOP`, `END`.
- Automatic `\r` → `\n` conversion on typewriter output.
- 20-bit sign extension for final variable values.
- Safety `max_steps` limit (default 100 000) to catch infinite loops.
- 57 integration tests across arithmetic, printing, loops, conditionals, classic
  programs, and error paths; coverage 100 %.
