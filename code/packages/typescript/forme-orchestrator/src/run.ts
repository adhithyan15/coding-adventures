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
 *   - Computes a `buildId` from the input revisions of every source
 *     stage.  For v0 we hash the joined source instance ids — the
 *     full revision-tracking story lands when sources start emitting
 *     real revisions.
 *   - Times the wall-clock end-to-end.
 */

import { computeRevisionId } from "@coding-adventures/forme-identity";
import {
  consoleLogger,
  createCancellationTokenSource,
  silentLogger,
} from "@coding-adventures/forme-stage";
import type { Logger } from "@coding-adventures/forme-stage";
import { executeDag } from "./scheduler.js";
import type { Pipeline, RunOptions, RunResult } from "./types.js";

export interface RunOnceContext {
  readonly logger?: Logger;
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

  const result = await executeDag(pipeline.dag, {
    cancellation,
    bestEffort,
    logger,
  });

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

  const buildId = computeRevisionId({
    pipeline: pipeline.config.name,
    sources: pipeline.dag.sources,
    sinks: pipeline.dag.sinks,
  });

  return {
    outcome: result.outcome,
    stages: result.summaries,
    outputs,
    errors: result.errors,
    elapsedMs: Date.now() - start,
    buildId,
  };
}

// Internal — silentLogger is exported so the dispose() flow can
// silence late warnings without re-importing forme-stage.
export { silentLogger };
