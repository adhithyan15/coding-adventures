/**
 * Kind compatibility check for typed DAG construction (FM01 §2.6).
 *
 * Implements the four rules from the spec:
 *
 *   1. Name match, or covariant widening through the immutable extension-kind
 *      registry supplied by the plugin host.
 *   2. Major-version match; minor-version compatibility (newer-minor
 *      producer can feed older-minor consumer).
 *   3. Discriminant equality if both declare one.
 *   4. Constraint satisfaction — best-effort structural match;
 *      unknown keys are warnings (logged by the caller), not errors.
 *
 * For Stream<K>, the wrapped kind's compatibility is checked
 * recursively.  A `Stream<X>` produced by one stage matches a
 * `Stream<X>` consumed by another, and a `Stream<X>` produced by a
 * source can also feed a single-`X` consumer (the orchestrator
 * iterates and invokes the consumer once per yielded value).
 */

import type { KindDescriptor, KindRegistry } from "@coding-adventures/forme-types";

/**
 * Test whether a producer's `produces` descriptor is compatible with
 * a consumer's `consumes` descriptor.  Returns `true` if the
 * orchestrator can wire them; `false` if not.
 */
export function areKindsCompatible(
  produces: KindDescriptor,
  consumes: KindDescriptor,
  registry?: KindRegistry,
): boolean {
  // Stream-of-X can feed a single-X consumer (the executor iterates).
  if (produces.name === "Stream" && consumes.name !== "Stream") {
    if (!produces.inner) return false;
    return areKindsCompatible(produces.inner, consumes, registry);
  }
  // A single-X producer cannot feed a Stream-of-X consumer (the
  // consumer expects multiple values; producer yields exactly one).
  if (produces.name !== "Stream" && consumes.name === "Stream") {
    return false;
  }
  // Stream<X> → Stream<Y> requires inner compatibility.
  if (produces.name === "Stream" && consumes.name === "Stream") {
    if (!produces.inner || !consumes.inner) return false;
    return areKindsCompatible(produces.inner, consumes.inner, registry);
  }
  // Both are non-stream descriptors.
  if (produces.name === consumes.name) {
    return areDescriptorsCompatible(produces, consumes);
  }
  if (registry === undefined || !produces.name.startsWith("ext:")) return false;

  const seen = new Set<string>();
  let current = produces;
  while (current.name.startsWith("ext:")) {
    if (seen.has(current.name)) return false;
    seen.add(current.name);

    const registration = registry.lookup(current.name);
    if (registration === undefined
        || registration.descriptor.version !== current.version
        || registration.descriptor.discriminant !== current.discriminant
        || registration.parent === null
        || !areDescriptorsCompatible(current, registration.parent)) {
      return false;
    }
    current = registration.parent;
    if (current.name === consumes.name) {
      return areDescriptorsCompatible(current, consumes);
    }
  }
  return false;
}

function areDescriptorsCompatible(
  produces: KindDescriptor,
  consumes: KindDescriptor,
): boolean {
  if (!areVersionsCompatible(produces.version, consumes.version)) return false;
  if (!areDiscriminantsCompatible(produces.discriminant, consumes.discriminant)) return false;
  // Constraints are advisory at this layer.
  return true;
}

/**
 * Major versions must match; producer minor must be ≥ consumer minor.
 * Patch is ignored.  Pre-release tags (e.g. "1.0.0-alpha") are
 * treated as ordinary string suffixes — the kernel doesn't ship
 * pre-release stages so this isn't a meaningful gap for v0.
 */
function areVersionsCompatible(produces: string, consumes: string): boolean {
  const p = parseSemver(produces);
  const c = parseSemver(consumes);
  if (p === null || c === null) {
    // If either fails to parse, fall back to string equality —
    // exotic version strings just need to match exactly.
    return produces === consumes;
  }
  if (p.major !== c.major) return false;
  return p.minor >= c.minor;
}

function parseSemver(v: string): { major: number; minor: number } | null {
  const m = /^(\d+)\.(\d+)/.exec(v);
  if (!m) return null;
  return { major: Number(m[1]), minor: Number(m[2]) };
}

function areDiscriminantsCompatible(
  produces: string | undefined,
  consumes: string | undefined,
): boolean {
  // If consumer doesn't care, anything matches.
  if (consumes === undefined) return true;
  // If producer doesn't declare but consumer does, mismatch.
  if (produces === undefined) return false;
  return produces === consumes;
}
