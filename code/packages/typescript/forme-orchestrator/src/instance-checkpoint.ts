/** Validated materialized instance-output checkpoints for exact scheduling. */

import {
  cacheKey,
  makeEntry,
  type CacheBackend,
} from "@coding-adventures/forme-cache";
import { computeBinaryRevisionId } from "@coding-adventures/forme-identity";
import type { Logger } from "@coding-adventures/forme-stage";
import type { RevisionId } from "@coding-adventures/forme-types";
import type { ResolvedInstance } from "./dag.js";
import {
  decodeCachedStageOutput,
  encodeCachedStageOutput,
  encodeCacheValue,
  type CachedStageOutput,
} from "./cache-codec.js";

const CHECKPOINT_STAGE_PREFIX = "@coding-adventures/forme-orchestrator/instance-output:";

export function canCheckpointInstance(instance: ResolvedInstance): boolean {
  const hasInput = instance.producer !== null || instance.inputProducers.size !== 0;
  if (!hasInput) {
    // A source's external-state hook is its explicit observation boundary.
    // It may need read/write capabilities to take the snapshot (for example,
    // source-fs persists first-seen identities), but an unchanged validated
    // manifest makes replaying its materialized value safe.
    return typeof instance.stage.externalState === "function";
  }
  return instance.capabilities.length === 0;
}

export async function loadInstanceCheckpoint(
  backend: CacheBackend,
  namespace: string,
  instance: ResolvedInstance,
  inputRevision: RevisionId,
  expectedOutputRevision: RevisionId,
  logger: Logger,
): Promise<CachedStageOutput | null> {
  const key = instanceCheckpointKey(namespace, instance, inputRevision);
  let entry;
  try {
    entry = await backend.get(key);
  } catch (error) {
    logger.warn(`instance checkpoint read skipped for ${instance.stage.name} (${instance.id})`, {
      error: String(error),
    });
    return null;
  }
  if (entry === null) return null;

  try {
    const checkpoint = decodeCachedStageOutput(entry.payload);
    const actualRevision = computeBinaryRevisionId(encodeCacheValue(checkpoint.value));
    if (actualRevision !== expectedOutputRevision) {
      throw new Error(
        `output revision mismatch: expected ${expectedOutputRevision}, received ${actualRevision}`,
      );
    }
    return checkpoint;
  } catch (decodeError) {
    try {
      await backend.invalidate(key);
    } catch (error) {
      logger.warn(
        `malformed instance checkpoint could not be invalidated for ${instance.stage.name} (${instance.id})`,
        { decodeError: String(decodeError), error: String(error) },
      );
    }
    return null;
  }
}

export async function persistInstanceCheckpoint(
  backend: CacheBackend,
  namespace: string,
  instance: ResolvedInstance,
  inputRevision: RevisionId,
  output: CachedStageOutput,
  logger: Logger,
): Promise<void> {
  try {
    await backend.put(
      instanceCheckpointKey(namespace, instance, inputRevision),
      makeEntry(encodeCachedStageOutput(output)),
    );
  } catch (error) {
    logger.warn(`instance checkpoint write skipped for ${instance.stage.name} (${instance.id})`, {
      error: String(error),
    });
  }
}

export function instanceCheckpointKey(
  namespace: string,
  instance: ResolvedInstance,
  inputRevision: RevisionId,
): string {
  return cacheKey({
    stageName: `${CHECKPOINT_STAGE_PREFIX}${instance.stage.name}`,
    stageVersion: instance.stage.version,
    stageConfig: { namespace, instanceId: instance.id },
    inputRevision,
    capabilities: [],
  });
}
