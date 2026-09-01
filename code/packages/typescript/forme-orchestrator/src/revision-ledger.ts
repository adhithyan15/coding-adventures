/** Project-persistent per-instance revision ledger (FM-B035). */

import {
  cacheKey,
  makeEntry,
  type CacheBackend,
} from "@coding-adventures/forme-cache";
import {
  computeRevisionId,
  isRevisionIdShape,
} from "@coding-adventures/forme-identity";
import type { JsonValue, RevisionId } from "@coding-adventures/forme-types";
import type { Logger } from "@coding-adventures/forme-stage";
import type { Pipeline, StageRunSummary } from "./types.js";

export interface InstanceRevisionState {
  readonly inputRevision: RevisionId | null;
  readonly outputRevision: RevisionId | null;
  readonly externalStateRevision: RevisionId | null;
}

export interface PipelineRevisionLedger {
  readonly version: 1;
  readonly pipeline: string;
  readonly instances: Readonly<Record<string, InstanceRevisionState>>;
}

const LEDGER_STAGE_NAME = "@coding-adventures/forme-orchestrator/revision-ledger";
const LEDGER_STAGE_VERSION = "1.0.0";

export function revisionLedgerKey(pipeline: Pipeline): string {
  const topology = pipeline.dag.topoOrder.map(id => {
    const instance = pipeline.dag.instances.get(id)!;
    return {
      id,
      stageName: instance.stage.name,
      stageVersion: instance.stage.version,
      config: (instance.config ?? null) as JsonValue,
      producer: instance.producer,
      inputProducers: [...instance.inputProducers.entries()]
        .sort(([left], [right]) => left < right ? -1 : left > right ? 1 : 0)
        .map(([port, producer]) => ({ port, producer })),
    };
  });
  const fingerprint = computeRevisionId({
    pipeline: pipeline.config.name,
    topology,
    sources: pipeline.dag.sources,
    sinks: pipeline.dag.sinks,
  });
  return cacheKey({
    stageName: LEDGER_STAGE_NAME,
    stageVersion: LEDGER_STAGE_VERSION,
    stageConfig: { pipeline: pipeline.config.name },
    inputRevision: fingerprint,
    capabilities: [],
  });
}

export async function loadRevisionLedger(
  backend: CacheBackend,
  pipeline: Pipeline,
  logger: Logger,
): Promise<PipelineRevisionLedger | null> {
  try {
    const key = revisionLedgerKey(pipeline);
    const entry = await backend.get(key);
    if (entry === null) return null;
    const parsed = JSON.parse(new TextDecoder().decode(entry.payload)) as unknown;
    if (isPipelineRevisionLedger(parsed, pipeline.config.name)) return parsed;
    await backend.invalidate(key);
    return null;
  } catch (error) {
    logger.warn("revision ledger read skipped", { error: String(error) });
    return null;
  }
}

export async function persistRevisionLedger(
  backend: CacheBackend,
  pipeline: Pipeline,
  summaries: readonly StageRunSummary[],
  logger: Logger,
): Promise<void> {
  const instances: Record<string, InstanceRevisionState> = Object.create(null);
  for (const summary of summaries) {
    instances[summary.instanceId] = {
      inputRevision: summary.inputRevision,
      outputRevision: summary.outputRevision,
      externalStateRevision: summary.externalStateRevision,
    };
  }
  const ledger: PipelineRevisionLedger = {
    version: 1,
    pipeline: pipeline.config.name,
    instances,
  };
  try {
    const payload = new TextEncoder().encode(JSON.stringify(ledger));
    await backend.put(revisionLedgerKey(pipeline), makeEntry(payload));
  } catch (error) {
    logger.warn("revision ledger write skipped", { error: String(error) });
  }
}

export function compareWithRevisionLedger(
  summaries: readonly StageRunSummary[],
  previous: PipelineRevisionLedger | null,
): readonly StageRunSummary[] {
  return summaries.map(summary => {
    const prior = previous?.instances[summary.instanceId];
    return {
      ...summary,
      inputChanged: prior === undefined
        ? null
        : prior.inputRevision !== summary.inputRevision,
    };
  });
}

function isPipelineRevisionLedger(
  value: unknown,
  pipeline: string,
): value is PipelineRevisionLedger {
  if (typeof value !== "object" || value === null) return false;
  const candidate = value as Partial<PipelineRevisionLedger>;
  if (candidate.version !== 1 || candidate.pipeline !== pipeline) return false;
  if (typeof candidate.instances !== "object" || candidate.instances === null) return false;
  for (const state of Object.values(candidate.instances)) {
    if (typeof state !== "object" || state === null) return false;
    const revision = state as Partial<InstanceRevisionState>;
    if (!isNullableRevision(revision.inputRevision)) return false;
    if (!isNullableRevision(revision.outputRevision)) return false;
    if (!isNullableRevision(revision.externalStateRevision)) return false;
  }
  return true;
}

function isNullableRevision(value: unknown): value is RevisionId | null {
  return value === null || (typeof value === "string" && isRevisionIdShape(value));
}
