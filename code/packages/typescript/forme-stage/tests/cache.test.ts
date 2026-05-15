/**
 * forme-stage — in-memory cache tests
 */

import { describe, it, expect, vi } from "vitest";
import { inMemoryCache } from "../src/index.js";

describe("inMemoryCache.getOrCompute", () => {
  it("computes and stores on miss", async () => {
    const c = inMemoryCache();
    const compute = vi.fn(async () => 42);
    expect(await c.getOrCompute("k", compute)).toBe(42);
    expect(compute).toHaveBeenCalledTimes(1);
  });

  it("returns cached value on hit (compute not invoked again)", async () => {
    const c = inMemoryCache();
    const compute = vi.fn(async () => 42);
    await c.getOrCompute("k", compute);
    await c.getOrCompute("k", compute);
    expect(compute).toHaveBeenCalledTimes(1);
  });

  it("coalesces concurrent misses into a single computation", async () => {
    const c = inMemoryCache();
    let resolveCompute!: (v: number) => void;
    const compute = vi.fn(() => new Promise<number>(resolve => {
      resolveCompute = resolve;
    }));
    const p1 = c.getOrCompute("k", compute);
    const p2 = c.getOrCompute("k", compute);
    expect(compute).toHaveBeenCalledTimes(1); // shared
    resolveCompute(99);
    expect(await p1).toBe(99);
    expect(await p2).toBe(99);
  });

  it("drops the entry on rejection so retries succeed", async () => {
    const c = inMemoryCache();
    let attempts = 0;
    const compute = async () => {
      attempts++;
      if (attempts === 1) throw new Error("boom");
      return "ok";
    };
    await expect(c.getOrCompute("k", compute)).rejects.toThrow("boom");
    expect(await c.getOrCompute("k", compute)).toBe("ok");
    expect(attempts).toBe(2);
  });
});

describe("inMemoryCache.invalidate", () => {
  it("drops a stored entry", async () => {
    const c = inMemoryCache();
    let n = 0;
    const compute = async () => ++n;
    await c.getOrCompute("k", compute); // n=1
    await c.invalidate("k");
    await c.getOrCompute("k", compute); // n=2
    expect(n).toBe(2);
  });

  it("is a silent no-op for absent keys", async () => {
    const c = inMemoryCache();
    await expect(c.invalidate("never-set")).resolves.toBeUndefined();
  });
});

describe("inMemoryCache.keyFor", () => {
  it("joins primitives", () => {
    const c = inMemoryCache();
    expect(c.keyFor(["stage", "v1", 42])).toBe("stage\x1Fv1\x1F42");
  });

  it("escapes embedded separators so distinct parts can't collide", () => {
    const c = inMemoryCache();
    // Without escaping, ["a:b", "c"] could collide with ["a", "b:c"] — make sure
    // any literal Unit Separator in input is escaped before joining.
    const a = c.keyFor(["a\x1Fb", "c"]);
    const b = c.keyFor(["a", "b\x1Fc"]);
    expect(a).not.toBe(b);
  });

  it("handles empty input", () => {
    const c = inMemoryCache();
    expect(c.keyFor([])).toBe("");
  });

  it("converts numbers to strings", () => {
    const c = inMemoryCache();
    expect(c.keyFor([1, 2, 3])).toBe("1\x1F2\x1F3");
  });
});
