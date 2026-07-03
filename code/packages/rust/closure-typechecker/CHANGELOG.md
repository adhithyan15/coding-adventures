# Changelog

All notable changes to the `coding-adventures-closure-typechecker` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- **Stage 3 begins.** First crate in the typechecker + pass pipeline per CLOC06.
- `check(program: &Program, sidecar: &Sidecar, cv: &mut CVLog) -> CheckResult` — v1 passthrough: every `Program`-root CvId with a `ty`-bearing record in the sidecar lands in `CheckResult::judgments` as a typed entry, and a `"judged"` `Contribution` gets appended to the CV log per CLOC03 §"Stage 3 — Typechecker."
- `CheckResult { judgments: HashMap<String, Type>, diagnostics: Vec<Diagnostic> }` with `has_judgment(cv)` and `judgment(cv)` accessors plus `new()` / `Default`.
- `Diagnostic { cv: String, severity: Severity, group: DiagnosticGroup, message: String }` — the diagnostic shape from CLOC08 pinned for downstream consumers.
- `Severity::Error / Warning / Note`.
- `DiagnosticGroup(String)` with `DiagnosticGroup::new(name)`.
- `type_label` helper renders `Type` values into the strings used in `Contribution.meta["type"]`.
- 10 tests covering: empty sidecar, record with `ty` → judgment + CV contribution, record with `ty = None` → no judgment, no sidecar entry → no judgment, accessor methods, Diagnostic API buildability, `type_label` for primitives and `Opaque`, disabled CV log still works (CLOC03 production fast path).

### Notes
- Dependencies: `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar`, `coding_adventures_correlation_vector`, `serde_json`. No `closure-pass-*` (those depend on this crate); no `javascript-parser`/`-lexer` (the typechecker operates on the AST + sidecar, not on tokens).
- v1 is **scaffolding**: real inference lands once `javascript-ast` grows `Statement` / `Expression` variants. Even so, this PR does real work — establishes the `check` API the future `closurec` CLI calls, plumbs CV contributions, and pins the Diagnostic surface for passes and CLI to be written against now.
