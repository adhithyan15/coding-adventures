/**
 * Pipeline DAG construction (FM03 §3.3).
 *
 * Given a validated config (`ResolvedPipelineConfig`), build a typed
 * directed acyclic graph the executor can walk.
 *
 * === Algorithm ===
 *
 * For each consumer instance, find the most recently declared
 * producer of a compatible kind.  "Most recent" means the latest
 * instance in the `stages` array before the consumer.  Falls back to
 * any prior instance (without that "before" constraint) if no later
 * one exists.
 *
 * Sources (`consumes: Void`) have no incoming edge.  Sinks (no
 * downstream consumer) are marked as terminal.
 *
 * v0 simplification: explicit `wires` overrides are not yet
 * implemented; pipelines must be inferable from kind compatibility.
 * The validator already accepts only inferable shapes for v0.
 */

import { isStageRef } from "@coding-adventures/forme-pipeline-config";
import type { ResolvedPipelineConfig } from "@coding-adventures/forme-pipeline-config";
import type { Capability } from "@coding-adventures/forme-capability";
import type { KindDescriptor } from "@coding-adventures/forme-types";
import type { Stage } from "@coding-adventures/forme-stage";
import { areKindsCompatible } from "./typecheck.js";

/** A resolved stage instance ready to run. */
export interface ResolvedInstance {
  readonly id: string;
  readonly stage: Stage<KindDescriptor, KindDescriptor>;
  readonly config: unknown;
  readonly capabilities: readonly Capability[];
  /** Producer instance id whose output feeds this instance, or null for sources. */
  readonly producer: string | null;
}

export interface PipelineDag {
  /** All instances, keyed by id. */
  readonly instances: ReadonlyMap<string, ResolvedInstance>;
  /** Topological order (one valid linearisation). */
  readonly topoOrder: readonly string[];
  /** Stages with no consumer — produce final outputs. */
  readonly sinks: readonly string[];
  /** Stages with no producer (consumes: Void). */
  readonly sources: readonly string[];
}

/**
 * Build a `PipelineDag` from a validated config.  Throws `Error` on
 * type-incompatibility or unresolvable producer.
 */
export function buildDag(resolved: ResolvedPipelineConfig): PipelineDag {
  const instances = new Map<string, ResolvedInstance>();
  const sources: string[] = [];
  const sinks: string[] = [];

  // First pass: collect ResolvedInstance with producer = null.
  for (let i = 0; i < resolved.config.stages.length; i++) {
    const spec = resolved.config.stages[i]!;
    const id = resolved.resolvedIds[i]!;
    if (isStageRef(spec.stage)) {
      // Validator rejects this; defensive guard.
      throw new Error(`buildDag: instance ${id} is an unresolved StageRef`);
    }
    instances.set(id, {
      id,
      stage: spec.stage,
      config: spec.config,
      capabilities: spec.capabilities ?? spec.stage.capabilities,
      producer: null,
    });
  }

  // Second pass: resolve producers by walking earlier instances and
  // finding the most-recent compatible kind.  Track which producers
  // are consumed so we can identify sinks.
  const consumedAsProducer = new Set<string>();
  const ids = resolved.resolvedIds;
  const updated: ResolvedInstance[] = [];
  for (let i = 0; i < ids.length; i++) {
    const id = ids[i]!;
    const inst = instances.get(id)!;
    const consumes = inst.stage.consumes;

    // Sources: consumes: Void (or void).
    if (consumes.name === "Void") {
      sources.push(id);
      updated.push(inst);
      continue;
    }

    // Walk earlier instances in declaration order from most-recent backwards.
    let producerId: string | null = null;
    for (let j = i - 1; j >= 0; j--) {
      const prior = instances.get(ids[j]!)!;
      if (areKindsCompatible(prior.stage.produces, consumes)) {
        producerId = prior.id;
        break;
      }
    }
    // FM03 §3.3 step 2: fallback to any prior instance if none follows
    // — for v0 we only walk earlier instances anyway, so the same loop
    // covers it.

    if (producerId === null) {
      throw new Error(
        `buildDag: instance ${JSON.stringify(id)} consumes ` +
        `${describeKind(consumes)} but no earlier instance produces a compatible kind`,
      );
    }
    consumedAsProducer.add(producerId);
    updated.push({ ...inst, producer: producerId });
  }

  // Replace map with updated instances (now carrying producer info).
  for (const inst of updated) instances.set(inst.id, inst);

  // Sinks: instances NOT in consumedAsProducer.
  for (const id of ids) {
    if (!consumedAsProducer.has(id)) sinks.push(id);
  }

  return {
    instances,
    topoOrder: ids,
    sinks,
    sources,
  };
}

function describeKind(k: KindDescriptor): string {
  if (k.name === "Stream" && k.inner) {
    return `Stream<${k.inner.name}>`;
  }
  return k.name;
}
