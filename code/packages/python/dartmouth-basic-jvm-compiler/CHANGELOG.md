# Changelog

## [0.2.0] — 2026-04-20

### Changed

- **Removed `syscall_arg_reg=0` from the JVM pipeline stage.**  Now that the
  SYSCALL IR instruction carries the arg register explicitly as `operands[1]`,
  `JvmBackendConfig` no longer has a `syscall_arg_reg` field.  The `runner.py`
  call to `lower_ir_to_jvm_class_file` passes only `class_name` and
  `emit_main_wrapper`.

## [0.1.0] — 2026-04-19

### Added

- Initial release of `dartmouth-basic-jvm-compiler`.
- `run_basic(source)` — compiles a Dartmouth BASIC program through the full
  four-stage pipeline and returns its standard output:
  1. Lex & parse via `dartmouth_basic_parser`
  2. IR compilation via `dartmouth_basic_ir_compiler` with `char_encoding="ascii"`
  3. JVM lowering via `ir_to_jvm_class_file`
  4. JVM simulation via `jvm_runtime.JVMRuntime.run_method` — invokes the
     `_start()` method compiled into the `.class` file and captures stdout
- `RunResult` dataclass with `output`, `var_values` (always `{}`), `steps`
  (always `0`), and `halt_address` (always `0`).
- `BasicError` exception wrapping parse, IR compile, JVM lower, and runtime
  failures.
- 59 end-to-end tests covering LET/arithmetic, PRINT (strings and numerics),
  FOR/NEXT loops, IF/THEN conditionals (all six relational operators), GOTO
  (forward and backward), classic programs (Fibonacci, Gauss sum, Collatz,
  countdown, multiplication table, factorial), REM, STOP, and error paths
  (parse errors, IR compile errors, JVM lowering errors, runtime errors).
