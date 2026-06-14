/**
 * parse-page-bundle.ts — safe JSON parse + shape check for the
 * incoming page bundle string.
 *
 * Defensive design notes:
 *
 *   - We use `JSON.parse` with no reviver and immediately copy
 *     every field we care about into fresh local objects (via
 *     `Object.create(null)`-style accumulators).  This means
 *     even if the input JSON contained `__proto__` or
 *     `constructor` keys (which `JSON.parse` happily accepts),
 *     they end up as own properties on the parsed object —
 *     never reaching `Object.prototype`.  We never iterate them
 *     via `for...in`; we walk known fields by name only.
 *
 *   - We don't trust the bundle's `routes` keys; we validate
 *     each entry's `route` / `outputPath` / `contentType` /
 *     `sizeBytes` / `sha256` field individually.
 *
 *   - We don't re-hash the page HTML (we don't have it — only
 *     the metadata).  We trust the page bundle's hashes
 *     verbatim; if the caller wanted a fresh hash, they would
 *     re-run `forme-aot-page-bundle-emitter`.
 *
 * @module parse-page-bundle
 */

import type { DeployFileEntry } from "./types.js";
import { validateOutputPath } from "./validate.js";

interface RawRoute {
  readonly route: string;
  readonly outputPath: string;
  readonly contentType: string;
  readonly sizeBytes: number;
  readonly sha256: string;
  readonly lastmod?: string;
}

interface ParsedPageBundle {
  readonly baseUrl?: string;
  readonly routes: readonly RawRoute[];
}

/**
 * Parse + validate a page-bundle JSON string.  Throws
 * `TypeError` for any structural problem.
 */
export function parsePageBundle(json: string): ParsedPageBundle {
  let parsed: unknown;
  try {
    parsed = JSON.parse(json);
  } catch (e) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: pageBundle is not valid JSON: ${(e as Error).message}`,
    );
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: pageBundle root must be a JSON object`,
    );
  }
  const obj = parsed as Record<string, unknown>;

  if (obj.version !== 1) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: pageBundle.version must be 1; got ${JSON.stringify(obj.version)}`,
    );
  }

  let baseUrl: string | undefined;
  if (obj.baseUrl !== undefined) {
    if (typeof obj.baseUrl !== "string") {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: pageBundle.baseUrl must be a string; got ${typeof obj.baseUrl}`,
      );
    }
    baseUrl = obj.baseUrl;
  }

  if (obj.routes === null || typeof obj.routes !== "object" || Array.isArray(obj.routes)) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: pageBundle.routes must be a JSON object`,
    );
  }
  const routesObj = obj.routes as Record<string, unknown>;

  const routes: RawRoute[] = [];
  // Walk via `Object.keys` (own enumerable only — no prototype walk).
  for (const key of Object.keys(routesObj)) {
    const entry = routesObj[key];
    if (entry === null || typeof entry !== "object" || Array.isArray(entry)) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: pageBundle.routes[${JSON.stringify(key)}] must be a JSON object`,
      );
    }
    const e = entry as Record<string, unknown>;
    if (typeof e.route !== "string") throw badField(key, "route", e.route);
    if (typeof e.outputPath !== "string") throw badField(key, "outputPath", e.outputPath);
    // Defence-in-depth: even though the page-bundle emitter
    // validated routes and derived outputPaths, our caller can
    // pass ANY JSON string as `pageBundle` — there's no
    // type-system guarantee it came from the trusted upstream.
    // Re-validate every outputPath against the same shape rules
    // we apply to extraFiles.
    validateOutputPath(e.outputPath, `pageBundle.routes[${JSON.stringify(key)}].outputPath`);
    if (typeof e.contentType !== "string") throw badField(key, "contentType", e.contentType);
    if (typeof e.sizeBytes !== "number" || !Number.isInteger(e.sizeBytes) || e.sizeBytes < 0) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: pageBundle.routes[${JSON.stringify(key)}].sizeBytes must be a non-negative integer; got ${JSON.stringify(e.sizeBytes)}`,
      );
    }
    if (typeof e.sha256 !== "string") throw badField(key, "sha256", e.sha256);
    let lastmod: string | undefined;
    if (e.lastmod !== undefined) {
      if (typeof e.lastmod !== "string") throw badField(key, "lastmod", e.lastmod);
      lastmod = e.lastmod;
    }
    routes.push({
      route: e.route,
      outputPath: e.outputPath,
      contentType: e.contentType,
      sizeBytes: e.sizeBytes,
      sha256: e.sha256,
      lastmod,
    });
  }

  return { baseUrl, routes };
}

/**
 * Convert a parsed page-bundle route into a deploy file entry.
 */
export function routeToDeployEntry(route: RawRoute): DeployFileEntry {
  return {
    outputPath: route.outputPath,
    contentType: route.contentType,
    sizeBytes: route.sizeBytes,
    sha256: route.sha256,
    route: route.route,
    source: "page-bundle",
    lastmod: route.lastmod,
  };
}

function badField(key: string, field: string, got: unknown): Error {
  return new TypeError(
    `forme-aot-deploy-manifest-emitter: pageBundle.routes[${JSON.stringify(key)}].${field} must be a string; got ${typeof got}`,
  );
}
