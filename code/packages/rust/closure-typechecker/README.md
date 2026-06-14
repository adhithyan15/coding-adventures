# coding-adventures-closure-typechecker

JavaScript type checker (Closure-style). Consumes `(Program, Sidecar)`
and produces typed judgments plus diagnostics per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md), with the
severity model from
[CLOC08](../../../specs/CLOC08-closurec-cli-surface.md).

## What's here (v1)

- `check(program, sidecar, cv) -> CheckResult` — passthrough judgment.
  Looks up each node's CvId in the sidecar; if a record has a concrete
  `ty`, the result map gets the judgment and a `"judged"` CV
  contribution is appended per
  [CLOC03 §"Stage 3 — Typechecker"](../../../specs/CLOC03-correlation-vector-plumbing.md).
- `CheckResult { judgments: HashMap<CvId, Type>, diagnostics: Vec<Diagnostic> }`
  with `has_judgment()`/`judgment()` accessors.
- `Diagnostic { cv, severity, group, message }`, `Severity::Error / Warning / Note`,
  `DiagnosticGroup(String)` — the surface CLOC08's CLI will render.

## What's deferred

- **Real inference.** v1 is passthrough — once `javascript-ast` grows
  `Statement` / `Expression` variants (deferred from CLOC02 Phase 1),
  the inference engine slots in here.
- **Diagnostic emission.** v1 emits no diagnostics; the type surface
  is pinned so passes and the CLI can be written against it now.

## Dependency whitelist

- `coding-adventures-javascript-ast` — Program input.
- `coding-adventures-type-sidecar` — Sidecar/Type/Record input.
- `coding_adventures_correlation_vector` — for `CVLog` + `Contribution`
  plumbing.
- `serde_json` — for `Contribution.meta` JSON values.

No `closure-pass-*` deps — passes depend on *this* crate, not the
other way. No `javascript-parser`/`-lexer` — the typechecker operates
on the AST + sidecar, not on tokens.
