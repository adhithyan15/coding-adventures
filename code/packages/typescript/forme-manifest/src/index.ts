/**
 * @coding-adventures/forme-manifest
 *
 * Forme plugin manifest layer (FM02 §3, §14.1).
 *
 * The first FM02 implementation package: parses `plugin.toml`,
 * validates per §3.3, serialises canonically for signing/hashing,
 * computes a content-addressed hash that incorporates the entry
 * file, signs/verifies via Ed25519, and resolves `$variable`
 * templates against the runtime environment.
 *
 * Consumed by `forme-plugin-host` (FM02 §14.2) and the CLI's
 * `forme install` (FM07).  Nothing else: this package only
 * understands the manifest itself.
 *
 * See `code/specs/FM02-forme-plugin-host.md` for the full design,
 * and `README.md` for usage examples.
 */

// Types
export type {
  Manifest,
  PluginIdentity,
  RuntimeSpec,
  RuntimeKind,
  CapabilityBlock,
  CapabilityEntry,
  ContributesBlock,
  StageContribution,
  KindContribution,
  ResourceLimits,
  SignatureBlock,
  SignatureAlgorithm,
} from "./manifest-types.js";
export { RUNTIME_KINDS, SIGNATURE_ALGORITHMS } from "./manifest-types.js";

// Errors
export type { ManifestErrorCode, ManifestErrorEntry, ManifestErrorInit } from "./errors.js";
export { ManifestError, MANIFEST_ERROR_CODES } from "./errors.js";

// Parsing
export { parseManifest } from "./parse-toml.js";

// Validation
export {
  validateManifest,
  PLUGIN_NAME_REGEX,
  SEMVER_REGEX,
  EXT_KIND_NAME_REGEX,
  RESERVED_KIND_NAMES,
  RESOURCE_CEILINGS,
  SUPPORTED_API_VERSIONS,
  SUPPORTED_MANIFEST_VERSIONS,
} from "./validate.js";

// Canonical encoding
export { canonicalManifestToml } from "./canonical.js";

// Hash
export {
  computeManifestHash,
  isManifestHashShape,
  MANIFEST_HASH_ALGORITHM,
  MANIFEST_HASH_DIGEST_BYTES,
  MANIFEST_HASH_SEPARATOR,
} from "./manifest-hash.js";

// Signature
export { signManifest, verifyManifest, assertManifestSigned } from "./signature.js";

// Templating
export {
  resolveCapabilityTemplate,
  hasTemplate,
  RECOGNISED_VARIABLES,
} from "./templating.js";
export type { TemplateEnv } from "./templating.js";
