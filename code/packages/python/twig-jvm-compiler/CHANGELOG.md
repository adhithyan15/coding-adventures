# Changelog — twig-jvm-compiler

## [0.1.0] — TW02 v1

### Added

- ``compile_to_ir(source)`` — Twig → ``IrProgram`` for the v1
  surface (integer arithmetic, ``if``/``let``/``begin``,
  top-level ``define`` for both values and functions,
  non-recursive function calls — see "Known limitation" below
  for the recursion gap).
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

### Known limitation: recursion is broken

``ir-to-jvm-class-file`` stores every "IR register" in a
class-level static int array shared across every method
invocation.  That works for one-level calls but breaks
recursion: when ``fact(5)`` calls ``fact(4)``, ``fact(5)``'s own
parameter register r2 gets overwritten with 4, and the outer
multiplication then uses 4 instead of 5.

The fix is in the JVM backend itself — it should use real
per-method JVM locals instead of a static array — and is its
own infrastructure PR (tracked alongside CLR01 under the
"real-runtime correctness" banner).  The factorial test is
marked ``pytest.mark.xfail(strict=True)`` so when that fix
lands, the test starts passing and pytest flags the marker
for removal.

Non-recursive function calls work correctly today, including
multi-arg functions and nested calls like ``(inc (dbl 5))``.
