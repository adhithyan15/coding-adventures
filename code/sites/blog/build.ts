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
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import {
  createOrchestrator,
} from "@coding-adventures/forme-orchestrator";
import { consoleLogger } from "@coding-adventures/forme-stage";
import type { DeployArtifact } from "@coding-adventures/forme-types";
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

  const articles = result.outputs.articles as DeployArtifact | undefined;
  const surface = result.outputs.surface as DeployArtifact | undefined;
  if (!articles || !surface) {
    throw new Error("Blog pipeline must expose both deploy sinks: articles and surface");
  }

  const articleRoutes = articles.manifest.routes.map((route) => route.pattern).sort();
  const surfaceRoutes = surface.manifest.routes.map((route) => route.pattern).sort();
  const expectedSurfaceRoutes = [
    "/blog/atom.xml",
    "/blog/index.html",
    "/blog/rss.xml",
    "/blog/sitemap.xml",
  ];
  if (JSON.stringify(surfaceRoutes) !== JSON.stringify(expectedSurfaceRoutes)) {
    throw new Error(`Blog surface routes differ: ${JSON.stringify(surfaceRoutes)}`);
  }
  if (articleRoutes.length === 0 || articleRoutes.some((route) => !route.endsWith(".html"))) {
    throw new Error("Article sink must contain at least one HTML route");
  }
  if (articles.manifest.assets.length !== 1) {
    throw new Error(`Article sink must contain exactly one asset, got ${articles.manifest.assets.length}`);
  }
  if (surface.manifest.assets.length !== 0) {
    throw new Error("Collection-derived surface sink must not claim article assets");
  }

  const [asset] = articles.manifest.assets;
  if (
    asset === undefined ||
    asset.id !== "01952c0d-7e63-7000-8000-000000000201" ||
    !asset.path.startsWith("blog/assets/forme-pipeline.") ||
    !asset.path.endsWith(".svg") ||
    asset.mime !== "image/svg+xml"
  ) {
    throw new Error(`Article asset manifest entry differs: ${JSON.stringify(asset)}`);
  }
  const artifactAssetBytes = articles.files[asset.path];
  if (artifactAssetBytes === undefined) {
    throw new Error(`Article artifact is missing manifest asset ${asset.path}`);
  }
  const actualSha256 = createHash("sha256").update(artifactAssetBytes).digest("hex");
  if (actualSha256 !== asset.sha256) {
    throw new Error(`Article asset sha256 differs: ${actualSha256} !== ${asset.sha256}`);
  }

  const helloPath = "blog/2026-05-15-hello-forme.html";
  const helloBytes = articles.files[helloPath];
  if (helloBytes === undefined) {
    throw new Error(`Article artifact is missing ${helloPath}`);
  }
  const helloHtml = new TextDecoder().decode(helloBytes);
  const expectedAssetUrl = `/coding-adventures/${asset.path}#pipeline`;
  if (!helloHtml.includes(expectedAssetUrl) || helloHtml.includes("forme-asset:")) {
    throw new Error(`Article HTML did not contain the rewritten asset URL ${expectedAssetUrl}`);
  }
  const diskHtml = await readFile(resolve(here, "dist", helloPath), "utf8");
  const diskAssetBytes = new Uint8Array(await readFile(resolve(here, "dist", asset.path)));
  const diskAssetSha256 = createHash("sha256").update(diskAssetBytes).digest("hex");
  if (diskHtml !== helloHtml || diskAssetSha256 !== asset.sha256) {
    throw new Error("On-disk article or asset bytes differ from the DeployArtifact");
  }

  logger.info("Build complete", {
    outcome: result.outcome,
    elapsedMs: result.elapsedMs,
    buildId: result.buildId,
    stages: result.stages.length,
    articles: articleRoutes.length,
    assets: articles.manifest.assets.length,
    surfaceFiles: surfaceRoutes.length,
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
