/**
 * `validateConfig` — checks a `PipelineConfig` against every FM03 §2.4
 * rule and returns a `ResolvedPipelineConfig` (the input plus
 * orchestrator-friendly derived fields like resolved instance IDs).
 *
 * Rules enforced (per FM03 §2.4 + the pragmatic v0 cuts):
 *
 *   1. Every `StageInstanceSpec.stage` is a loaded `Stage` value
 *      (StageRef → STAGE_REF_UNRESOLVED in v0; FM02 will resolve).
 *   2. Every stage value carries the required identification fields.
 *   3. Every stage's `apiVersion` matches `KERNEL_API_VERSION`.
 *   4. Every instance has a unique resolved ID.  The validator
 *      auto-derives `stage.name` when no collision exists; collisions
 *      require explicit `id` on every colliding spec.
 *   5. Per-instance capability grants must be a subset of the stage's
 *      declared capabilities (FM01 §5.5: a stage can't be granted
 *      what it didn't ask for).
 *   6. Stages with non-null `configSchema` need a non-undefined config.
 *      (Full JSON-Schema validation is deferred to the orchestrator;
 *      we only enforce the presence rule here.)
 *   7. Wires/outputs reference real instance IDs.
 *   8. If more than one terminal stage exists (no consumer), each
 *      must have a corresponding `OutputSpec`.
 *
 * The validator collects ALL violations rather than throwing on the
 * first.  Users want to see every problem in one pass — chasing them
 * one at a time is the slowest possible feedback loop for config UX.
 */

import { KERNEL_API_VERSION } from "@coding-adventures/forme-types";
import { isStageRef } from "./types.js";
import { CONFIG_ERROR_CODES, ConfigError } from "./errors.js";
import type { ConfigErrorEntry } from "./errors.js";
import type {
  PipelineConfig,
  PipelineSettings,
  StageInstanceSpec,
} from "./types.js";

/** Resolved view of a config — same as input plus derived instance IDs. */
export interface ResolvedPipelineConfig {
  readonly config: PipelineConfig;
  /**
   * Per-spec resolved instance ID, in the same order as
   * `config.stages`.  `resolvedIds[i]` is the ID for `config.stages[i]`.
   */
  readonly resolvedIds: readonly string[];
}

/**
 * Validate a config and return a resolved view.  Throws `ConfigError`
 * (carrying the full list of violations) on any failure.
 */
export function validateConfig(config: PipelineConfig): ResolvedPipelineConfig {
  const errors: ConfigErrorEntry[] = [];

  // Top-level shape.  We record violations but only bail when the
  // remaining checks would crash (config null, or `stages` not an
  // array we can iterate).  Continuing past lesser top-level mistakes
  // lets us surface every downstream problem in one pass — users want
  // the full punch list, not just the first symptom.
  if (typeof config !== "object" || config === null) {
    errors.push({ path: "$", code: CONFIG_ERROR_CODES.MALFORMED, message: "config is not an object" });
    throw new ConfigError(errors);
  }
  recordTopShapeIssues(config, errors);
  if (!Array.isArray(config.stages)) {
    // Already recorded.  Per-instance loop would crash without an array.
    throw new ConfigError(errors);
  }

  if (typeof config.settings === "object" && config.settings !== null) {
    validateSettings(config.settings, errors);
  }

  const resolvedIds = resolveInstanceIds(config.stages, errors);

  for (let i = 0; i < config.stages.length; i++) {
    validateStageInstance(config.stages[i]!, i, errors);
  }

  // ID-dependent rules require the resolution pass to have run first.
  const idSet = new Set(resolvedIds);
  validateWires(config, idSet, errors);
  validateOutputs(config, idSet, errors);
  validateMultipleTerminals(config, resolvedIds, idSet, errors);

  if (errors.length > 0) throw new ConfigError(errors);
  return { config, resolvedIds };
}

// ─── Top-level shape ──────────────────────────────────────────────────────

/**
 * Record top-level shape problems.  Does not bail — the caller decides
 * which (if any) failures are fatal enough to skip later passes.
 */
function recordTopShapeIssues(c: PipelineConfig, errors: ConfigErrorEntry[]): void {
  if (typeof c.name !== "string" || c.name.length === 0) {
    errors.push({ path: "name", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a non-empty string" });
  }
  if (typeof c.settings !== "object" || c.settings === null) {
    errors.push({ path: "settings", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be an object" });
  }
  if (!Array.isArray(c.stages) || c.stages.length === 0) {
    errors.push({ path: "stages", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a non-empty array" });
  }
}

// ─── Settings ─────────────────────────────────────────────────────────────

const VALID_LOG_LEVELS = new Set(["trace", "debug", "info", "warn", "error"]);

function validateSettings(s: PipelineSettings, errors: ConfigErrorEntry[]): void {
  if (typeof s.storageRoot !== "string" || s.storageRoot.length === 0) {
    errors.push({ path: "settings.storageRoot", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a non-empty string" });
  }
  if (s.cacheDir !== null && typeof s.cacheDir !== "string") {
    errors.push({ path: "settings.cacheDir", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a string or null" });
  }
  if (typeof s.reproducibleBuild !== "boolean") {
    errors.push({ path: "settings.reproducibleBuild", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a boolean" });
  }
  if (s.maxConcurrency !== null
      && (typeof s.maxConcurrency !== "number" || !Number.isInteger(s.maxConcurrency) || s.maxConcurrency < 1)) {
    errors.push({ path: "settings.maxConcurrency", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a positive integer or null" });
  }
  if (typeof s.logLevel !== "string" || !VALID_LOG_LEVELS.has(s.logLevel)) {
    errors.push({ path: "settings.logLevel", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be one of trace|debug|info|warn|error" });
  }
  if (typeof s.bestEffort !== "boolean") {
    errors.push({ path: "settings.bestEffort", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a boolean" });
  }
  if (s.deadlineMs !== null
      && (typeof s.deadlineMs !== "number" || !Number.isFinite(s.deadlineMs) || s.deadlineMs <= 0)) {
    errors.push({ path: "settings.deadlineMs", code: CONFIG_ERROR_CODES.MALFORMED, message: "must be a positive number or null" });
  }
}

// ─── Per-stage validation ─────────────────────────────────────────────────

function validateStageInstance(
  spec: StageInstanceSpec,
  index: number,
  errors: ConfigErrorEntry[],
): void {
  const path = `stages[${index}]`;

  if (isStageRef(spec.stage)) {
    errors.push({
      path: `${path}.stage`,
      code: CONFIG_ERROR_CODES.STAGE_REF_UNRESOLVED,
      message:
        `StageRef ${JSON.stringify(spec.stage.packageName)} requires a plugin host (FM02). ` +
        `v0 supports direct-import flows only — import the stage value and pass it directly.`,
    });
    return; // remaining checks are stage-shape-dependent
  }

  const stage = spec.stage;
  if (typeof stage !== "object" || stage === null) {
    errors.push({ path: `${path}.stage`, code: CONFIG_ERROR_CODES.INVALID_STAGE_VALUE, message: "stage must be an object" });
    return;
  }
  if (typeof stage.name !== "string" || stage.name.length === 0) {
    errors.push({ path: `${path}.stage.name`, code: CONFIG_ERROR_CODES.INVALID_STAGE_VALUE, message: "stage.name must be a non-empty string" });
  }
  if (typeof stage.version !== "string") {
    errors.push({ path: `${path}.stage.version`, code: CONFIG_ERROR_CODES.INVALID_STAGE_VALUE, message: "stage.version must be a string" });
  }
  if (typeof stage.apiVersion !== "number") {
    errors.push({ path: `${path}.stage.apiVersion`, code: CONFIG_ERROR_CODES.INVALID_STAGE_VALUE, message: "stage.apiVersion must be a number" });
  } else if (stage.apiVersion !== KERNEL_API_VERSION) {
    errors.push({
      path: `${path}.stage.apiVersion`,
      code: CONFIG_ERROR_CODES.API_VERSION_MISMATCH,
      message:
        `stage targets apiVersion ${stage.apiVersion}; kernel is ${KERNEL_API_VERSION}. ` +
        `Upgrade the stage package or pin a kernel version that matches.`,
    });
  }
  if (!Array.isArray(stage.capabilities)) {
    errors.push({ path: `${path}.stage.capabilities`, code: CONFIG_ERROR_CODES.INVALID_STAGE_VALUE, message: "stage.capabilities must be an array" });
  } else if (Array.isArray(spec.capabilities)) {
    // Per-instance grants must be a subset of the stage's declarations.
    const declared = new Set(stage.capabilities as readonly string[]);
    for (const requested of spec.capabilities) {
      if (!declared.has(requested)) {
        errors.push({
          path: `${path}.capabilities`,
          code: CONFIG_ERROR_CODES.CAPABILITY_NOT_DECLARED,
          message:
            `Instance asks for capability ${JSON.stringify(requested)} but the stage's manifest does not declare it. ` +
            `Add it to the stage's capabilities array, or remove the per-instance grant.`,
        });
      }
    }
  }
  // Config presence rule (FM03 §2.4 #3 — full JSON-Schema validation
  // is the orchestrator's job; we only enforce the presence half here).
  if (stage.configSchema !== null && spec.config === undefined) {
    errors.push({
      path: `${path}.config`,
      code: CONFIG_ERROR_CODES.CONFIG_REQUIRED,
      message:
        `Stage ${JSON.stringify(stage.name)} declares a configSchema but no config was supplied.`,
    });
  }
}

// ─── Instance ID resolution ───────────────────────────────────────────────

/**
 * Resolve every instance's ID.  `id` field wins; otherwise we use
 * `stage.name`.  Collisions on the resolved ID are an error — the user
 * must add an explicit `id` to disambiguate.
 *
 * Returns the array of resolved IDs aligned with the input order, with
 * `""` placeholders for entries that couldn't be resolved (validator
 * still reports their problems individually).
 */
function resolveInstanceIds(
  specs: readonly StageInstanceSpec[],
  errors: ConfigErrorEntry[],
): string[] {
  const out: string[] = new Array(specs.length).fill("");
  const seen = new Map<string, number[]>();

  for (let i = 0; i < specs.length; i++) {
    const spec = specs[i]!;
    let id: string | null = null;
    if (typeof spec.id === "string" && spec.id.length > 0) {
      id = spec.id;
    } else if (!isStageRef(spec.stage) && typeof spec.stage?.name === "string") {
      id = spec.stage.name;
    }
    if (id === null) {
      // No explicit id and stage.name unavailable; leave placeholder.
      // Per-instance validation will surface the underlying problem.
      continue;
    }
    out[i] = id;
    const list = seen.get(id);
    if (list) list.push(i);
    else seen.set(id, [i]);
  }

  // Report duplicates with paths to *every* colliding occurrence.
  for (const [id, indices] of seen) {
    if (indices.length > 1) {
      const paths = indices.map(i => `stages[${i}]`).join(", ");
      errors.push({
        path: paths,
        code: CONFIG_ERROR_CODES.DUPLICATE_INSTANCE_ID,
        message:
          `Multiple stage instances resolve to id ${JSON.stringify(id)}. ` +
          `Add explicit \`id\` fields to disambiguate.`,
      });
    }
  }

  return out;
}

// ─── Wires / outputs / terminals ──────────────────────────────────────────

function validateWires(
  c: PipelineConfig,
  idSet: ReadonlySet<string>,
  errors: ConfigErrorEntry[],
): void {
  if (!c.wires) return;
  for (let i = 0; i < c.wires.length; i++) {
    const w = c.wires[i]!;
    if (!idSet.has(w.from.id)) {
      errors.push({
        path: `wires[${i}].from.id`,
        code: CONFIG_ERROR_CODES.UNKNOWN_INSTANCE_ID,
        message: `Edge references unknown instance ${JSON.stringify(w.from.id)}.`,
      });
    }
    if (!idSet.has(w.to.id)) {
      errors.push({
        path: `wires[${i}].to.id`,
        code: CONFIG_ERROR_CODES.UNKNOWN_INSTANCE_ID,
        message: `Edge references unknown instance ${JSON.stringify(w.to.id)}.`,
      });
    }
  }
}

function validateOutputs(
  c: PipelineConfig,
  idSet: ReadonlySet<string>,
  errors: ConfigErrorEntry[],
): void {
  if (!c.outputs) return;
  for (let i = 0; i < c.outputs.length; i++) {
    const o = c.outputs[i]!;
    if (!idSet.has(o.fromInstance)) {
      errors.push({
        path: `outputs[${i}].fromInstance`,
        code: CONFIG_ERROR_CODES.UNKNOWN_INSTANCE_ID,
        message: `Output references unknown instance ${JSON.stringify(o.fromInstance)}.`,
      });
    }
  }
}

/**
 * If the pipeline has more than one terminal stage (no consumer in the
 * declaration), each must be named in `outputs`.  We approximate
 * "terminal" here as "produces something but isn't followed by anything
 * that consumes it" — for v0 we use a coarse heuristic: every instance
 * whose produced kind name is in TERMINAL_KIND_NAMES counts as a
 * potential output sink.  If there are 2+ such instances, each must
 * appear in `outputs`.
 *
 * The orchestrator's full DAG construction (FM03 §3.3) does the
 * precise determination later; this is a config-time tripwire that
 * catches the common "I forgot to name my outputs" mistake without
 * needing the full DAG.
 */
const TERMINAL_KIND_NAMES = new Set([
  "DeployArtifact", "RequestHandler", "Feed", "SearchIndex",
]);

function validateMultipleTerminals(
  c: PipelineConfig,
  resolvedIds: readonly string[],
  idSet: ReadonlySet<string>,
  errors: ConfigErrorEntry[],
): void {
  void idSet; // (currently unused; kept for symmetry with sibling validators)
  const terminals: string[] = [];
  for (let i = 0; i < c.stages.length; i++) {
    const spec = c.stages[i]!;
    if (isStageRef(spec.stage)) continue;
    // Defensive — a malformed entry (`stage: null`, `stage: 42`) was
    // already flagged by validateStageInstance; we just skip it here so
    // a property-access TypeError doesn't escape the validator.
    const stage = spec.stage as { produces?: { name?: unknown } } | null | undefined;
    if (!stage || typeof stage !== "object") continue;
    const produced = stage.produces?.name;
    if (typeof produced === "string" && TERMINAL_KIND_NAMES.has(produced)) {
      terminals.push(resolvedIds[i] ?? "");
    }
  }
  if (terminals.length < 2) return;
  const named = new Set((c.outputs ?? []).map(o => o.fromInstance));
  const missing = terminals.filter(id => id !== "" && !named.has(id));
  if (missing.length > 0) {
    errors.push({
      path: "outputs",
      code: CONFIG_ERROR_CODES.MULTIPLE_OUTPUTS_UNNAMED,
      message:
        `Pipeline has multiple terminal stages (${terminals.map(t => JSON.stringify(t)).join(", ")}) ` +
        `but ${missing.length === terminals.length ? "no" : "some"} `+
        `OutputSpec entries name them.  Add OutputSpec entries to ` +
        `\`outputs\` for: ${missing.map(m => JSON.stringify(m)).join(", ")}.`,
    });
  }
}
