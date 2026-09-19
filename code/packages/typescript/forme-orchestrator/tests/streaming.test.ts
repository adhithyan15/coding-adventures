import { describe, expect, it, vi } from "vitest";
import { CancellationError } from "@coding-adventures/forme-errors";
import {
  createCancellationTokenSource,
  neverCancelledToken,
} from "@coding-adventures/forme-stage";
import {
  createBoundedFanOut,
  DEFAULT_STREAM_WINDOW,
} from "../src/streaming.js";

describe("bounded stream fan-out", () => {
  it("opens the source lazily, pulls each value once, and preserves branch order", async () => {
    let opened = 0;
    let pulls = 0;
    const source: AsyncIterable<number> = {
      [Symbol.asyncIterator]() {
        opened += 1;
        let value = 0;
        return {
          async next() {
            pulls += 1;
            return value < 4
              ? { done: false, value: value++ }
              : { done: true, value: undefined };
          },
        };
      },
    };

    const fanOut = createBoundedFanOut(source, 2, neverCancelledToken(), 4);
    expect(opened).toBe(0);
    expect(fanOut.stats()).toEqual({
      activeBranches: 2,
      upstreamPulls: 0,
      retainedValues: 0,
      peakRetainedValues: 0,
    });

    const [left, right] = fanOut.branches;
    expect(await collect(left!)).toEqual([0, 1, 2, 3]);
    expect(await collect(right!)).toEqual([0, 1, 2, 3]);
    expect(opened).toBe(1);
    expect(pulls).toBe(5);
    expect(fanOut.stats().upstreamPulls).toBe(5);
  });

  it("backpressures a fast branch when a slow branch fills its window", async () => {
    const fanOut = createBoundedFanOut(range(5), 2, neverCancelledToken(), 2);
    const fast = iterator(fanOut.branches[0]!);
    const slow = iterator(fanOut.branches[1]!);

    await expect(fast.next()).resolves.toEqual({ done: false, value: 0 });
    await expect(fast.next()).resolves.toEqual({ done: false, value: 1 });
    expect(fanOut.stats()).toMatchObject({
      upstreamPulls: 2,
      retainedValues: 2,
      peakRetainedValues: 2,
    });

    let settled = false;
    const blocked = fast.next().finally(() => { settled = true; });
    await flushMicrotasks();
    expect(settled).toBe(false);
    expect(fanOut.stats().upstreamPulls).toBe(2);

    await expect(slow.next()).resolves.toEqual({ done: false, value: 0 });
    await expect(blocked).resolves.toEqual({ done: false, value: 2 });
    expect(fanOut.stats().retainedValues).toBeLessThanOrEqual(2);
  });

  it("detaches a returned branch and immediately releases its backpressure", async () => {
    const upstreamReturn = vi.fn(async () => ({ done: true, value: undefined }));
    let value = 0;
    const source: AsyncIterable<number> = {
      [Symbol.asyncIterator]() {
        return {
          async next() { return { done: false as const, value: value++ }; },
          return: upstreamReturn,
        };
      },
    };
    const fanOut = createBoundedFanOut(source, 2, neverCancelledToken(), 1);
    const fast = iterator(fanOut.branches[0]!);
    const slow = iterator(fanOut.branches[1]!);

    await expect(fast.next()).resolves.toEqual({ done: false, value: 0 });
    const blocked = fast.next();
    await flushMicrotasks();
    expect(fanOut.stats().upstreamPulls).toBe(1);

    await expect(slow.return?.()).resolves.toEqual({ done: true, value: undefined });
    await expect(blocked).resolves.toEqual({ done: false, value: 1 });
    expect(fanOut.stats().activeBranches).toBe(1);
    expect(fanOut.stats().retainedValues).toBe(0);
    expect(upstreamReturn).not.toHaveBeenCalled();

    await expect(fast.return?.()).resolves.toEqual({ done: true, value: undefined });
    expect(upstreamReturn).toHaveBeenCalledTimes(1);
  });

  it("delivers an upstream failure to every branch after its queued prefix", async () => {
    const failure = new Error("source exploded");
    const source = (async function* () {
      yield 10;
      yield 20;
      throw failure;
    })();
    const fanOut = createBoundedFanOut(source, 2, neverCancelledToken(), 4);
    const fast = iterator(fanOut.branches[0]!);
    const slow = iterator(fanOut.branches[1]!);

    await expect(fast.next()).resolves.toEqual({ done: false, value: 10 });
    await expect(fast.next()).resolves.toEqual({ done: false, value: 20 });
    await expect(fast.next()).rejects.toBe(failure);

    await expect(slow.next()).resolves.toEqual({ done: false, value: 10 });
    await expect(slow.next()).resolves.toEqual({ done: false, value: 20 });
    await expect(slow.next()).rejects.toBe(failure);
  });

  it("propagates cancellation, clears buffers, and closes upstream once", async () => {
    const cancellation = createCancellationTokenSource();
    const upstreamReturn = vi.fn(async () => ({ done: true, value: undefined }));
    let releasePull: ((result: IteratorResult<number>) => void) | undefined;
    const source: AsyncIterable<number> = {
      [Symbol.asyncIterator]() {
        return {
          next: () => new Promise<IteratorResult<number>>(resolve => { releasePull = resolve; }),
          return: upstreamReturn,
        };
      },
    };
    const fanOut = createBoundedFanOut(source, 2, cancellation.token);
    const left = iterator(fanOut.branches[0]!);
    const pending = left.next();
    await flushMicrotasks();

    cancellation.cancel("stop the pipeline");
    await expect(pending).rejects.toBeInstanceOf(CancellationError);
    await flushMicrotasks();
    expect(upstreamReturn).toHaveBeenCalledTimes(1);
    expect(fanOut.stats()).toMatchObject({ activeBranches: 0, retainedValues: 0 });

    releasePull?.({ done: false, value: 99 });
    await flushMicrotasks();
    expect(fanOut.stats().retainedValues).toBe(0);
  });

  it("rejects concurrent reads and a second iterator for one branch", async () => {
    let releasePull: ((result: IteratorResult<number>) => void) | undefined;
    const source: AsyncIterable<number> = {
      [Symbol.asyncIterator]() {
        return {
          next: () => new Promise<IteratorResult<number>>(resolve => { releasePull = resolve; }),
        };
      },
    };
    const fanOut = createBoundedFanOut(source, 1, neverCancelledToken());
    const branch = fanOut.branches[0]!;
    const first = iterator(branch);
    const pending = first.next();
    await expect(first.next()).rejects.toThrow("one unresolved next() call");
    expect(() => branch[Symbol.asyncIterator]()).toThrow("single-use");
    releasePull?.({ done: true, value: undefined });
    await expect(pending).resolves.toEqual({ done: true, value: undefined });
  });

  it("bounds retained values across several branches and a stream larger than 64", async () => {
    const fanOut = createBoundedFanOut(
      range(DEFAULT_STREAM_WINDOW * 3 + 1),
      3,
      neverCancelledToken(),
    );
    const outputs = await Promise.all(fanOut.branches.map(collect));
    expect(outputs[0]).toHaveLength(DEFAULT_STREAM_WINDOW * 3 + 1);
    expect(outputs[1]).toEqual(outputs[0]);
    expect(outputs[2]).toEqual(outputs[0]);
    expect(fanOut.stats().peakRetainedValues)
      .toBeLessThanOrEqual(DEFAULT_STREAM_WINDOW * 2);
  });

  it.each([
    [-1, DEFAULT_STREAM_WINDOW, "branch count"],
    [1.5, DEFAULT_STREAM_WINDOW, "branch count"],
    [1, 0, "window size"],
    [1, Number.POSITIVE_INFINITY, "window size"],
  ])("rejects invalid bounds: branches=%s window=%s", (branches, window, label) => {
    expect(() => createBoundedFanOut(range(1), branches, neverCancelledToken(), window))
      .toThrow(label);
  });

  it("does not open an upstream when there are no consumers", () => {
    const open = vi.fn(() => iterator(range(1)));
    const fanOut = createBoundedFanOut({ [Symbol.asyncIterator]: open }, 0, neverCancelledToken());
    expect(fanOut.branches).toEqual([]);
    expect(open).not.toHaveBeenCalled();
  });
});

function range(count: number): AsyncIterable<number> {
  return (async function* () {
    for (let value = 0; value < count; value += 1) yield value;
  })();
}

function iterator<T>(source: AsyncIterable<T>): AsyncIterator<T> {
  return source[Symbol.asyncIterator]();
}

async function collect<T>(source: AsyncIterable<T>): Promise<T[]> {
  const values: T[] = [];
  for await (const value of source) values.push(value);
  return values;
}

async function flushMicrotasks(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
}
