# Changelog — twig-jvm-compiler

## [0.1.1] — 2026-04-28

### Fixed — JVM01 paired fix: copy params to body-local holding registers

`_emit_function` now copies each parameter out of its arrival
register (`r2`, `r3`, …) into a fresh body-local holding
register at function entry.  Without this, a recursive body
read of `n` happened against the same register the upcoming
call's arg-marshalling overwrote — the caller-save in
`ir-to-jvm-class-file` then snapshotted the *already-clobbered*
value, defeating the fix.  Together with the
`ir-to-jvm-class-file` 0.6.1 caller-saves change, recursion
(e.g. `(fact 5) → 120`) now runs correctly on real `java`.

### Added

- `tests/test_real_jvm.py::test_recursion_factorial` — runs
  `(define (fact n) (if (= n 0) 1 (* n (fact (- n 1))))) (fact 5)`
  through real `java` and asserts stdout is byte 120.

### Removed

- "Known limitation: recursion (tracked as JVM01)" note — the
  limitation is gone.

## [0.1.0] — TW02 v1

### Added

- ``compile_to_ir(source)`` — Twig → ``IrProgram`` for the v1
  surface (integer arithmetic, ``if``/``let``/``begin``,
  top-level ``define`` for both values and functions,
  function calls including recursion — see 0.1.1 for the
  JVM01 recursion fix).
- ``compile_source(source)`` — full pipeline: parse + extract +
  IR emit + ir-optimizer + lower_ir_to_jvm_class_file → .class
  bytes.
- ``run_source(source)`` — write the .class file to a temp dir
  and invoke real ``java -cp <dir> <ClassName>``.  Returns the
  process's stdout / stderr / exit code.
- TW02 spec at ``code/specs/TW02-twig-jvm-compiler.md``.

### Notes

- Lambdas / cons cells / symbols / ``print`` raise
  ``TwigCompileError``.  TW02.5 adds them.
- Output is a single byte to stdout via ``SYSCALL 1`` — same
  convention ``test_oct_8bit_e2e.py`` uses for the existing
  Oct-on-JVM tests.

### Note

Recursion was a known limitation in 0.1.0 tracked as JVM01;
0.1.1 lands the fix (see entry above).
