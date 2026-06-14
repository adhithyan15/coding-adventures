/**
 * src/build.ts — drive the Forme orchestrator end-to-end.
 *
 * `buildBlog({ contentRoot, outDir, logger? })` is the function the
 * CLI and the test both call.  It:
 *
 *   1. Builds the PipelineConfig from `makePipelineConfig`.
 *   2. Creates an orchestrator with the (optional) logger.
 *   3. Validates the config + constructs the typed DAG via
 *      `orchestrator.buildPipeline(config)`.
 *   4. Runs the DAG once via `orchestrator.runOnce(pipeline)`.
 *   5. Calls `orchestrator.dispose()` regardless of outcome.
 *   6. Returns the RunResult to the caller — the caller decides
 *      whether to exit with a non-zero code, snapshot the output,
 *      pretty-print summaries, etc.
 *
 * The orchestrator's `dispose()` is wrapped in try/finally so that a
 * thrown error in `runOnce` (e.g. a CapabilityError on a
 * misconfigured stage) still releases the cache + watcher handles.
 * This matches the FM03 §3.2 "Dispose" invariant.
 *
 * @module build
 */

import {
  createOrchestrator,
  type RunResult,
} from "@coding-adventures/forme-orchestrator";
import { silentLogger, type Logger } from "@coding-adventures/forme-stage";
import { makePipelineConfig } from "./config.js";

/** Inputs to {@link buildBlog}. */
export interface BuildBlogOptions {
  /** Absolute path to the directory containing `*.md` files. */
  readonly contentRoot: string;
  /** Absolute path to the directory the build writes HTML into. */
  readonly outDir: string;
  /**
   * Logger sink.  Defaults to `silentLogger()` — the CLI overrides
   * with `consoleLogger()`; the e2e test leaves it silent so vitest
   * output stays clean.
   */
  readonly logger?: Logger;
  /** Forwarded to `makePipelineConfig`.  Default: false. */
  readonly reproducibleBuild?: boolean;
}

/**
 * Run the hello-world Forme pipeline once.  Returns the orchestrator's
 * `RunResult` — the caller decides what to do with non-success outcomes
 * and which fields to surface (the CLI prints stage summaries; the
 * test asserts on `outcome` + the DeployArtifact + the file on disk).
 */
export async function buildBlog(options: BuildBlogOptions): Promise<RunResult> {
  const {
    contentRoot,
    outDir,
    logger = silentLogger(),
    reproducibleBuild = false,
  } = options;

  const config = makePipelineConfig({
    contentRoot,
    outDir,
    reproducibleBuild,
  });

  const orchestrator = createOrchestrator({ logger });
  try {
    const pipeline = await orchestrator.buildPipeline(config);
    const result = await orchestrator.runOnce(pipeline);
    return result;
  } finally {
    // Dispose ALWAYS runs — matches FM03 §3.2 "Dispose" invariant
    // ("every `dispose` hook has been called regardless of outcome").
    await orchestrator.dispose();
  }
}
