# Changelog

All notable changes to the `semantic-ir` crate are documented here.

## 0.1.0 — initial release (SIR10 v0)

First cut of the narrow-waist Semantic IR.  Implements the v0
surface defined in
[SIR10-narrow-waist-semantic-ir.md](../../../specs/SIR10-narrow-waist-semantic-ir.md).

### Added

- `Module`, `Function`, `Block`, `Stmt`, `Expr`, `Scope`, and all
  supporting node types per SIR10 §"Module structure" through
  §"Expressions".
- `SirType` carrier — `Any`, `Int`, `Bool`, `Nil`, `Symbol`, `Str`,
  `Pair`, `Closure`, parametric `Fn`.
- `EffectSet` bitset with effect tags `MayThrow`, `MayPrint`,
  `MayAllocate`, `MayBlock`, `Divergent`; pure is the empty set.
- `FeatureManifest` with the v0 feature list (`Closures`, `Pairs`,
  `Symbols`, `Strings`, `DynamicTyping`,
  `OptionalTypeAnnotations`, `MutualRecursion`, `TailCalls`,
  `Globals`, `Intrinsics`).
- `Metadata` carrier with source-language/version and SIR-version
  fields; advisory only — IR correctness must not depend on it.
- `Span` source-position carrier (1-indexed line and column).
- `validate(module)` — structural and semantic checks:
  - Manifest covers every feature actually used
  - No `VarRef` references an undefined name in its scope
  - `Intrinsic.targets` is non-empty
  - Function / global name uniqueness
  - Parallel `let` does not leak bindings into sibling RHS
  - Sequential `let*` allows prior bindings on subsequent RHS
- `Visitor` trait + free `walk_*_default` functions for read-only
  traversal.
- Canonical S-expression text printer (`print_module`, `print_expr`,
  `print_block`, `print_function`).  Output is deterministic and
  byte-stable.
- `Backend` trait and `BackendRegistry` with built-in capability
  enforcement (manifest features + intrinsic whitelist + target-tag
  matching).
- Test coverage for every public surface, including parallel/let*
  semantics distinguishing tests.

### Deferred to future versions

- Text-format parser (round-trip currently relies on printer
  determinism).
- Ownership / borrow markers (Move / Copy / Borrow).
- Async / await / coroutines.
- Exception handling (Raise / Try / Catch).
- Pattern matching (Match) and record / union / type-alias forms.
- Effect inference (manual annotation only in v0).
- Sequence / Map / Set / Option / Result primitives.
