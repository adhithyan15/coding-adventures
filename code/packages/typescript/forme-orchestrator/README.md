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
await o.dispose();
```

| Function/Type             | Purpose                                                                |
| ------------------------- | ---------------------------------------------------------------------- |
| `createOrchestrator()`    | Build a runtime handle (cache + logger + pipeline lifecycle).          |
| `buildPipeline(config)`   | Validate config + construct typed DAG.                                 |
| `runOnce(pipeline, opts?)`| Execute the DAG; return a structured `RunResult`.                       |
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
- **No streaming pipelining.** A `Stream<X>` producer is fully drained into memory before downstream consumers see values. Lazy streaming lands in v1 alongside parallelism.
- **No incremental rebuild.** Every run executes every stage; the cache backend exists but isn't hit yet (FM03 §6).
- **No reproducible-build mode.** Stages get `systemClock`, not `frozenClock` (FM03 §8).
- **No watch mode.** `forme watch` lives in a future companion package (FM03 §7).
- **No OpenTelemetry traces.** Telemetry surface is no-op by default.

## What v0 *does* implement

- Topological execution honouring kind-compatibility wiring
- Per-stage `StageContext` construction with denied-by-default capability APIs
- `init` / `dispose` lifecycle hooks (init failure aborts before any `run`; dispose always runs)
- Fail-fast and best-effort error handling
- Cancellation propagation (composes a fresh token if caller doesn't supply one)
- Per-stage timing + error counts + outcome in `StageRunSummary`
- Named outputs from `OutputSpec` overriding sink instance ids
- `buildId` derived from `computeRevisionId` over pipeline source/sink ids

## Coverage

```bash
npm install
npx vitest run --coverage
```

Targets ≥85% line coverage. The scheduler has many branches (init failure, fail-fast, best-effort, cancellation, dispose-on-failure) — integration tests cover the canonical paths.
