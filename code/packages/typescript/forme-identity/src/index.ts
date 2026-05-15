/**
 * @coding-adventures/forme-identity
 *
 * Forme kernel identity layer.  Three responsibilities:
 *
 *   - **Canonical JSON.**  RFC 8785 serialiser; what makes content
 *     hashes stable across runs.
 *   - **RevisionId.**  Content-addressed identity computed by hashing
 *     the canonical JSON with BLAKE2b-256 and prefixing the digest
 *     with `blake2b:`.
 *   - **LogicalId.**  Time-ordered UUIDv7 generation for "this thing
 *     over time" identity (FM01 §7.2).
 *
 * See FM01 §7 for the design.  See per-module headers for the rationale
 * behind each implementation choice.
 */

export { canonicalJson } from "./canonical-json.js";

export {
  computeRevisionId,
  isRevisionIdShape,
  REVISION_ALGORITHM,
  REVISION_DIGEST_BYTES,
} from "./revision.js";

export {
  generateLogicalId,
  buildLogicalIdFrom,
  isLogicalIdShape,
} from "./logical-id.js";
