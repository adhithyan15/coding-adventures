# Changelog — twig-type-checker

## [0.8.0] — 2026-05-17

### Added (LANG70 — TW05-P Part 1: Generated Symbol Registration)

- **`TypeEnv::register_record` extended** — in addition to registering the
  record constructor (`TwigKind::Record(name)`), now also registers:
  - `{to_lowercase(name)}?` → `TwigKind::Any`  (record predicate)
  - `{to_lowercase(name)}-{field}` → `TwigKind::Any`  (field accessors, one
    per declared field)

  Example for `(record IirBuilder (name : any) (instrs : any) (reg-count : int) …)`:
  - `iirbuilder?`, `iirbuilder-name`, `iirbuilder-instrs`, `iirbuilder-reg-count`,
    `iirbuilder-label-count`

  These naming conventions match `twig-ir-compiler` exactly (lines 343-348 of
  `compiler.rs`): `prefix = RecordName.to_lowercase()`.

- **`TypeEnv::register_union` extended** — in addition to registering variant
  constructors (`TwigKind::Function { arity }`), now also registers:
  - `{VariantName}?` → `TwigKind::Any`  (variant predicate, **original case**,
    NOT lowercased — mirrors IR compiler line 355)
  - `{to_lowercase(VariantName)}-{field}` → `TwigKind::Any`  (variant field
    accessors, lowercased prefix — mirrors IR compiler lines 357-359)

  Example for `(union Expr (IntLit (value : int) (span : any)) …)`:
  - `IntLit?`, `intlit-value`, `intlit-span`

  The asymmetry between record predicates (lowercased) and union variant
  predicates (original case) is intentional — it mirrors the IR compiler's
  own asymmetry.

### Why `TwigKind::Any` for all generated symbols

- The type checker does not verify accessor return types or enforce field-count
  arity on generated functions; that is done by the IR compiler.
- `Any` suppresses "unresolved variable" errors without introducing false
  arity mismatches for generated accessor calls.

### Tests added (6 — `mod tw05p1_tests`)

| Test | What it verifies |
|------|-----------------|
| `record_predicate_resolves_in_strict` | `(record Point …) (define (f r) (point? r))` → ok |
| `record_accessor_resolves_in_strict` | `(record Point …) (define (f r) (point-x r))` → ok |
| `union_variant_predicate_resolves_in_strict` | `(union Color …) (define (f v) (Red? v))` → ok |
| `union_variant_field_accessor_resolves_in_strict` | `(union Shape (Circle (radius : int))) …` → ok |
| `diagnostic_tw_strict_mode_compiles` | Full `diagnostic.tw` snippet in strict mode → ok |
| `iir_builder_tw_strict_mode_compiles` | Key `iir-builder.tw` functions in strict mode → ok |

All 92 prior tests continue to pass (98 total).

### Compiler modules converted to `(typed strict)`

With generated symbols now registered, 5 additional compiler modules can run
in strict mode:

| Module | Reason safe after LANG70 |
|--------|--------------------------|
| `token.tw` | 0 `define` forms — trivially safe |
| `ast.tw` | 0 `define` forms — trivially safe |
| `iir-types.tw` | 0 `define` forms — trivially safe |
| `diagnostic.tw` | Only calls own constructors (`Diagnostic`, `SevError`, …) |
| `iir-builder.tw` | Calls own accessors (`iirbuilder-name`, …) — safe after this change |

Combined with `span.tw` (LANG69), **6 of 11** compiler modules now use
`(typed strict)`.  The remaining 5 modules (`lexer.tw`, `cst-parser.tw`,
`parser.tw`, `emit.tw`, `main.tw`) call names from imported modules; converting
them requires TW05-P Part 2 (LANG71, multi-module import propagation).

### Backward compatibility

No public API changes.  All registered names are additive; no existing lookups
change.  The `TypeEnv`, `ScopeStack`, `TwigKind`, `type_check`, `check_program`,
and `type_check_source` interfaces are unchanged.

---

## [0.7.0] — 2026-05-17

### Added (LANG69 — TW05-O: Builtin Prelude + span.tw Strict Mode)

- **`TypeEnv::new()` now pre-registers all Twig runtime builtins** as
  `TwigKind::Any` so that calls to builtins no longer produce "unresolved
  variable" warnings in lenient mode or fatal errors in strict mode.

  Registered names include all 43 names from `twig-ir-compiler`'s `BUILTINS`
  const:
  - Arithmetic / comparison: `+`, `-`, `*`, `/`, `=`, `<`, `>`, `<=`, `>=`,
    `modulo`, `remainder`, `quotient`
  - Cons cells: `cons`, `car`, `cdr`
  - Predicates: `null?`, `pair?`, `number?`, `symbol?`, `not`, `boolean?`,
    `equal?`, `list?`
  - List stdlib: `list`, `length`, `append`, `reverse`, `list-ref`, `assoc`
  - Symbol utilities: `symbol-append`
  - Conversions: `number->string`, `string->symbol`, `symbol->string`
  - String / char ops: `string-length`, `string-ref`, `substring`,
    `string-append`, `string->number`, `string=?`, `string<?`, `string>?`,
    `char->integer`, `integer->char`, `char-alphabetic?`, `char-numeric?`,
    `char-whitespace?`
  - I/O: `print`, `host/write_string`, `host/read_line`, `host/read_file`
  - Higher-order: `map`, `filter`, `fold-left`, `fold-right`

  Additionally, **`and` and `or`** are pre-registered.  These two forms are
  special-cased in the IR compiler's `compile_apply` (not in `BUILTINS`),
  but the type checker sees them as plain `Apply` nodes.  Without this
  registration they produced "unresolved variable `and`" errors.

  `TwigKind::Any` is used rather than `Function { arity }` because several
  builtins are variadic (`list`, `string-append`); `Any` suppresses arity
  checks without introducing false positives.  Explicit `(define ...)` stubs
  in test prelude code shadow the pre-registered entries exactly as before.

- **New private method `TypeEnv::register_builtins`** — called from
  `TypeEnv::new()`; isolated so the list is easy to extend without changing
  the constructor signature.

### Tests added (8 — `mod tw05o_tests`)

| Test | What it verifies |
|------|-----------------|
| `builtin_arithmetic_resolves` | `(+ 1 2)`, `(< 1 2)`, `(>= 3 3)` in strict mode → `ok: true` |
| `builtin_list_ops_resolve` | `null?`, `cons`, `car`, `list`, `length` → ok |
| `builtin_string_ops_resolve` | `string-length`, `string-append`, `string=?` → ok |
| `builtin_and_or_resolve` | `(and #t #f)`, `(or #f #t)` → ok |
| `builtin_host_io_resolves` | `(host/read_file "x")` → ok |
| `builtin_hof_resolves` | `(map nil nil)`, `(filter nil nil)` → ok |
| `builtin_does_not_block_stub_shadow` | `(define (+ a b) 0)` shadows pre-registered `+`; call ok |
| `span_tw_strict_mode_compiles` | Full `span.tw` snippet in strict mode → `ok: true`, no errors |

All 84 prior tests continue to pass (92 total).

### No backward-compatibility impact

`TypeEnv`, `ScopeStack`, `TwigKind`, `type_check`, `check_program`, and
`type_check_source` are all unchanged.  Pre-registered builtins are shadowed
by any explicit `define` in user code, just like before.

---

## [0.6.0] — 2026-05-14

### Changed (LANG54 — Generic Refinement Protocol adoption)

- **`src/bridge.rs`** — new module.  Exports `TwigRefinementBridge`, which
  implements `lang_refinement_protocol::RefinementBridge` for Twig's `Expr`
  AST and `TwigKind` type system:
  - `evidence_for` — `IntLit → Concrete`, `VarRef+RefinedInt → Predicated`,
    everything else → `Unconstrained`.
  - `narrowing_facts` — delegates to `narrowing::extract_narrowing_facts`
    (unchanged from LANG53).
  - `narrow_kind` — delegates to `narrowing::merge_kind_with_predicate`
    (unchanged from LANG53).

- **`check.rs` `infer_apply`** — the 40-line manual `Checker::check` loop
  replaced with a single call to `check_call_site_refinements` from
  `lang-refinement-protocol`.  Behaviour is identical; the loop now lives
  in the generic crate and is shared with other languages.

- **`check.rs` `infer_if`** — the 35-line manual narrowing block replaced
  with a call to `compute_if_narrowing` from `lang-refinement-protocol`.
  Scope push/pop and branch inference are unchanged; only the predicate
  extraction + kind-narrowing computation is delegated.

- **`arg_to_evidence` helper removed** — this private function has been
  absorbed into `TwigRefinementBridge::evidence_for`.  No public API change.

- **`Cargo.toml`** — `lang-refinement-checker` dep replaced by
  `lang-refinement-protocol` (which re-exports `Evidence`, `Checker`,
  `CheckOutcome`, etc.).  `lang-refined-types` kept for `TwigKind::RefinedInt`.

### Tests added (10)

10 new tests in `bridge::tests` covering `TwigRefinementBridge` specifically:
`evidence_int_lit_is_concrete`, `evidence_var_ref_with_refined_kind_is_predicated`,
`evidence_var_ref_with_plain_int_is_unconstrained`, `evidence_bool_lit_is_unconstrained`,
`narrow_int_produces_refined_int`, `narrow_refined_int_intersects_predicates`,
`narrow_bool_is_unchanged`, `check_call_site_int_lit_in_range_no_diagnostic`,
`check_call_site_int_lit_out_of_range_is_diagnostic`,
`compute_if_narrowing_narrows_variable`.

All 74 LANG53 tests continue to pass (84 total).

### Backward compatibility

No public API changes.  `type_check`, `type_check_source`, `check_program`,
`TwigKind`, `TypeEnv` are all unchanged.  `TwigRefinementBridge` is new public
API (additive).

---

## [0.5.0] — 2026-05-14

### Added (LANG53 — TW05-C: Refinement Checker Bridge)

- **`TwigKind::RefinedInt(Predicate)`** — new variant carrying a
  `lang_refined_types::Predicate` through the type-checker pass.
  `RefinedInt(p)` is a subtype of `Int`: every value satisfying `p` is a
  valid `Int`, but not every `Int` satisfies `p`.  Mnemonic is `"int"` (same
  as plain `Int`) for stable downstream API compatibility.

- **`type_annotation_to_kind` updated** — `RangeInt { lo, hi }` now produces
  `RefinedInt(Predicate::Range { lo, hi, inclusive_hi: false })` rather than
  the old lossy `Int`.  `MembershipInt { values }` produces
  `RefinedInt(Predicate::Membership { values })`.

- **`annotation_to_refined_type`** — new helper in `kinds.rs`.  Converts a
  `TypeAnnotation` directly to a `lang_refined_types::RefinedType` for storage
  in `TypeEnv::fn_param_refinements`.  Returns `None` for unrefined
  annotations (no proof obligation).

- **`TwigKind::unify` updated** — integer-subtyping rules:
  - `unify(RefinedInt(p), RefinedInt(p))` = `RefinedInt(p)` (same predicate preserved)
  - `unify(RefinedInt(_), RefinedInt(_))` = `Int` (different predicates widen to Int)
  - `unify(RefinedInt(_), Int)` = `Int`  (one refined, one not → Int)
  - `unify(RefinedInt(_), Any)` = `Any`  (Any dominates)

- **`TypeEnv::fn_param_refinements`** — new field
  `HashMap<String, Vec<Option<RefinedType>>>`.  Stores the fully-lowered
  `RefinedType` per parameter for each top-level function that has at least
  one refined annotation.  Only functions with refinement-annotated params are
  stored.

- **`TypeEnv::register_fn_refinements`** — populates `fn_param_refinements`
  during Pass 1.

- **`classify_define` updated** — when the Lambda body has `param_annotations`,
  calls `annotation_to_refined_type` per-param and stores results via
  `register_fn_refinements`.

- **`infer_apply` updated** — call-site refinement checking (TW05-C).
  After arity checking, looks up `fn_param_refinements` for the callee and
  runs `lang_refinement_checker::Checker::check` per annotated argument.
  - `IntLit(n)` → `Evidence::Concrete(n)` → checked exactly.
  - `VarRef → RefinedInt(p)` → `Evidence::Predicated([p])` → checked under `p`.
  - `VarRef → Int / Any` → `Evidence::Unconstrained` → `Unknown`.
  - `ProvenUnsafe(cx)` → `TypeErrorDiagnostic` in all modes.
  - `Unknown` + `Strict` → `TypeErrorDiagnostic`.
  - `Unknown` + `Lenient` → silent.

- **`narrowing.rs`** — new module.  Provides:
  - `extract_narrowing_facts(guard: &Expr) -> Vec<(String, Predicate)>` —
    AST-level guard analysis.  Handles `<`, `<=`, `>`, `>=`, `=` comparisons
    (`VarRef op IntLit` form), `and` conjunction (merges facts with
    `Predicate::and`), and `not` negation (negates facts with `Predicate::not`).
    Conservative: anything else returns an empty Vec.
  - `merge_kind_with_predicate(base: &TwigKind, pred: Predicate) -> TwigKind` —
    narrows a variable's kind by adding a guard predicate.  `Int` → `RefinedInt(p)`;
    `RefinedInt(existing)` → `RefinedInt(and([existing, p]))`; everything else
    unchanged.

- **`infer_if` updated** — flow-sensitive narrowing.  After inferring the
  condition, calls `extract_narrowing_facts` and applies narrowing predicates
  to the true branch (via `push_frame` + `bind`) and negated predicates to the
  false branch.  Uses the existing `ScopeStack` push/pop mechanism with no
  structural changes.

### Tests added (12)

- `refined_kind_from_range_annotation` — `(Int 0 128)` annotation → `RefinedInt`
- `refined_kind_from_membership_annotation` — `(Member int 1 2 5)` → `RefinedInt`
- `unrefined_int_annotation_stays_int` — `int` annotation → plain `Int` (regression)
- `call_site_literal_in_range_no_error` — `(ascii-info 42)` → no error
- `call_site_literal_out_of_range_error` — `(ascii-info 200)` → `TypeErrorDiagnostic`
- `call_site_unconstrained_lenient_silent` — unresolved arg in lenient mode → silent
- `call_site_unconstrained_strict_error` — unresolved arg in strict mode → error
- `narrowing_lt_proves_call` — `(if (< x 128) (ascii-info x) 0)` → no error in then
- `narrowing_and_both_bounds` — `(if (and (>= x 0) (< x 128)) (ascii-info x) 0)` → no error
- `narrowing_not_in_else` — `(if (< x 128) 0 (ascii-info x))` → error in else branch
- `refined_kinds_unify_to_int` — `unify(RefinedInt(p1), RefinedInt(p2)) = Int`
- `no_narrowing_for_non_numeric` — bool/Any guards do not crash narrowing

Plus 13 unit tests added to `narrowing.rs` covering each guard form,
`not`, `and`-merging, and `merge_kind_with_predicate`.

### Dependencies added

- `lang-refined-types = { path = "../lang-refined-types" }`
- `lang-refinement-checker = { path = "../lang-refinement-checker" }`

### Intentional deferrals (planned for TW05-D)

- **Inter-procedural narrowing**: `(if (byte? x) (f x) …)` where `byte?` is a
  user predicate with a declared refinement effect.
- **CFG-based loop invariants**: `FunctionChecker` from `lang-refinement-checker`
  for path-sensitive multi-return-site checking.
- **Return-type annotation checking**: done by `iir-refinement-pass` (LANG42)
  at the IIR level; no duplication here.
- **`let`/`let*` binding refinements**: narrowing inside `let` RHS bindings.

---

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

---

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
