# Changelog — @coding-adventures/forme-orchestrator

## 0.1.0 — 2026-05-15

Initial release.  FM03 §3-4, §9-10 — the runtime that takes a
`PipelineConfig`, builds a typed DAG, and executes it.  Last big
package of the FM03 orchestrator stack.

### Added

- `createOrchestrator(options?)` → `Orchestrator` factory.
- `Orchestrator.buildPipeline(config)` → `Pipeline` (validate + DAG).
- `Orchestrator.runOnce(pipeline, options?)` → `RunResult`.
- `Orchestrator.dispose()` lifecycle.
- `buildPipeline(config)` standalone (`createOrchestrator` is a thin
  factory wrapper around it).
- `runOnce(pipeline, options?, ctx?)` standalone.
- `buildDag(resolved)` — direct DAG construction.
- `areKindsCompatible(produces, consumes)` — FM01 §2.6 typecheck
  predicate covering name match, version compatibility (major must
  match, producer minor ≥ consumer minor), discriminant equality,
  and Stream wrapping (Stream<X> can feed single-X via iteration;
  single-X cannot feed Stream<X>).
- `RunResult` with `outcome`, `stages`, `outputs`, `errors`,
  `elapsedMs`, `buildId`.
- `RunError` and `StageRunSummary` for per-stage reporting.

### v0 simplifications (deferred)

- **No parallelism.** Sequential topological execution.
  `settings.maxConcurrency` is honoured at 1.
- **No streaming pipelining.** Stream producers are fully drained
  before downstream consumers see values.
- **No incremental rebuild.** Cache backend exists but isn't hit yet.
- **No reproducible-build mode.**
- **No watch mode.**
- **No OpenTelemetry traces.**

### What works

- Linear and fan-out-1 pipelines (source → transform... → sink)
- Stream producers feeding single-input consumers (executor iterates)
- init / dispose lifecycle (init failure → abort + dispose initialized)
- Fail-fast (default) and best-effort error handling
- Cancellation: composing tokens, throwIfCancelled inside loops
- Named outputs via `OutputSpec` mapping
- buildId via `computeRevisionId` over source/sink ids

### Dependencies

- `@coding-adventures/forme-types` — Kinds, KindDescriptor
- `@coding-adventures/forme-stage` — Stage, StageContext, defaults
- `@coding-adventures/forme-errors` — StageError, CancellationError
- `@coding-adventures/forme-pipeline-config` — validateConfig
- `@coding-adventures/forme-cache` — CacheBackend (held but not
  invoked in v0)
- `@coding-adventures/forme-identity` — computeRevisionId for buildId
- `@coding-adventures/forme-capability` — Capability (type only)
