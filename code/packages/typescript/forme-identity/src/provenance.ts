/** Deterministic construction and validation of output provenance. */

import type {
  JsonValue,
  OutputProvenance,
  ProvenanceContributor,
} from "@coding-adventures/forme-types";
import { isLogicalIdShape } from "./logical-id.js";
import { computeRevisionId, isRevisionIdShape } from "./revision.js";

const PROVENANCE_DOMAIN = "forme-output-provenance-v1";
const MAX_PROVENANCE_CONTRIBUTORS = 65_536;

/**
 * Validate, deduplicate, and canonically order an output's contributors.
 *
 * The returned revision hashes a domain-separated JSON value containing the
 * normalized contributor set. Input order therefore cannot perturb aggregate
 * provenance. An empty set is valid for collection outputs such as an empty
 * site's index or feeds.
 */
export function createOutputProvenance(
  contributors: readonly ProvenanceContributor[],
): OutputProvenance {
  if (!Array.isArray(contributors)) {
    throw new TypeError("createOutputProvenance: contributors must be an array");
  }

  const byIdentity = new Map<string, ProvenanceContributor>();
  contributors.forEach((contributor, index) => {
    if (contributor === null || typeof contributor !== "object" || Array.isArray(contributor)) {
      throw new TypeError(
        `createOutputProvenance: contributors[${index}] must be an object`,
      );
    }
    if (typeof contributor.identity !== "string" || !isLogicalIdShape(contributor.identity)) {
      throw new TypeError(
        `createOutputProvenance: contributors[${index}].identity must be a lowercase UUIDv7 LogicalId; got ${JSON.stringify(contributor.identity)}`,
      );
    }
    if (typeof contributor.revision !== "string" || !isRevisionIdShape(contributor.revision)) {
      throw new TypeError(
        `createOutputProvenance: contributors[${index}].revision must be a RevisionId (<algorithm>:<lowercase hex>); got ${JSON.stringify(contributor.revision)}`,
      );
    }

    const existing = byIdentity.get(contributor.identity);
    if (existing !== undefined && existing.revision !== contributor.revision) {
      throw new TypeError(
        `createOutputProvenance: logical identity ${contributor.identity} has conflicting revisions ${existing.revision} and ${contributor.revision}`,
      );
    }
    byIdentity.set(contributor.identity, Object.freeze({
      identity: contributor.identity,
      revision: contributor.revision,
    }));
  });

  const normalized = Object.freeze(
    [...byIdentity.values()].sort((left, right) =>
      left.identity < right.identity ? -1 : 1
    ),
  );
  const hashInput: JsonValue = {
    domain: PROVENANCE_DOMAIN,
    contributors: normalized.map(({ identity, revision }) => ({ identity, revision })),
  };

  return Object.freeze({
    contributors: normalized,
    revision: computeRevisionId(hashInput),
  });
}

/**
 * Admit provenance received across an untyped plugin boundary.
 *
 * Every admitted property is read from an own data descriptor after proxies,
 * accessors, and sparse arrays have been rejected. Unrelated own properties
 * are never enumerated: they are inert because the returned value is a fresh
 * canonical snapshot, and ignoring them prevents a hostile key flood from
 * forcing an unbounded allocation. The bounded snapshot is then normalized
 * and hashed through the canonical constructor; callers cannot merely claim
 * the revision of an unordered or forged set.
 */
export function validateOutputProvenance(
  value: unknown,
  isProxy: (value: object) => boolean,
): OutputProvenance {
  const provenance = boundedRecord(value, "provenance", ["contributors", "revision"], isProxy);
  const rawContributors = dataProperty(provenance, "contributors", "provenance.contributors");
  const rawRevision = dataProperty(provenance, "revision", "provenance.revision");
  const entries = exactArray(rawContributors, "provenance.contributors", isProxy);
  const contributors: ProvenanceContributor[] = [];

  entries.forEach((entry, index) => {
    const path = `provenance.contributors[${index}]`;
    const contributor = boundedRecord(entry, path, ["identity", "revision"], isProxy);
    const identity = dataProperty(contributor, "identity", `${path}.identity`);
    const revision = dataProperty(contributor, "revision", `${path}.revision`);
    if (typeof identity !== "string" || identity.length > 64) {
      throw new TypeError(`validateOutputProvenance: ${path}.identity must be a bounded string`);
    }
    if (typeof revision !== "string" || revision.length > 256) {
      throw new TypeError(`validateOutputProvenance: ${path}.revision must be a bounded string`);
    }
    contributors.push({
      identity: identity as ProvenanceContributor["identity"],
      revision: revision as ProvenanceContributor["revision"],
    });
  });

  if (typeof rawRevision !== "string" || rawRevision.length > 256) {
    throw new TypeError("validateOutputProvenance: provenance.revision must be a bounded string");
  }

  const canonical = createOutputProvenance(contributors);
  if (canonical.contributors.length !== contributors.length
      || canonical.contributors.some((contributor, index) =>
        contributor.identity !== contributors[index]?.identity
        || contributor.revision !== contributors[index]?.revision)) {
    throw new TypeError(
      "validateOutputProvenance: provenance.contributors must be in canonical order without duplicates",
    );
  }
  if (rawRevision !== canonical.revision) {
    throw new TypeError(
      "validateOutputProvenance: provenance.revision does not match the canonical contributor set",
    );
  }
  return canonical;
}

function boundedRecord(
  value: unknown,
  path: string,
  expectedKeys: readonly string[],
  isProxy: (value: object) => boolean,
): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value) || isProxy(value)) {
    throw new TypeError(`validateOutputProvenance: ${path} must be a non-proxy object`);
  }
  for (const key of expectedKeys) {
    dataProperty(value as Record<string, unknown>, key, `${path}.${key}`);
  }
  return value as Record<string, unknown>;
}

function dataProperty(record: Record<string, unknown>, key: string, path: string): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(record, key);
  if (descriptor === undefined) {
    throw new TypeError(`validateOutputProvenance: ${path} is required`);
  }
  if (!("value" in descriptor)) {
    throw new TypeError(`validateOutputProvenance: ${path} must not be an accessor`);
  }
  return descriptor.value;
}

function exactArray(
  value: unknown,
  path: string,
  isProxy: (value: object) => boolean,
): readonly unknown[] {
  if (!Array.isArray(value) || isProxy(value) || value.length > MAX_PROVENANCE_CONTRIBUTORS) {
    throw new TypeError(
      `validateOutputProvenance: ${path} must be a non-proxy array of at most ${MAX_PROVENANCE_CONTRIBUTORS} entries`,
    );
  }
  const copy: unknown[] = [];
  for (let index = 0; index < value.length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (descriptor === undefined) {
      throw new TypeError(`validateOutputProvenance: ${path}[${index}] is sparse`);
    }
    if (!("value" in descriptor)) {
      throw new TypeError(`validateOutputProvenance: ${path}[${index}] must not be an accessor`);
    }
    copy.push(descriptor.value);
  }
  return copy;
}
