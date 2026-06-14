/**
 * @coding-adventures/forme-orchestrator
 *
 * The runtime that takes a `PipelineConfig`, builds a typed DAG, and
 * executes it (FM03 §3-4, §9-10).
 *
 * v0 surface: `createOrchestrator()` → `buildPipeline(config)` →
 * `runOnce(pipeline, options?)` → `RunResult`.  Linear and
 * fan-out-1 pipelines work; full parallelism, watch mode, incremental
 * rebuild, reproducible-build mode, and OpenTelemetry traces are
 * deferred to follow-ups.
 *
 * See FM03 §3 for the lifecycle, §9 for error handling, §10 for
 * cancellation.  See `scheduler.ts` for the v0 simplifications listed
 * verbatim.
 */

export { createOrchestrator } from "./orchestrator.js";
export { buildPipeline } from "./build-pipeline.js";
export { runOnce } from "./run.js";
export { buildDag } from "./dag.js";
export { areKindsCompatible } from "./typecheck.js";
export { REPRO_BUILD_FROZEN_TIMESTAMP_MS } from "./scheduler.js";

export type {
  Orchestrator,
  OrchestratorOptions,
  Pipeline,
  RunOptions,
  RunOutcome,
  RunResult,
  RunError,
  StageRunSummary,
} from "./types.js";

export type { PipelineDag, ResolvedInstance } from "./dag.js";
