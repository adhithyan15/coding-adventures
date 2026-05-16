/**
 * @coding-adventures/forme-emit-fs
 *
 * Forme emit stage: `Stream<RenderedPage>` → `DeployArtifact`.
 *
 *   consumes:    streamOf(Kinds.RenderedPage)
 *   produces:    Kinds.DeployArtifact
 *   capabilities: ["filesystem:write"]
 *   configSchema: { outDir: string }   ← REQUIRED
 *
 * For each incoming `RenderedPage`:
 *
 *   1. Map `route` → on-disk path under `outDir` (see `path-utils.ts`
 *      for the traversal-guarded mapping).
 *   2. Ensure the parent directory exists (`mkdir -p`).
 *   3. Write the page's `html` as UTF-8 bytes.
 *   4. Stash `{ path, bytes }` in an in-memory map so the final
 *      manifest can be assembled without re-reading.
 *
 * After the stream finishes:
 *
 *   5. Emit one `DeployArtifact`:
 *        - `variant: { kind: "dist-tree" }` (static-files tree)
 *        - `files: ReadonlyRecord<routePath, Uint8Array>`
 *        - `manifest.routes: DeployRoute[]` (one per emitted page)
 *        - `manifest.assets: []` (asset emission is a future stage)
 *        - `manifest.buildTime: ISO timestamp from ctx.time`
 *        - `manifest.buildId: blake2b over { route → sha256 }`
 *
 * === Capability discipline ===
 *
 * Emit stages have the same chicken-and-egg problem source stages do:
 * `ctx.filesystem` is the orchestrator-provided `FilesystemApi`, but
 * for forme-emit-fs to *write* disk *something* has to be the
 * implementation.  v0 resolves it the same way `forme-source-fs` does:
 *
 *   - Declare `filesystem:write` in `required_capabilities.json` and
 *     in the stage's `capabilities` array (so the manifest audit
 *     layer sees correct intent).
 *   - Read via `node:fs/promises` directly (the runtime path).
 *
 * When FM02 lands and the orchestrator wires real `FilesystemApi`s
 * around stages, this stage will be one of the implementations.
 * Documented in README + required_capabilities.json + this header.
 *
 * === Spec adherence ===
 *
 * No deliberate divergences from FM00/FM01.  v0 simplifications:
 *
 *   - `manifest.assets` is always `[]` (no asset stages yet).
 *   - `DeployRoute.islands` / `DeployRoute.css` are always `[]`
 *     (renderer doesn't track them yet).
 *   - `DeployRoute.target` is always `{ kind: "file", path: <route> }`
 *     — no handler-typed routes in the static blog.
 *
 * @module index
 */

import { mkdir, writeFile } from "node:fs/promises";
import { dirname, relative as relPath } from "node:path";
import { createHash } from "node:crypto";
import {
  Kinds,
  streamOf,
  type RenderedPage,
  type DeployArtifact,
  type DeployRoute,
  type DeployManifest,
  type JsonValue,
} from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import { computeRevisionId } from "@coding-adventures/forme-identity";
import { routeToOutPath } from "./path-utils.js";

export interface EmitFsConfig {
  /** Directory under which all pages are written.  REQUIRED. */
  readonly outDir: string;
}

const encoder = new TextEncoder();

/**
 * SHA-256 a buffer to hex.  Used inside the build-id derivation; we
 * deliberately keep file-identity in SHA-256 (a wider/older standard
 * than blake2b) for the manifest `assets[*].sha256` field — the
 * DeployAssetEntry shape explicitly names sha256, so changing to a
 * different hash would be a divergence.
 */
function sha256Hex(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

const emitFs = defineStage({
  name: "@coding-adventures/forme-emit-fs",
  version: "0.1.0",
  apiVersion: 1,
  description: "Write each RenderedPage to disk under outDir; emit one DeployArtifact summarising the result.",
  consumes: streamOf(Kinds.RenderedPage),
  produces: Kinds.DeployArtifact,
  capabilities: ["filesystem:write"],
  configSchema: {
    type: "object",
    required: ["outDir"],
    properties: {
      outDir: { type: "string" },
    },
  },
  async run(rawInput, rawConfig, ctx) {
    const config = rawConfig as EmitFsConfig;
    if (typeof config?.outDir !== "string" || config.outDir.length === 0) {
      throw new Error("forme-emit-fs: config.outDir must be a non-empty string");
    }
    const stream = rawInput as AsyncIterable<RenderedPage>;

    // Collect file contents and per-route metadata as we go.  Using a
    // Map preserves insertion order so manifest.routes mirrors stream
    // order — useful for diff stability across runs.
    const files = new Map<string, Uint8Array>();
    const routes: DeployRoute[] = [];
    let writtenCount = 0;

    for await (const page of stream) {
      ctx.cancellation.throwIfCancelled();

      const absPath = routeToOutPath(config.outDir, page.route);
      const bytes = encoder.encode(page.html);

      await mkdir(dirname(absPath), { recursive: true });
      await writeFile(absPath, bytes);
      writtenCount++;

      // Record the route in the manifest using the OS-relative path
      // under outDir, kept POSIX-style (forward slashes) so the
      // manifest is reproducible regardless of host platform.
      const posixPath = relPath(config.outDir, absPath).split(/[/\\]/).join("/");
      files.set(posixPath, bytes);
      routes.push({
        pattern: page.route,
        target: { kind: "file", path: posixPath },
        islands: [],   // renderer doesn't track islands in v0
        css: [],       // theme CSS is inlined; no external bundles
      });
    }

    // Build-time: ISO string from the clock facility (not new Date(),
    // so frozenClock-based reproducible builds stay deterministic).
    const buildTime = ctx.time.nowIso();

    // Build-id: blake2b over the route→sha256 map.  The canonical-
    // JSON serialiser sorts keys, so this is insensitive to
    // stream-arrival order — different schedulings of the same pages
    // produce the same buildId.  Deliberately does NOT include
    // `outDir` — the same site deployed to two different directories
    // is the same build.
    const fileHashes: Record<string, string> = {};
    for (const [path, bytes] of files) fileHashes[path] = sha256Hex(bytes);
    const buildIdInput: JsonValue = { files: fileHashes };
    const buildId = computeRevisionId(buildIdInput);

    const filesRecord: Record<string, Uint8Array> = {};
    for (const [path, bytes] of files) filesRecord[path] = bytes;

    const manifest: DeployManifest = {
      routes,
      assets: [],   // no asset stages yet (v0)
      buildTime,
      buildId,
    };
    const artifact: DeployArtifact = {
      variant: { kind: "dist-tree" },
      files: filesRecord,
      manifest,
    };
    ctx.logger.info("forme-emit-fs: wrote pages", {
      count: writtenCount,
      outDir: config.outDir,
      buildId,
    });
    return artifact as never;
  },
});

export default emitFs;
export { emitFs, routeToOutPath, sha256Hex };
