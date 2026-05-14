# Changelog — twig-type-checker

## [0.1.0] — 2026-05-14

Initial release. TW05-B — base static type checker for Twig.

### Added

- `type_check(source) -> Result<TypeCheckResult<TypedProgram>, TwigTypeCheckError>`
  — parse + check in one call.
- `check_program(program, mode_override) -> TypeCheckResult<TypedProgram>`
  — check an already-parsed `Program`.
- `TwigTypeCheckerImpl` — implements `TypeChecker<Program, TypedProgram>` from
  `type-checker-protocol`.
- `TypedProgram { program, env }` — original AST plus populated `TypeEnv`.
- `TwigTypeCheckError` (`#[non_exhaustive]`) — `Parse(TwigParseError)` only;
  type errors live in `TypeCheckResult::errors`.
- `TwigKind` enum (`#[non_exhaustive]`): `Int`, `Bool`, `Nil`, `Symbol`,
  `Str`, `List`, `Record(String)`, `Union(String)`, `Function { arity }`, `Any`.
- `TwigKind::mnemonic() -> &'static str` — stable lowercase display string.
- `TwigKind::unify(a, b) -> TwigKind` — return shared kind or widen to `Any`.
- `TypeEnv` — global declaration table with `globals`, `aliases`, `records`,
  `unions` maps.
- `ScopeStack` — push/pop/bind/lookup for lexical scope.

### Behaviour

**Two-pass algorithm:**

1. **Pass 1 (declaration collection)**: walk all `Form`s, populate `TypeEnv`
   with records, unions, type aliases, and top-level define kinds before any
   body is walked.  Forward references and mutual recursion work correctly.

2. **Pass 2 (expression walking)**: infer `TwigKind` for every `Expr`,
   accumulate `TypeErrorDiagnostic`s.  Checks performed:
   - **Unresolved variables**: `VarRef` not in scope → diagnostic.
   - **Call arity**: `Apply` where callee resolves to `Function { arity }` →
     `check_arity` compares expected vs actual.
   - **Match exhaustiveness**: `Match` with a `Union` scrutinee →
     `check_exhaustiveness` checks every variant is covered (or wildcard present).

**Typed mode enforcement:**

- `Off` (no module or `(typed off)`) → checker skipped entirely; `ok: true`.
- `Lenient` → errors collected; `ok: true` regardless.
- `Strict` → errors collected; `ok: errors.is_empty()`.

### Notes

- Pure data → typed AST + diagnostics.  Two deps (`twig-parser`,
  `type-checker-protocol`), both capability-empty.  No I/O, no FFI, no unsafe.
- 34 unit tests (28 explicit + 3 doc tests + 3 module-level) covering each atom
  kind, variable resolution, define classification, arity checking (correct/
  too-few/too-many/zero-arity), type aliases (registered + opaque resolution),
  record and union registration, match exhaustiveness (wildcard/binding/all-
  variants/non-exhaustive), lambda and let scoping, begin last-kind, all three
  typed modes, `check_program` direct path, and parse error path.
- Filed as follow-ups in README: TW05-C (refinement solver for `RangeInt` /
  `MembershipInt`), field type threading in match patterns, dependent-type
  resolution, cross-module import checking.
