/**
 * build.ts — drive the blog pipeline end-to-end.
 *
 * Loads `forme.config.ts`, hands it to `createOrchestrator`,
 * `buildPipeline`, then `runOnce`.  Asserts a clean outcome and
 * reports a short summary.  Exits non-zero if anything fails so CI
 * can gate on it.
 *
 * Runs via `tsx` (a devDependency).  `tsx` strips the TypeScript
 * types at execution time so we don't need a separate `tsc` step
 * just to drive the pipeline — the stage packages compile their own
 * published types when published, and the site driver runs straight
 * from source.
 */

import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import {
  createOrchestrator,
} from "@coding-adventures/forme-orchestrator";
import { consoleLogger } from "@coding-adventures/forme-stage";
import type { Collection, DeployArtifact } from "@coding-adventures/forme-types";
import config from "./forme.config.ts";

const here = dirname(fileURLToPath(import.meta.url));
process.chdir(here);  // make config.stages[*].config.root/outDir paths relative to the site dir

const logger = consoleLogger({ level: "info" }).child({ pipeline: config.name });
const orchestrator = createOrchestrator({ logger });

logger.info("Building pipeline", { name: config.name });

try {
  const pipeline = await orchestrator.buildPipeline(config);
  const result = await orchestrator.runOnce(pipeline);

  if (result.outcome !== "success") {
    logger.error("Pipeline did not complete cleanly", {
      outcome: result.outcome,
      errors: result.errors.length,
    });
    for (const err of result.errors) {
      const ctorName = (err.error as { constructor?: { name?: string } })?.constructor?.name ?? "Error";
      const msg = (err.error as { message?: string })?.message ?? String(err.error);
      logger.error(`  [${err.instance}] ${ctorName}: ${msg}`);
    }
    process.exit(1);
  }

  const posts = result.outputs.posts as Collection | undefined;
  const site = result.outputs.site as DeployArtifact | undefined;
  if (!posts || !site) {
    throw new Error("Blog pipeline must expose both named sinks: posts and site");
  }

  const collectionRoutes = posts.entries.map((entry) => {
    if (entry.route === null) {
      throw new Error(`Collection entry ${entry.identity} has no canonical route`);
    }
    return entry.route;
  });
  const emittedRoutes = site.manifest.routes.map((route) => route.pattern);
  const sortedCollectionRoutes = [...collectionRoutes].sort();
  const sortedEmittedRoutes = [...emittedRoutes].sort();
  if (JSON.stringify(sortedCollectionRoutes) !== JSON.stringify(sortedEmittedRoutes)) {
    throw new Error("Collection routes must match the emitted page routes exactly");
  }

  const chronologicalDates = posts.entries.map((entry) => {
    if (entry.orderKey.kind !== "date") {
      throw new Error(`Collection entry ${entry.identity} is not date-ordered`);
    }
    return entry.orderKey.value;
  });
  if (chronologicalDates.some((date, index) => index > 0 && chronologicalDates[index - 1]! < date)) {
    throw new Error("Posts collection must be ordered newest first");
  }

  logger.info("Build complete", {
    outcome: result.outcome,
    elapsedMs: result.elapsedMs,
    buildId: result.buildId,
    stages: result.stages.length,
    posts: posts.entries.length,
  });

  // Print a short summary of what got emitted.
  for (const [name, value] of Object.entries(result.outputs)) {
    const v = value as { variant?: { kind?: string }; files?: Record<string, unknown> };
    logger.info(`Output[${name}]`, {
      variantKind: v.variant?.kind,
      fileCount: v.files ? Object.keys(v.files).length : 0,
    });
  }

  logger.info("dist/ written under", { absolute: resolve(here, "dist") });
} finally {
  await orchestrator.dispose();
}
