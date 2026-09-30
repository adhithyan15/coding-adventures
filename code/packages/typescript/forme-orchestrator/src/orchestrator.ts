/**
 * `createOrchestrator` — factory for the runtime handle.
 *
 * Holds the persistent cache backend, default logger, and any other
 * cross-call state the orchestrator needs.  The returned `Orchestrator`
 * exposes the FM03 §3.1 lifecycle methods, including a host-driven watch
 * session. Pure stage invocations reuse the injected cache; successful runs
 * persist source observations and per-instance revision state. Live streams
 * use bounded fan-out and content-addressed checkpoints across concurrent DAG
 * branches.
 */

import { memoryCache } from "@coding-adventures/forme-cache";
import { silentLogger } from "@coding-adventures/forme-stage";
import { buildPipeline } from "./build-pipeline.js";
import { runOnce } from "./run.js";
import { createWatchSession } from "./watch.js";
import type {
  Orchestrator,
  OrchestratorOptions,
  Pipeline,
  RunOptions,
  RunResult,
  WatchOptions,
  WatchSession,
} from "./types.js";

class OrchestratorImpl implements Orchestrator {
  private disposed = false;
  private readonly sessions = new Set<WatchSession>();
  constructor(private readonly options: {
    readonly cache: NonNullable<OrchestratorOptions["cache"]>;
    readonly logger: NonNullable<OrchestratorOptions["logger"]>;
    readonly pluginHost?: OrchestratorOptions["pluginHost"];
  }) {}

  async buildPipeline(config: Parameters<Orchestrator["buildPipeline"]>[0]): Promise<Pipeline> {
    this.assertNotDisposed();
    return buildPipeline(config, this.options.pluginHost);
  }

  async runOnce(pipeline: Pipeline, options?: RunOptions): Promise<RunResult> {
    this.assertNotDisposed();
    return runOnce(pipeline, options, {
      logger: this.options.logger,
      cache: this.options.cache,
    });
  }

  watch(pipeline: Pipeline, options: WatchOptions): WatchSession {
    this.assertNotDisposed();
    const inner = createWatchSession(
      pipeline,
      options,
      (value, runOptions) => this.runOnce(value, runOptions),
    );
    const session: WatchSession = {
      results: () => inner.results(),
      rebuild: () => inner.rebuild(),
      stop: async () => {
        await inner.stop();
        this.sessions.delete(session);
      },
    };
    this.sessions.add(session);
    return session;
  }

  async dispose(): Promise<void> {
    if (this.disposed) return;
    this.disposed = true;
    const sessionResults = await Promise.allSettled(
      [...this.sessions].map(session => session.stop()),
    );
    this.sessions.clear();
    const resourceResults = await Promise.allSettled([
      this.options.cache.dispose(),
      this.options.pluginHost?.dispose?.(),
    ]);
    const failures = [...sessionResults, ...resourceResults]
      .filter((result): result is PromiseRejectedResult => result.status === "rejected");
    if (failures.length > 0) throw new AggregateError(failures.map(result => result.reason));
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
    pluginHost: options.pluginHost,
  });
}
