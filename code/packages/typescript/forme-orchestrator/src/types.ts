/**
 * Public API types for the orchestrator (FM03 §3.1).
 *
 * Five surfaces:
 *
 *   - `Orchestrator` — the runtime handle returned by
 *     `createOrchestrator`.  Three methods: buildPipeline, runOnce,
 *     dispose.  v0 omits `watch` (FM03 §7) — that's deferred to a
 *     follow-up.
 *
 *   - `Pipeline` — the resolved + typechecked + DAG-constructed value
 *     handed to runOnce.  Carries the original config so the caller
 *     can introspect, plus the constructed DAG.
 *
 *   - `RunOptions` — per-call knobs (cancellation, best-effort
 *     override, useCache toggle).
 *
 *   - `RunResult` — the structured return.  Outcome + per-stage
 *     summaries + collected outputs + errors + timing + buildId.
 *
 *   - `StageRunSummary` — one entry per stage instance describing
 *     what happened during this run.
 */

import type { JsonValue, RevisionId } from "@coding-adventures/forme-types";
import type { CancellationToken, Logger } from "@coding-adventures/forme-stage";
import type { CacheBackend } from "@coding-adventures/forme-cache";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import type { PipelineDag } from "./dag.js";

/** Outcome of a single end-to-end run. */
export type RunOutcome = "success" | "partial" | "failed" | "cancelled";

/** Per-stage execution summary returned in `RunResult.stages`. */
export interface StageRunSummary {
  readonly instanceId: string;
  readonly stageName: string;
  readonly itemsConsumed: number;
  readonly itemsProduced: number;
  readonly elapsedMs: number;
  readonly cacheHits: number;
  readonly cacheMisses: number;
  readonly outcome: "success" | "skipped" | "failed";
  readonly errorCount: number;
}

/** Error surface in `RunResult.errors`. */
export interface RunError {
  readonly stageName: string;
  readonly instanceId: string;
  readonly code: string;
  readonly message: string;
  readonly recoverable: boolean;
  readonly fields: Readonly<Record<string, JsonValue>>;
}

/** Aggregated result of a single end-to-end run. */
export interface RunResult {
  readonly outcome: RunOutcome;
  readonly stages: readonly StageRunSummary[];
  /** Final outputs keyed by `OutputSpec.name` (or stage instance id when no OutputSpec). */
  readonly outputs: Readonly<Record<string, unknown>>;
  readonly errors: readonly RunError[];
  readonly elapsedMs: number;
  /** Hash identifying this build's inputs.  Stable across re-runs of identical content. */
  readonly buildId: RevisionId;
}

/** Per-call options for `runOnce`. */
export interface RunOptions {
  /** Override the pipeline-wide cancellation token. */
  readonly cancellation?: CancellationToken;
  /** Override `bestEffort` from the config. */
  readonly bestEffort?: boolean;
  /** Reuse cached results when available.  Default: true. */
  readonly useCache?: boolean;
}

/** Pipeline value — the resolved+validated config plus its constructed DAG. */
export interface Pipeline {
  readonly config: PipelineConfig;
  readonly dag: PipelineDag;
}

/** Construction-time options for the orchestrator. */
export interface OrchestratorOptions {
  /** Cache backend to use; default: a fresh in-memory cache. */
  readonly cache?: CacheBackend;
  /** Logger to use; default: silent (caller wires console logging through the CLI). */
  readonly logger?: Logger;
}

/** The orchestrator runtime handle. */
export interface Orchestrator {
  /** Validate a config and build its typed DAG.  Throws on invalid configs. */
  buildPipeline(config: PipelineConfig): Promise<Pipeline>;
  /** Run a pipeline once. */
  runOnce(pipeline: Pipeline, options?: RunOptions): Promise<RunResult>;
  /** Tear down resources held by the orchestrator. */
  dispose(): Promise<void>;
}
