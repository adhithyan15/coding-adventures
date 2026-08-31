import { createCancellationTokenSource, type CancellationTokenSource } from "@coding-adventures/forme-stage";
import type {
  Pipeline,
  RunOptions,
  RunResult,
  WatchOptions,
  WatchSession,
} from "./types.js";

type Runner = (pipeline: Pipeline, options?: RunOptions) => Promise<RunResult>;

interface RebuildWaiter {
  resolve(result: RunResult): void;
  reject(error: Error): void;
}

/**
 * Create the FM03 watch loop around an injected runner.
 *
 * FM-B009 deliberately re-runs the complete pipeline. The persistent cache
 * and exact changed-and-downstream affected set belong to FM-B010; this loop
 * provides lifecycle, coalescing, cancellation, and the result stream.
 */
export function createWatchSession(
  pipeline: Pipeline,
  options: WatchOptions,
  runner: Runner,
): WatchSession {
  return new WatchSessionImpl(pipeline, options, runner);
}

class WatchSessionImpl implements WatchSession {
  private readonly iterator: AsyncIterator<unknown>;
  private readonly debounceMs: number;
  private readonly queue: RunResult[] = [];
  private readonly readers: Array<{
    resolve(result: IteratorResult<RunResult>): void;
    reject(error: Error): void;
  }> = [];
  private pending = false;
  private pendingWaiters: RebuildWaiter[] = [];
  private active: Promise<void> | null = null;
  private activeCancellation: CancellationTokenSource | null = null;
  private timer: ReturnType<typeof setTimeout> | null = null;
  private stopped = false;
  private terminalError: Error | null = null;

  constructor(
    private readonly pipeline: Pipeline,
    options: WatchOptions,
    private readonly runner: Runner,
  ) {
    const debounceMs = options.debounceMs ?? 200;
    if (!Number.isInteger(debounceMs) || debounceMs < 0) {
      throw new Error("watch debounceMs must be a non-negative integer");
    }
    this.debounceMs = debounceMs;
    this.iterator = options.changes[Symbol.asyncIterator]();
    options.cancellation?.onCancel(() => { void this.stop(); });
    this.requestBuild();
    void this.consumeChanges().catch(error => this.fail(error));
  }

  async *results(): AsyncIterable<RunResult> {
    while (true) {
      const result = await this.nextResult();
      if (result.done) return;
      yield result.value;
    }
  }

  rebuild(): Promise<RunResult> {
    if (this.stopped) return Promise.reject(new Error("watch session is stopped"));
    return new Promise<RunResult>((resolve, reject) => {
      this.requestBuild({ resolve, reject });
    });
  }

  async stop(): Promise<void> {
    if (this.stopped) return;
    this.stopped = true;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.activeCancellation?.cancel("watch session stopped");
    await this.iterator.return?.();
    if (this.active !== null) await this.active;
    const error = new Error("watch session is stopped");
    for (const waiter of this.pendingWaiters.splice(0)) waiter.reject(error);
    for (const reader of this.readers.splice(0)) reader.resolve({ done: true, value: undefined });
  }

  private async consumeChanges(): Promise<void> {
    try {
      while (!this.stopped) {
        const next = await this.iterator.next();
        if (next.done || this.stopped) return;
        if (this.timer !== null) clearTimeout(this.timer);
        this.timer = setTimeout(() => {
          this.timer = null;
          this.requestBuild();
        }, this.debounceMs);
      }
    } finally {
      if (this.timer !== null) clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private requestBuild(waiter?: RebuildWaiter): void {
    if (this.stopped || this.terminalError !== null) {
      waiter?.reject(new Error("watch session is stopped"));
      return;
    }
    if (this.active !== null) {
      this.pending = true;
      if (waiter !== undefined) this.pendingWaiters.push(waiter);
      return;
    }
    this.launch(waiter === undefined ? [] : [waiter]);
  }

  private launch(waiters: RebuildWaiter[]): void {
    const cancellation = createCancellationTokenSource();
    this.activeCancellation = cancellation;
    this.active = this.runner(this.pipeline, { cancellation: cancellation.token })
      .then(result => {
        if (!this.stopped && this.terminalError === null) this.pushResult(result);
        for (const waiter of waiters) waiter.resolve(result);
      })
      .catch(error => {
        const failure = error instanceof Error ? error : new Error(String(error));
        for (const waiter of waiters) waiter.reject(failure);
        this.fail(failure);
      })
      .finally(() => {
        this.active = null;
        this.activeCancellation = null;
        if (!this.stopped && this.terminalError === null && this.pending) {
          this.pending = false;
          const nextWaiters = this.pendingWaiters.splice(0);
          this.launch(nextWaiters);
        }
      });
  }

  private nextResult(): Promise<IteratorResult<RunResult>> {
    const value = this.queue.shift();
    if (value !== undefined) return Promise.resolve({ done: false, value });
    if (this.terminalError !== null) return Promise.reject(this.terminalError);
    if (this.stopped) return Promise.resolve({ done: true, value: undefined });
    return new Promise((resolve, reject) => this.readers.push({ resolve, reject }));
  }

  private pushResult(result: RunResult): void {
    const reader = this.readers.shift();
    if (reader !== undefined) reader.resolve({ done: false, value: result });
    else this.queue.push(result);
  }

  private fail(error: unknown): void {
    if (this.stopped || this.terminalError !== null) return;
    const failure = error instanceof Error ? error : new Error(String(error));
    this.terminalError = failure;
    if (this.timer !== null) clearTimeout(this.timer);
    this.timer = null;
    this.activeCancellation?.cancel("watch change stream failed");
    void this.iterator.return?.();
    for (const waiter of this.pendingWaiters.splice(0)) waiter.reject(failure);
    for (const reader of this.readers.splice(0)) reader.reject(failure);
  }
}
