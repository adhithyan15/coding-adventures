# @coding-adventures/forme-orchestrator

The runtime that takes a `PipelineConfig`, builds a typed DAG, and executes it (FM03 §3-4, §9-10).

Last big package of the FM03 orchestrator stack.

## v0 surface

```typescript
import { createOrchestrator } from "@coding-adventures/forme-orchestrator";

const o = createOrchestrator();
const pipeline = await o.buildPipeline(config);
const result = await o.runOnce(pipeline);
console.log(result.outcome, result.outputs);

const session = o.watch(pipeline, { changes, debounceMs: 200 });
for await (const next of session.results()) console.log(next.outcome);
await session.stop();
await o.dispose();
```

| Function/Type             | Purpose                                                                |
| ------------------------- | ---------------------------------------------------------------------- |
| `createOrchestrator()`    | Build a runtime handle (cache + logger + pipeline lifecycle).          |
| `buildPipeline(config)`   | Validate config + construct typed DAG.                                 |
| `runOnce(pipeline, opts?)`| Execute the DAG; return a structured `RunResult`.                       |
| `watch(pipeline, opts)`   | Run initially and coalesce host change events into rebuilds.           |
| `WatchSession`            | Result stream, manual rebuild, and cooperative stop lifecycle.         |
| `Orchestrator`            | The runtime handle interface.                                          |
| `Pipeline`                | The built pipeline (config + DAG).                                     |
| `RunResult`               | Outcome, per-stage summaries, outputs, errors, timing, buildId.         |
| `RunError`                | Per-stage error surface.                                               |
| `StageRunSummary`         | Per-stage execution summary.                                            |
| `buildDag`                | Direct DAG construction (used by `buildPipeline`; exported for tests).  |
| `areKindsCompatible`      | Type-compatibility predicate (FM01 §2.6).                               |

## v0 simplifications

These are deferred to follow-up packages:

- **No parallelism.** Stages execute sequentially in topological order. `settings.maxConcurrency` is honoured at `1`.
- **No streaming pipelining yet.** A `Stream<X>` producer is still fully drained
  before downstream consumers see values. The bounded, content-addressed
  checkpoint tree is implemented, while FM-B039/FM-B040 own live multicast
  transport and scheduler integration.
- **Replay is explicit.** Exact affected scheduling restores untouched pure
  stages directly and invokes `Stage.replay` before skipping an effectful
  collector. Capability-bearing stages without that hook still execute
  conservatively (FM03 §6).
- **Partial reproducible-build mode.** Stages receive a frozen wall clock, but input-mtime derivation, deterministic randomness, and telemetry policy remain (FM03 §8).
- **No OpenTelemetry traces.** Telemetry surface is no-op by default.

## What v0 *does* implement

- Explicit `wires` with deterministic fan-out, typed named fan-in, and stable
  topological execution; unwired default inputs still infer the nearest
  compatible producer
- One materialization per producer and a fresh replayable `AsyncIterable` for
  every stream input port; a multi-input join is invoked exactly once
- Per-stage `StageContext` construction with denied-by-default capability APIs
- `init` / `dispose` lifecycle hooks (init failure aborts before any `run`; dispose always runs)
- Fail-fast and best-effort error handling
- Cancellation propagation (composes a fresh token if caller doesn't supply one)
- Per-stage timing + error counts + outcome in `StageRunSummary`
- Per-instance input/output revision summaries, validated source-state
  manifests, cross-process `inputChanged` comparisons, and a fail-open
  topology-keyed persistent revision ledger
- Exact changed-and-downstream scheduling with validated whole-instance
  checkpoints, explicit `skipped` summaries, restored sink outputs, and
  fail-open replay for opt-in capability-bearing collectors
- Bounded stream-checkpoint storage with an `O(log n)` writer frontier,
  manifest-last publication, full pre-replay validation, lazy ordered reads,
  content deduplication, and cancellation-safe fail-open behavior
- Deterministic tagged cache encoding for plain Forme values and bytes;
  per-invocation cache hits/misses for safe pure stages, with `useCache: false`
  bypass and fail-open behavior for unsupported/corrupt entries
- Named outputs from `OutputSpec` overriding sink instance ids
- Reproducible-build frozen clocks shared across stage lifecycles
- Host-driven watch sessions with an initial run, debounced change coalescing,
  one queued follow-up during an active build, manual rebuild, watcher-error
  propagation, and cancellation-safe teardown
- `buildId` derived from the observed external state (or materialized output)
  of every source plus the pipeline sink set

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets ≥85% line coverage. The scheduler has many branches (init failure, fail-fast, best-effort, cancellation, dispose-on-failure) — integration tests cover the canonical paths.
