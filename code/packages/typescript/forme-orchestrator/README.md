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

## v0 boundaries

These limits remain after the concurrent scheduler milestone:

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
- Live stream publication before producer completion, with one bounded branch
  per statically known edge and one source traversal across fan-out
- Replayable `AsyncIterable` inputs at named fan-in boundaries; a multi-input
  join is invoked exactly once
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
- Bounded lazy stream multicast with one upstream pull per value, independent
  ordered 64-value consumer windows, slow-branch backpressure, safe consumer
  detachment, shared terminal errors, cancellation cleanup, and retained-value
  instrumentation
- Shared FIFO concurrency control with one pipeline-wide permit budget,
  cancellation-safe queued work, exact failure cleanup, wait-time permit
  suspension, fair reacquisition, and active/queued/peak instrumentation
- Stable DAG-ready scheduling and source-ordered per-item parallelism using
  that shared budget; `null` concurrency resolves to host hardware capacity,
  while fatal failure and cancellation reject queued work before it starts
- Permit-aware stream pulls that make progress with `maxConcurrency: 1`, lazy
  validated stream restore, manifest-last checkpoint publication, and exact
  incremental fallback when a producer revision is not known at readiness
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
