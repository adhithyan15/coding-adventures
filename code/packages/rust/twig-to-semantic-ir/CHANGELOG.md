# Changelog

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
