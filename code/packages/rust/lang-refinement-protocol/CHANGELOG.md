# Changelog — lang-refinement-protocol

## [0.1.0] — 2026-05-14

Initial release. LANG54 — Generic Refinement-Type Checker Protocol.

### Added

- **`RefinementBridge` trait** — three-method trait Language X implements to
  get call-site refinement checking and flow-sensitive narrowing for free:
  - `evidence_for(expr, inferred_kind) -> Evidence` — classify a call-site
    argument as `Concrete`, `Predicated`, or `Unconstrained`.
  - `narrowing_facts(guard) -> Vec<(String, Predicate)>` — extract per-variable
    predicates implied by a guard expression being true.
  - `narrow_kind(base, pred) -> Kind` — merge a base kind with a narrowing
    predicate (e.g., `Int + (x < 128)` → `RefinedInt(x < 128)`).

- **`check_call_site_refinements`** generic free function — drives
  `lang_refinement_checker::Checker::check` per annotated parameter at a
  function call site:
  - `ProvenUnsafe(cx)` → `RefinementDiagnostic` (error in all modes).
  - `Unknown` + `Strict` → `RefinementDiagnostic`.
  - `Unknown` + `Lenient` → silent.
  - `ProvenSafe` → silent.

- **`compute_if_narrowing`** generic free function — computes `NarrowedBindings<K>`
  for both branches of an `if`-expression:
  - Calls `bridge.narrowing_facts(guard)` to extract facts.
  - Looks up each variable via the provided `scope_lookup` closure.
  - `true_branch`: `bridge.narrow_kind(base, pred)` for each fact.
  - `false_branch`: `bridge.narrow_kind(base, Predicate::not(pred))`.
  - Variables not in scope are silently skipped (conservative: no narrowing).

- **`NarrowedBindings<K>`** — returned by `compute_if_narrowing`:
  - `true_branch: Vec<(String, K)>`
  - `false_branch: Vec<(String, K)>`

- **`RefinementDiagnostic`** — error/warning from refinement checking:
  - `message: String`, `line: usize`, `column: usize`.

- **`RefinementMode`** — `Lenient` | `Strict`, controls `Unknown` handling.

- **Re-exports** from `lang-refinement-checker`: `Evidence`, `CheckOutcome`,
  `CounterExample`, `Checker`, `Obligation`, `check_all`.

- **Re-exports** from `lang-refined-types`: `Predicate`, `RefinedType`,
  `Kind as RefKind`.

### Language X adoption cost

~70 lines total:
- Implement `RefinementBridge` (~50 lines: 3 methods).
- Wire `check_call_site_refinements` into apply handler (~10 lines).
- Wire `compute_if_narrowing` into if-expression handler (~10 lines).

### Tests

14 unit tests with a `MockBridge` (`MockExpr` + `MockKind`) covering:
- Call-site: concrete in range, concrete out of range, unconstrained lenient,
  unconstrained strict, no annotation, no param_refinements, predicated safe,
  predicated unsafe.
- Narrowing: true + false branches extracted, no facts → empty, variable not
  in scope skipped, false branch is negated predicate, `and` guard produces
  two facts, non-numeric kind unchanged.
