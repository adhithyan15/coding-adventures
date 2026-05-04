# Changelog — twig-clr-compiler

## 0.7.0 — 2026-05-04 — Phase 4e multi-module CLR compilation + Phase 4c host calls

### Added — `compile_modules` and `run_modules`

New API for compiling and executing multi-module Twig programs on the CLR:

```python
from twig_clr_compiler import compile_modules, run_modules

result = compile_modules(resolved_modules, entry_module="user/hello")
# result.assembly_bytes — single PE file containing all modules as TypeDefs
# result.module_results — list of ModuleClrCompileResult, entry first
# result.entry_type_name — CLR type name derived from entry module name

exec_result = run_modules(resolved_modules, entry_module="user/hello")
# exec_result.returncode  — dotnet process exit code
# exec_result.stdout / .stderr
```

Each non-host Twig module maps to one TypeDef in the PE assembly.  The
entry module's TypeDef uses `main` as the entry point; dep modules'
TypeDefs expose their exported functions as static methods called from
the entry module via `call` instructions with stable MethodDef tokens.

### Added — `module_name_to_clr_type`

Helper that replaces `/` with `_` in module names for CLR type names:
`"user/hello"` → `"user_hello"`.

### Added — `MultiModuleClrResult` and `ModuleClrCompileResult`

Frozen dataclasses returned by `compile_modules`:
- `MultiModuleClrResult.module_results` — `list[ModuleClrCompileResult]`
  with the entry module first
- `ModuleClrCompileResult.callable_names` — set of exported function names
- `ModuleClrCompileResult.artifact` — `CILProgramArtifact` for introspection

### Fixed — inline host calls

`compile_source` and `compile_modules` now pass
`inline_host_syscalls=True` to the CIL backend, so `host/write-byte`,
`host/read-byte`, and `host/exit` emit direct `System.Console.Write`,
`System.Console.Read`, and `System.Environment.Exit` calls instead of
the brainfuck-only `__ca_syscall` external helper.  Previously any Twig
program using `(import host)` would crash with `TypeLoadException`.

### Fixed — local_count floor for cross-module calls

When compiling multiple modules, `compile_modules` computes
`global_local_count` (the maximum local count across all modules) and
passes it as `call_register_count` to every module's backend config.
A floor in `_analyze_program` ensures every module declares at least
that many locals, preventing `InvalidProgramException` when the entry
module has fewer registers than `global_local_count`.

### Acceptance criterion (TW04 Phase 4e)

```
(a/math/add 17 25)  →  exit code 42
```

All 99 tests pass on net9.0, including recursive dep-module functions,
three-module chains, and host-call + dep-module combinations.

## 0.6.0 — 2026-04-29 — multi-arity closures

Multi-arg lambdas like `(lambda (x y) (+ x y))` now run on real
`dotnet`.  Pre-fix, the CLR backend hard-rejected `APPLY_CLOSURE`
with arity != 1 and `IClosure.Apply` took a single `int32`.

Frontend now records each lifted lambda's source-level param
count in `closure_explicit_arities` (parallel to
`closure_free_var_counts`).  Combined with `ir-to-cil-bytecode`
0.11.0's widened `IClosure.Apply(int32[])` interface, closures of
any arity share a single uniform call site.

### Acceptance

* `((lambda (x y) (+ x y)) 4 5) → 9`
* `((lambda (a b c) (+ a (+ b c))) 1 2 3) → 6`
* `((make-add-pair 10) 4 5) → 19`  (capture + 2 explicit args)

All on real `dotnet`.

## 0.5.0 — 2026-04-30 — closure-returning closures (3-deep curry)

Closure-returning closures now run end-to-end on real `dotnet`:

```
(define (mk2 a) (lambda (b) (lambda (c) (+ a (+ b c)))))
(((mk2 10) 20) 12)
→ 42
```

Was unblocked by ir-to-cil-bytecode v0.9.0 — IClosure.Apply now
returns object polymorphically (boxes int returns, returns obj
refs directly), and APPLY_CLOSURE callers forward-scan to pick
unbox.any vs stloc-obj.

Mutual recursion `(evp 8) → 1` also works (it always did on CLR
— noted here for cross-backend completeness).

## 0.4.0 — 2026-04-30 — recursive heap programs run on real dotnet

Heap programs now run end-to-end on real `dotnet` from raw Twig
source.  The headline TW03 Phase 3 acceptance criterion:

```
(define (length xs)
  (if (null? xs) 0 (+ 1 (length (cdr xs)))))
(length (cons 1 (cons 2 (cons 3 nil))))
→ 3
```

This brings CLR to parity with JVM and BEAM for recursive heap
programs.

Was unblocked by ir-to-cil-bytecode v0.8.0 — per-region parameter
typing + obj-source-read inference + obj-aware CALL marshalling.

### Test additions (real-dotnet end-to-end)

- `test_heap_car_of_singleton_returns_int` — `(car (cons 42 nil)) → 42`
- `test_heap_function_with_cons_param` — `(define (head xs) (car xs)) (head (cons 42 nil)) → 42` (obj-typed parameter passing)
- `test_heap_recursive_length_returns_3` — the headline `length`
  test on real `dotnet`, returning exit code 3

64/64 tests pass; 90% coverage.

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
