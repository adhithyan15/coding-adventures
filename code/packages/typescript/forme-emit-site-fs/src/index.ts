/** Emit rendered pages and referenced assets as one deterministic static site. */

import { createHash, randomUUID } from "node:crypto";
import { lstat, mkdir, realpath, rename, unlink, writeFile } from "node:fs/promises";
import { isAbsolute, posix, relative, resolve, sep } from "node:path";
import { types as utilTypes } from "node:util";
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
import { computeRevisionId, isLogicalIdShape } from "@coding-adventures/forme-identity";
import { defineStage, type StageContext } from "@coding-adventures/forme-stage";

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
  readonly role: Asset["role"];
}

interface PageSnapshot {
  readonly route: string;
  readonly html: string;
  readonly usedAssets: readonly LogicalId[];
  readonly usedIslands: readonly RenderedPage["usedIslands"][number][];
  readonly islandModules: readonly NonNullable<RenderedPage["islandModules"]>[number][];
}

const encoder = new TextEncoder();
const PLACEHOLDER_PREFIX = "forme-asset:";

function validateConfig(rawConfig: unknown): Required<Pick<EmitSiteFsConfig, "outDir">> & EmitSiteFsConfig {
  const config = rawConfig as EmitSiteFsConfig;
  if (typeof config?.outDir !== "string" || config.outDir.length === 0) {
    throw new Error("forme-emit-site-fs: config.outDir must be a non-empty string");
  }
  return config;
}

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
  return rewriteAssetPlaceholdersFromSnapshot(snapshotPage(page), publicPathById);
}

function rewriteAssetPlaceholdersFromSnapshot(
  page: PageSnapshot,
  publicPathById: ReadonlyMap<LogicalId, string>,
): string {
  const seen = new Set<LogicalId>();
  const replacements = new Map<string, string>();
  for (const id of page.usedAssets) {
    if (seen.has(id)) continue;
    seen.add(id);
    const publicPath = publicPathById.get(id);
    if (publicPath === undefined) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} references missing asset ${JSON.stringify(id)}`,
      );
    }
    replacements.set(id, publicPath);
  }
  const html = page.html.replace(
    /forme-asset:([0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12})/g,
    (match, id: string) => replacements.get(id) ?? match,
  );
  if (html.includes(PLACEHOLDER_PREFIX)) {
    throw new Error(
      `forme-emit-site-fs: page ${JSON.stringify(page.route)} contains an undeclared or malformed Forme asset placeholder`,
    );
  }
  return html;
}

const emitSiteFs = defineStage({
  name: "@coding-adventures/forme-emit-site-fs",
  version: "0.2.0",
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
    const config = validateConfig(rawConfig);
    const assetDir = config.assetDir ?? "assets";
    validateAssetDir(assetDir);
    const publicPathPrefix = config.publicPathPrefix ?? "";
    validatePublicPathPrefix(publicPathPrefix);
    const assetStream = input.assets as AsyncIterable<Asset>;
    const pageStream = input.default as AsyncIterable<RenderedPage>;

    const assetsById = new Map<LogicalId, PlannedAsset>();
    const files = new Map<string, Uint8Array>();
    const assetsByPath = new Map<string, PlannedAsset>();
    const outputPaths = new Map<string, string>();
    const assetIterator = assetStream[Symbol.asyncIterator]();
    const pageIterator = pageStream[Symbol.asyncIterator]();
    const collectAssets = (async () => {
      for await (const asset of { [Symbol.asyncIterator]: () => assetIterator }) {
        ctx.cancellation.throwIfCancelled();
        const planned = planAsset(asset, assetDir, publicPathPrefix);
        if (assetsById.has(asset.identity)) {
          throw new Error(
            `forme-emit-site-fs: duplicate asset identity ${JSON.stringify(asset.identity)}`,
          );
        }
        const existingAsset = assetsByPath.get(planned.entry.path);
        const collisionKey = portableCollisionKey(planned.entry.path);
        const existingPortablePath = outputPaths.get(collisionKey);
        if (existingPortablePath !== undefined && existingPortablePath !== planned.entry.path) {
          throw new Error(
            `forme-emit-site-fs: portable asset path collision between ${JSON.stringify(existingPortablePath)} and ${JSON.stringify(planned.entry.path)}`,
          );
        }
        if (existingAsset !== undefined && (
          !sameBytes(existingAsset.bytes, planned.bytes) ||
          existingAsset.role !== planned.role ||
          existingAsset.entry.mime !== planned.entry.mime
        )) {
          throw new Error(`forme-emit-site-fs: incompatible asset path collision at ${JSON.stringify(planned.entry.path)}`);
        }
        assetsById.set(asset.identity, planned);
        if (existingAsset === undefined) {
          assetsByPath.set(planned.entry.path, planned);
          outputPaths.set(collisionKey, planned.entry.path);
        }
        if (asset.role !== "script" && !files.has(planned.entry.path)) files.set(planned.entry.path, planned.bytes);
      }
    })();
    const collectPages = (async () => {
      const pages: PageSnapshot[] = [];
      for await (const page of { [Symbol.asyncIterator]: () => pageIterator }) {
        ctx.cancellation.throwIfCancelled();
        if (pages.length >= MAX_SITE_PAGES) {
          throw new Error(`forme-emit-site-fs: site exceeds the ${MAX_SITE_PAGES}-page safety limit`);
        }
        pages.push(snapshotPage(page));
      }
      return pages;
    })();
    let pageSnapshots: PageSnapshot[];
    try {
      [, pageSnapshots] = await Promise.all([collectAssets, collectPages]);
    } catch (error) {
      try { await assetIterator.return?.(); } catch { /* preserve the collection failure */ }
      try { await pageIterator.return?.(); } catch { /* preserve the collection failure */ }
      throw error;
    }

    const publicPathById = new Map(
      [...assetsById].map(([id, planned]) => [id, planned.publicPath] as const),
    );
    const routes: DeployRoute[] = [];
    const selectedScriptAssets = new Set<LogicalId>();
    let pageCount = 0;
    for (const snapshot of pageSnapshots) {
      ctx.cancellation.throwIfCancelled();
      const rewrittenHtml = rewriteAssetPlaceholdersFromSnapshot(snapshot, publicPathById);
      const moduleTags = islandModuleTags(snapshot, assetsById, selectedScriptAssets);
      const html = appendModuleTags(rewrittenHtml, moduleTags, snapshot.route);
      const bytes = encoder.encode(html);
      const path = routeToArtifactPath(config.outDir, snapshot.route);
      const collisionKey = portableCollisionKey(path);
      const existingPath = outputPaths.get(collisionKey);
      if (files.has(path) || assetsByPath.has(path) || existingPath !== undefined) {
        throw new Error(
          `forme-emit-site-fs: page route ${JSON.stringify(snapshot.route)} collides with output ${JSON.stringify(existingPath ?? path)}`,
        );
      }
      outputPaths.set(collisionKey, path);
      files.set(path, bytes);
      routes.push({
        pattern: snapshot.route,
        target: { kind: "file", path },
        islands: [...snapshot.usedIslands],
        css: [],
      });
      pageCount += 1;
    }

    for (const id of selectedScriptAssets) {
      const planned = assetsById.get(id)!;
      const existing = files.get(planned.entry.path);
      if (existing !== undefined && !sameBytes(existing, planned.bytes)) {
        throw new Error(
          `forme-emit-site-fs: fingerprint path collision at ${JSON.stringify(planned.entry.path)}`,
        );
      }
      if (existing === undefined) files.set(planned.entry.path, planned.bytes);
    }

    const orderedFiles = [...files].sort(([left], [right]) => compareCodeUnits(left, right));
    for (const [path, bytes] of orderedFiles) {
      ctx.cancellation.throwIfCancelled();
      await writeContainedFile(config.outDir, validatedArtifactPath(path), bytes);
    }

    const fileHashes: Record<string, string> = Object.create(null);
    const fileRecord: Record<string, Uint8Array> = Object.create(null);
    for (const [path, bytes] of orderedFiles) {
      fileHashes[path] = sha256Hex(bytes);
      fileRecord[path] = new Uint8Array(bytes);
    }
    const buildId = computeRevisionId({ files: fileHashes } as JsonValue);
    const assets = [...assetsById.entries()]
      .filter(([id, planned]) => planned.role !== "script" || selectedScriptAssets.has(id))
      .map(([, planned]) => planned)
      .map(planned => planned.entry)
      .sort((left, right) => compareCodeUnits(left.path, right.path) || compareCodeUnits(left.id, right.id));
    const artifact: DeployArtifact = {
      variant: { kind: "dist-tree" },
      files: fileRecord,
      manifest: {
        routes,
        assets,
        buildTime: await ctx.time.nowIso(),
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
  async replay(artifact, rawConfig, ctx) {
    const config = validateConfig(rawConfig);
    const count = await materializeArtifact(artifact, config.outDir, ctx);
    ctx.logger.info("forme-emit-site-fs: replayed static site", {
      files: count,
      outDir: config.outDir,
      buildId: artifact.manifest.buildId,
    });
  },
});

async function materializeArtifact(
  artifact: DeployArtifact,
  outDir: string,
  ctx: StageContext,
): Promise<number> {
  if (artifact?.variant?.kind !== "dist-tree" || !isPlainObject(artifact.files)) {
    throw new Error("forme-emit-site-fs: replay requires a dist-tree DeployArtifact");
  }
  const collisionPaths = new Map<string, string>();
  const files = Object.entries(artifact.files)
    .map(([path, bytes]) => {
      if (!(bytes instanceof Uint8Array)) {
        throw new Error(`forme-emit-site-fs: replay file ${JSON.stringify(path)} is not bytes`);
      }
      const portablePath = validatedArtifactPath(path);
      const collisionKey = portableCollisionKey(portablePath);
      const existing = collisionPaths.get(collisionKey);
      if (existing !== undefined) {
        throw new Error(
          `forme-emit-site-fs: replay paths ${JSON.stringify(existing)} and ${JSON.stringify(portablePath)} collide on portable filesystems`,
        );
      }
      collisionPaths.set(collisionKey, portablePath);
      return {
        path: portablePath,
        bytes,
      };
    })
    .sort((left, right) => compareCodeUnits(left.path, right.path));
  for (const { path, bytes } of files) {
    ctx.cancellation.throwIfCancelled();
    await writeContainedFile(outDir, path, bytes);
  }
  return files.length;
}

async function writeContainedFile(
  outDir: string,
  portablePath: string,
  bytes: Uint8Array,
): Promise<void> {
  // A portable relative path may still traverse outside outDir through a
  // pre-existing symlink. Verify each parent both lexically and canonically,
  // then publish an exclusive sibling temp file by rename. This replaces a
  // final symlink or hard link rather than following it and keeps partial file
  // bytes invisible if writing fails.
  const root = resolve(outDir);
  await mkdir(root, { recursive: true });
  await requireRealDirectory(root, "output root");
  const canonicalRoot = await realpath(root);
  const segments = portablePath.split("/");
  let parent = root;
  for (const segment of segments.slice(0, -1)) {
    parent = resolve(parent, segment);
    requireLexicalContainment(root, parent);
    try {
      await mkdir(parent);
    } catch (error) {
      if (!isAlreadyExists(error)) throw error;
    }
    await requireRealDirectory(parent, "output path component");
    requireCanonicalContainment(canonicalRoot, await realpath(parent));
  }

  const absolutePath = resolve(parent, segments.at(-1)!);
  await rejectExistingLinkOrNonFile(absolutePath);
  requireCanonicalContainment(canonicalRoot, await realpath(parent));
  const temporaryPath = resolve(parent, `.forme-${randomUUID()}.tmp`);
  try {
    await writeFile(temporaryPath, bytes, { flag: "wx" });
    await rename(temporaryPath, absolutePath);
  } finally {
    try {
      await unlink(temporaryPath);
    } catch (error) {
      if (!isNotFound(error)) throw error;
    }
  }
}

async function requireRealDirectory(path: string, label: string): Promise<void> {
  const stats = await lstat(path);
  if (stats.isSymbolicLink() || !stats.isDirectory()) {
    throw new Error(`forme-emit-site-fs: ${label} ${JSON.stringify(path)} must be a real directory`);
  }
}

async function rejectExistingLinkOrNonFile(path: string): Promise<void> {
  try {
    const stats = await lstat(path);
    if (stats.isSymbolicLink() || !stats.isFile()) {
      throw new Error(
        `forme-emit-site-fs: output file ${JSON.stringify(path)} must not be a symbolic link or directory`,
      );
    }
  } catch (error) {
    if (!isNotFound(error)) throw error;
  }
}

function requireCanonicalContainment(root: string, candidate: string): void {
  const relativePath = relative(root, candidate);
  if (
    relativePath === "" ||
    (relativePath !== ".." && !relativePath.startsWith(`..${sep}`) && !isAbsolute(relativePath))
  ) {
    return;
  }
  throw new Error("forme-emit-site-fs: resolved output path would escape outDir");
}

function isAlreadyExists(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error
    && (error as { code?: unknown }).code === "EEXIST";
}

function isNotFound(error: unknown): boolean {
  return typeof error === "object" && error !== null && "code" in error
    && (error as { code?: unknown }).code === "ENOENT";
}

function validatedArtifactPath(path: string): string {
  if (
    path.length === 0 || path.includes("\\") || path.includes("\0") || path.includes(":") ||
    hasWindowsDrivePrefix(path) || posix.isAbsolute(path) ||
    posix.normalize(path) !== path ||
    path.split("/").some(segment => segment.length === 0 || segment === "." || segment === "..")
  ) {
    throw new Error(
      `forme-emit-site-fs: replay path ${JSON.stringify(path)} is not a normalized portable relative path`,
    );
  }
  validateWindowsPortableSegments(path, "replay path");
  return path;
}

function requireLexicalContainment(root: string, candidate: string): void {
  const relativePath = relative(root, candidate);
  if (
    relativePath === "" ||
    (relativePath !== ".." && !relativePath.startsWith(`..${sep}`) && !isAbsolute(relativePath))
  ) {
    return;
  }
  throw new Error("forme-emit-site-fs: output path would escape outDir");
}

function isPlainObject(value: unknown): value is Readonly<Record<string, Uint8Array>> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
}

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
    role: asset.role,
  };
}

function islandModuleTags(
  page: PageSnapshot,
  assetsById: ReadonlyMap<LogicalId, PlannedAsset>,
  selected: Set<LogicalId>,
): string {
  const uses = page.islandModules;
  if (uses.length !== page.usedIslands.length) {
    throw new Error(
      `forme-emit-site-fs: page ${JSON.stringify(page.route)} islandModules must match usedIslands exactly`,
    );
  }
  const seenIslands = new Set<string>();
  const taggedAssets = new Set<LogicalId>();
  const usedAssets = new Set(page.usedAssets);
  const tags: string[] = [];
  for (let index = 0; index < page.usedIslands.length; index++) {
    const island = page.usedIslands[index]!;
    const use = uses[index];
    if (use === undefined || use.island !== island || seenIslands.has(island)) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} islandModules must match usedIslands exactly`,
      );
    }
    seenIslands.add(island);
    if (!usedAssets.has(use.asset)) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} island ${JSON.stringify(island)} module asset is absent from usedAssets`,
      );
    }
    const planned = assetsById.get(use.asset);
    if (planned === undefined) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} references missing island module asset ${JSON.stringify(use.asset)}`,
      );
    }
    if (planned.role !== "script" || !isJavaScriptMime(planned.entry.mime)) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} island ${JSON.stringify(island)} must reference a script asset with a JavaScript MIME type`,
      );
    }
    if (planned.entry.sha256 !== use.sha256) {
      throw new Error(
        `forme-emit-site-fs: page ${JSON.stringify(page.route)} island ${JSON.stringify(island)} executable bytes do not match the reviewed SHA-256`,
      );
    }
    selected.add(use.asset);
    if (!taggedAssets.has(use.asset)) {
      taggedAssets.add(use.asset);
      tags.push(`<script type="module" src="${planned.publicPath}"></script>`);
    }
  }
  return tags.join("\n");
}

const MAX_PAGE_ISLANDS = 256;
const MAX_PAGE_ASSETS = 65_536;
const MAX_SITE_PAGES = 65_536;
const PACKAGE_NAME = /^(?:@[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*|[a-z0-9][a-z0-9._-]*)$/;
const EXPORT_NAME = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;
const SHA256 = /^[0-9a-f]{64}$/;

function snapshotPage(page: RenderedPage): PageSnapshot {
  if (typeof page !== "object" || page === null || utilTypes.isProxy(page)) {
    throw new TypeError("forme-emit-site-fs: page must be a non-proxy object");
  }
  const route = dataProperty(page, "route");
  const html = dataProperty(page, "html");
  if (typeof route !== "string" || route.length === 0 || route.length > 2_048) {
    throw new TypeError("forme-emit-site-fs: page.route must be a non-empty string of at most 2048 characters");
  }
  if (typeof html !== "string") throw new TypeError("forme-emit-site-fs: page.html must be a string");
  const usedAssets = logicalIdArray(dataProperty(page, "usedAssets"), "page.usedAssets", MAX_PAGE_ASSETS);
  const usedIslands = islandIdArray(dataProperty(page, "usedIslands"), "page.usedIslands");
  const rawModules = dataProperty(page, "islandModules", false);
  const islandModules = moduleUseArray(rawModules === undefined ? [] : rawModules);
  return Object.freeze({ route, html, usedAssets, usedIslands, islandModules });
}

function dataProperty(value: object, key: string, required = true): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  if (descriptor === undefined) {
    if (!required) return undefined;
    throw new TypeError(`forme-emit-site-fs: page.${key} is required`);
  }
  if (!("value" in descriptor)) throw new TypeError(`forme-emit-site-fs: page.${key} must not be an accessor`);
  return descriptor.value;
}

function exactArray(value: unknown, path: string, maximum: number): readonly unknown[] {
  if (!Array.isArray(value) || utilTypes.isProxy(value) || value.length > maximum) {
    throw new TypeError(`forme-emit-site-fs: ${path} must be an array of at most ${maximum} entries`);
  }
  if (Object.getOwnPropertySymbols(value).length !== 0) {
    throw new TypeError(`forme-emit-site-fs: ${path} must not contain symbol keys`);
  }
  const copy: unknown[] = [];
  for (let index = 0; index < value.length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (descriptor === undefined) throw new TypeError(`forme-emit-site-fs: ${path}[${index}] is sparse`);
    if (!("value" in descriptor)) throw new TypeError(`forme-emit-site-fs: ${path}[${index}] must not be an accessor`);
    copy.push(descriptor.value);
  }
  for (const key of Object.getOwnPropertyNames(value)) {
    if (key === "length") continue;
    const index = Number(key);
    if (!Number.isInteger(index) || index < 0 || index >= value.length || String(index) !== key) {
      throw new TypeError(`forme-emit-site-fs: ${path} has an unknown property`);
    }
  }
  return copy;
}

function logicalIdArray(value: unknown, path: string, maximum: number): readonly LogicalId[] {
  return Object.freeze(exactArray(value, path, maximum).map((entry, index) => {
    if (typeof entry !== "string" || !isLogicalIdShape(entry)) {
      throw new TypeError(`forme-emit-site-fs: ${path}[${index}] must be a lowercase UUIDv7 LogicalId`);
    }
    return entry as LogicalId;
  }));
}

function islandIdArray(value: unknown, path: string): readonly RenderedPage["usedIslands"][number][] {
  const seen = new Set<string>();
  return Object.freeze(exactArray(value, path, MAX_PAGE_ISLANDS).map((entry, index) => {
    if (typeof entry !== "string" || !/^[A-Za-z_][A-Za-z0-9_-]{0,63}$/.test(entry) || seen.has(entry)) {
      throw new TypeError(`forme-emit-site-fs: ${path}[${index}] must be a unique IslandId`);
    }
    seen.add(entry);
    return entry as RenderedPage["usedIslands"][number];
  }));
}

function moduleUseArray(value: unknown): readonly NonNullable<RenderedPage["islandModules"]>[number][] {
  return Object.freeze(exactArray(value, "page.islandModules", MAX_PAGE_ISLANDS).map((entry, index) => {
    const path = `page.islandModules[${index}]`;
    if (typeof entry !== "object" || entry === null || Array.isArray(entry) || utilTypes.isProxy(entry)) {
      throw new TypeError(`forme-emit-site-fs: ${path} must be a plain object`);
    }
    const prototype = Object.getPrototypeOf(entry);
    if (prototype !== Object.prototype && prototype !== null) {
      throw new TypeError(`forme-emit-site-fs: ${path} must be a plain object`);
    }
    const allowed = new Set(["island", "asset", "packageName", "export", "sha256"]);
    if (Object.getOwnPropertySymbols(entry).length !== 0 || Object.getOwnPropertyNames(entry).some(key => !allowed.has(key))) {
      throw new TypeError(`forme-emit-site-fs: ${path} contains unknown keys`);
    }
    const island = dataField(entry, "island", path);
    const asset = dataField(entry, "asset", path);
    const packageName = dataField(entry, "packageName", path);
    const exportName = dataField(entry, "export", path);
    const sha256 = dataField(entry, "sha256", path);
    if (typeof island !== "string" || !/^[A-Za-z_][A-Za-z0-9_-]{0,63}$/.test(island) ||
        typeof asset !== "string" || !isLogicalIdShape(asset) ||
        typeof packageName !== "string" || !PACKAGE_NAME.test(packageName) ||
        typeof exportName !== "string" || !EXPORT_NAME.test(exportName) ||
        typeof sha256 !== "string" || !SHA256.test(sha256)) {
      throw new TypeError(`forme-emit-site-fs: ${path} is not a reviewed island-module binding`);
    }
    return Object.freeze({ island, asset, packageName, export: exportName, sha256 }) as NonNullable<RenderedPage["islandModules"]>[number];
  }));
}

function dataField(value: object, key: string, path: string): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  if (descriptor === undefined || !("value" in descriptor)) {
    throw new TypeError(`forme-emit-site-fs: ${path}.${key} must be a data property`);
  }
  return descriptor.value;
}

function appendModuleTags(html: string, tags: string, route: string): string {
  if (tags.length === 0) return html;
  const bodyClose = html.lastIndexOf("</body>");
  if (bodyClose < 0) {
    throw new Error(
      `forme-emit-site-fs: interactive page ${JSON.stringify(route)} has no closing </body> tag`,
    );
  }
  return `${html.slice(0, bodyClose)}${tags}${html.slice(bodyClose)}`;
}

function isJavaScriptMime(mime: string): boolean {
  const essence = mime.split(";", 1)[0]!.trim().toLowerCase();
  return essence === "text/javascript" || essence === "application/javascript";
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
  validateWindowsPortableSegments(assetDir, "config.assetDir");
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
  validateWindowsPortableSegments(sourcePath, "asset sourcePath");
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
  const portablePath = relative(absoluteRoot, absolutePath).split(sep).join("/");
  validateWindowsPortableSegments(portablePath, "page route");
  return portablePath;
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

function portableCollisionKey(path: string): string {
  return path.normalize("NFC").toLowerCase();
}

function validateWindowsPortableSegments(path: string, label: string): void {
  const reserved = /^(?:con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i;
  for (const segment of path.split("/")) {
    if (
      segment.endsWith(".") || segment.endsWith(" ") ||
      /[\u0000-\u001f<>:"|?*]/.test(segment) || reserved.test(segment)
    ) {
      throw new Error(`forme-emit-site-fs: ${label} is not portable across filesystems`);
    }
  }
}

export default emitSiteFs;
export { emitSiteFs };
