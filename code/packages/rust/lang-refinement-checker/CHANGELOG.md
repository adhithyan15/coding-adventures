# Changelog — `lang-refinement-checker`

## 0.1.0 — 2026-05-04

Initial release.  **LANG23 PR 23-C.**

### Added

- `Evidence` enum: `Concrete(i128)`, `Predicated(Vec<Predicate>)`, `Unconstrained`.
- `CheckOutcome` enum: `ProvenSafe`, `ProvenUnsafe(CounterExample)`, `Unknown(String)`.
  - `is_safe()`, `is_unsafe()`, `is_unknown()`, `counter_example()` accessors.
- `CounterExample` struct: `value: i128` + `description: String`.
- `Checker` struct: stateless between calls; builds and runs constraint programs.
  - `check(&annotation, &evidence) -> CheckOutcome`.
  - Fast path: direct predicate evaluation for `Concrete` evidence (no solver call).
  - Two-pass SAT strategy for `Predicated` evidence: check-sat first, then get-model
    only if SAT (avoids `VmError::NoModel` when the obligation is proven safe).
  - Kind bounds injected into LIA programs for bounded kinds (U8, I8, …).
- `Obligation` struct: label + annotation + evidence, for deferred/batch checking.
- `check_all(obligations)`: runs a batch of obligations through a shared `Checker`.
- 22 unit tests + 1 doc-test covering:
  - Unrefined annotations (always safe).
  - Concrete evidence: Range, Membership, And, Or, Not, LinearCmp.
  - Unconstrained evidence (always Unknown).
  - Predicated evidence: subset-implies-annotation (ProvenSafe) and
    partial-overlap (ProvenUnsafe with model), CFG guard narrowing.
  - Unsupported kinds (Float, Str → Unknown).
  - Opaque predicates (Unknown regardless of evidence).
  - Batch obligation checking via `check_all`.
