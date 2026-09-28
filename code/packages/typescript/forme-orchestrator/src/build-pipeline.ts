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

import { isStageRef, validateConfig } from "@coding-adventures/forme-pipeline-config";
import { ConfigError } from "@coding-adventures/forme-pipeline-config";
import type { PipelineConfig } from "@coding-adventures/forme-pipeline-config";
import { buildDag } from "./dag.js";
import type { Pipeline, PluginStageLoader } from "./types.js";

export async function buildPipeline(
  config: PipelineConfig,
  pluginHost?: PluginStageLoader,
): Promise<Pipeline> {
  assertResolvableShape(config);
  const withResolvedStages: PipelineConfig = pluginHost
    ? {
        ...config,
        stages: await Promise.all(config.stages.map(async (spec) => {
          if (!isStageRef(spec.stage)) return spec;
          return {
            ...spec,
            stage: await pluginHost.loadStage(spec.stage, spec.id, spec.capabilities),
          };
        })),
      }
    : config;
  const resolved = validateConfig(withResolvedStages);
  const dag = buildDag(resolved);
  return { config: resolved.config, dag };
}

function assertResolvableShape(config: PipelineConfig): void {
  if (typeof config !== "object" || config === null || !Array.isArray(config.stages)) {
    validateConfig(config);
    return;
  }
  const errors: Array<{ path: string; code: string; message: string }> = [];
  for (let index = 0; index < config.stages.length; index += 1) {
    const value = config.stages[index] as unknown;
    const path = `stages[${index}]`;
    if (typeof value !== "object" || value === null) {
      errors.push({ path, code: "MALFORMED", message: "must be an object" });
      continue;
    }
    const spec = value as Record<string, unknown>;
    if (spec.id !== undefined && (typeof spec.id !== "string" || spec.id.length === 0)) {
      errors.push({ path: `${path}.id`, code: "MALFORMED", message: "must be a non-empty string" });
    }
    if (spec.capabilities !== undefined
        && (!Array.isArray(spec.capabilities)
          || spec.capabilities.some(capability => typeof capability !== "string"))) {
      errors.push({
        path: `${path}.capabilities`,
        code: "MALFORMED",
        message: "must be an array of capability strings",
      });
    }
    if (typeof spec.stage !== "object" || spec.stage === null) continue;
    const stage = spec.stage as Record<string, unknown>;
    if (stage.kind === "stage-ref"
        && (typeof stage.packageName !== "string" || stage.packageName.length === 0
          || (stage.export !== undefined && typeof stage.export !== "string"))) {
      errors.push({
        path: `${path}.stage`,
        code: "MALFORMED",
        message: "StageRef requires a non-empty packageName and optional string export",
      });
    }
  }
  if (errors.length > 0) throw new ConfigError(errors);
}
