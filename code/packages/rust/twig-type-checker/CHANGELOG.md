# Changelog — twig-type-checker

## [0.4.0] — 2026-05-14

### Added (LANG51 + LANG52 — string literal and let* type inference)

#### LANG51: `TwigKind::Str` for string literals

- **`infer_expr` `Expr::StrLit` arm** — returns `TwigKind::Str`.  String literals propagate
  their type through the entire inference pass.
- **`profile.rs` `literal_kind`** — `"STRING"` token maps to `Some(KindDecl::Str)`, enabling
  the grammar-type-checker profile to classify string literal tokens as `Str`.

#### LANG52: `let*` sequential binding inference

- **`infer_let_star`** — new helper function; pushes a new scope frame, then for each
  binding: infers the RHS type, binds the name in the current scope (so subsequent
  bindings see it), and infers the body.  Returns the type of the last body expression.
- **`Expr::LetStar` arm** — dispatches to `infer_let_star`.

### Note on version numbering

0.3.0 was the planned standalone LANG51 release; both LANG51 and LANG52 land here as 0.4.0.

## [0.2.0] — 2026-05-14

### Added (LANG50 — Generic Grammar Type Checker integration)

- `type_check_source(source) -> Result<TypeCheckResult<AnnotatedNode>, TwigTypeCheckError>`
  — new **compilation-first** entry point.  Runs `parse_to_ast` + `emit_type_declarations`
  + `grammar_type_checker::check` to return a fully-annotated `AnnotatedNode` tree where
  every node carries a `KindDecl` (and thus an IIR `type_hint` via `iir_hint()`).
  Annotation is **always built** — even in `Off` mode — so the IIR compiler can use it
  regardless of enforcement level.
- `TwigLanguageProfile` (`src/profile.rs`) — implements `grammar_type_checker::LanguageProfile`
  for Twig grammar rule names.  Transparent descent through `expr`/`compound`/`form` wrapper
  nodes via `unwrap_expr`.  Handles: literals (`atom` + INTEGER/BOOL_TRUE/BOOL_FALSE/nil,
  `quoted`), var refs (`atom` + NAME), function application (`apply`), lambda binding
  (`lambda_form`), let binding (`let_form`), function-sugar define (`define` with LPAREN sig),
  match expressions (`match_form`), begin sequences (`begin_form`), and if conditionals
  (via `child_exprs` fallback).
- `TwigLanguageProfile` re-exported from `crate` root.
- `AnnotatedNode` re-exported from `crate` root.
- 13 unit tests in `profile.rs` for the new path.

### Dependencies added

- `grammar-type-checker = { path = "../grammar-type-checker" }`
- `type-declarations = { path = "../type-declarations" }`
- `parser = { path = "../parser" }`

### Backward compatibility

- `type_check` and `check_program` are **unchanged** — existing callers work without
  modification.  The new `type_check_source` path is additive only.

---

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
