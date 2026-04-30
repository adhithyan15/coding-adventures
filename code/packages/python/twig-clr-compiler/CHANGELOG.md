# Changelog — twig-clr-compiler

## 0.3.0 — 2026-04-30 — TW03 Phase 3e (heap primitives from Twig source)

Twig source containing `cons` / `car` / `cdr` / `null?` / `pair?` /
`symbol?` / `'foo` / `nil` now compiles via the CLR backend.
Builds on the Phase 3c heap-op lowering (which ships structural
PE/CLI bytecode for the eight heap opcodes).

End-to-end on real `dotnet` is deferred until CLR Phase 3c.5
wires writer-side support for symbol-name string interning and
the singleton Nil instance — until then heap programs compile to
verifier-correct CIL but may not execute correctly.

Mirrors twig-jvm-compiler v0.3.0 (TW03 Phase 3e for JVM) — same
lambda-lifting + heap-builtin emission table, just routes to the
CLR backend's Phase 3c lowering instead of the JVM Phase 3b path.

### Added — heap-primitive emission

- `nil` literal → `LOAD_NIL`.
- `'foo` / `(quote foo)` → `MAKE_SYMBOL` with the symbol name as
  an `IrLabel`.
- `cons` → `MAKE_CONS dst, head, tail` (2 args).
- `car` / `cdr` → `CAR` / `CDR` (1 arg each).
- `null?` / `pair?` / `symbol?` → `IS_NULL` / `IS_PAIR` /
  `IS_SYMBOL` (1 arg each, result is an int 0/1 ready to feed
  BRANCH_Z).
- `_HEAP_BUILTINS` table maps Twig source names to (op, arity)
  pairs; the apply-site dispatches uniformly with arity validation.
- Free-variable analysis treats `_HEAP_BUILTINS` as globals so
  closures over `cons` / `car` / etc compile correctly.

### Removed — obsolete rejection tests

- `test_quoted_symbol_rejected` — now compiles to MAKE_SYMBOL.
- `test_cons_rejected` — now compiles to MAKE_CONS.

### Test additions

- 8 new IR-shape tests covering each heap op's emission shape.
- 2 new arity-validation tests.
- All 61 tests pass; coverage 90%.

### Limitations

- End-to-end on real `dotnet` is deferred to Phase 3c.5 (writer-side
  intern table for symbol names, singleton Nil instance).
- `print` and `number?` builtins still raise (TW04 territory).

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
