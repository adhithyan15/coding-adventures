/** Emit rendered pages and referenced assets as one deterministic static site. */

import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, posix, relative, resolve, sep } from "node:path";
import {
  Kinds,
  streamOf,
  type Asset,
  type DeployArtifact,
  type DeployAssetEntry,
  type DeployRoute,
  type JsonValue,
  type LogicalId,
  type RenderedPage,
} from "@coding-adventures/forme-types";
import { computeRevisionId } from "@coding-adventures/forme-identity";
import { defineStage } from "@coding-adventures/forme-stage";

export interface EmitSiteFsConfig {
  /** Directory under which the complete static site is written. */
  readonly outDir: string;
  /** Portable artifact-relative asset directory. Defaults to "assets". */
  readonly assetDir?: string;
  /** Root-relative deployment prefix, for example "/coding-adventures". */
  readonly publicPathPrefix?: string;
}

interface PlannedAsset {
  readonly entry: DeployAssetEntry;
  readonly bytes: Uint8Array;
  readonly publicPath: string;
}

const encoder = new TextEncoder();
const PLACEHOLDER_PREFIX = "forme-asset:";

/** SHA-256 hex used by both filenames and DeployAssetEntry. */
export function sha256Hex(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

/** Build a content-fingerprinted filename while retaining the source suffix. */
export function fingerprintedAssetFilename(sourcePath: string, sha256: string): string {
  validatePortableSourcePath(sourcePath);
  if (!/^[0-9a-f]{64}$/.test(sha256)) {
    throw new Error("forme-emit-site-fs: asset sha256 must be 64 lowercase hexadecimal characters");
  }
  const sourceName = posix.basename(sourcePath);
  const extension = posix.extname(sourceName);
  const stem = extension.length === 0 ? sourceName : sourceName.slice(0, -extension.length);
  return `${stem}.${sha256}${extension}`;
}

/**
 * Replace only renderer-owned Forme placeholders declared by usedAssets.
 * Query strings and fragments remain after the replaced prefix verbatim.
 */
export function rewriteAssetPlaceholders(
  page: RenderedPage,
  publicPathById: ReadonlyMap<LogicalId, string>,
): string {
  let html = page.html;
  const seen = new Set<LogicalId>();
  for (const id of page.usedAssets) {
    if (seen.has(id)) continue;
    seen.add(id);
    const publicPath = publicPathById.get(id);
    if (publicPath === undefined) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} references missing asset ${JSON.stringify(id)}`,
      );
    }
    html = html.split(`${PLACEHOLDER_PREFIX}${encodeURIComponent(id)}`).join(publicPath);
  }
  if (html.includes(PLACEHOLDER_PREFIX)) {
    throw new Error(
      `forme-emit-site-fs: page ${JSON.stringify(page.route)} contains an undeclared or malformed Forme asset placeholder`,
    );
  }
  return html;
}

const emitSiteFs = defineStage({
  name: "@coding-adventures/forme-emit-site-fs",
  version: "0.1.0",
  apiVersion: 1,
  description: "Join rendered pages with Asset IR and emit a fingerprinted static site.",
  consumes: streamOf(Kinds.RenderedPage),
  inputPorts: { assets: streamOf(Kinds.Asset) },
  produces: Kinds.DeployArtifact,
  capabilities: ["filesystem:write"],
  configSchema: {
    type: "object",
    required: ["outDir"],
    properties: {
      outDir: { type: "string" },
      assetDir: { type: "string" },
      publicPathPrefix: { type: "string" },
    },
  },
  async run(input, rawConfig, ctx) {
    const config = rawConfig as EmitSiteFsConfig;
    if (typeof config?.outDir !== "string" || config.outDir.length === 0) {
      throw new Error("forme-emit-site-fs: config.outDir must be a non-empty string");
    }
    const assetDir = config.assetDir ?? "assets";
    validateAssetDir(assetDir);
    const publicPathPrefix = config.publicPathPrefix ?? "";
    validatePublicPathPrefix(publicPathPrefix);
    const assetStream = input.assets as AsyncIterable<Asset>;
    const pageStream = input.default as AsyncIterable<RenderedPage>;

    const assetsById = new Map<LogicalId, PlannedAsset>();
    const files = new Map<string, Uint8Array>();
    for await (const asset of assetStream) {
      ctx.cancellation.throwIfCancelled();
      const planned = planAsset(asset, assetDir, publicPathPrefix);
      if (assetsById.has(asset.identity)) {
        throw new Error(
          `forme-emit-site-fs: duplicate asset identity ${JSON.stringify(asset.identity)}`,
        );
      }
      const existing = files.get(planned.entry.path);
      if (existing !== undefined && !sameBytes(existing, planned.bytes)) {
        throw new Error(
          `forme-emit-site-fs: fingerprint path collision at ${JSON.stringify(planned.entry.path)}`,
        );
      }
      assetsById.set(asset.identity, planned);
      if (existing === undefined) files.set(planned.entry.path, planned.bytes);
    }

    const publicPathById = new Map(
      [...assetsById].map(([id, planned]) => [id, planned.publicPath] as const),
    );
    const routes: DeployRoute[] = [];
    let pageCount = 0;
    for await (const page of pageStream) {
      ctx.cancellation.throwIfCancelled();
      const html = rewriteAssetPlaceholders(page, publicPathById);
      const bytes = encoder.encode(html);
      const path = routeToArtifactPath(config.outDir, page.route);
      if (files.has(path)) {
        throw new Error(
          `forme-emit-site-fs: page route ${JSON.stringify(page.route)} collides with output ${JSON.stringify(path)}`,
        );
      }
      files.set(path, bytes);
      routes.push({
        pattern: page.route,
        target: { kind: "file", path },
        islands: page.usedIslands,
        css: [],
      });
      pageCount += 1;
    }

    const orderedFiles = [...files].sort(([left], [right]) => compareCodeUnits(left, right));
    for (const [path, bytes] of orderedFiles) {
      ctx.cancellation.throwIfCancelled();
      const absolutePath = resolve(config.outDir, ...path.split("/"));
      await mkdir(dirname(absolutePath), { recursive: true });
      await writeFile(absolutePath, bytes);
    }

    const fileHashes: Record<string, string> = {};
    const fileRecord: Record<string, Uint8Array> = {};
    for (const [path, bytes] of orderedFiles) {
      fileHashes[path] = sha256Hex(bytes);
      fileRecord[path] = new Uint8Array(bytes);
    }
    const buildId = computeRevisionId({ files: fileHashes } as JsonValue);
    const assets = [...assetsById.values()]
      .map(planned => planned.entry)
      .sort((left, right) => compareCodeUnits(left.path, right.path) || compareCodeUnits(left.id, right.id));
    const artifact: DeployArtifact = {
      variant: { kind: "dist-tree" },
      files: fileRecord,
      manifest: {
        routes,
        assets,
        buildTime: ctx.time.nowIso(),
        buildId,
      },
    };
    ctx.logger.info("forme-emit-site-fs: wrote static site", {
      pages: pageCount,
      assets: assets.length,
      files: orderedFiles.length,
      outDir: config.outDir,
      buildId,
    });
    return artifact;
  },
});

function planAsset(asset: Asset, assetDir: string, publicPathPrefix: string): PlannedAsset {
  if (!(asset.bytes instanceof Uint8Array) || asset.byteLength !== asset.bytes.byteLength) {
    throw new Error(
      `forme-emit-site-fs: asset ${JSON.stringify(asset.identity)} has inconsistent bytes and byteLength`,
    );
  }
  if (typeof asset.mimeType !== "string" || asset.mimeType.length === 0) {
    throw new Error(`forme-emit-site-fs: asset ${JSON.stringify(asset.identity)} has no MIME type`);
  }
  const sourcePath = asset.meta.sourcePath;
  if (typeof sourcePath !== "string") {
    throw new Error(
      `forme-emit-site-fs: asset ${JSON.stringify(asset.identity)} has no meta.sourcePath from the filesystem loader`,
    );
  }
  const bytes = new Uint8Array(asset.bytes);
  const sha256 = sha256Hex(bytes);
  const filename = fingerprintedAssetFilename(sourcePath, sha256);
  const path = `${assetDir}/${filename}`;
  const encodedPrefix = publicPathPrefix.length === 0
    ? ""
    : `/${publicPathPrefix.slice(1).split("/").map(segment => encodeURIComponent(segment)).join("/")}`;
  const encodedPath = path.split("/").map(segment => encodeURIComponent(segment)).join("/");
  const publicPath = `${encodedPrefix}/${encodedPath}`;
  return {
    bytes,
    publicPath,
    entry: { id: asset.identity, path, mime: asset.mimeType, sha256 },
  };
}

function validateAssetDir(assetDir: string): void {
  if (
    typeof assetDir !== "string" || assetDir.length === 0 ||
    assetDir.includes("\\") || assetDir.includes("\0") ||
    assetDir.includes("?") || assetDir.includes("#") ||
    hasWindowsDrivePrefix(assetDir) || posix.isAbsolute(assetDir) ||
    posix.normalize(assetDir) !== assetDir ||
    assetDir.split("/").some(segment => segment.length === 0 || segment === "." || segment === "..")
  ) {
    throw new Error("forme-emit-site-fs: config.assetDir must be a normalized portable relative path");
  }
}

function validatePublicPathPrefix(prefix: string): void {
  if (prefix.length === 0) return;
  const relativePrefix = prefix.slice(1);
  if (
    !prefix.startsWith("/") || prefix.startsWith("//") || prefix.endsWith("/") ||
    prefix.includes("\\") || prefix.includes("\0") || prefix.includes("?") || prefix.includes("#") ||
    posix.normalize(prefix) !== prefix ||
    relativePrefix.split("/").some(segment => segment.length === 0 || segment === "." || segment === "..")
  ) {
    throw new Error(
      "forme-emit-site-fs: config.publicPathPrefix must be empty or a normalized root-relative URL path",
    );
  }
}

function validatePortableSourcePath(sourcePath: string): void {
  if (
    sourcePath.length === 0 || sourcePath.includes("\\") || sourcePath.includes("\0") ||
    hasWindowsDrivePrefix(sourcePath) || posix.isAbsolute(sourcePath) ||
    posix.normalize(sourcePath) !== sourcePath ||
    sourcePath.split("/").some(segment => segment.length === 0 || segment === "." || segment === "..")
  ) {
    throw new Error(
      `forme-emit-site-fs: asset sourcePath ${JSON.stringify(sourcePath)} is not a normalized portable path`,
    );
  }
}

function routeToArtifactPath(outDir: string, route: string): string {
  if (typeof route !== "string" || route.length === 0) {
    throw new Error("forme-emit-site-fs: empty route is not a valid output path");
  }
  const relativeRoute = route.startsWith("/") ? route.slice(1) : route;
  if (relativeRoute.length === 0) {
    throw new Error('forme-emit-site-fs: route "/" has no filename component');
  }
  if (relativeRoute.startsWith("/")) {
    throw new Error(`forme-emit-site-fs: route ${JSON.stringify(route)} starts with multiple slashes`);
  }
  const absoluteRoot = resolve(outDir);
  const absolutePath = resolve(absoluteRoot, relativeRoute);
  const guard = absoluteRoot.endsWith(sep) ? absoluteRoot : `${absoluteRoot}${sep}`;
  if (!absolutePath.startsWith(guard)) {
    throw new Error(`forme-emit-site-fs: route ${JSON.stringify(route)} would escape outDir`);
  }
  return relative(absoluteRoot, absolutePath).split(sep).join("/");
}

function sameBytes(left: Uint8Array, right: Uint8Array): boolean {
  return left.byteLength === right.byteLength && left.every((value, index) => value === right[index]);
}

function hasWindowsDrivePrefix(value: string): boolean {
  if (value.length < 2 || value[1] !== ":") return false;
  const first = value.charCodeAt(0);
  return (first >= 65 && first <= 90) || (first >= 97 && first <= 122);
}

function compareCodeUnits(left: string, right: string): number {
  return left < right ? -1 : left > right ? 1 : 0;
}

export default emitSiteFs;
export { emitSiteFs };
