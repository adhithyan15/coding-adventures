/**
 * `buildPipeline` — turn a `PipelineConfig` into an executable
 * `Pipeline` (FM03 §3.2 "Resolve" + "Typecheck").
 *
 * Two phases:
 *
 *   1. **Validate** — delegated to `forme-pipeline-config`'s
 *      `validateConfig`.  Throws `ConfigError` with a structured
 *      list of every violation.
 *
 *   2. **Build DAG** — honor explicit wires, infer unwired producers,
 *      kind-check edges, and compute a stable topological order.
 *      Throws if any consumer cannot be wired or the graph cycles.
 *
 * The result is a `Pipeline` carrying the original config and the
 * fully-built DAG, ready for `runOnce`.
 */

import { validateConfig } from "@coding-adventures/forme-pipeline-config";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { buildDag } from "./dag.js";
import type { Pipeline } from "./types.js";

export async function buildPipeline(config: PipelineConfig): Promise<Pipeline> {
  const resolved = validateConfig(config);
  const dag = buildDag(resolved);
  return { config: resolved.config, dag };
}
