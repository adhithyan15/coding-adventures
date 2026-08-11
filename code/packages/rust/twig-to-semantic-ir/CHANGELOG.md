# Changelog

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
