# Changelog

## 0.1.0 — initial release (SIR12 v0)

First backend for the narrow-waist Semantic IR.  Emits self-contained
TypeScript source from a `semantic_ir::Module`.

### Added

- `TypeScriptBackend` implementing `semantic_ir::Backend` with:
  - `target_tag() = "typescript"`
  - `accepts_features()` covering Closures, Pairs, Symbols, Strings,
    DynamicTyping, OptionalTypeAnnotations, MutualRecursion, Globals.
  - `accepts_intrinsics()` empty in v0 — all intrinsics rejected.
- `compile(module)` convenience function returning an
  `Artifact { filename, source, metadata }`.
- Per-node lowering rules per SIR12:
  - Literals → JS literals (`null`, `true`, `false`, numbers,
    quoted strings).
  - Symbols → `__Sir.intern("...")`.
  - VarRef Local / Param / Capture / Global → bare identifier.
  - VarRef Builtin → `__Sir.builtins["<name>"]`.
  - If → ternary using `__Sir.truthy(cond)`.
  - Block with statements → IIFE; block without statements → bare
    value expression.
  - LetBinding / LetStarBinding → `const`.  Parallel-let semantics
    were preserved by the frontend; sequential `let*` is naturally
    honored by top-down `const` emission.
  - DirectCall → `<fn>(...)`.
  - IndirectCall → `__Sir.applyClosure(target, [...args])`.
  - BuiltinCall → one of the `__Sir.<op>` helpers, with an
    unrecognised-name fallback through the dispatch table.
  - MakeClosure → `new __Sir.Closure((..._a) => <fn>(<caps>, ..._a))`.
- Inlined `__Sir` namespace runtime (~110 lines) with:
  - `Val` discriminated union type
  - `Sym` / `Pair` / `Closure` classes
  - `intern`, `applyClosure`, `globalSet`, `globalGet`
  - All v0 builtins (`plus`, `minus`, `times`, `divide`, `eq`,
    `lt`, `gt`, `cons`, `car`, `cdr`, `isNull`, `isPair`,
    `isNumber`, `isSymbol`, `print`)
  - `format` and `truthy` helpers
  - `builtins` dispatch table for VarRef Builtin
- Identifier sanitisation: SIR names containing `?`, `!`, `-`, `+`,
  etc. (e.g. `null?`, `pair?`) are rewritten to `_$<hex-escaped>`
  forms; valid TS identifiers pass through unchanged.  TS reserved
  words are also rewritten.
- TypeScript-safe string literal escaping including `\uXXXX` for
  control characters.
- Pre-lowering validation via `semantic_ir::validate`; module-level
  capability check via `Backend::check_module` default impl.
- Special-case lowering of the `BuiltinCall("global_set", SymLit,
  value)` pattern emitted by `_init` — rendered as a direct
  `<global> = <value>;` assignment for readable output.

### Deferred

- Source-map generation (only function-level span comments today).
- Optimisation passes (constant folding, block flattening).
- Async / `await`, top-level await.
- Intrinsic support — the v0 backend rejects all intrinsics; a
  future revision may add `typescript`-tagged ones for raw-TS
  embedding.
