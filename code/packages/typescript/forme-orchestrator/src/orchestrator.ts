/**
 * `createOrchestrator` — factory for the runtime handle.
 *
 * Holds the persistent cache backend, default logger, and any other
 * cross-call state the orchestrator needs.  The returned `Orchestrator`
 * exposes the three FM03 §3.1 lifecycle methods: `buildPipeline`,
 * `runOnce`, `dispose`.
 *
 * v0 omits `watch` (FM03 §7) — that lands in a follow-up package.
 */

import { memoryCache } from "@coding-adventures/forme-cache";
import { silentLogger } from "@coding-adventures/forme-stage";
import { buildPipeline } from "./build-pipeline.js";
import { runOnce } from "./run.js";
import type {
  Orchestrator,
  OrchestratorOptions,
  Pipeline,
  RunOptions,
  RunResult,
} from "./types.js";

class OrchestratorImpl implements Orchestrator {
  private disposed = false;
  constructor(private readonly options: Required<OrchestratorOptions>) {}

  async buildPipeline(config: Parameters<Orchestrator["buildPipeline"]>[0]): Promise<Pipeline> {
    this.assertNotDisposed();
    return buildPipeline(config);
  }

  async runOnce(pipeline: Pipeline, options?: RunOptions): Promise<RunResult> {
    this.assertNotDisposed();
    return runOnce(pipeline, options, { logger: this.options.logger });
  }

  async dispose(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    await this.options.cache.dispose();
  }

  private assertNotDisposed(): void {
    if (this.disposed) {
      throw new Error("Orchestrator: instance has been disposed");
    }
  }
}

/**
 * Build a new orchestrator instance.  Defaults to an in-memory cache
 * and a silent logger — callers (CLI, dev-server) plug in
 * `consoleLogger()` and `filesystemCache(dir)` as needed.
 */
export function createOrchestrator(
  options: OrchestratorOptions = {},
): Orchestrator {
  return new OrchestratorImpl({
    cache: options.cache ?? memoryCache(),
    logger: options.logger ?? silentLogger(),
  });
}
