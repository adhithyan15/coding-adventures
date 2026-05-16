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
import { relative } from "node:path";
import {
  Kinds,
  streamOf,
  type ContentSource,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import {
  computeRevisionId,
  generateLogicalId,
} from "@coding-adventures/forme-identity";
import { parseGlob, walkFiles } from "./walker.js";

export interface SourceFsConfig {
  /** Glob pattern.  v0 supports only "**\/*.<ext>". */
  readonly glob: string;
  /** Directory to search; defaults to process.cwd(). */
  readonly root?: string;
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
    const stats = { matched: 0 };

    for await (const absPath of walkFiles(root, ext)) {
      ctx.cancellation.throwIfCancelled();
      const relPath = relative(root, absPath);
      const fileBytes = await readFile(absPath);
      const fileStat = await getFileStat(absPath);
      const bytes = new Uint8Array(fileBytes);
      const source: ContentSource = {
        path: relPath,
        bytes,
        mimeType: mimeFor(ext),
        identity: generateLogicalId(),
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

    ctx.logger.debug("forme-source-fs scan complete", { root, ext, matched: stats.matched });
  },
});

async function getFileStat(path: string): Promise<{ mtimeMs: number; size: number }> {
  const { stat } = await import("node:fs/promises");
  const s = await stat(path);
  return { mtimeMs: s.mtimeMs, size: s.size };
}

export default sourceFs;
export { sourceFs };
