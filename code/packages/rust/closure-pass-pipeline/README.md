# coding-adventures-closure-pass-pipeline

The **harness** all `closure-pass-*` crates plug into. Defines the
[`Pass`] trait every optimization implements and ships the
[`PassPipeline`] scheduler that runs them. Per
[CLOC06](../../../specs/CLOC06-pass-interface-contract.md).

## What's here

- **`Pass` trait** — `name`, `depends_on`, `invalidates`,
  `iteration_policy`, `cost`, `run(ctx) -> Result<PassOutput, PassError>`.
  Object-safe so passes can be held as `Box<dyn Pass>`.
- **`IterationPolicy::OneShot | FixedPoint` with real fixed-point
  iteration** — `run` sweeps the topo order repeatedly while any
  `FixedPoint` pass reports `changed`, so transforms cascade
  (`inline`'s `7 * 2` → next sweep's `constant-fold` `14`). OneShot
  passes re-run each sweep but don't drive the loop; a `MAX_SWEEPS`
  cap (100) backstops a non-convergent pass with a
  `pipeline.fixed-point-cap-reached` note.
- **`PassContext { program, sidecar, cv }`** — the minimal context
  passes need today. CLOC06's `options` / `prior` slots arrive when
  they're actually used.
- **`PassOutput { program, contributions, changed, diagnostics, stats }`**
  with `PassStats { nodes_touched }`.
- **`PassError { pass_name, message }`** with `Display` + `Error`.
- **`PassPipeline`** with `new()`, `add(pass)`, `run(program,
  sidecar, cv) -> Result<PipelineOutput>`.
- **`PipelineOutput { program, diagnostics, stats, execution_order }`**
  — `stats` keyed by pass name; `execution_order` is the actual
  topological order the scheduler used (useful for debugging
  dependency-graph issues).
- **Topo-sort by `depends_on`** with stable tie-breaking by
  registration order. Cycles → `PassError`. Duplicate pass names →
  `PassError`. Unknown deps are silently dropped in v1.

## What's deferred (follow-up PRs)

- **Cost / budget gating** — v1 ignores `Pass::cost()` entirely.
- **Per-pass convergence skipping** — every registered pass re-runs
  each sweep; a pass that has provably converged could be skipped.
  Correctness doesn't need it (idempotent passes are no-ops), so it's
  a performance follow-up.
- **`PassOptions`** for SIMPLE/ADVANCED/CUSTOM mode + enable/disable
  lists per CLOC06.
- **`PassResults`** so passes can read prior passes' outputs without
  consulting the CV log.
- **Coarse-grained invalidation** (CLOC06 Open Question 1): v1
  doesn't re-run passes when their `invalidates()` targets change.

## Dependency whitelist

- `coding-adventures-javascript-ast` — `Program` flowing through.
- `coding-adventures-type-sidecar` — `Sidecar` flowing through.
- `coding-adventures-closure-typechecker` — for `Diagnostic` /
  `Severity` / `DiagnosticGroup` (every pass surface diagnostics in
  the same shape).
- `coding_adventures_correlation_vector` — `CVLog` + `Contribution`.
- `serde_json` — for `Contribution.meta` JSON values.

No `closure-pass-*` deps — those depend on this crate, not the
other way.
