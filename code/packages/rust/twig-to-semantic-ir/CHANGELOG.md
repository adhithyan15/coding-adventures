# Changelog

## 0.2.1 — Call position resolves names lexically (issue #18456)

**Bug fix.** TW00 gives one name-resolution order, whether a name is
used as a value (`car`) or called (`(car p)`):

```text
row  the name is bound by…                    (name args…) lowers to
1    param / let / let* / lambda param /       IndirectCall on the local
     capture (innermost first)
2    a top-level function define               DirectCall name
3    a top-level value define                  IndirectCall on the Global
4    the builtin table                         BuiltinCall name
```

Value position (`resolve_varref`) already followed it.  Call position
(`lower_apply`) did not: it asked the builtin table first, then the
top-level function names, and only then the locals and value defines.
So three kinds of program called the wrong function:

```scheme
(define (car p) 42)
(car (cons 1 2))                  ; was BuiltinCall car → 1;  now DirectCall car → 42

(define (helper x) (* x 100))
(define (f helper) (helper 1))    ; was DirectCall helper → 100;  now the parameter
(f (lambda (x) (+ x 1)))          ;                                   → 2

(define (f print) (print 5))      ; was __sys_write__;  now calls the parameter
```

The bug was memory-safe (every backend's `car` checks its argument's
tag), but the program ran the wrong code.

### Changed

- `lower_apply` checks the four rows in order.  A lexical binding
  (`FunctionCtx::is_lexical`) gives an `IndirectCall` on that local, a
  top-level function a `DirectCall`, a top-level value an
  `IndirectCall` on the `Global`, and only a name none of those bind
  reaches the builtin table, including `print`'s SIR28 `__sys_write__`
  lowering.  `(define car 5) (car p)` therefore calls 5, a run-time
  "not a closure" error as in Scheme, rather than quietly calling the
  builtin.
- A call to a name bound nowhere is still the `unresolved name` error.
  Value and call position now build it with one shared `unresolved`
  helper.
- The inline-lambda capture filter uses the same `is_lexical` test, so
  capture analysis and call dispatch agree on what counts as a local.
- Special forms (`if`, `let`, `let*`, `lambda`, `begin`, `quote`) are
  unaffected: the parser turns them into their own AST nodes before
  name resolution runs.  Records, unions and `match` are still
  rejected, so their generated accessors cannot shadow a builtin yet.

### Added

- `tests/name_resolution.rs`: 24 tests that walk the lowered module
  with a `semantic_ir::Visitor` and pin every call node.  They cover
  `BUILTIN_SHADOWING_PROGRAMS`, ported from `twig-ir-compiler`
  (#18438) to this frontend's builtin table.  Parameters, `let`,
  `let*`, lambda parameters and captures named like builtins and like
  user functions are covered, as are user value defines named like
  builtins in call position, a parallel `let` whose right-hand side
  still sees the outer builtin, a shadow ending with its scope, and
  user `print`.
- `semantic-ir-to-javascript/tests/twig_name_resolution.rs` runs 19 of
  these programs under `node` and checks the printed answers.

### Known gap (not changed here)

The lowerer never declares `Feature::Pairs`, although SIR11's manifest
rules say a `cons` / `car` / `cdr` / `pair?` builtin call should.  The
validator, and every backend that validates, then rejects any Twig
program that uses pairs.  The new tests tolerate that one validator
issue (or declare the feature before compiling) and say why.  It
needs its own fix.

`twig-to-semantic-ir` 0.2.0 -> 0.2.1.

## 0.2.0 — SIR28 (final frontend): `print` lowers to `__sys_write__`

Twig was the one frontend the SIR28 arc missed on its first pass —
`ruby-to-semantic-ir`, `python-to-semantic-ir`, `javascript-to-semantic-ir`,
`apl-to-semantic-ir`, `j-to-semantic-ir`, `q-to-semantic-ir`,
`idl-to-semantic-ir`, `matlab-to-semantic-ir`, `scilab-to-semantic-ir`, and
`c-to-semantic-ir` all migrated first; a follow-up sweep for Slice 7's
cleanup found Twig's `print` still lowering to a bare
`BuiltinCall("print", ...)` via its generic builtin-table dispatch. This
release closes that gap — see `code/specs/SIR28-syscall-primitives.md`.

**Behavior change**: `(print x)` no longer lowers to
`BuiltinCall("print", [x])`. It now lowers to
`BuiltinCall("__sys_write__", [StrLit("stdout"), StrLit("none"),
BoolLit(false), x])` (SIR28 §2.1's table; `print`'s existing single-arg
arity carries over unchanged). `Feature::ConsoleIO` is declared whenever
`print` is used. `Effect::MayPrint` was already set correctly on the old
bare `print` builtin (via the builtin table's `BuiltinSig.effects`) and
still is on the new `__sys_write__` call — Twig didn't have the
`MayPrint` gap every other frontend in this arc had.

`twig-to-semantic-ir` 0.1.0 -> 0.2.0.

## 0.1.0 — initial release (SIR11 v0)

First Twig → SIR frontend.  Implements the TW00 Twig surface as
specified in
[SIR11-twig-to-semantic-ir.md](../../../specs/SIR11-twig-to-semantic-ir.md).

### Added

- `compile(program, module_name)` — typed-AST entry point
- `compile_source(source, module_name)` — string entry point (parse
  + lower)
- `TwigLowerError` with source positions
- Builtin table covering TW00 surface: `+`, `-`, `*`, `/`, `=`, `<`,
  `>`, `cons`, `car`, `cdr`, `null?`, `pair?`, `number?`, `symbol?`,
  `print`, `global_get`, `global_set`.  Effect tags pre-populated
  per builtin (`MayPrint` for print, `MayAllocate` for cons, others
  pure).
- Closure lowering — each `lambda` becomes a fresh top-level
  `__lambda_<N>` function with explicit captures + `MakeClosure` at
  the source position.  Free-variable analysis filters out globals
  / functions / builtins from the capture set; only true closure
  bindings flow through `captures`.
- Apply-site dispatch — `DirectCall` for known top-level functions,
  `BuiltinCall` for builtins, `IndirectCall` for values held in
  locals / params / captures / globals.
- Scope resolution honouring let / let* shadowing and the
  `Local → Param → Capture → Global → Builtin` lookup order.
- `_init` and `main` synthesis matching TW00 spec.
- LANG48 type aliases, records, unions, and `match` rejected with a
  clear error pointing at the source position.

### Deferred

- LANG48 typed forms (records, unions, match, type annotations).
  The frontend rejects them with a clear "not supported in SIR v0"
  error; future revision will lower them once SIR adds the matching
  node kinds.
- Multi-module compilation via `twig-module-driver`.
- Tail-call markers in emitted IR.
