# Changelog — grammar-type-checker

## Unreleased — #18474 security review: bounded diagnostics

- The exhaustiveness message names at most 8 unmatched variants, then
  "… and K more", instead of joining every one. All three diagnostic sites
  push through `type_checker_protocol::bounded::push_bounded`: at most 256
  diagnostics and 512 KiB in all, then one note. The exhaustiveness check is
  skipped once diagnostics are closed.
- Test `generic_match_lists_at_most_max_listed_names`: 8 missing variants
  are listed, 9 are summarised, and 50,000 give a message under 200 bytes.

## [0.1.0] — 2026-05-14

### Added (LANG50 — Generic Grammar Type Checker)

Initial release.

- `check<P: LanguageProfile>(root, decls, profile) -> TypeCheckResult<AnnotatedNode>` —
  generic type-check entry point.  Annotation is always-on; enforcement is
  mode-gated (`Off` → no errors, `Lenient` → ok always, `Strict` → ok iff no errors).
- `LanguageProfile` trait — language-specific tree navigation.  Implementors
  match on `GrammarASTNode.rule_name` to recognise literals, variable
  references, function applications, binders, match expressions, and begin
  blocks.
- `BinderKind` enum (`Let { bindings, body }`, `Lambda { params, body }`) —
  distinguishes let-style from lambda-style binder forms.
- `BinderInfo`, `AppInfo`, `MatchInfo`, `MatchArmInfo`, `ArmPattern` —
  extracted information returned by `LanguageProfile` methods.
- `ScopeStack` — push/pop/bind/lookup lexical scope (inner-first lookup).
- `check::infer` — recursive core algorithm.
  - Literal detection via `literal_kind`.
  - Variable resolution: scope → globals → `Any` + error.
  - Arity check when callee is `KindDecl::Function { arity }`.
  - Union exhaustiveness check when scrutinee is `KindDecl::Named(union_name)`.
  - Scheme `let` semantics: all RHS evaluated in outer scope.
  - Depth cap: 256.
- `profile::{ast_node_children, ast_nodes_named, first_token_value, has_token_type}` —
  shared helpers for `LanguageProfile` implementations.
- 20 unit tests covering literals, variable resolution, arity, exhaustiveness,
  typed modes, and annotation propagation.
