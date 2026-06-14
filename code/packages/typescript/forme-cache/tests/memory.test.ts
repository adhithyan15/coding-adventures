/**
 * forme-cache — memory backend tests
 */

import { describe, it, expect } from "vitest";
import { makeEntry, memoryCache } from "../src/index.js";

describe("memoryCache — get/put", () => {
  it("returns null on miss", async () => {
    const c = memoryCache();
    expect(await c.get("never-set")).toBeNull();
  });

  it("get after put returns the entry", async () => {
    const c = memoryCache();
    const entry = makeEntry(new Uint8Array([1, 2, 3]));
    await c.put("k", entry);
    const out = await c.get("k");
    expect(out).toEqual(entry);
  });

  it("put overwrites existing entry under the same key", async () => {
    const c = memoryCache();
    await c.put("k", makeEntry(new Uint8Array([1])));
    await c.put("k", makeEntry(new Uint8Array([2])));
    const out = await c.get("k");
    expect(out!.payload).toEqual(new Uint8Array([2]));
  });
});

describe("memoryCache — integrity", () => {
  it("returns null AND invalidates a corrupt entry", async () => {
    const c = memoryCache();
    const good = makeEntry(new Uint8Array([1, 2, 3]));
    // Bypass makeEntry's consistency to inject a corrupt entry.
    await c.put("k", { ...good, contentHash: "0".repeat(64) });
    expect(await c.get("k")).toBeNull();
    // Invalidation cleanup happened — a subsequent put should
    // succeed cleanly.
    await c.put("k", good);
    expect((await c.get("k"))!.payload).toEqual(good.payload);
  });
});

describe("memoryCache — invalidate", () => {
  it("removes a single entry", async () => {
    const c = memoryCache();
    await c.put("k", makeEntry(new Uint8Array([1])));
    await c.invalidate("k");
    expect(await c.get("k")).toBeNull();
  });

  it("is a no-op for absent keys", async () => {
    const c = memoryCache();
    await expect(c.invalidate("never-set")).resolves.toBeUndefined();
  });
});

describe("memoryCache — invalidatePrefix", () => {
  it("removes every entry whose key starts with the prefix", async () => {
    const c = memoryCache();
    await c.put("aa-1", makeEntry(new Uint8Array([1])));
    await c.put("aa-2", makeEntry(new Uint8Array([2])));
    await c.put("bb-1", makeEntry(new Uint8Array([3])));
    await c.invalidatePrefix?.("aa");
    expect(await c.get("aa-1")).toBeNull();
    expect(await c.get("aa-2")).toBeNull();
    expect((await c.get("bb-1"))!.payload).toEqual(new Uint8Array([3]));
  });

  it("empty prefix clears the entire cache", async () => {
    const c = memoryCache();
    await c.put("a", makeEntry(new Uint8Array([1])));
    await c.put("b", makeEntry(new Uint8Array([2])));
    await c.invalidatePrefix?.("");
    expect(await c.get("a")).toBeNull();
    expect(await c.get("b")).toBeNull();
  });
});

describe("memoryCache — gc", () => {
  it("removes entries older than the cutoff", async () => {
    const c = memoryCache();
    const now = Date.now();
    await c.put("old", makeEntry(new Uint8Array([1]), () => now - 60_000));
    await c.put("new", makeEntry(new Uint8Array([2]), () => now));
    const removed = await c.gc(30_000); // older than 30s
    expect(removed).toBe(1);
    expect(await c.get("old")).toBeNull();
    expect((await c.get("new"))!.payload).toEqual(new Uint8Array([2]));
  });

  it("removes nothing if every entry is fresh", async () => {
    const c = memoryCache();
    await c.put("a", makeEntry(new Uint8Array([1])));
    expect(await c.gc(60_000)).toBe(0);
  });
});

describe("memoryCache — dispose", () => {
  it("makes subsequent operations throw", async () => {
    const c = memoryCache();
    await c.put("k", makeEntry(new Uint8Array([1])));
    await c.dispose();
    await expect(c.get("k")).rejects.toThrow(/disposed/);
    await expect(c.put("k", makeEntry(new Uint8Array([2])))).rejects.toThrow(/disposed/);
    await expect(c.invalidate("k")).rejects.toThrow(/disposed/);
    await expect(c.invalidatePrefix?.("k")).rejects.toThrow(/disposed/);
    await expect(c.gc(0)).rejects.toThrow(/disposed/);
  });

  it("dispose is itself idempotent", async () => {
    const c = memoryCache();
    await c.dispose();
    await expect(c.dispose()).resolves.toBeUndefined();
  });
});
