/**
 * @coding-adventures/forme-aot-deploy-manifest-emitter
 *
 * Compose the per-stage FM00 v0 emitter outputs (page bundle
 * JSON + sitemap.xml + robots.txt + Web App Manifest JSON +
 * caller-supplied extra files) into a single, byte-deterministic
 * deploy manifest JSON.  A downstream deploy runner reads the
 * manifest to apply every file to the deploy target.
 *
 * Pure transform.  Uses Node's built-in `node:crypto` for
 * SHA-256.  No I/O / fs / network / shell / env.  Capabilities:
 * `[]`.
 *
 * Twentieth FM00 v0 stage package.
 *
 * @module index
 */

export { generateDeployManifest } from "./generate.js";
export { sha256Base64, utf8ByteLength } from "./hash.js";
export { validateOutputPath, validateString } from "./validate.js";
export { parsePageBundle, routeToDeployEntry } from "./parse-page-bundle.js";
export type {
  DeployManifestConfig,
  ExtraFile,
  DeployFileEntry,
  DeployManifest,
} from "./types.js";
