/**
 * @coding-adventures/forme-orchestrator
 *
 * The runtime that takes a `PipelineConfig`, builds a typed DAG, and
 * executes it (FM03 §3-4, §9-10).
 *
 * v0 surface: `createOrchestrator()` → `buildPipeline(config)` →
 * `runOnce(pipeline, options?)` → `RunResult`, or `watch(pipeline)` for a
 * long-lived conservative rebuild loop. Explicit wires,
 * deterministic fan-out, stable topological execution, and partial
 * reproducible-build mode and watch lifecycle work; parallelism,
 * exact affected-stage scheduling, bounded streaming, and OpenTelemetry traces are deferred.
 *
 * See FM03 §3 for the lifecycle, §9 for error handling, §10 for
 * cancellation.  See `scheduler.ts` for the v0 simplifications listed
 * verbatim.
 */

export { createOrchestrator } from "./orchestrator.js";
export { buildPipeline } from "./build-pipeline.js";
export { runOnce } from "./run.js";
export { createWatchSession } from "./watch.js";
export { buildDag } from "./dag.js";
export { areKindsCompatible } from "./typecheck.js";
export { REPRO_BUILD_FROZEN_TIMESTAMP_MS } from "./scheduler.js";
export { revisionLedgerKey } from "./revision-ledger.js";

export type {
  Orchestrator,
  OrchestratorOptions,
  Pipeline,
  RunOptions,
  RunOutcome,
  RunResult,
  RunError,
  StageRunSummary,
  WatchOptions,
  WatchSession,
} from "./types.js";

export type {
  InstanceRevisionState,
  PipelineRevisionLedger,
} from "./revision-ledger.js";

export type { PipelineDag, ResolvedInstance } from "./dag.js";
