/**
 * Identity types — the two stable identifiers every piece of Forme content
 * carries.  See FM01 §7 for the full theory; the short version is below.
 *
 * === Two distinct IDs ===
 *
 * - LogicalId  — identifies a *thing over time*.  "This particular post,"
 *                "this author."  Stable across content edits and file
 *                renames.  Encoded as a UUIDv7 string.
 *
 * - RevisionId — identifies *a specific version of a thing*.  Changes
 *                whenever the canonical serialisation changes.  Two
 *                entities with the same RevisionId are byte-identical.
 *                Encoded as `<algo>:<hex>` (algo is `blake2b` in the v0
 *                implementation; FM01 originally specified `blake3` and
 *                we will migrate when a from-scratch BLAKE3 lands in the
 *                monorepo — the format is forward-compatible).
 *
 * === Why brand the strings? ===
 *
 * Both IDs are strings at runtime.  Without TypeScript brands, a function
 * expecting `LogicalId` would silently accept any string — including a
 * RevisionId, a file path, or `""`.  Branding turns these mistakes into
 * compile errors at the API boundary while costing nothing at runtime.
 *
 * The actual *value-producing* functions — `computeRevisionId`,
 * `canonicalJson`, UUIDv7 generation — live in `@coding-adventures/forme-identity`.
 * `forme-types` only declares the type aliases so every package that
 * carries an ID can import them without depending on the hashing code.
 *
 * === Cross-package usage ===
 *
 *   import type { LogicalId, RevisionId } from "@coding-adventures/forme-types";
 *   import { computeRevisionId } from "@coding-adventures/forme-identity";
 *
 *   const rev: RevisionId = computeRevisionId(payload);
 */

/**
 * Stable logical identity.  A UUIDv7 string, branded.  Assigned to an
 * entity the first time a source observes it; preserved across renames
 * and content edits.
 */
export type LogicalId = string & { readonly __brand: "LogicalId" };

/**
 * Content-addressed revision identity.  A `<algo>:<hex>` string, branded.
 * Two entities with the same RevisionId are byte-identical under the
 * canonical-JSON encoding (FM01 §7.3).
 */
export type RevisionId = string & { readonly __brand: "RevisionId" };
