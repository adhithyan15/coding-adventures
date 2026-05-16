/**
 * src/cli.ts — runnable entry point for the hello-world demo.
 *
 * Resolves `contentRoot` and `outDir` relative to THIS file's
 * directory (not `process.cwd()`) so the demo works regardless of
 * where the user runs `npm start` from.
 *
 * On success:
 *   - Prints a one-line "wrote N files in M ms" summary.
 *   - Prints per-stage summaries (instance id, items consumed/produced,
 *     elapsed ms, outcome).
 *   - Exits 0.
 *
 * On any non-success outcome:
 *   - Prints every error in `result.errors`.
 *   - Exits 1.
 *
 * @module cli
 */

import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { consoleLogger } from "@coding-adventures/forme-stage";
import { buildBlog } from "./build.js";

// ── Path resolution ──────────────────────────────────────────────────
//
// __dirname is unavailable under ESM.  Recompute it from import.meta.url
// so the script can be invoked via `npm start`, `tsx src/cli.ts`, or
// `node --loader tsx src/cli.ts` from any working directory.

const __filename = fileURLToPath(import.meta.url);
const __dirname  = dirname(__filename);

// The package root is the parent of src/.
const PACKAGE_ROOT = resolve(__dirname, "..");
const CONTENT_ROOT = resolve(PACKAGE_ROOT, "content");
const OUT_DIR      = resolve(PACKAGE_ROOT, "dist");

// ── Run ──────────────────────────────────────────────────────────────

const logger = consoleLogger({ level: "info" });

const startMs = Date.now();
const result = await buildBlog({
  contentRoot: CONTENT_ROOT,
  outDir:      OUT_DIR,
  logger,
});
const elapsedMs = Date.now() - startMs;

// ── Report ───────────────────────────────────────────────────────────

// One-line headline — useful when piping to a log file or CI artifact.
process.stdout.write(
  `\nforme-hello-world: outcome=${result.outcome} elapsed=${elapsedMs}ms` +
  ` buildId=${result.buildId}\n`,
);

// Per-stage summary table.  Aligned by name length of the longest
// stage so columns line up without pulling in a table-formatter dep.
const maxNameLen = Math.max(...result.stages.map(s => s.stageName.length), 4);
process.stdout.write(`\n  ${"stage".padEnd(maxNameLen)}  in   out  elapsed  outcome\n`);
process.stdout.write(`  ${"".padEnd(maxNameLen, "─")}  ───  ───  ───────  ───────\n`);
for (const s of result.stages) {
  process.stdout.write(
    `  ${s.stageName.padEnd(maxNameLen)}  ` +
    `${String(s.itemsConsumed).padStart(3)}  ` +
    `${String(s.itemsProduced).padStart(3)}  ` +
    `${String(s.elapsedMs).padStart(7)}  ` +
    `${s.outcome}\n`,
  );
}

if (result.errors.length > 0) {
  process.stderr.write(`\n${result.errors.length} error(s):\n`);
  for (const err of result.errors) {
    process.stderr.write(
      `  - [${err.code}] ${err.stageName} (${err.instanceId}): ${err.message}\n`,
    );
  }
}

if (result.outcome !== "success") {
  process.exit(1);
}

process.stdout.write(`\n  wrote ${OUT_DIR}/blog/hello.html\n`);
