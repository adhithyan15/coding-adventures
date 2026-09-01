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
 * The pragmatic v0 resolution: source-fs declares `storage:read` and
 * `storage:write` in required_capabilities.json and accesses the
 * filesystem via `node:fs/promises` directly.  Its writes are limited
 * to exclusive creation of missing identity sidecars. When FM02
 * lands and the orchestrator wires real StorageApis around stages,
 * source-fs will be one of the implementations rather than a
 * consumer.
 *
 * Documented as a stage-level exception both here and in the README.
 */

import { readFile, writeFile } from "node:fs/promises";
import { basename, dirname, join, relative } from "node:path";
import {
  Kinds,
  streamOf,
  type ContentSource,
  type LogicalId,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import {
  computeBinaryRevisionId,
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
   * When true (default), resolve each stable `LogicalId` through an
   * adjacent `<dirname>/.<basename>.id.json` sidecar. Missing sidecars
   * are created atomically on first encounter. Malformed sidecars are
   * rejected without overwriting user-authored state.
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
 * persisted `LogicalId` on success or `null` when the sidecar is missing.
 *
 * Validation failures throw: silently replacing an existing sidecar could
 * change the identity of already-published content. A missing sidecar is the
 * common case and is not logged.
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
  if (text.length === 0) {
    throw new Error(`forme-source-fs: identity sidecar contains invalid JSON: ${sidecarPath}`);
  }
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw new Error(`forme-source-fs: identity sidecar contains invalid JSON: ${sidecarPath}`);
  }
  if (typeof parsed !== "object" || parsed === null) {
    throw new Error(`forme-source-fs: identity sidecar is not a JSON object: ${sidecarPath}`);
  }
  const raw = (parsed as { logicalId?: unknown }).logicalId;
  if (typeof raw !== "string" || !isLogicalIdShape(raw)) {
    throw new Error(`forme-source-fs: identity sidecar has missing/malformed logicalId: ${sidecarPath}`);
  }
  return raw as LogicalId;
}

/** Resolve an identity, creating a missing sidecar without racing other builds. */
async function resolvePersistedIdentity(
  absSourcePath: string,
  logger: Logger,
): Promise<{ identity: LogicalId; created: boolean }> {
  const existing = await readPersistedIdentity(absSourcePath, logger);
  if (existing !== null) return { identity: existing, created: false };

  const sidecarPath = identitySidecarPath(absSourcePath);
  const generated = generateLogicalId();
  const text = `${JSON.stringify({ logicalId: generated }, null, 2)}\n`;
  try {
    await writeFile(sidecarPath, text, { encoding: "utf-8", flag: "wx" });
    return { identity: generated, created: true };
  } catch (error) {
    if ((error as { code?: string }).code !== "EEXIST") throw error;
    const winner = await readPersistedIdentity(absSourcePath, logger);
    if (winner === null) {
      throw new Error(`forme-source-fs: identity sidecar disappeared during creation: ${sidecarPath}`);
    }
    return { identity: winner, created: false };
  }
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
  version: "0.3.0",
  apiVersion: 1,
  description: "Walk a filesystem directory and emit one ContentSource per matching file.",
  consumes: Kinds.Void,
  produces: streamOf(Kinds.ContentSource),
  capabilities: ["storage:read", "storage:write"],
  configSchema: {
    type: "object",
    required: ["glob"],
    properties: {
      glob: { type: "string" },
      root: { type: "string" },
      persistIdentities: { type: "boolean" },
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
    const stats = { matched: 0, sidecarsLoaded: 0, sidecarsCreated: 0 };

    for await (const absPath of walkFiles(root, ext)) {
      ctx.cancellation.throwIfCancelled();
      const relPath = relative(root, absPath);
      const fileBytes = await readFile(absPath);
      const fileStat = await getFileStat(absPath);
      const bytes = new Uint8Array(fileBytes);
      // Persistence is opt-out. Enabled builds read or exclusively create a
      // stable sidecar; ephemeral builds deliberately generate a fresh UUID.
      let identity: LogicalId;
      if (persistIdentities) {
        const persisted = await resolvePersistedIdentity(absPath, ctx.logger);
        identity = persisted.identity;
        if (persisted.created) stats.sidecarsCreated++;
        else stats.sidecarsLoaded++;
      } else {
        identity = generateLogicalId();
      }
      const source: ContentSource = {
        path: relPath,
        bytes,
        mimeType: mimeFor(ext),
        identity,
        // Revisions identify content, not its current locator. Renaming a file
        // together with its sidecar therefore preserves both IDs.
        revision: computeBinaryRevisionId(bytes),
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
      sidecarsCreated: stats.sidecarsCreated,
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
