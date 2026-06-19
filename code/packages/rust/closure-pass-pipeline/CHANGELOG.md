# Changelog

All notable changes to the `coding-adventures-closure-pass-pipeline` crate will be documented in this file.

## [0.3.0] - 2026-06-17

### Added — real fixed-point iteration (CLOC13.F)

`PassPipeline::run` no longer runs each pass exactly once. It topo-sorts the
passes once, then runs that order in repeated **sweeps**, continuing while any
`FixedPoint` pass reports `PassOutput::changed` — so a transform one pass
exposes is picked up by an earlier pass on the next sweep:

```text
inline turns double(7) into 7 * 2   (sweep 1)
constant-fold folds 7 * 2 into 14   (sweep 2 — fold ran before inline in sweep 1)
no FixedPoint change                (sweep 3 → converged)
```

- **`OneShot` passes re-run each sweep but do NOT drive the loop** — only a
  `FixedPoint` pass's `changed` triggers another sweep. OneShot passes are
  expected to be idempotent at the fixed point, so re-running them just lets
  them observe the converged program (e.g. `rename` shortens names on the
  fully-folded output) without risking a spin.
- **`MAX_SWEEPS` cap (100)** — a backstop against a buggy pass that reports
  `changed = true` forever (e.g. two passes that undo each other). Real
  cascades converge in a handful of sweeps. Hitting the cap emits a
  `pipeline.fixed-point-cap-reached` note instead of looping or silently
  under-optimizing.
- **Removed** the `pipeline.fixed-point-not-yet-iterated` diagnostic — the
  limitation it described is gone.
- Diagnostics and per-pass stats now reflect the **final (converged) sweep**;
  CV contributions accumulate across all sweeps (each is a real transformation
  in the provenance record).

### Tests
- New: a counting `FixedPoint` pass that changes N times then converges
  (asserts N+1 runs — N changing sweeps + 1 confirming), an always-changing
  `OneShot` pass that must NOT spin the loop, and a runaway `FixedPoint` pass
  that hits the cap and surfaces the note.

Crate bumped `0.2.0 → 0.3.0`.

## [0.2.0] - 2026-05-24

### Added — `PassRegistry` runtime-discovery layer (CLOC10.A)
- `PassRegistry` per [CLOC10 §5](../../../specs/CLOC10-pass-plugin-api.md#5-passregistry-runtime-discovery): a runtime catalog of named pass factories. Lets a host (typically `closurec`) build pipelines by *naming* passes instead of holding onto concrete `Box<dyn Pass>` values. This is what makes user-facing pass selection (`--enable=<name>`, `--passes <config>`, REPLs, plugins) practical.
- `PassFactory = Box<dyn Fn() -> Box<dyn Pass> + Send + Sync>` — factories rather than stored boxes so the registry can hand out fresh instances per `build_pipeline` call (no shared state across pipeline runs, no one-shot registry).
- `PassRegistry::new() / Default / Debug` — `Debug` lists registered names (factories can't be debug-printed).
- `register(name, factory) -> Result<(), RegistryError>` — `Err(DuplicateName)` if `name` is already taken. Strict by design: silent shadowing of pass names would be a debugging nightmare.
- `contains(name) -> bool`, `len() -> usize`, `is_empty() -> bool`, `registered_names() -> Vec<String>` (sorted alphabetically — what `closurec --list-passes` will emit per CLOC10 §6).
- `build_pipeline(&[&str]) -> Result<PassPipeline, RegistryError>` — instantiates a fresh pipeline containing the named passes in the given order. Stops at first `Err(UnknownPass)`. Input order is preserved (relevant as the topo-sort's tie-breaker).
- `RegistryError { UnknownPass(String), DuplicateName(String) }` with `Display` + `std::error::Error` — friendly CLI messages, no panics.
- 9 new tests: empty registry, register-and-contains, duplicate-name error preserves first registration, sorted `registered_names()`, `build_pipeline` preserves input order, unknown-name error, factories produce fresh instances each call (verified via shared `AtomicUsize` counter), empty input → empty pipeline, `Debug` lists names, `RegistryError` is `std::error::Error`.

### Notes
- `PassRegistry::new()` deliberately does NOT pre-populate the 8 canonical passes (`constant-fold`, `dce`, `fold-control-flow`, `inline`, `rename`, `treeshake`, `collapse-properties`, `remove-unused-vars`). Auto-populating would force `closure-pass-pipeline` to depend on every `closure-pass-*` crate, which would create a circular dep (each pass already depends on pipeline for the `Pass` trait). The convention per CLOC10 is: the *host* (typically `closurec`) imports each pass crate and calls `registry.register(...)` for each canonical pass at startup. CLOC10.C will wire this up in the CLI.
- No `Pass` trait changes — all existing `closure-pass-*` crates and their tests are unaffected.

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
