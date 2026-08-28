/**
 * Resolve local Document AST image references into first-class AssetRef values.
 *
 * The stage owns reference discovery and storage-root containment, but not
 * asset-byte loading or emission. Those remain separate stages so each value
 * has one typed output and the pipeline can fan out annotated content to both
 * rendering and Asset IR loading.
 */

import { readFile } from "node:fs/promises";
import {
  basename,
  dirname,
  isAbsolute,
  relative,
  resolve,
  sep,
} from "node:path";
import { posix } from "node:path";
import type { DocumentNode, ImageNode } from "@coding-adventures/document-ast";
import {
  Kinds,
  streamOf,
  type AssetRef,
  type ContentNode,
  type JsonValue,
  type LogicalId,
} from "@coding-adventures/forme-types";
import {
  computeRevisionId,
  generateLogicalId,
  isLogicalIdShape,
} from "@coding-adventures/forme-identity";
import { defineStage } from "@coding-adventures/forme-stage";
import type { Logger } from "@coding-adventures/forme-stage";

export interface ResolveAssetRefsFsConfig {
  /** Storage root. Defaults to process.cwd(). */
  readonly root?: string;
  /** Read adjacent identity sidecars. Defaults to true. */
  readonly persistIdentities?: boolean;
}

export interface ResolvedAssetSource {
  /** Portable path relative to the configured storage root. */
  readonly sourcePath: string;
  /** Platform-native absolute path used only for sidecar lookup. */
  readonly absolutePath: string;
  /** Authored query string and/or fragment, including its leading delimiter. */
  readonly urlSuffix: string;
}

interface LocatedImage {
  readonly image: ImageNode;
  readonly nodePath: readonly number[];
}

const EXTERNAL_SCHEME = /^[A-Za-z][A-Za-z0-9+.-]*:/;

/** Whether an authored destination names a local filesystem asset. */
export function isLocalAssetDestination(destination: string): boolean {
  const value = destination.trim();
  if (value.length === 0 || value.startsWith("#") || value.startsWith("//")) {
    return false;
  }
  return !EXTERNAL_SCHEME.test(value);
}

/**
 * Resolve an authored asset URL beneath `root`, rejecting lexical traversal.
 * Query strings and fragments do not participate in filesystem identity.
 */
export function resolveAssetSource(
  root: string,
  contentSourcePath: string,
  destination: string,
): ResolvedAssetSource | null {
  if (!isLocalAssetDestination(destination)) return null;
  const suffixIndex = destination.search(/[?#]/);
  const urlPath = suffixIndex < 0 ? destination : destination.slice(0, suffixIndex);
  const urlSuffix = suffixIndex < 0 ? "" : destination.slice(suffixIndex);
  if (urlPath.length === 0) return null;

  let decoded: string;
  try {
    decoded = decodeURIComponent(urlPath).replaceAll("\\", "/");
  } catch {
    throw new Error(
      `forme-resolve-asset-refs-fs: malformed percent encoding in asset destination ${JSON.stringify(destination)}`,
    );
  }
  if (decoded.includes("\0")) {
    throw new Error("forme-resolve-asset-refs-fs: asset destinations cannot contain null bytes");
  }

  const normalizedContentPath = contentSourcePath.replaceAll("\\", "/");
  const candidate = decoded.startsWith("/")
    ? posix.normalize(decoded.replace(/^\/+/, ""))
    : posix.normalize(posix.join(posix.dirname(normalizedContentPath), decoded));
  if (candidate === ".." || candidate.startsWith("../") || posix.isAbsolute(candidate)) {
    throw new Error(
      `forme-resolve-asset-refs-fs: asset ${JSON.stringify(destination)} from ` +
      `${JSON.stringify(contentSourcePath)} escapes storage root`,
    );
  }

  const absoluteRoot = resolve(root);
  const absolutePath = resolve(absoluteRoot, ...candidate.split("/"));
  const relativePath = relative(absoluteRoot, absolutePath);
  if (relativePath === ".." || relativePath.startsWith(`..${sep}`) || isAbsolute(relativePath)) {
    throw new Error(
      `forme-resolve-asset-refs-fs: asset ${JSON.stringify(destination)} from ` +
      `${JSON.stringify(contentSourcePath)} escapes storage root`,
    );
  }
  const sourcePath = relativePath.split(sep).join("/");
  if (sourcePath.length === 0) {
    throw new Error("forme-resolve-asset-refs-fs: an asset reference cannot resolve to the storage root");
  }
  return { sourcePath, absolutePath, urlSuffix };
}

const resolveAssetRefsFs = defineStage({
  name: "@coding-adventures/forme-resolve-asset-refs-fs",
  version: "0.1.0",
  apiVersion: 1,
  description: "Resolve local image references beneath a storage root and attach AssetRef metadata.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.ContentNode),
  capabilities: ["storage:read"],
  configSchema: {
    type: "object",
    properties: {
      root: { type: "string" },
      persistIdentities: { type: "boolean" },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = (rawConfig ?? {}) as ResolveAssetRefsFsConfig;
    if (config.root !== undefined && (typeof config.root !== "string" || config.root.length === 0)) {
      throw new Error("forme-resolve-asset-refs-fs: config.root must be a non-empty string when provided");
    }
    const root = config.root ?? process.cwd();
    const persistIdentities = config.persistIdentities !== false;
    const stream = rawInput as AsyncIterable<ContentNode>;
    const identityBySource = new Map<string, LogicalId>();
    const sourceByIdentity = new Map<LogicalId, string>();
    let resolvedCount = 0;

    for await (const node of stream) {
      ctx.cancellation.throwIfCancelled();
      const preserved = node.assetRefs.filter(ref => ref.role !== "image");
      const refs: AssetRef[] = [...preserved];

      for (const located of locateImages(node.document)) {
        ctx.cancellation.throwIfCancelled();
        const source = resolveAssetSource(root, node.sourcePath, located.image.destination);
        if (source === null) continue;

        const existing = node.assetRefs.find(ref =>
          ref.role === "image" &&
          ref.sourcePath === source.sourcePath &&
          samePath(ref.nodePath, located.nodePath));
        const knownIdentity = identityBySource.get(source.sourcePath);
        if (existing !== undefined && knownIdentity !== undefined && existing.id !== knownIdentity) {
          throw new Error(
            `forme-resolve-asset-refs-fs: source ${JSON.stringify(source.sourcePath)} is claimed by both ` +
            `${JSON.stringify(knownIdentity)} and ${JSON.stringify(existing.id)}`,
          );
        }
        let identity = knownIdentity ?? existing?.id;
        if (identity === undefined) {
          identity = persistIdentities
            ? await readPersistedIdentity(source.absolutePath, ctx.logger) ?? generateLogicalId()
            : generateLogicalId();
        }
        const claimedSource = sourceByIdentity.get(identity);
        if (claimedSource !== undefined && claimedSource !== source.sourcePath) {
          throw new Error(
            `forme-resolve-asset-refs-fs: logical id ${JSON.stringify(identity)} is claimed by both ` +
            `${JSON.stringify(claimedSource)} and ${JSON.stringify(source.sourcePath)}`,
          );
        }
        identityBySource.set(source.sourcePath, identity);
        sourceByIdentity.set(identity, source.sourcePath);
        refs.push({
          id: identity,
          nodePath: located.nodePath,
          role: "image",
          sourcePath: source.sourcePath,
          ...(source.urlSuffix.length === 0 ? {} : { urlSuffix: source.urlSuffix }),
        });
        resolvedCount++;
      }

      const revision = computeRevisionId({
        document: node.document as unknown as JsonValue,
        frontmatter: node.frontmatter,
        assetRefs: refs as unknown as JsonValue,
        sourcePath: node.sourcePath,
      });
      yield { ...node, revision, assetRefs: refs } as never;
    }

    ctx.logger.debug("forme-resolve-asset-refs-fs: stream complete", {
      resolved: resolvedCount,
      uniqueSources: identityBySource.size,
    });
  },
});

function locateImages(document: DocumentNode): readonly LocatedImage[] {
  const images: LocatedImage[] = [];
  walkNode(document, [], images);
  return images;
}

function walkNode(node: unknown, path: readonly number[], images: LocatedImage[]): void {
  if (typeof node !== "object" || node === null) return;
  const record = node as Readonly<Record<string, unknown>>;
  if (record.type === "image" && typeof record.destination === "string") {
    images.push({ image: record as unknown as ImageNode, nodePath: path });
  }
  if (!Array.isArray(record.children)) return;
  record.children.forEach((child, index) => walkNode(child, [...path, index], images));
}

function samePath(left: readonly number[], right: readonly number[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index]);
}

function identitySidecarPath(absoluteAssetPath: string): string {
  return resolve(dirname(absoluteAssetPath), `.${basename(absoluteAssetPath)}.id.json`);
}

async function readPersistedIdentity(
  absoluteAssetPath: string,
  logger: Logger,
): Promise<LogicalId | null> {
  const sidecarPath = identitySidecarPath(absoluteAssetPath);
  let text: string;
  try {
    text = await readFile(sidecarPath, "utf8");
  } catch (error) {
    if ((error as { code?: string }).code !== "ENOENT") {
      logger.debug("forme-resolve-asset-refs-fs: identity sidecar read failed", {
        sidecarPath,
        error: String(error),
      });
    }
    return null;
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    logger.warn("forme-resolve-asset-refs-fs: identity sidecar contains invalid JSON; generating fresh id", {
      sidecarPath,
    });
    return null;
  }
  const logicalId = typeof parsed === "object" && parsed !== null
    ? (parsed as { logicalId?: unknown }).logicalId
    : undefined;
  if (typeof logicalId !== "string" || !isLogicalIdShape(logicalId)) {
    logger.warn("forme-resolve-asset-refs-fs: identity sidecar has missing/malformed logicalId; generating fresh id", {
      sidecarPath,
    });
    return null;
  }
  return logicalId as LogicalId;
}

export default resolveAssetRefsFs;
export { resolveAssetRefsFs };
