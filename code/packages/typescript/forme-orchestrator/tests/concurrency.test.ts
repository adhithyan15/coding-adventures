import { describe, expect, it } from "vitest";
import { CancellationError } from "@coding-adventures/forme-errors";
import {
  createCancellationTokenSource,
  neverCancelledToken,
} from "@coding-adventures/forme-stage";
import { createConcurrencyPool } from "../src/concurrency.js";

describe("shared concurrency pool", () => {
  it.each([0, -1, 1.5, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])(
    "rejects invalid limit %s",
    limit => {
      expect(() => createConcurrencyPool(limit, neverCancelledToken()))
        .toThrow("positive safe integer");
    },
  );

  it("enforces the limit and starts queued work in FIFO order", async () => {
    const pool = createConcurrencyPool(2, neverCancelledToken());
    const releases = Array.from({ length: 5 }, deferred<void>);
    const started: number[] = [];
    const tasks = releases.map((gate, index) => pool.run(async () => {
      started.push(index);
      await gate.promise;
      return index;
    }));

    await flushMicrotasks();
    expect(started).toEqual([0, 1]);
    expect(pool.stats()).toEqual({ active: 2, queued: 3, peakActive: 2 });

    releases[0]!.resolve();
    await flushMicrotasks();
    expect(started).toEqual([0, 1, 2]);
    releases[1]!.resolve();
    await flushMicrotasks();
    expect(started).toEqual([0, 1, 2, 3]);
    releases[2]!.resolve();
    releases[3]!.resolve();
    await flushMicrotasks();
    expect(started).toEqual([0, 1, 2, 3, 4]);
    releases[4]!.resolve();
    await expect(Promise.all(tasks)).resolves.toEqual([0, 1, 2, 3, 4]);
    expect(pool.stats()).toEqual({ active: 0, queued: 0, peakActive: 2 });
  });

  it("releases capacity after synchronous and asynchronous failures", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const syncFailure = new Error("sync failure");
    const asyncFailure = new Error("async failure");

    await expect(pool.run(() => { throw syncFailure; })).rejects.toBe(syncFailure);
    await expect(pool.run(async () => { throw asyncFailure; })).rejects.toBe(asyncFailure);
    await expect(pool.run(async () => "recovered")).resolves.toBe("recovered");
    expect(pool.stats()).toEqual({ active: 0, queued: 0, peakActive: 1 });
  });

  it("rejects queued and future work on cancellation while active work unwinds", async () => {
    const cancellation = createCancellationTokenSource();
    const pool = createConcurrencyPool(1, cancellation.token);
    const activeGate = deferred<void>();
    const active = pool.run(async () => { await activeGate.promise; return "active"; });
    const queued = pool.run(async () => "queued");
    await flushMicrotasks();
    expect(pool.stats()).toMatchObject({ active: 1, queued: 1 });

    cancellation.cancel("stop scheduling");
    await expect(queued).rejects.toBeInstanceOf(CancellationError);
    await expect(pool.run(async () => "late")).rejects.toBeInstanceOf(CancellationError);
    expect(pool.stats()).toMatchObject({ active: 1, queued: 0 });

    activeGate.resolve();
    await expect(active).resolves.toBe("active");
    expect(pool.stats()).toMatchObject({ active: 0, queued: 0 });
  });

  it("lets a holder yield its only permit while waiting for upstream", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const upstream = deferred<number>();
    const trace: string[] = [];

    const consumer = pool.run(async permit => {
      trace.push("consumer-wait");
      const value = await permit.yieldWhile(() => upstream.promise);
      trace.push(`consumer-resume:${value}`);
    });
    const producer = pool.run(async () => {
      trace.push("producer");
      upstream.resolve(42);
    });

    await Promise.all([consumer, producer]);
    expect(trace).toEqual(["consumer-wait", "producer", "consumer-resume:42"]);
    expect(pool.stats()).toEqual({ active: 0, queued: 0, peakActive: 1 });
  });

  it("reacquires a yielded permit at the FIFO tail", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const wait = deferred<void>();
    const firstGate = deferred<void>();
    const secondGate = deferred<void>();
    const trace: string[] = [];

    const holder = pool.run(async permit => {
      trace.push("holder-yield");
      await permit.yieldWhile(() => wait.promise);
      trace.push("holder-resume");
    });
    const first = pool.run(async () => {
      trace.push("first");
      await firstGate.promise;
    });
    const second = pool.run(async () => {
      trace.push("second");
      await secondGate.promise;
    });
    await flushMicrotasks();
    expect(trace).toEqual(["holder-yield", "first"]);

    wait.resolve();
    firstGate.resolve();
    await flushMicrotasks();
    expect(trace).toEqual(["holder-yield", "first", "second"]);
    secondGate.resolve();
    await Promise.all([holder, first, second]);
    expect(trace).toEqual(["holder-yield", "first", "second", "holder-resume"]);
  });

  it("reacquires before exposing a failed wait to task code", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const failure = new Error("upstream failed");
    const trace: string[] = [];
    const holder = pool.run(async permit => {
      try {
        await permit.yieldWhile(async () => { throw failure; });
      } catch (error) {
        trace.push(`holder-catch:${pool.stats().active}`);
        throw error;
      }
    });
    const follower = pool.run(async () => { trace.push("follower"); });

    await expect(holder).rejects.toBe(failure);
    await expect(follower).resolves.toBeUndefined();
    expect(trace).toEqual(["follower", "holder-catch:1"]);
    expect(pool.stats()).toMatchObject({ active: 0, queued: 0 });
  });

  it("settles and releases an unawaited successful yield before rejecting the task", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const wait = deferred<void>();
    const task = pool.run(permit => {
      void permit.yieldWhile(() => wait.promise);
      return "returned too early";
    });
    await flushMicrotasks();
    expect(pool.stats()).toMatchObject({ active: 0 });

    wait.resolve();
    await expect(task).rejects.toThrow("must await yieldWhile");
    expect(pool.stats()).toMatchObject({ active: 0, queued: 0 });
    await expect(pool.run(async () => "still usable")).resolves.toBe("still usable");
  });

  it("contains an unawaited failed yield and releases its reacquired permit", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const wait = deferred<void>();
    const failure = new Error("discarded wait failed");
    const task = pool.run(permit => {
      void permit.yieldWhile(() => wait.promise);
    });
    await flushMicrotasks();

    wait.reject(failure);
    await expect(task).rejects.toBe(failure);
    expect(pool.stats()).toMatchObject({ active: 0, queued: 0 });
    await expect(pool.run(async () => "still usable")).resolves.toBe("still usable");
  });

  it("contains an unawaited yielded wait when cancellation fires", async () => {
    const cancellation = createCancellationTokenSource();
    const pool = createConcurrencyPool(1, cancellation.token);
    const wait = deferred<void>();
    const task = pool.run(permit => {
      void permit.yieldWhile(() => wait.promise);
    });
    await flushMicrotasks();

    cancellation.cancel("stop yielded work");
    await expect(task).rejects.toBeInstanceOf(CancellationError);
    expect(pool.stats()).toMatchObject({ active: 0, queued: 0 });
    wait.resolve();
    await flushMicrotasks();
  });

  it("rejects cancellation while a yielded holder waits to reacquire", async () => {
    const cancellation = createCancellationTokenSource();
    const pool = createConcurrencyPool(1, cancellation.token);
    const wait = deferred<void>();
    const blocker = deferred<void>();
    const holder = pool.run(async permit => permit.yieldWhile(() => wait.promise));
    const other = pool.run(async () => { await blocker.promise; });
    await flushMicrotasks();
    wait.resolve();
    await flushMicrotasks();
    expect(pool.stats()).toMatchObject({ active: 1, queued: 1 });

    cancellation.cancel("cancel reacquire");
    await expect(holder).rejects.toBeInstanceOf(CancellationError);
    blocker.resolve();
    await expect(other).resolves.toBeUndefined();
    expect(pool.stats()).toMatchObject({ active: 0, queued: 0 });
  });

  it("rejects concurrent yield attempts and use after the task completes", async () => {
    const pool = createConcurrencyPool(1, neverCancelledToken());
    const wait = deferred<void>();
    let savedPermit: { yieldWhile<T>(wait: () => Promise<T>): Promise<T> } | undefined;
    const task = pool.run(async permit => {
      savedPermit = permit;
      const pending = permit.yieldWhile(() => wait.promise);
      await expect(permit.yieldWhile(async () => undefined))
        .rejects.toThrow("one yield");
      wait.resolve();
      await pending;
    });
    await task;

    await expect(savedPermit!.yieldWhile(async () => undefined))
      .rejects.toThrow("active permit");
  });

  it("starts already-cancelled pools closed without running work", async () => {
    const cancellation = createCancellationTokenSource();
    cancellation.cancel("already done");
    const pool = createConcurrencyPool(2, cancellation.token);
    let ran = false;

    await expect(pool.run(async () => { ran = true; }))
      .rejects.toBeInstanceOf(CancellationError);
    expect(ran).toBe(false);
    expect(pool.stats()).toEqual({ active: 0, queued: 0, peakActive: 0 });
  });
});

interface Deferred<T> {
  readonly promise: Promise<T>;
  readonly resolve: (value: T) => void;
  readonly reject: (error: unknown) => void;
}

function deferred<T>(): Deferred<T> {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((innerResolve, innerReject) => {
    resolve = innerResolve;
    reject = innerReject;
  });
  return { promise, resolve, reject };
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}
