# Changelog

All notable changes to `coding-adventures-dartmouth-basic-ge225-compiler` will be documented here.

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
