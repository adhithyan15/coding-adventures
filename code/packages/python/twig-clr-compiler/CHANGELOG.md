# Changelog — twig-clr-compiler

## 0.2.0 — 2026-04-29 — CLR02 Phase 2d (closures from Twig source)

### Added — anonymous lambdas and closure invocation

- Anonymous `(lambda (x) body)` forms in expression position
  are lifted to fresh `_lambda_N` top-level regions via the
  same `twig.free_vars` analysis the BEAM compiler uses.  The
  use site emits `MAKE_CLOSURE` with the current values of
  the captured variables.
- `Apply` whose `fn` slot is anything other than a known
  builtin or top-level function name lowers to
  `APPLY_CLOSURE`.  Covers chained calls like
  `((make-adder 7) 35)` and let-bound closures.
- `compile_source` populates
  `CILBackendConfig.closure_free_var_counts` from the
  lifted-lambda table so ir-to-cil-bytecode emits the
  per-lambda `Closure_<name>` TypeDef + auto-generated
  `IClosure` interface.
- The full pipeline runs end-to-end on real `dotnet`:
  `((make-adder 7) 35) → 42`.

### Test additions

- `test_lambda_lifts_to_top_level_region` — IR-shape unit
  test covering MAKE_CLOSURE emission for a captured value.
- `test_closure_call_emits_apply_closure` — IR-shape unit
  test for chained closure invocation.
- `test_closure_make_adder` — end-to-end on real `dotnet`:
  `((make-adder 7) 35) → 42`.
- `test_closure_let_bound` — end-to-end:
  `(let ((adder (make-adder 7))) (adder 35)) → 42`.

## 0.1.0 — 2026-04-29

### Added — Twig source → real `dotnet` (completes the Twig trilogy)

- ``compile_to_ir(source) -> IrProgram`` — Twig → compiler-IR for
  the v1 surface (arithmetic, ``let``, ``begin``, integer
  literals).
- ``compile_source(source, *, assembly_name=...)`` — full
  pipeline: parse → IR → optimise → lower to CIL → write
  CLR01-conformant ``.exe`` bytes.
- ``run_source(source)`` — drops the assembly + a
  ``runtimeconfig.json`` to a temp dir and invokes real
  ``dotnet <name>.exe``.  Skips when ``dotnet`` is not on PATH.
- Real-``dotnet`` smoke tests proving end-to-end execution
  (parity with the JVM and BEAM real-runtime tests):
  - ``(+ 1 2)`` exits 3
  - ``(* 6 7)`` exits 42
  - ``(let ((x 5)) (* x x))`` exits 25

### Out of scope (future iterations)

- ``define``, recursion, ``if`` (needs more IR lowering
  scaffolding in ``ir-to-cil-bytecode``).
- Closures, cons cells, lists, symbols (TW02.5 / TW03 work).
- ``Console.WriteLine`` for explicit I/O — v1 uses the process
  exit code as the result channel.

### Security

- ``assembly_name`` is interpolated into a tempfile path; we
  validate it against a strict allowlist regex
  (``^[A-Za-z][A-Za-z0-9_]{0,63}$``) to block path traversal.
  Same defense pattern as ``twig-beam-compiler``'s
  ``module_name`` validation (caught and fixed in BEAM04
  security review).
