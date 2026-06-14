/**
 * @coding-adventures/forme-source-fs
 *
 * The first actual pipeline stage — walks a filesystem directory and
 * emits one ContentSource per matching file (FM00 §5.1).
 *
 * Config:
 *
 *     { glob: "**\/*.md", root?: "." }
 *
 * `root` defaults to the current working directory.  `glob` is the
 * v0 simplified pattern (see `walker.ts` for the constraints).
 *
 * === Capability discipline ===
 *
 * Source stages have a chicken-and-egg problem with ctx.storage:
 * `ctx.storage` is the orchestrator-provided StorageApi — but for
 * the source-fs stage to read disk, *something* has to be the
 * StorageApi implementation.  We are that something.
 *
 * The pragmatic v0 resolution: source-fs declares `storage:read` in
 * its required_capabilities.json (so the manifest layer audits it
 * correctly) and reads via `node:fs/promises` directly.  When FM02
 * lands and the orchestrator wires real StorageApis around stages,
 * source-fs will be one of the implementations rather than a
 * consumer.
 *
 * Documented as a stage-level exception both here and in the README.
 */

import { readFile } from "node:fs/promises";
import { basename, dirname, join, relative } from "node:path";
import {
  Kinds,
  streamOf,
  type ContentSource,
  type LogicalId,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import {
  computeRevisionId,
  generateLogicalId,
  isLogicalIdShape,
} from "@coding-adventures/forme-identity";
import type { Logger } from "@coding-adventures/forme-stage";
import { parseGlob, walkFiles } from "./walker.js";

export interface SourceFsConfig {
  /** Glob pattern.  v0 supports only "**\/*.<ext>". */
  readonly glob: string;
  /** Directory to search; defaults to process.cwd(). */
  readonly root?: string;
  /**
   * When true (default), read existing `<dirname>/.<basename>.id.json`
   * sidecar files to derive each source's stable `LogicalId`.  When
   * the sidecar is missing, malformed, or contains a non-UUIDv7
   * value, a fresh LogicalId is generated instead (current v0
   * behaviour preserved as the fallback).
   *
   * The sidecar contains JSON of shape:
   *
   *     { "logicalId": "<uuid-v7>", "createdAt": "<iso?>", "note": "<?>" }
   *
   * Only `logicalId` is required.  Future revisions may extend the
   * shape (parent ids, semantic tags); the reader ignores unknown
   * keys.
   *
   * Setting this to `false` always generates a fresh id (the v0.1.0
   * behaviour).  Useful for ephemeral builds where stable identities
   * matter less than predictable test fixtures.
   */
  readonly persistIdentities?: boolean;
}

/**
 * Filename of the identity sidecar.  Hidden (leading dot) so file
 * managers and shells don't render it as a "real" content file, and
 * named after the source basename so multiple sources in the same
 * directory don't collide:
 *
 *     posts/
 *       hello.md           ← source
 *       .hello.md.id.json  ← sidecar
 *       intro.md           ← source
 *       .intro.md.id.json  ← sidecar
 *
 * The full extension (`.id.json`) keeps the file recognisable as
 * structured metadata when authors stumble across it in `ls -la`.
 */
function identitySidecarPath(absSourcePath: string): string {
  return join(dirname(absSourcePath), "." + basename(absSourcePath) + ".id.json");
}

/**
 * Read the identity sidecar adjacent to `absSourcePath`.  Returns the
 * persisted `LogicalId` on success, `null` when the sidecar is missing
 * or its contents fail validation.
 *
 * Validation failures (malformed JSON, non-string logicalId, wrong
 * UUIDv7 shape) are logged at `warn` so users notice broken sidecars
 * without the pipeline failing.  A missing sidecar is the common
 * case and is NOT logged.
 */
async function readPersistedIdentity(
  absSourcePath: string,
  logger: Logger,
): Promise<LogicalId | null> {
  const sidecarPath = identitySidecarPath(absSourcePath);
  let text: string;
  try {
    text = (await readFile(sidecarPath, "utf-8")).trim();
  } catch (err) {
    // ENOENT = no sidecar; that's the common case, silent.
    // Anything else is unusual; log at debug for visibility.
    if ((err as { code?: string }).code !== "ENOENT") {
      logger.debug("forme-source-fs: identity sidecar read failed", {
        sidecarPath,
        error: String(err),
      });
    }
    return null;
  }
  if (text.length === 0) return null;
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    logger.warn("forme-source-fs: identity sidecar contains invalid JSON; generating fresh id", {
      sidecarPath,
    });
    return null;
  }
  if (typeof parsed !== "object" || parsed === null) {
    logger.warn("forme-source-fs: identity sidecar is not a JSON object; generating fresh id", {
      sidecarPath,
    });
    return null;
  }
  const raw = (parsed as { logicalId?: unknown }).logicalId;
  if (typeof raw !== "string" || !isLogicalIdShape(raw)) {
    logger.warn("forme-source-fs: identity sidecar has missing/malformed logicalId; generating fresh id", {
      sidecarPath,
      raw: typeof raw === "string" ? raw : typeof raw,
    });
    return null;
  }
  return raw as LogicalId;
}

const MIME_BY_EXT: Record<string, string> = {
  ".md":   "text/markdown",
  ".markdown": "text/markdown",
  ".mdx":  "text/markdown",
  ".html": "text/html",
  ".txt":  "text/plain",
  ".json": "application/json",
};

function mimeFor(ext: string): string | null {
  return MIME_BY_EXT[ext.toLowerCase()] ?? null;
}

const sourceFs = defineStage({
  name: "@coding-adventures/forme-source-fs",
  version: "0.1.0",
  apiVersion: 1,
  description: "Walk a filesystem directory and emit one ContentSource per matching file.",
  consumes: Kinds.Void,
  produces: streamOf(Kinds.ContentSource),
  capabilities: ["storage:read"],
  configSchema: {
    type: "object",
    required: ["glob"],
    properties: {
      glob: { type: "string" },
      root: { type: "string" },
    },
  },
  async *run(_input, rawConfig, ctx) {
    const config = rawConfig as SourceFsConfig;
    if (typeof config?.glob !== "string" || config.glob.length === 0) {
      throw new Error("forme-source-fs: config.glob must be a non-empty string");
    }
    const root = config.root ?? process.cwd();
    const { ext } = parseGlob(config.glob);
    // Persistence defaults to TRUE.  Tests can opt out by setting
    // `persistIdentities: false` in the config to get the legacy
    // generate-fresh-every-time behaviour.
    const persistIdentities = config.persistIdentities !== false;
    const stats = { matched: 0, sidecarsLoaded: 0 };

    for await (const absPath of walkFiles(root, ext)) {
      ctx.cancellation.throwIfCancelled();
      const relPath = relative(root, absPath);
      const fileBytes = await readFile(absPath);
      const fileStat = await getFileStat(absPath);
      const bytes = new Uint8Array(fileBytes);
      // Resolve LogicalId.  Persistence is opt-out — when the
      // sidecar is present and valid we get stable identities
      // across runs; otherwise we fall through to a fresh UUID
      // (the v0.1.0 behaviour, preserved as the fallback).
      let identity: LogicalId;
      if (persistIdentities) {
        const persisted = await readPersistedIdentity(absPath, ctx.logger);
        if (persisted !== null) {
          identity = persisted;
          stats.sidecarsLoaded++;
        } else {
          identity = generateLogicalId();
        }
      } else {
        identity = generateLogicalId();
      }
      const source: ContentSource = {
        path: relPath,
        bytes,
        mimeType: mimeFor(ext),
        identity,
        // Hash the bytes via canonical-json wrapping so RevisionId
        // semantics are uniform across kinds.  (Hashing raw bytes
        // would require a second `bytesToHex` API to compose into the
        // revision-id format string; cheaper to wrap.)
        revision: computeRevisionId({ path: relPath, bytes: Array.from(bytes) }),
        providerMeta: {
          mtimeMs: fileStat.mtimeMs,
          sizeBytes: fileStat.size,
        },
      };
      stats.matched++;
      yield source as never;
    }

    ctx.logger.debug("forme-source-fs scan complete", {
      root, ext,
      matched: stats.matched,
      sidecarsLoaded: stats.sidecarsLoaded,
    });
  },
});

async function getFileStat(path: string): Promise<{ mtimeMs: number; size: number }> {
  const { stat } = await import("node:fs/promises");
  const s = await stat(path);
  return { mtimeMs: s.mtimeMs, size: s.size };
}

export default sourceFs;
export { sourceFs };
