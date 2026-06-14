# Changelog — @coding-adventures/forme-orchestrator

## 0.2.0 — 2026-05-16

### Added — reproducible-build mode (FM03 §8 — partial)

- When `pipeline.config.settings.reproducibleBuild === true`, every
  `StageContext` (and `StageInitContext`, and the dispose-time context)
  now receives a `frozenClock` instead of a `systemClock`.  Two runs
  of the same pipeline against the same inputs produce identical
  `ctx.time.nowMs()` / `nowIso()` values, which is sufficient for the
  hello-world topology to produce byte-identical artifacts across
  runs.
- Frozen wall-clock value: `REPRO_BUILD_FROZEN_TIMESTAMP_MS = 0`
  (1970-01-01T00:00:00Z).  Per FM03 §8 the production value should
  be the max input mtime; v0 always uses the fallback since the
  orchestrator doesn't yet thread input mtimes from source stages.
- Frozen monotonic counter advances by 1 ms per call so stages
  measuring their own elapsed time still get non-zero values (the
  reproducible contract is on the wall clock only).
- New public export: `REPRO_BUILD_FROZEN_TIMESTAMP_MS`.

### What's still deferred from FM03 §8

- **Iteration-order sorting.**  Sources should iterate paths in
  lexicographic order under repro mode.  Lives in source stages,
  not the orchestrator (`forme-source-fs` already sorts within
  directories; repro-mode-specific cross-directory ordering is a
  follow-up).
- **Deterministic randomness.**  `ctx.random.deterministic(name)`
  doesn't exist in the kernel yet (FM01 future work).  No stage
  currently uses randomness so this is harmless in practice.
- **Telemetry suppression.**  Telemetry events still fire in repro
  mode; embedding them into artifacts is the artifact-producer's
  policy, which today is correct (no stage embeds telemetry into
  output).

### Tests

6 new tests in `tests/repro-build.test.ts` covering:
- Off-mode: two runs produce different timestamps.
- On-mode: two runs produce identical timestamps.
- On-mode: source `ctx.time.nowMs()` is also frozen.
- On-mode: `dispose()` sees the same frozen clock as `run()`.
- Public-API constant export.
- Default (off) mode produces realistic timestamps.

## 0.1.1 — 2026-05-15

### Fixed

- **DAG typecheck now agrees with the scheduler on stream-iteration
  promotion.**  When a per-item stage (consumes X, produces Y) sits
  between a stream source (Stream<X>) and a stream sink
  (Stream<Y>), the scheduler iterates the source and invokes the
  per-item stage N times — yielding N Y values that downstream
  consumers see as Stream<Y>.  The DAG builder used to reject the
  wire before scheduling ever ran (it compared the per-item stage's
  declared `produces: Y` against the consumer's `consumes:
  Stream<Y>` and failed).  `buildDag` now tracks an `effectiveProduces`
  per instance — when an instance's input is a stream and its declared
  consumes/produces are single values, its effective downstream output
  is promoted to `Stream<produces>`.  Two new integration tests pin
  the behaviour (Stream → per-item → Stream builds; pure stream-stream
  chains aren't double-wrapped).

  This was discovered while wiring the blog site (FM00 §5 demo): the
  natural shape — source-fs (Stream) → parse-markdown (per-item) →
  render-static (Stream) → emit-fs (Stream) — couldn't be built.

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
