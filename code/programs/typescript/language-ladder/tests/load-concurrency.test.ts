// The corpus loader must never open every lesson at once.
//
// `loadBundledLessons` used to start all of its lazy `?raw` loaders together
// (a bare `Promise.all`). With tens of thousands of lessons that exhausted the
// Windows runner's file handles (`EMFILE: too many open files`). These tests
// pin the throttle that replaced it: results in input order, never more than
// `limit` calls in flight, and errors that still reach the caller.
import { describe, expect, it } from "vitest";
import { LESSON_LOAD_CONCURRENCY, mapWithConcurrency } from "../src/lessons.ts";

/** A promise we resolve by hand, so the test controls when each "file" arrives. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => (resolve = r));
  return { promise, resolve };
}

describe("mapWithConcurrency", () => {
  it("returns results in input order, whatever order the work finishes in", async () => {
    const delays = [30, 5, 20, 1, 10, 0, 15];
    const out = await mapWithConcurrency(delays, 3, async (ms, i) => {
      await new Promise((r) => setTimeout(r, ms));
      return `${i}:${ms}`;
    });
    expect(out).toEqual(delays.map((ms, i) => `${i}:${ms}`));
  });

  it("never has more than `limit` calls in flight", async () => {
    let inFlight = 0;
    let peak = 0;
    const items = Array.from({ length: 500 }, (_, i) => i);
    const out = await mapWithConcurrency(items, 7, async (n) => {
      inFlight++;
      peak = Math.max(peak, inFlight);
      await Promise.resolve();
      await new Promise((r) => setTimeout(r, n % 3));
      inFlight--;
      return n * 2;
    });
    expect(peak).toBe(7);
    expect(out).toEqual(items.map((n) => n * 2));
  });

  it("starts the next call as soon as any earlier one settles (a throttle, not batches)", async () => {
    const gates = [deferred<string>(), deferred<string>(), deferred<string>()];
    const started: number[] = [];
    const run = mapWithConcurrency([0, 1, 2], 2, (i) => {
      started.push(i);
      return gates[i]!.promise;
    });
    await Promise.resolve();
    expect(started).toEqual([0, 1]);
    // Finishing the SECOND call frees a slot even though the first is still open.
    gates[1]!.resolve("b");
    await new Promise((r) => setTimeout(r, 0));
    expect(started).toEqual([0, 1, 2]);
    gates[0]!.resolve("a");
    gates[2]!.resolve("c");
    expect(await run).toEqual(["a", "b", "c"]);
  });

  it("rejects with the first error, as Promise.all does", async () => {
    await expect(
      mapWithConcurrency([1, 2, 3, 4], 2, async (n) => {
        if (n === 3) throw new Error("boom at 3");
        return n;
      }),
    ).rejects.toThrow("boom at 3");
  });

  it("handles an empty list and a limit larger than the list", async () => {
    expect(await mapWithConcurrency([], 4, async (n: number) => n)).toEqual([]);
    expect(await mapWithConcurrency([1, 2], 100, async (n) => n + 1)).toEqual([2, 3]);
  });

  it("refuses a limit that would deadlock or make no sense", async () => {
    for (const bad of [0, -1, 1.5, Number.NaN]) {
      await expect(mapWithConcurrency([1], bad, async (n) => n)).rejects.toThrow(RangeError);
    }
  });

  it("keeps the corpus throttle well under every runner's file-handle limit", () => {
    expect(LESSON_LOAD_CONCURRENCY).toBeGreaterThanOrEqual(8);
    expect(LESSON_LOAD_CONCURRENCY).toBeLessThanOrEqual(256);
  });
});
