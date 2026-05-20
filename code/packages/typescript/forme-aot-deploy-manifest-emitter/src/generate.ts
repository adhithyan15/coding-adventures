/**
 * generate.ts — main `generateDeployManifest` entry.
 *
 * Two-pass fail-fast:
 *   1. Parse pageBundle, validate extraFiles (paths + content
 *      strings), measure + hash each synthetic file
 *      (sitemap / robots / web-app-manifest).
 *   2. Sort by outputPath and serialise to canonical JSON.
 *
 * Output JSON shape (byte-deterministic):
 *
 *     {
 *       "version": 1,
 *       "baseUrl": "...",          // present only if pageBundle had one
 *       "fileCount": <int>,
 *       "totalSizeBytes": <int>,
 *       "files": {
 *         "<outputPath>": {
 *           "outputPath": "...",
 *           "contentType": "...",
 *           "sizeBytes": <int>,
 *           "sha256": "...=",
 *           "route": "..."         // only for page-bundle entries
 *           "source": "page-bundle|sitemap|robots|web-app-manifest|extra",
 *           "lastmod": "..."       // only when present
 *         },
 *         ...
 *       }
 *     }
 *
 * Files are sorted by `outputPath` lexicographically.  Within
 * each entry, keys are emitted in a fixed order.  Same input →
 * byte-identical output.
 *
 * Duplicate output paths (across pageBundle + sitemap + robots
 * + manifest + extraFiles) throw — the manifest must be
 * unambiguous about what gets written where.
 *
 * @module generate
 */

import { sha256Base64, utf8ByteLength } from "./hash.js";
import { parsePageBundle, routeToDeployEntry } from "./parse-page-bundle.js";
import type {
  DeployFileEntry,
  DeployManifestConfig,
  ExtraFile,
} from "./types.js";
import { validateOutputPath, validateString } from "./validate.js";

const SITEMAP_PATH = "sitemap.xml";
const SITEMAP_CONTENT_TYPE = "application/xml";
const ROBOTS_PATH = "robots.txt";
const ROBOTS_CONTENT_TYPE = "text/plain; charset=utf-8";
const WEB_APP_MANIFEST_PATH = "manifest.webmanifest";
const WEB_APP_MANIFEST_CONTENT_TYPE = "application/manifest+json";

/**
 * Build the deploy manifest JSON string from the composed
 * outputs of the FM00 v0 emitters.  Synchronous, pure,
 * deterministic.
 */
export function generateDeployManifest(config: DeployManifestConfig): string {
  if (config === null || typeof config !== "object") {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: config must be a non-null object; got ${typeof config}`,
    );
  }
  if (typeof config.pageBundle !== "string") {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: config.pageBundle must be a string; got ${typeof config.pageBundle}`,
    );
  }

  // 1. Page bundle is the source of truth for HTML pages.
  const bundle = parsePageBundle(config.pageBundle);
  const seenPaths = new Set<string>();
  const files: DeployFileEntry[] = [];
  for (const route of bundle.routes) {
    if (seenPaths.has(route.outputPath)) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: pageBundle has duplicate outputPath ${JSON.stringify(route.outputPath)}`,
      );
    }
    seenPaths.add(route.outputPath);
    files.push(routeToDeployEntry(route));
  }

  // 2. sitemap / robots / web-app-manifest — synthesise a
  //    file entry each (hash + measure + fixed output path).
  if (config.sitemapXml !== undefined) {
    const content = validateString(config.sitemapXml, "sitemapXml");
    addSynthetic(files, seenPaths, SITEMAP_PATH, content, SITEMAP_CONTENT_TYPE, "sitemap");
  }
  if (config.robotsTxt !== undefined) {
    const content = validateString(config.robotsTxt, "robotsTxt");
    addSynthetic(files, seenPaths, ROBOTS_PATH, content, ROBOTS_CONTENT_TYPE, "robots");
  }
  if (config.manifestJson !== undefined) {
    const content = validateString(config.manifestJson, "manifestJson");
    addSynthetic(files, seenPaths, WEB_APP_MANIFEST_PATH, content, WEB_APP_MANIFEST_CONTENT_TYPE, "web-app-manifest");
  }

  // 3. Extra files — caller-supplied, must be validated.
  if (config.extraFiles !== undefined) {
    if (!Array.isArray(config.extraFiles)) {
      throw new TypeError(
        `forme-aot-deploy-manifest-emitter: extraFiles must be an array; got ${typeof config.extraFiles}`,
      );
    }
    for (let i = 0; i < config.extraFiles.length; i++) {
      const f = config.extraFiles[i];
      if (f === null || typeof f !== "object") {
        throw new TypeError(
          `forme-aot-deploy-manifest-emitter: extraFiles[${i}] must be a non-null object; got ${typeof f}`,
        );
      }
      const entry = validateExtraFile(f, i);
      if (seenPaths.has(entry.outputPath)) {
        throw new TypeError(
          `forme-aot-deploy-manifest-emitter: extraFiles[${i}].outputPath ${JSON.stringify(entry.outputPath)} duplicates an earlier entry`,
        );
      }
      seenPaths.add(entry.outputPath);
      files.push(entry);
    }
  }

  // Sort by outputPath, lexicographic.
  files.sort((a, b) => (a.outputPath < b.outputPath ? -1 : a.outputPath > b.outputPath ? 1 : 0));

  // Build the canonical object.  `Object.create(null)` so a
  // crafted outputPath of literally `"__proto__"` (already
  // rejected at validation time, but defense-in-depth) would
  // land as an own property here rather than mutating the
  // prototype chain.
  const filesObj: Record<string, Record<string, unknown>> = Object.create(null);
  let totalSizeBytes = 0;
  for (const entry of files) {
    const inner: Record<string, unknown> = {
      outputPath: entry.outputPath,
      contentType: entry.contentType,
      sizeBytes: entry.sizeBytes,
      sha256: entry.sha256,
      source: entry.source,
    };
    if (entry.route !== undefined) inner.route = entry.route;
    if (entry.lastmod !== undefined) inner.lastmod = entry.lastmod;
    filesObj[entry.outputPath] = inner;
    totalSizeBytes += entry.sizeBytes;
  }

  const top: Record<string, unknown> = { version: 1 };
  if (bundle.baseUrl !== undefined) top.baseUrl = bundle.baseUrl;
  top.fileCount = files.length;
  top.totalSizeBytes = totalSizeBytes;
  top.files = filesObj;

  return `${JSON.stringify(top, null, 2)}\n`;
}

function addSynthetic(
  files: DeployFileEntry[],
  seen: Set<string>,
  path: string,
  content: string,
  contentType: string,
  source: "sitemap" | "robots" | "web-app-manifest",
): void {
  if (seen.has(path)) {
    throw new TypeError(
      `forme-aot-deploy-manifest-emitter: ${source} output path ${JSON.stringify(path)} collides with a page-bundle route`,
    );
  }
  seen.add(path);
  files.push({
    outputPath: path,
    contentType,
    sizeBytes: utf8ByteLength(content),
    sha256: sha256Base64(content),
    source,
  });
}

function validateExtraFile(f: ExtraFile, i: number): DeployFileEntry {
  const outputPath = validateOutputPath(f.outputPath, `extraFiles[${i}].outputPath`);
  const content = validateString(f.content, `extraFiles[${i}].content`);
  const contentType = validateString(f.contentType, `extraFiles[${i}].contentType`);
  let lastmod: string | undefined;
  if (f.lastmod !== undefined) {
    lastmod = validateString(f.lastmod, `extraFiles[${i}].lastmod`);
  }
  return {
    outputPath,
    contentType,
    sizeBytes: utf8ByteLength(content),
    sha256: sha256Base64(content),
    source: "extra",
    lastmod,
  };
}
