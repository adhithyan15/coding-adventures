# Changelog — twig-jvm-compiler

## [0.2.0] — 2026-04-29 — JVM02 Phase 2d (closures from Twig source)

Completes the JVM closure trilogy: anonymous lambdas in Twig
source compile to a JAR that runs on stock `java -jar` and
returns the expected exit code.  `((make-adder 7) 35) → 42`
runs end-to-end on real `java`.

Mirrors what twig-clr-compiler shipped for CLR Phase 2d (and
twig-beam-compiler for BEAM Phase 2d) — same lambda-lifting +
free-variable analysis approach, just routes to the JVM
backend's Phase 2c.5 closure plumbing instead of the BEAM
apply/3 or CLR typed-locals path.

### Added — anonymous lambdas and closure invocation

- Anonymous `(lambda (x) body)` forms in expression position
  are lifted to fresh `_lambda_N` top-level regions via
  `twig.free_vars`.  The use site emits `MAKE_CLOSURE`.
- `Apply` whose `fn` slot is anything other than a known
  builtin or top-level function name lowers to
  `APPLY_CLOSURE`.  Covers chained calls like
  `((make-adder 7) 35)` and let-bound closures.
- `compile_source` populates
  `JvmBackendConfig.closure_free_var_counts` so
  ir-to-jvm-class-file emits the multi-class artifact
  (main + Closure interface + per-lambda Closure_<name>
  subclasses).
- `PackageResult` gains an optional `multi_class_artifact:
  JVMMultiClassArtifact | None` field — populated when the
  program contains closures so callers can package the full
  bundle.
- `run_source` packages the multi-class artifact via
  `jvm-jar-writer` and runs `java -jar` when closures are
  present; falls back to the existing single-class +
  `java -cp` flow for closure-free programs.

### Test additions

- `test_lambda_lifts_to_top_level_region` — IR-shape unit
  test covering MAKE_CLOSURE emission for a captured value.
- `test_closure_call_emits_apply_closure` — IR-shape unit
  test for chained closure invocation.
- `test_closure_make_adder` — end-to-end on real `java`:
  `((make-adder 7) 35) → 42` (stdout = `b'*'`).
- `test_closure_let_bound` — end-to-end:
  `(let ((adder (make-adder 7))) (adder 35)) → 42`.

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
