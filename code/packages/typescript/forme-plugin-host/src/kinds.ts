import type { Manifest } from "@coding-adventures/forme-manifest";
import {
  Kinds,
  streamOf,
  type KindDescriptor,
  type KindName,
  type KindRegistry,
  type RegisteredKind,
} from "@coding-adventures/forme-types";
import { PluginHostError } from "./errors.js";

const MAX_CONTRIBUTED_KINDS = 4_096;

export function descriptorForKindReference(
  reference: string,
  manifest: Manifest,
  registry?: KindRegistry,
): KindDescriptor {
  const stream = /^Stream<([^<>]+)>$/.exec(reference);
  if (stream) return streamOf(descriptorForBareKind(stream[1]!, manifest, registry));
  return descriptorForBareKind(reference, manifest, registry);
}

function descriptorForBareKind(
  reference: string,
  manifest: Manifest,
  registry?: KindRegistry,
): KindDescriptor {
  const builtin = (Kinds as Readonly<Record<string, KindDescriptor>>)[reference];
  if (builtin) return builtin;
  const registered = registry?.lookup(reference as KindName);
  if (registered) return registered.descriptor;
  const contributed = manifest.contributes.kinds.find(kind => kind.name === reference);
  if (contributed) {
    return Object.freeze({ name: contributed.name as KindName, version: contributed.version });
  }
  throw new PluginHostError("MANIFEST_INVALID", `unknown kind reference ${JSON.stringify(reference)}`);
}

/**
 * Build the one immutable kind registry used by every stage in a host.
 *
 * Construction is deliberately split into collect, validate, and publish
 * phases. No partially built registry escapes, duplicate names fail before
 * parent resolution, and cycle detection is iterative so a hostile but
 * otherwise bounded manifest set cannot exhaust the JavaScript call stack.
 */
export function buildKindRegistry(manifests: Iterable<Manifest>): KindRegistry {
  const descriptors = new Map<KindName, KindDescriptor>();
  const parents = new Map<KindName, KindName | null>();
  let contributedKinds = 0;

  for (const descriptor of Object.values(Kinds)) {
    descriptors.set(descriptor.name, descriptor);
    parents.set(descriptor.name, null);
  }

  for (const manifest of manifests) {
    for (const contribution of manifest.contributes.kinds) {
      contributedKinds += 1;
      if (contributedKinds > MAX_CONTRIBUTED_KINDS) {
        throw new PluginHostError(
          "RESOURCE_LIMIT_EXCEEDED",
          `kind registry exceeds the ${MAX_CONTRIBUTED_KINDS} contributed kind limit`,
        );
      }
      const name = contribution.name as KindName;
      if (descriptors.has(name)) {
        throw new PluginHostError(
          "MANIFEST_INVALID",
          `duplicate kind name ${JSON.stringify(contribution.name)} across plugins`,
        );
      }
      const descriptor = Object.freeze({
        name,
        version: contribution.version,
      }) satisfies KindDescriptor;
      descriptors.set(name, descriptor);
      parents.set(name, contribution.subtypeOf === undefined
        ? null
        : contribution.subtypeOf as KindName);
    }
  }

  for (const [name, parentName] of parents) {
    if (parentName !== null && !descriptors.has(parentName)) {
      throw new PluginHostError(
        "MANIFEST_INVALID",
        `kind ${JSON.stringify(name)} has missing parent ${JSON.stringify(parentName)}`,
      );
    }
  }

  const complete = new Set<KindName>();
  for (const start of parents.keys()) {
    if (complete.has(start)) continue;
    const path: KindName[] = [];
    const position = new Map<KindName, number>();
    let current: KindName | null = start;
    while (current !== null && parents.has(current) && !complete.has(current)) {
      const cycleStart = position.get(current);
      if (cycleStart !== undefined) {
        const cycle = [...path.slice(cycleStart), current];
        throw new PluginHostError(
          "MANIFEST_INVALID",
          `kind subtype cycle detected: ${cycle.join(" -> ")}`,
        );
      }
      position.set(current, path.length);
      path.push(current);
      current = parents.get(current) ?? null;
    }
    for (const name of path) complete.add(name);
  }

  const registrations = new Map<KindName, RegisteredKind>();
  for (const [name, descriptor] of descriptors) {
    const parentName = parents.get(name) ?? null;
    const parent = parentName === null ? null : descriptors.get(parentName)!;
    registrations.set(name, Object.freeze({ descriptor, parent }));
  }

  return Object.freeze({
    lookup(name: KindName): RegisteredKind | undefined {
      return registrations.get(name);
    },
  });
}
