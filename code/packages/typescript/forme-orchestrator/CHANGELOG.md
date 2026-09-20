# Changelog — @coding-adventures/forme-orchestrator

## 0.10.0 — 2026-09-19

### Added — shared scheduler permit pool

- A FIFO concurrency pool now provides one positive safe-integer permit budget
  for stage and per-item scheduler work, with exact release on success,
  synchronous failure, and asynchronous failure.
- A holder can yield its permit while awaiting upstream and reacquire at the
  queue tail. This supplies the deadlock-free `maxConcurrency: 1` primitive
  needed by the live-stream scheduler without bypassing older waiters.
- Pipeline cancellation rejects queued and future work with
  `CancellationError`, while active work retains cooperative cancellation and
  releases capacity when it unwinds.
- Read-only instrumentation reports active, queued, and peak-active work.
  FM-B042 and FM-B043 own DAG and live-stream integration.

### Tests

- Fourteen focused cases cover input bounds, peak concurrency, FIFO starts and
  reacquisition, sync/async failure cleanup, queued/future cancellation,
  cancellation during reacquisition, one-permit producer/consumer progress,
  failed waits, concurrent-yield rejection, and already-cancelled pools.

## 0.9.0 — 2026-09-19

### Added — bounded lazy stream fan-out

- One lazily opened upstream iterator can now feed a statically known set of
  single-use consumer branches without recomputing source values.
- Every attached branch has an ordered 64-value window. A full slow branch
  backpressures upstream pulls, while `return()` detaches that consumer and
  immediately releases its queued values and pressure.
- Completion and source failures reach every branch after its delivered
  prefix. Pipeline cancellation rejects pending reads, clears every window,
  and closes the upstream iterator exactly once.
- Read-only instrumentation exposes upstream pulls, active consumers, current
  retained values, and peak retained values for boundedness tests. FM-B040
  owns integration with the pipeline-wide concurrent scheduler.

### Tests

- Nineteen focused cases cover lazy source opening, order, one-pull multicast,
  slow-branch backpressure, detachment, source failure and hostile iterator
  results, cancellation before/during/after delivery, invalid bounds,
  single-use branches, concurrent-read rejection, zero consumers, and streams
  larger than the default 64-value window.

## 0.8.0 — 2026-09-19

### Added — bounded stream checkpoint foundation

- Stream values can be written incrementally into a content-addressed,
  left-complete ordered tree. The writer persists completed nodes immediately
  and retains only an `O(log n)` binary-counter frontier.
- A small manifest is published separately and last, so interrupted writes
  cannot expose a partial stream checkpoint. The stream revision commits to
  the ordered root and item count without re-encoding a materialized array.
- Loading performs a complete integrity, content-key, count, shape, and
  revision validation pass before exposing a second lazy replay traversal.
  Invalid state fails open by invalidating the manifest; cancellation
  propagates without destroying valid cache state.
- This release establishes FM-B038's storage boundary. The existing scheduler
  continues to materialize streams until FM-B039 and FM-B040 connect bounded
  multicast transport and pipeline-wide scheduling to it.

### Tests

- Thirty focused cases cover empty, duplicate, power-of-two boundary, and
  257-value streams; logarithmic retained state; content deduplication; lazy
  iteration; malformed manifests and nodes; non-canonical trees; missing and
  wrong-key entries; invalidation failure; and cancellation during validation
  and replay.
- The new checkpoint module exceeds 96% line coverage.

## 0.7.0 — 2026-09-19

### Added — side-effect replay

- Capability-bearing stream collectors and named-input joins that implement
  `Stage.replay` now persist whole-instance checkpoints and reapply their
  effects before reporting an unchanged instance as skipped.
- Missing, corrupt, revision-mismatched, or rejected replay state fails open
  to normal stage execution and refreshes the checkpoint after success.
- Revision-ledger and checkpoint namespaces include effective per-instance
  capability grants, so changing authority cannot reuse prior materialization.
- Per-item capability-bearing stages remain conservative because one stage
  output does not describe the scheduler's aggregate invocation effects.

### Tests

- Coverage proves successful replay, replay failure fallback, and a real
  filesystem emitter restoring a deleted tree across fresh orchestrators.

## 0.6.0 — 2026-09-01

### Added — exact affected scheduling

- The scheduler now compares each observed input with the prior successful
  revision ledger, propagates changes through the transitive downstream
  closure, and restores untouched capability-free instances as one validated
  materialized checkpoint.
- Restored instances report `outcome: "skipped"`, retain their input/output
  revisions and sink outputs, and count the checkpoint as a cache hit.
- Affected stream stages still use the per-item cache, so an edited source can
  reuse unchanged items while the complete downstream stage remains scheduled.
- Observed sources may be restored after their external-state hook proves the
  input unchanged. Capability-bearing and legacy sources continue to execute
  conservatively until side-effect replay is available.
- Missing, malformed, unavailable, or revision-mismatched checkpoints fail
  open to normal execution. Checkpoints are topology-scoped and only the prior
  successful ledger can authorize a whole-instance skip.

### Tests

- Fresh-orchestrator coverage proves all-unchanged skipping, exact isolation
  between independent branches, revision-mismatch recovery, and the
  capability boundary between conservative execution and pure downstream
  restoration.

## 0.5.0 — 2026-09-01

### Added — external revisions and persistent ledger

- Source `externalState` manifests are structurally validated, checked for
  unique sorted locators, and verified against their canonical digest before
  the source is allowed to run.
- Every stage summary now reports materialized input/output revisions, a source
  external-state revision when present, and whether its input changed from the
  prior successful run.
- A topology-keyed revision ledger persists through the injected cache backend
  and fails open on missing, malformed, or unavailable state. Fresh CLI
  processes can therefore compare the same project across runs.
- `buildId` now hashes observed source state (or the materialized legacy-source
  output) instead of static source instance names.

### Tests

- Fresh-orchestrator filesystem-cache coverage proves unchanged and edited
  source/downstream comparisons, stable/changed build IDs, cache reuse, and
  rejection of a dishonest external-state digest.

## 0.4.0 — 2026-09-01

### Added

- Added deterministic tagged encoding for materialized stage outputs, including
  binary `Uint8Array` values, canonical object ordering, and a versioned cache
  envelope.
- The scheduler now uses its injected `CacheBackend` for capability-free,
  non-source invocations. Cache hits skip `stage.run`, per-item transforms count
  hits and misses accurately, changed inputs invalidate naturally, and
  `RunOptions.useCache = false` bypasses reuse.
- Sources and capability-bearing stages deliberately rerun until FM-B032 adds
  explicit external-state revisions and side-effect replay contracts.

- Added the FM03 `watch` lifecycle: initial build, host-provided change stream,
  configurable debouncing, coalesced follow-up builds, manual rebuild, result
  streaming, watcher-error propagation, and cooperative stop/dispose behavior.
- Watch deliberately re-runs the complete pipeline; persistent cache hits and
  exact affected-stage scheduling remain owned by FM-B010.

- `buildDag` now honors explicit `PipelineConfig.wires`. Explicit edges may
  point forward in declaration order and override inferred producers.
- One producer may feed multiple consumers. A stable topological sort executes
  each materialized stream once and makes it available to every branch.
- Stages may declare required named `inputPorts` alongside their default input.
  The DAG type-checks each port, includes every side dependency in cycle/sink/
  source discovery, and supports explicit forward wires.
- The scheduler invokes a named fan-in stage exactly once with `default` plus a
  lexicographically stable named-port map. Materialized streams are wrapped in
  independently replayable `AsyncIterable` values for each port.
- Explicit edges are kind-checked after stream-promotion semantics are applied;
  incompatible edges, incoming wires to sources, and cycles fail before run.
- `@types/node` is now a direct development dependency, so `npm run build`
  covers Node APIs imported through source-linked dependencies.

### Tests

- Integration coverage pins explicit-over-inferred selection, forward wires,
  incompatible wires, cycle rejection, true sink discovery, and a stream
  fanning out to per-item and collector consumers.
- A rendered-page + asset join test pins forward named dependencies, stable
  port order, replayability, one invocation, summary counts, and deploy output.

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

- Linear pipelines (source → transform... → sink)
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
