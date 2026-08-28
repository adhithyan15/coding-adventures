/**
 * Load resolved filesystem references into immutable Forme Asset IR values.
 *
 * The stage buffers reference claims so collisions fail before any output is
 * emitted, then reads each unique source in portable-path order. Lexical and
 * realpath containment are both enforced: a path may use an in-root symlink,
 * but a link that resolves outside the configured root fails closed.
 */

import { lstat, readFile, realpath } from "node:fs/promises";
import { extname, isAbsolute, relative, resolve, sep } from "node:path";
import { posix } from "node:path";
import {
  Kinds,
  streamOf,
  type Asset,
  type AssetRole,
  type ContentNode,
  type LogicalId,
} from "@coding-adventures/forme-types";
import { computeBinaryRevisionId } from "@coding-adventures/forme-identity";
import { defineStage } from "@coding-adventures/forme-stage";

export interface LoadAssetsFsConfig {
  /** Storage root. Defaults to process.cwd(). */
  readonly root?: string;
}

interface AssetClaim {
  readonly identity: LogicalId;
  readonly role: AssetRole;
  readonly sourcePath: string;
}

const MIME_BY_EXTENSION: Readonly<Record<string, string>> = {
  ".avif": "image/avif",
  ".bmp": "image/bmp",
  ".css": "text/css",
  ".gif": "image/gif",
  ".heic": "image/heic",
  ".heif": "image/heif",
  ".ico": "image/x-icon",
  ".jpeg": "image/jpeg",
  ".jpg": "image/jpeg",
  ".js": "text/javascript",
  ".json": "application/json",
  ".m4a": "audio/mp4",
  ".mp3": "audio/mpeg",
  ".mp4": "video/mp4",
  ".ogg": "audio/ogg",
  ".otf": "font/otf",
  ".pdf": "application/pdf",
  ".png": "image/png",
  ".svg": "image/svg+xml",
  ".tif": "image/tiff",
  ".tiff": "image/tiff",
  ".ttf": "font/ttf",
  ".wav": "audio/wav",
  ".webm": "video/webm",
  ".webp": "image/webp",
  ".woff": "font/woff",
  ".woff2": "font/woff2",
  ".xml": "application/xml",
};

/** Detect common binary signatures before falling back to the source suffix. */
export function detectMimeType(sourcePath: string, bytes: Uint8Array): string {
  if (startsWith(bytes, [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])) {
    return "image/png";
  }
  if (startsWith(bytes, [0xff, 0xd8, 0xff])) return "image/jpeg";
  if (ascii(bytes, 0, 6) === "GIF87a" || ascii(bytes, 0, 6) === "GIF89a") {
    return "image/gif";
  }
  if (ascii(bytes, 0, 4) === "RIFF" && ascii(bytes, 8, 4) === "WEBP") {
    return "image/webp";
  }
  if (startsWith(bytes, [0x00, 0x00, 0x01, 0x00])) return "image/x-icon";
  if (ascii(bytes, 0, 4) === "%PDF") return "application/pdf";
  if (ascii(bytes, 0, 4) === "wOFF") return "font/woff";
  if (ascii(bytes, 0, 4) === "wOF2") return "font/woff2";
  if (ascii(bytes, 4, 4) === "ftyp") {
    const brand = ascii(bytes, 8, 4);
    if (["avif", "avis"].includes(brand)) return "image/avif";
    if (["heic", "heix", "hevc", "hevx", "mif1", "msf1"].includes(brand)) {
      return "image/heic";
    }
    return "video/mp4";
  }
  const prefix = new TextDecoder().decode(bytes.subarray(0, 1024)).replace(/^\uFEFF/, "");
  if (/^\s*(?:<\?xml[^>]*>\s*)?(?:<!--[^]*?-->\s*)*<svg\b/i.test(prefix)) {
    return "image/svg+xml";
  }
  return MIME_BY_EXTENSION[extname(sourcePath).toLowerCase()] ?? "application/octet-stream";
}

/** Resolve and validate a portable source path beneath a canonical root. */
export async function resolveContainedAssetPath(
  absoluteRoot: string,
  canonicalRoot: string,
  sourcePath: string,
): Promise<string> {
  validatePortableSourcePath(sourcePath);
  const lexicalPath = resolve(absoluteRoot, ...sourcePath.split("/"));
  assertContained(absoluteRoot, lexicalPath, sourcePath, "lexically escapes storage root");

  let canonicalPath: string;
  try {
    canonicalPath = await realpath(lexicalPath);
  } catch (error) {
    if ((error as { code?: string }).code === "ENOENT") {
      throw new Error(`forme-load-assets-fs: referenced asset ${JSON.stringify(sourcePath)} does not exist`);
    }
    throw new Error(
      `forme-load-assets-fs: cannot resolve referenced asset ${JSON.stringify(sourcePath)}: ${errorCode(error)}`,
    );
  }
  assertContained(canonicalRoot, canonicalPath, sourcePath, "resolves outside storage root via symlink");

  let stats;
  try {
    stats = await lstat(canonicalPath);
  } catch (error) {
    throw new Error(
      `forme-load-assets-fs: cannot inspect referenced asset ${JSON.stringify(sourcePath)}: ${errorCode(error)}`,
    );
  }
  if (!stats.isFile()) {
    throw new Error(`forme-load-assets-fs: referenced asset ${JSON.stringify(sourcePath)} is not a regular file`);
  }
  return canonicalPath;
}

const loadAssetsFs = defineStage({
  name: "@coding-adventures/forme-load-assets-fs",
  version: "0.1.0",
  apiVersion: 1,
  description: "Load resolved filesystem AssetRefs into deterministic immutable Asset IR values.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.Asset),
  capabilities: ["storage:read"],
  configSchema: {
    type: "object",
    properties: { root: { type: "string" } },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = (rawConfig ?? {}) as LoadAssetsFsConfig;
    if (config.root !== undefined && (typeof config.root !== "string" || config.root.length === 0)) {
      throw new Error("forme-load-assets-fs: config.root must be a non-empty string when provided");
    }
    const absoluteRoot = resolve(config.root ?? process.cwd());
    let canonicalRoot: string;
    try {
      canonicalRoot = await realpath(absoluteRoot);
    } catch (error) {
      throw new Error(`forme-load-assets-fs: storage root is unavailable: ${errorCode(error)}`);
    }
    let rootStats;
    try {
      rootStats = await lstat(canonicalRoot);
    } catch (error) {
      throw new Error(`forme-load-assets-fs: cannot inspect storage root: ${errorCode(error)}`);
    }
    if (!rootStats.isDirectory()) {
      throw new Error("forme-load-assets-fs: storage root is not a directory");
    }

    const claimsBySource = new Map<string, AssetClaim>();
    const claimsByIdentity = new Map<LogicalId, AssetClaim>();
    for await (const node of rawInput as AsyncIterable<ContentNode>) {
      ctx.cancellation.throwIfCancelled();
      for (const ref of node.assetRefs) {
        if (ref.sourcePath === undefined) {
          throw new Error(
            `forme-load-assets-fs: asset ${JSON.stringify(ref.id)} has no resolved sourcePath; ` +
            "add forme-resolve-asset-refs-fs upstream",
          );
        }
        validatePortableSourcePath(ref.sourcePath);
        const claim: AssetClaim = { identity: ref.id, role: ref.role, sourcePath: ref.sourcePath };
        const sourceClaim = claimsBySource.get(ref.sourcePath);
        if (sourceClaim !== undefined &&
            (sourceClaim.identity !== ref.id || sourceClaim.role !== ref.role)) {
          throw new Error(
            `forme-load-assets-fs: source ${JSON.stringify(ref.sourcePath)} has conflicting ` +
            "identity or role claims",
          );
        }
        const identityClaim = claimsByIdentity.get(ref.id);
        if (identityClaim !== undefined &&
            (identityClaim.sourcePath !== ref.sourcePath || identityClaim.role !== ref.role)) {
          throw new Error(
            `forme-load-assets-fs: logical id ${JSON.stringify(ref.id)} has conflicting source or role claims`,
          );
        }
        claimsBySource.set(ref.sourcePath, claim);
        claimsByIdentity.set(ref.id, claim);
      }
    }

    // Source paths are unique by construction, so equality is impossible.
    // Raw code-unit ordering is stable across operating systems and ICU data.
    const claims = [...claimsBySource.values()].sort((left, right) =>
      left.sourcePath < right.sourcePath ? -1 : 1);
    let totalBytes = 0;
    for (const claim of claims) {
      ctx.cancellation.throwIfCancelled();
      const canonicalPath = await resolveContainedAssetPath(
        absoluteRoot,
        canonicalRoot,
        claim.sourcePath,
      );
      let fileBytes: Uint8Array;
      try {
        fileBytes = new Uint8Array(await readFile(canonicalPath));
      } catch (error) {
        throw new Error(
          `forme-load-assets-fs: cannot read referenced asset ${JSON.stringify(claim.sourcePath)}: ${errorCode(error)}`,
        );
      }
      ctx.cancellation.throwIfCancelled();
      const bytes = fileBytes;
      totalBytes += bytes.byteLength;
      const asset: Asset = {
        identity: claim.identity,
        revision: computeBinaryRevisionId(bytes),
        role: claim.role,
        mimeType: detectMimeType(claim.sourcePath, bytes),
        bytes,
        byteLength: bytes.byteLength,
        dimensions: null,
        durationMs: null,
        derivedFrom: null,
        meta: { sourcePath: claim.sourcePath },
      };
      yield asset as never;
    }
    ctx.logger.debug("forme-load-assets-fs: collection complete", {
      assets: claims.length,
      totalBytes,
    });
  },
});

function validatePortableSourcePath(sourcePath: string): void {
  const normalized = posix.normalize(sourcePath);
  if (sourcePath.length === 0 || sourcePath.includes("\0") || sourcePath.includes("\\") ||
      sourcePath.startsWith("/") || /^[A-Za-z]:/.test(sourcePath) ||
      normalized === "." || normalized === ".." || normalized.startsWith("../") ||
      normalized !== sourcePath) {
    throw new Error(`forme-load-assets-fs: invalid resolved sourcePath ${JSON.stringify(sourcePath)}`);
  }
}

function assertContained(root: string, candidate: string, sourcePath: string, reason: string): void {
  const rel = relative(root, candidate);
  if (rel === ".." || rel.startsWith(`..${sep}`) || isAbsolute(rel)) {
    throw new Error(`forme-load-assets-fs: asset ${JSON.stringify(sourcePath)} ${reason}`);
  }
}

function startsWith(bytes: Uint8Array, signature: readonly number[]): boolean {
  return signature.length <= bytes.length && signature.every((byte, index) => bytes[index] === byte);
}

function ascii(bytes: Uint8Array, start: number, length: number): string {
  if (start + length > bytes.byteLength) return "";
  return String.fromCharCode(...bytes.subarray(start, start + length));
}

function errorCode(error: unknown): string {
  const code = (error as { code?: unknown }).code;
  return typeof code === "string" ? code : "filesystem error";
}

export default loadAssetsFs;
