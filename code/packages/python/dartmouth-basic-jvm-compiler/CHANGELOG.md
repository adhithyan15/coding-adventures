# Changelog

## [0.1.0] — 2026-04-19

### Added

- Initial release of `dartmouth-basic-jvm-compiler`.
- `run_basic(source)` — compiles a Dartmouth BASIC program through the full
  four-stage pipeline and returns its standard output:
  1. Lex & parse via `dartmouth_basic_parser`
  2. IR compilation via `dartmouth_basic_ir_compiler` with `char_encoding="ascii"`
  3. JVM lowering via `ir_to_jvm_class_file` with `syscall_arg_reg=0` —
     the BASIC IR compiler places the PRINT argument in register 0 (unlike the
     Brainfuck convention of register 4), so the JVM lowerer must be configured
     to read the print character from register 0
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
