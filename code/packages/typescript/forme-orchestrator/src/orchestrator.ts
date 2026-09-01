/**
 * `createOrchestrator` — factory for the runtime handle.
 *
 * Holds the persistent cache backend, default logger, and any other
 * cross-call state the orchestrator needs.  The returned `Orchestrator`
 * exposes the FM03 §3.1 lifecycle methods, including a host-driven watch
 * session. Pure stage invocations reuse the injected cache; exact external-
 * state revisions and affected-stage scheduling remain a follow-up.
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
  constructor(private readonly options: Required<OrchestratorOptions>) {}

  async buildPipeline(config: Parameters<Orchestrator["buildPipeline"]>[0]): Promise<Pipeline> {
    this.assertNotDisposed();
    return buildPipeline(config);
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
    await Promise.all([...this.sessions].map(session => session.stop()));
    this.sessions.clear();
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
