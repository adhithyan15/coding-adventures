/** Deterministic construction and validation of output provenance. */

import type {
  JsonValue,
  OutputProvenance,
  ProvenanceContributor,
} from "@coding-adventures/forme-types";
import { isLogicalIdShape } from "./logical-id.js";
import { computeRevisionId, isRevisionIdShape } from "./revision.js";

const PROVENANCE_DOMAIN = "forme-output-provenance-v1";

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
