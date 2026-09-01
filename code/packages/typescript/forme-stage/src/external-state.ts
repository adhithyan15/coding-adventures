/**
 * Deterministic description of external state observed by a source stage.
 *
 * The manifest is deliberately small and backend-neutral. A locator names the
 * observed object inside the source provider, `identity` carries stable Forme
 * identity when the provider has one, and `revision` identifies its content.
 * Providers sort entries by locator before hashing and returning the manifest.
 */

import type { LogicalId, RevisionId } from "@coding-adventures/forme-types";

/** One externally observed source object. */
export interface ExternalStateEntry {
  readonly locator: string;
  readonly identity?: LogicalId;
  readonly revision: RevisionId;
}

/** Versioned, content-addressed source observation. */
export interface ExternalStateManifest {
  readonly version: 1;
  readonly revision: RevisionId;
  readonly entries: readonly ExternalStateEntry[];
}
