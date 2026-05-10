# Changelog — `lang-refinement-checker`

## 0.4.0 — 2026-05-04

Program-scope annotation-completeness checker.  **LANG23 PR 23-G.**

### Added

- `program_checker` module — the PR 23-G program-scope (rung 4) checking API.
  - `ViolationKind` enum: `UnrefinedParam { param_index: usize, param_name: String }` |
    `UnrefinedReturn`.  The kind of missing annotation.  `UnrefinedReturn` is
    only produced when the checker is constructed with `with_return_type_checking()`.
  - `AnnotationViolation` struct: `module_name`, `function_name`, `kind`,
    `description`.  One entry per missing annotation.  `description` carries a
    human-readable compiler error message: `"[text/ascii] decode: parameter 0
    'codepoint' has no refinement annotation (': any')"`.
  - `ProgramCheckResult` struct: `violations: Vec<AnnotationViolation>` + helpers:
    - `is_clean()` — true iff no violations found.
    - `has_violations()` — true iff at least one violation found.
    - `violation_count()` — length of `violations`.
    - `error_message()` — multi-line human-readable listing of all violations.
    - `violating_modules()` — deduplicated list of module names with violations.
  - `ProgramModule` struct: `name: String` + `scope: ModuleScope`.  A named
    module with its public function-signature registry.  Functions absent from
    the scope are excluded from the check (per-symbol opt-out, consistent with
    the module-scope checker).
  - `ProgramChecker` struct: purely structural — no solver, no CFG walk.
    - `new()` — default constructor; only parameter annotations are checked.
    - `with_return_type_checking()` — builder method enabling return-type checks.
    - `check_program(&[ProgramModule]) -> ProgramCheckResult` — iterates every
      module's `ModuleScope`, every registered function, every parameter; emits
      `UnrefinedParam` for `: any` parameters and optionally `UnrefinedReturn`
      for unrefined return types.
  - `ModuleScope::iter()` — added `pub(crate)` iterator to `module_checker`'s
    `ModuleScope` so `program_checker` can enumerate all registered functions
    without accessing the private `HashMap` field directly.
- 26 unit tests + 3 doc-tests in `program_checker` covering:
  - Primary acceptance criterion: strict mode rejects a refinement-incomplete
    module (one with an unrefined param alongside a fully annotated function).
  - Fully annotated program passes.
  - Empty program and empty module pass (vacuous).
  - Single unrefined first param — one violation.
  - Two unrefined params — two violations.
  - Mixed params (first annotated, second not) — one violation at index 1.
  - Zero-param function — always clean.
  - Unrefined return not flagged by default.
  - Unrefined return flagged when `with_return_type_checking()` enabled.
  - Annotated return passes return-type check.
  - Unrefined param + unrefined return → 2 violations with return checking on.
  - Violations across multiple modules all collected.
  - Clean module + dirty module → only dirty module produces violations.
  - `error_message()` empty when clean; contains count and param info when not.
  - `violating_modules()` returns correct names and deduplicates.
  - `violation_count()` matches `violations.len()`.
  - Violation description format for param and for return.
  - `ProgramChecker::default()` equivalent to `ProgramChecker::new()`.
  - `with_return_type_checking()` doc-test and module-level doc-test.

### Changed

- Crate version bumped `0.3.0 → 0.4.0`.
- Crate description updated to mention 23-C, 23-D, 23-F, and 23-G.
- `lib.rs` module-level doc updated to show the four-module structure and the
  new `ProgramChecker` entry in the module table and architecture diagram.
- `module_checker::ModuleScope` gained `pub(crate) fn iter()` to support the
  new `program_checker` module.

## 0.3.0 — 2026-05-04

Module-scope refinement checker.  **LANG23 PR 23-F.**

### Added

- `module_checker` module — the PR 23-F module-scope checking API.
  - `CallArg` enum: `Literal(i128)` | `Variable(String)`.  An argument at a
    cross-function call site.  `Literal` evidence is `Concrete(v)`; `Variable`
    evidence is gathered from the path-predicate scope (same substitution
    logic as `FunctionChecker`).
  - `ModuleCfgNode` enum: `Branch { guard, then_node, else_node }` |
    `Call { callee, args, next }` | `Return(ReturnValue)`.  Extends the
    function-scope `CfgNode` (PR 23-D) with a `Call` variant for cross-function
    call sites.  `Call` carries the callee name, argument list, and the
    remaining CFG path (`next`) so sequential calls on the same path are all
    checked.
  - `ModuleScope` struct: a `HashMap`-backed registry of `FunctionSignature`s.
    - `new()` — empty scope.
    - `register(name, sig)` — add or replace an entry; returns `&mut self` for
      chaining.
    - `get_signature(name)` — `Option<&FunctionSignature>` (opt-out via absence).
    - `len()` / `is_empty()` — introspection.
  - `CallSiteOutcome` struct: `callee: String`, `param_index: usize`,
    `label: String`, `outcome: CheckOutcome`.  One entry per argument per
    registered callee encountered during the DFS walk.
  - `FunctionBodyCheckResult` struct: `return_sites: Vec<ReturnSiteOutcome>` +
    `call_sites: Vec<CallSiteOutcome>`.  Aggregate for an entire function body.
    - `all_proven_safe()` — true iff every return site and call-site argument
      is `ProvenSafe` (false for vacuous results).
    - `all_call_sites_proven_safe()` — true iff every call-site argument is
      `ProvenSafe` (ignores return sites; vacuous call_sites → true).
    - `has_violation()` — true if any outcome is `ProvenUnsafe`.
    - `first_counter_example()` — first counter-example across return and call
      sites (return sites scanned first).
    - `runtime_check_count()` — number of `Unknown` outcomes across both vecs.
    - `is_vacuous()` — true iff both vecs are empty.
  - `ModuleChecker` struct: holds `ModuleScope` + inner `Checker`.
    - `new(scope)` — construct.
    - `check_function(&sig, &cfg) -> FunctionBodyCheckResult` — seeds the
      predicate scope from `sig.params`, walks the `ModuleCfgNode` tree
      path-sensitively; at `Branch` forks scope; at `Call` checks each arg
      against the callee's param annotation (if registered); at `Return` checks
      against `sig.return_type`.
  - Safety budgets: `MAX_MODULE_CFG_DEPTH = 64` (stack-overflow guard),
    `MAX_MODULE_RETURN_SITES = 1_024` (memory guard),
    `MAX_MODULE_CALL_SITES = 4_096` (memory guard).  All emit `Unknown` rather
    than crashing.
- 21 unit tests + 3 doc-tests in `module_checker` covering:
  - `latin1-decode` / `decode` example from the LANG23 spec — primary acceptance
    criterion: call proven safe by the `< cp 128` guard narrowing `cp`.
  - Direct call without guard — `ProvenUnsafe` (counter-example extractable).
  - Literal argument in range → `ProvenSafe` (fast path, no solver).
  - Literal argument out of range → `ProvenUnsafe` with correct counter-example.
  - Unregistered callee → no call-site outcomes (per-symbol opt-out).
  - Unconstrained variable (no annotations, no guards) → `Unknown`.
  - Multiple annotated params: all safe, and one unsafe.
  - Return site + call site in the same function body.
  - `all_proven_safe` / `all_call_sites_proven_safe` / `has_violation` /
    `first_counter_example` / `runtime_check_count` / `is_vacuous`.
  - `ModuleScope` `register`/`len`/`is_empty`/`get_signature` API.
  - Call-site outcome label format.
  - Depth-limit does not crash.
  - Call-site count limit bounds collection.

### Changed

- Crate version bumped `0.2.0 → 0.3.0`.
- Crate description updated to mention 23-C, 23-D, and 23-F.
- `lib.rs` module-level doc updated to show the three-module structure and
  the new `ModuleChecker` entry in the module table.

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
