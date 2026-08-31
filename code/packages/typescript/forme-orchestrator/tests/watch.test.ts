import { describe, expect, it } from "vitest";
import { createCancellationTokenSource } from "@coding-adventures/forme-stage";
import { createWatchSession, type Pipeline, type RunResult } from "../src/index.js";

class ChangeStream implements AsyncIterable<unknown>, AsyncIterator<unknown> {
  private readonly queued: unknown[] = [];
  private readonly readers: Array<{
    resolve(value: IteratorResult<unknown>): void;
    reject(error: Error): void;
  }> = [];
  private closed = false;

  [Symbol.asyncIterator](): AsyncIterator<unknown> { return this; }

  next(): Promise<IteratorResult<unknown>> {
    const value = this.queued.shift();
    if (value !== undefined) return Promise.resolve({ done: false, value });
    if (this.closed) return Promise.resolve({ done: true, value: undefined });
    return new Promise((resolve, reject) => this.readers.push({ resolve, reject }));
  }

  return(): Promise<IteratorResult<unknown>> {
    this.closed = true;
    for (const reader of this.readers.splice(0)) reader.resolve({ done: true, value: undefined });
    return Promise.resolve({ done: true, value: undefined });
  }

  emit(value: unknown): void {
    const reader = this.readers.shift();
    if (reader !== undefined) reader.resolve({ done: false, value });
    else this.queued.push(value);
  }

  fail(error: Error): void {
    for (const reader of this.readers.splice(0)) reader.reject(error);
  }
}

const pipeline = {} as Pipeline;

function result(id: number, outcome: RunResult["outcome"] = "success"): RunResult {
  return {
    outcome,
    stages: [],
    outputs: {},
    errors: [],
    elapsedMs: id,
    buildId: `blake2b:${id}` as never,
  };
}

describe("watch session", () => {
  it("runs initially and coalesces a burst into one full rebuild", async () => {
    const changes = new ChangeStream();
    let calls = 0;
    const session = createWatchSession(pipeline, { changes, debounceMs: 5 }, async () => result(++calls));
    const results = session.results()[Symbol.asyncIterator]();

    expect((await results.next()).value?.buildId).toBe("blake2b:1");
    changes.emit("a");
    changes.emit("b");
    changes.emit("c");
    expect((await results.next()).value?.buildId).toBe("blake2b:2");
    expect(calls).toBe(2);

    await session.stop();
    expect((await results.next()).done).toBe(true);
  });

  it("queues one follow-up when files change during an active build", async () => {
    const changes = new ChangeStream();
    const releases: Array<() => void> = [];
    let calls = 0;
    const session = createWatchSession(pipeline, { changes, debounceMs: 0 }, async () => {
      const id = ++calls;
      await new Promise<void>(resolve => releases.push(resolve));
      return result(id);
    });
    const results = session.results()[Symbol.asyncIterator]();

    releases.shift()!();
    expect((await results.next()).value?.buildId).toBe("blake2b:1");
    changes.emit("first");
    await tick();
    changes.emit("while-building-1");
    changes.emit("while-building-2");
    await tick();
    releases.shift()!();
    expect((await results.next()).value?.buildId).toBe("blake2b:2");
    await tick();
    releases.shift()!();
    expect((await results.next()).value?.buildId).toBe("blake2b:3");
    expect(calls).toBe(3);
    await session.stop();
  });

  it("resolves manual rebuilds with their scheduled run", async () => {
    const changes = new ChangeStream();
    let calls = 0;
    const session = createWatchSession(pipeline, { changes }, async () => result(++calls));
    const results = session.results()[Symbol.asyncIterator]();
    await results.next();
    expect((await session.rebuild()).buildId).toBe("blake2b:2");
    expect((await results.next()).value?.buildId).toBe("blake2b:2");
    await session.stop();
    await expect(session.rebuild()).rejects.toThrow(/stopped/);
  });

  it("cancels the active run and closes results on stop", async () => {
    const changes = new ChangeStream();
    const external = createCancellationTokenSource();
    let activeCancelled = false;
    const session = createWatchSession(pipeline, {
      changes,
      cancellation: external.token,
    }, async (_pipeline, options) => {
      await new Promise<void>(resolve => options?.cancellation?.onCancel(() => {
        activeCancelled = true;
        resolve();
      }));
      return result(1, "cancelled");
    });
    const results = session.results()[Symbol.asyncIterator]();
    const pending = results.next();
    external.cancel("test complete");
    expect((await pending).done).toBe(true);
    expect(activeCancelled).toBe(true);
    await session.stop();
  });

  it("validates debounce configuration", () => {
    expect(() => createWatchSession(pipeline, {
      changes: new ChangeStream(),
      debounceMs: -1,
    }, async () => result(1))).toThrow(/non-negative integer/);
  });

  it("surfaces host watcher failures through the result stream", async () => {
    const changes = new ChangeStream();
    const session = createWatchSession(pipeline, { changes }, async () => result(1));
    const results = session.results()[Symbol.asyncIterator]();
    await results.next();
    const pending = results.next();
    changes.fail(new Error("filesystem watcher failed"));
    await expect(pending).rejects.toThrow("filesystem watcher failed");
    await session.stop();
  });

  it("surfaces unexpected runner failures through the result stream", async () => {
    const changes = new ChangeStream();
    const session = createWatchSession(pipeline, { changes }, async () => {
      throw new Error("pipeline host crashed");
    });
    const results = session.results()[Symbol.asyncIterator]();

    await expect(results.next()).rejects.toThrow("pipeline host crashed");
    await session.stop();
  });
});

function tick(): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, 5));
}
