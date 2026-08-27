/**
 * Pipeline DAG construction (FM03 §3.3).
 *
 * Given a validated config (`ResolvedPipelineConfig`), build a typed
 * directed acyclic graph the executor can walk.
 *
 * === Algorithm ===
 *
 * Explicit `PipelineConfig.wires` are authoritative.  An unwired
 * consumer falls back to the most recently declared compatible
 * producer, preserving the concise linear-config experience.
 *
 * Sources (`consumes: Void`) have no incoming edge.  Sinks (no
 * downstream consumer) are marked as terminal.
 *
 * A stage has one input, but a producer may feed any number of
 * consumers.  The scheduler materializes each producer once, so this
 * graph-level fan-out is deterministic and does not consume a stream
 * once per branch.  Explicit wires may point forward in declaration
 * order; a stable topological sort determines execution order.
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
  const ids = resolved.resolvedIds;

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

  // Index explicit wires by consumer. validateConfig has already
  // rejected unknown IDs and multiple incoming wires.
  const explicitProducer = new Map<string, string>();
  for (const wire of resolved.config.wires ?? []) {
    explicitProducer.set(wire.to.id, wire.from.id);
  }

  const declarationIndex = new Map(ids.map((id, index) => [id, index]));
  const producerById = new Map<string, string | null>();
  const effectiveProduces = new Map<string, KindDescriptor>();
  const resolving = new Set<string>();

  /**
   * Resolve one instance's producer. Recursive resolution lets an
   * explicit forward wire participate in effective-kind promotion,
   * while the resolving set reports wire cycles before execution.
   */
  function resolveProducer(id: string): string | null {
    if (producerById.has(id)) return producerById.get(id)!;
    if (resolving.has(id)) {
      throw new Error(`buildDag: cycle detected while resolving instance ${JSON.stringify(id)}`);
    }
    resolving.add(id);

    const inst = instances.get(id)!;
    const consumes = inst.stage.consumes;
    if (consumes.name === "Void") {
      const wiredProducer = explicitProducer.get(id);
      if (wiredProducer !== undefined) {
        throw new Error(
          `buildDag: source instance ${JSON.stringify(id)} consumes Void and cannot have ` +
          `an incoming wire from ${JSON.stringify(wiredProducer)}`,
        );
      }
      producerById.set(id, null);
      resolving.delete(id);
      return null;
    }

    let producerId: string | null = null;
    const wiredProducer = explicitProducer.get(id);
    if (wiredProducer !== undefined) {
      producerId = wiredProducer;
    } else {
      // Inference intentionally considers earlier declarations only;
      // forward edges must be explicit so config remains reviewable.
      const index = declarationIndex.get(id)!;
      for (let priorIndex = index - 1; priorIndex >= 0; priorIndex--) {
        const priorId = ids[priorIndex]!;
        if (areKindsCompatible(effectiveKind(priorId), consumes)) {
          producerId = priorId;
          break;
        }
      }
    }

    if (producerId === null) {
      throw new Error(
        `buildDag: instance ${JSON.stringify(id)} consumes ` +
        `${describeKind(consumes)} but no earlier instance produces a compatible kind`,
      );
    }

    const producerKind = effectiveKind(producerId);
    if (!areKindsCompatible(producerKind, consumes)) {
      const edgeDescription = wiredProducer === undefined ? "inferred edge" : "explicit wire";
      throw new Error(
        `buildDag: ${edgeDescription} ${JSON.stringify(producerId)} → ${JSON.stringify(id)} ` +
        `is incompatible: ${describeKind(producerKind)} cannot feed ${describeKind(consumes)}`,
      );
    }

    producerById.set(id, producerId);
    resolving.delete(id);
    return producerId;
  }

  /** Effective output includes stream promotion for per-item stages. */
  function effectiveKind(id: string): KindDescriptor {
    const cached = effectiveProduces.get(id);
    if (cached) return cached;
    const inst = instances.get(id)!;
    const producerId = resolveProducer(id);
    const declared = inst.stage.produces;
    if (producerId === null) {
      effectiveProduces.set(id, declared);
      return declared;
    }
    const producerKind = effectiveKind(producerId);
    const promoted: KindDescriptor =
      producerKind.name === "Stream" &&
      inst.stage.consumes.name !== "Stream" &&
      declared.name !== "Stream"
        ? { name: "Stream", version: declared.version, inner: declared }
        : declared;
    effectiveProduces.set(id, promoted);
    return promoted;
  }

  for (const id of ids) resolveProducer(id);

  // Stable Kahn topological sort. Declaration order breaks ties, but
  // explicit forward wires are free to reorder dependent instances.
  const consumers = new Map<string, string[]>();
  const indegree = new Map<string, number>();
  for (const id of ids) {
    const producerId = producerById.get(id)!;
    indegree.set(id, producerId === null ? 0 : 1);
    if (producerId !== null) {
      const list = consumers.get(producerId);
      if (list) list.push(id);
      else consumers.set(producerId, [id]);
    }
  }
  const ready = ids.filter(id => indegree.get(id) === 0);
  const topoOrder: string[] = [];
  while (ready.length > 0) {
    const id = ready.shift()!;
    topoOrder.push(id);
    for (const consumerId of consumers.get(id) ?? []) {
      const next = indegree.get(consumerId)! - 1;
      indegree.set(consumerId, next);
      if (next === 0) {
        ready.push(consumerId);
        ready.sort((left, right) => declarationIndex.get(left)! - declarationIndex.get(right)!);
      }
    }
  }
  if (topoOrder.length !== ids.length) {
    const cyclic = ids.filter(id => !topoOrder.includes(id));
    throw new Error(`buildDag: cycle detected among instances ${cyclic.map(id => JSON.stringify(id)).join(", ")}`);
  }

  // Replace map entries with resolved producer information.
  for (const id of ids) {
    const inst = instances.get(id)!;
    instances.set(id, { ...inst, producer: producerById.get(id)! });
  }

  const consumedAsProducer = new Set(
    Array.from(producerById.values()).filter((id): id is string => id !== null),
  );
  const sinks = ids.filter(id => !consumedAsProducer.has(id));
  const sources = ids.filter(id => producerById.get(id) === null);

  return {
    instances,
    topoOrder,
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
