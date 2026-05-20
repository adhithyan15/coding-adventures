/**
 * generate.ts — main `generatePageBundle` entry.
 *
 * Two-pass fail-fast:
 *
 *   1. Validate config (baseUrl, every page's route + fields),
 *      check duplicate routes, build resolved entry list.
 *   2. Serialise to canonical JSON.
 *
 * Output JSON is byte-deterministic:
 *   - Top-level keys in fixed order: `version`, `baseUrl`
 *     (if present), `routes`.
 *   - `routes` object keys sorted lexicographically (using the
 *     route string, which is already canonical).
 *   - Each route entry's keys in fixed order:
 *     `route → outputPath → contentType → sizeBytes → sha256 →
 *     lastmod` (lastmod only if present).
 *   - JSON formatted with 2-space indent + trailing newline so
 *     it diffs cleanly and matches what most editors emit.
 *
 * @module generate
 */

import { sha256Base64, utf8ByteLength } from "./hash.js";
import { routeToOutputPath } from "./path.js";
import type { PageBundleConfig, RouteEntry } from "./types.js";
import { validateBaseUrl, validateRoute, validateString } from "./validate.js";

const DEFAULT_CONTENT_TYPE = "text/html; charset=utf-8";

/**
 * Build a deploy bundle manifest JSON string from a
 * `PageBundleConfig`.  Synchronous, pure, deterministic.
 *
 * ```ts
 * const manifest = generatePageBundle({
 *   baseUrl: "https://example.com",
 *   pages: [
 *     { route: "/", html: "<!doctype html>..." },
 *     { route: "/about", html: "<!doctype html>..." },
 *     { route: "/feed.xml", html: "<?xml ...?>",
 *       contentType: "application/rss+xml" },
 *   ],
 * });
 * // {
 * //   "version": 1,
 * //   "baseUrl": "https://example.com",
 * //   "routes": {
 * //     "/":         { "route": "/", "outputPath": "index.html", ... },
 * //     "/about":    { "route": "/about", "outputPath": "about/index.html", ... },
 * //     "/feed.xml": { "route": "/feed.xml", "outputPath": "feed.xml", ... }
 * //   }
 * // }
 * ```
 *
 * Routes are sorted alphabetically in the output regardless
 * of input order.  Duplicate routes throw.
 */
export function generatePageBundle(config: PageBundleConfig): string {
  if (config === null || typeof config !== "object") {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: config must be a non-null object; got ${typeof config}`,
    );
  }
  if (!Array.isArray(config.pages)) {
    throw new TypeError(
      `forme-aot-page-bundle-emitter: pages must be an array; got ${typeof config.pages}`,
    );
  }
  const baseUrl = config.baseUrl === undefined
    ? undefined
    : validateBaseUrl(config.baseUrl);

  // Validate every page; check duplicates.
  const seenRoutes = new Set<string>();
  const resolved: RouteEntry[] = new Array(config.pages.length);
  for (let i = 0; i < config.pages.length; i++) {
    const page = config.pages[i];
    if (page === null || typeof page !== "object") {
      throw new TypeError(
        `forme-aot-page-bundle-emitter: pages[${i}] must be a non-null object; got ${typeof page}`,
      );
    }
    const route = validateRoute(page.route, `pages[${i}].route`);
    if (seenRoutes.has(route)) {
      throw new TypeError(
        `forme-aot-page-bundle-emitter: pages[${i}].route duplicates an earlier entry: ${JSON.stringify(route)}`,
      );
    }
    seenRoutes.add(route);

    const html = validateString(page.html, `pages[${i}].html`);
    const contentType = page.contentType === undefined
      ? DEFAULT_CONTENT_TYPE
      : validateString(page.contentType, `pages[${i}].contentType`);
    const lastmod = page.lastmod === undefined
      ? undefined
      : validateString(page.lastmod, `pages[${i}].lastmod`);

    resolved[i] = {
      route,
      outputPath: routeToOutputPath(route),
      contentType,
      sizeBytes: utf8ByteLength(html),
      sha256: sha256Base64(html),
      lastmod,
    };
  }

  // Sort by route lexicographically for deterministic output.
  resolved.sort((a, b) => (a.route < b.route ? -1 : a.route > b.route ? 1 : 0));

  // Serialise.  We build the object explicitly key-by-key
  // rather than using `JSON.stringify(obj, null, 2)` directly
  // on a plain object — JSON.stringify *does* use insertion
  // order for object keys, but constructing the inner objects
  // in fixed key order is clearer and more obviously correct.
  const routesObj: Record<string, Record<string, unknown>> = {};
  for (const entry of resolved) {
    const inner: Record<string, unknown> = {
      route: entry.route,
      outputPath: entry.outputPath,
      contentType: entry.contentType,
      sizeBytes: entry.sizeBytes,
      sha256: entry.sha256,
    };
    if (entry.lastmod !== undefined) {
      inner.lastmod = entry.lastmod;
    }
    routesObj[entry.route] = inner;
  }

  // Top-level object in fixed key order.
  const top: Record<string, unknown> = { version: 1 };
  if (baseUrl !== undefined) top.baseUrl = baseUrl;
  top.routes = routesObj;

  // 2-space indent, trailing newline.
  return `${JSON.stringify(top, null, 2)}\n`;
}
