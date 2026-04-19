# Changelog

## [0.1.0] — 2026-04-19

### Added

- Initial release of `dartmouth-basic-wasm-compiler`.
- `run_basic(source)` — compiles a Dartmouth BASIC program through the full
  five-stage pipeline and returns its standard output:
  1. Lex & parse via `dartmouth_basic_parser`
  2. IR compilation via `dartmouth_basic_ir_compiler` with `char_encoding="ascii"`
  3. WASM backend via `ir_to_wasm_compiler` with `strategy="dispatch_loop"` —
     the dispatch-loop strategy handles arbitrary `GOTO` and `IF … THEN` jumps
     emitted by the BASIC IR compiler without any structured-CF restrictions
  4. Binary encoding via `wasm_module_encoder`
  5. WASI runtime execution via `wasm_runtime`
- `RunResult` dataclass with `output`, `var_values` (always `{}`), `steps`
  (always `0`), and `halt_address` (always `0`).
- `BasicError` exception wrapping parse, compile, lower, encode, and runtime
  failures.
- 60 end-to-end tests covering LET/arithmetic, PRINT (strings and numerics),
  FOR/NEXT loops, IF/THEN conditionals (all six relational operators), GOTO
  (forward and backward), classic programs (Fibonacci, Gauss sum, Collatz,
  countdown, multiplication table, factorial), REM, STOP, and error paths
  (parse errors, IR compile errors, WASM lowering errors, encode errors, runtime
  errors).
