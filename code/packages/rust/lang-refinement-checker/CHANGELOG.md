# Changelog — `lang-refinement-checker`

## 0.2.0 — 2026-05-04

Function-scope refinement checker.  **LANG23 PR 23-D.**

### Added

- `function_checker` module — the PR 23-D function-scope checking API.
  - `BranchGuard` struct: `var: String` + `predicate: Predicate`.  Represents a guard
    condition at an `if`/`cond` branch, expressed in the LANG23 `Predicate` vocabulary.
  - `ReturnValue` enum: `Literal(i128)` | `Variable(String)`.  What a return statement
    produces — a compile-time constant or a function-scope variable.
  - `CfgNode` enum: `Branch { guard, then_node, else_node }` | `Return(ReturnValue)`.
    A tree-structured CFG sufficient for LANG23 v1 `if`/`cond` vocabulary; each branch
    is an owned subtree so the checker can fork the predicate scope without allocation
    sharing.
  - `FunctionSignature` struct: `params: Vec<(String, RefinedType)>` + `return_type:
    RefinedType`.  Annotated function signature seeding the initial predicate scope.
  - `ReturnSiteOutcome` struct: `label: String` + `outcome: CheckOutcome`.  Outcome for
    one `CfgNode::Return` node reached by the path-sensitive traversal.
  - `FunctionCheckResult` struct: `return_sites: Vec<ReturnSiteOutcome>` + helpers:
    - `all_proven_safe()` — true if every return site is `ProvenSafe`.
    - `has_violation()` — true if any return site is `ProvenUnsafe`.
    - `first_counter_example()` — first counter-example found (if any).
    - `runtime_check_count()` — number of `Unknown` return sites.
    - `is_vacuous()` — true if no return sites exist (diverging function).
  - `FunctionChecker` struct: stateless between `check_function` calls; wraps
    an inner `Checker` (PR 23-C) and adds the CFG walk.
    - `check_function(&sig, &cfg) -> FunctionCheckResult`: path-sensitive DFS
      over the CFG tree.  At each `Branch`, forks the predicate scope: then-arm
      gets `guard.predicate`, else-arm gets `Predicate::not(guard.predicate)`.
      At each `Return`, builds `Evidence` from the scope and calls
      `Checker::check`.
  - `substitute_var(pred, from, to) -> Predicate`: substitutes variable names
    inside `LinearCmp` coefficients; `Range`/`Membership`/`Opaque` are returned
    unchanged.  Bridges the gap between "predicates over parameter `x`" and
    "predicates over `__v`" that the underlying `Checker` expects.
- 22 unit tests in `function_checker::tests` covering:
  - `clamp-byte` example — all three return paths proven safe (primary acceptance criterion).
  - `clamp-byte` with tighter return type `[0, 200)` — violation detected at `return 255`.
  - `clamp-byte` with annotation `[1, 256)` — `return 0` detected as unsafe with cx = 0.
  - Identity function with matching param annotation → `ProvenSafe`.
  - Identity function with no param annotation → `Unknown` (unconstrained).
  - Unrefined return type → all sites immediately `ProvenSafe`.
  - Single branch with two literal arms.
  - Guard narrows variable into annotation range.
  - Three-level deep nesting (4 return sites).
  - `LinearCmp` guard with `substitute_var` verifying variable renaming.
  - `substitute_var` on `Range`/`Membership` (identity), on `LinearCmp` (renames),
    on mismatched var (no rename), through `Not`, through `And`/`Or`.
  - `FunctionCheckResult` accessors.
  - Return site label format.
  - Vacuous CFG result.
  - Narrower param annotation implies return.
  - Wider param annotation → violation.
  - Guard on a different parameter from the returned variable.

### Changed

- Crate description updated to mention both 23-C and 23-D.
- `lib.rs` module-level doc updated to show the two-module structure.

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
