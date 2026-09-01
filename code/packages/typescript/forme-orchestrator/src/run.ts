/**
 * `runOnce` — execute a built `Pipeline` and return a structured `RunResult`.
 *
 * Wraps the scheduler with the public-API niceties:
 *
 *   - Composes a cancellation token (caller's override or fresh).
 *   - Resolves `bestEffort` from RunOptions / config.
 *   - Maps scheduler outputs (keyed by sink instance id) to the public
 *     `outputs` map (keyed by `OutputSpec.name` when one exists, else
 *     by instance id).
 *   - Computes a `buildId` from every source's validated external-state
 *     revision, falling back to a revision of a legacy source's materialized
 *     output.
 *   - Loads and persists the topology-keyed per-instance revision ledger.
 *   - Times the wall-clock end-to-end.
 */

import { computeRevisionId } from "@coding-adventures/forme-identity";
import { memoryCache, type CacheBackend } from "@coding-adventures/forme-cache";
import {
  consoleLogger,
  createCancellationTokenSource,
  silentLogger,
} from "@coding-adventures/forme-stage";
import type { Logger } from "@coding-adventures/forme-stage";
import { executeDag } from "./scheduler.js";
import type {
  Pipeline,
  RunOptions,
  RunResult,
  StageRunSummary,
} from "./types.js";
import {
  compareWithRevisionLedger,
  loadRevisionLedger,
  persistRevisionLedger,
} from "./revision-ledger.js";

export interface RunOnceContext {
  readonly logger?: Logger;
  readonly cache?: CacheBackend;
}

export async function runOnce(
  pipeline: Pipeline,
  options: RunOptions = {},
  ctx: RunOnceContext = {},
): Promise<RunResult> {
  const start = Date.now();

  const cancellation = options.cancellation
    ?? createCancellationTokenSource().token;
  const bestEffort = options.bestEffort
    ?? pipeline.config.settings.bestEffort;

  // Default logger respects the pipeline's logLevel; tests/CLI can
  // override via `ctx.logger`.
  const logger: Logger = ctx.logger
    ?? consoleLogger({ level: pipeline.config.settings.logLevel })
      .child({ pipeline: pipeline.config.name });

  const ownedCache = ctx.cache === undefined ? memoryCache() : null;
  const cache = ctx.cache ?? ownedCache!;
  let result;
  let stages: readonly StageRunSummary[];
  try {
    const previousLedger = await loadRevisionLedger(cache, pipeline, logger);
    result = await executeDag(pipeline.dag, {
      cancellation,
      bestEffort,
      logger,
      cache,
      useCache: options.useCache ?? true,
      // Honour the pipeline's reproducible-build setting (FM03 §8).
      reproducibleBuild: pipeline.config.settings.reproducibleBuild,
    });
    stages = compareWithRevisionLedger(result.summaries, previousLedger);
    if (result.outcome === "success") {
      await persistRevisionLedger(cache, pipeline, stages, logger);
    }
  } finally {
    if (ownedCache !== null) await ownedCache.dispose();
  }

  // Map sink-keyed outputs to OutputSpec names when present.
  const outputs: Record<string, unknown> = {};
  const namedOutputs = new Map<string, string>();
  for (const o of pipeline.config.outputs ?? []) {
    namedOutputs.set(o.fromInstance, o.name);
  }
  for (const [instanceId, value] of result.outputs) {
    const key = namedOutputs.get(instanceId) ?? instanceId;
    outputs[key] = value;
  }

  const sourceRevisions = pipeline.dag.sources.map(instanceId => {
    const summary = stages.find(stage => stage.instanceId === instanceId)!;
    return {
      instanceId,
      revision: summary.externalStateRevision ?? summary.outputRevision,
    };
  });
  const buildId = computeRevisionId({
    pipeline: pipeline.config.name,
    sources: sourceRevisions,
    sinks: pipeline.dag.sinks,
  });

  return {
    outcome: result.outcome,
    stages,
    outputs,
    errors: result.errors,
    elapsedMs: Date.now() - start,
    buildId,
  };
}

// Internal — silentLogger is exported so the dispose() flow can
// silence late warnings without re-importing forme-stage.
export { silentLogger };
