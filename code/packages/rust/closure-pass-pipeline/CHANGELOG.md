# Changelog

All notable changes to the `coding-adventures-closure-pass-pipeline` crate will be documented in this file.

## [0.1.0] - 2026-05-23

### Added
- New crate per CLOC06. The harness all `closure-pass-*` crates plug into.
- `Pass` trait per CLOC06 §"The `Pass` trait": `name()` (required), `depends_on()`/`invalidates()` (default `&[]`), `iteration_policy()` (default `OneShot`), `cost()` (default `1`), `run(ctx) -> Result<PassOutput, PassError>`. Object-safe so passes can be `Box<dyn Pass>`.
- `IterationPolicy::OneShot | FixedPoint` — FixedPoint executes once in v1 with a diagnostic note (fixed-point looping lands with the first mutating pass).
- `PassContext<'a> { program: &Program, sidecar: &Sidecar, cv: &mut CVLog }` — minimal v1 context. CLOC06's `options` / `prior` slots arrive when they're actually used.
- `PassOutput { program, contributions, changed, diagnostics, stats }` with `PassStats { nodes_touched: u32 }`.
- `PassError { pass_name, message }` with `Display` + `std::error::Error` impls.
- `PassPipeline` with `new()`, `add(pass)`, `len()`, `is_empty()`, `run(program, sidecar, cv) -> Result<PipelineOutput, PassError>`. `Default` impl returns an empty pipeline.
- `PipelineOutput { program, diagnostics, stats, execution_order }` — `stats` is a `HashMap<String, PassStats>` keyed by pass name; `execution_order` records the topological order the scheduler used.
- Topological sort by `depends_on` with stable tie-breaking by registration order. Cycles produce `PassError { pass_name: <cycle-member>, message: "dependency cycle detected …" }`. Duplicate pass `name()` returns also error.
- Unknown `depends_on` targets are silently dropped (CLOC06 doesn't pin behavior; v1 picks "permissive").
- 11 tests covering: empty pipeline → identity, single pass runs and reports stats, `depends_on` forces reordering against registration order, independent passes keep registration order, dependency cycle errors cleanly with helpful Display, duplicate names error, FixedPoint runs once with diagnostic, unknown dep silently dropped, diamond dependency resolves correctly, `Default` impl, `PassError` Display + Error.

### Notes
- Dependencies: `coding-adventures-javascript-ast`, `coding-adventures-type-sidecar`, `coding-adventures-closure-typechecker` (for `Diagnostic` / `Severity` / `DiagnosticGroup`), `coding_adventures_correlation_vector`, `serde_json`. Plus `coding-adventures-javascript-tokens` as a dev-dependency for `EsVersion` in tests.
- No `closure-pass-*` deps — those depend on this crate, not the other way.
- Deferred: `Pass::cost()` budget gating, FixedPoint iteration loop, `PassOptions` for SIMPLE/ADVANCED/CUSTOM modes + enable/disable lists, `PassResults` for cross-pass data, coarse-grained invalidation re-run logic.
