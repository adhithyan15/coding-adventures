/**
 * forme-cache — filesystem backend tests
 *
 * Each test uses a fresh temp directory under `os.tmpdir()` so they
 * don't interfere with each other or with concurrent CI runs.
 */

import { afterEach, beforeEach, describe, it, expect } from "vitest";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { filesystemCache, makeEntry } from "../src/index.js";

let root: string;

beforeEach(async () => {
  root = await mkdtemp(join(tmpdir(), "forme-cache-test-"));
});

afterEach(async () => {
  await rm(root, { recursive: true, force: true });
});

describe("filesystemCache — construction", () => {
  it("rejects empty root", () => {
    expect(() => filesystemCache("")).toThrow(/root/);
  });

  it("rejects non-string root", () => {
    expect(() => filesystemCache(null as never)).toThrow(/root/);
  });
});

describe("filesystemCache — get/put", () => {
  it("returns null on miss", async () => {
    const c = filesystemCache(root);
    expect(await c.get("aabbccdd")).toBeNull();
  });

  it("get-after-put round-trips", async () => {
    const c = filesystemCache(root);
    const entry = makeEntry(new Uint8Array([10, 20, 30, 40]));
    await c.put("aabbccdd", entry);
    const out = await c.get("aabbccdd");
    expect(out).not.toBeNull();
    expect(out!.payload).toEqual(entry.payload);
    expect(out!.contentHash).toBe(entry.contentHash);
    expect(out!.sizeBytes).toBe(entry.sizeBytes);
    expect(out!.writtenMs).toBe(entry.writtenMs);
  });

  it("put overwrites existing entry under the same key", async () => {
    const c = filesystemCache(root);
    await c.put("aabbccdd", makeEntry(new Uint8Array([1])));
    await c.put("aabbccdd", makeEntry(new Uint8Array([2])));
    const out = await c.get("aabbccdd");
    expect(out!.payload).toEqual(new Uint8Array([2]));
  });

  it("rejects keys shorter than 2 chars", async () => {
    const c = filesystemCache(root);
    await expect(c.put("a", makeEntry(new Uint8Array([1])))).rejects.toThrow(/too short/);
  });

  it("shards by first 2 chars of the key", async () => {
    const c = filesystemCache(root);
    await c.put("ababab", makeEntry(new Uint8Array([1])));
    await c.put("cdcdcd", makeEntry(new Uint8Array([2])));
    const { readdir } = await import("node:fs/promises");
    const shards = await readdir(root);
    expect(new Set(shards)).toEqual(new Set(["ab", "cd"]));
  });
});

describe("filesystemCache — integrity", () => {
  it("returns null AND removes a corrupt-payload entry", async () => {
    const c = filesystemCache(root);
    await c.put("aabbcc", makeEntry(new Uint8Array([1, 2, 3])));
    // Tamper with the payload file directly.
    await writeFile(join(root, "aa", "aabbcc.entry"), new Uint8Array([9, 9, 9]));
    expect(await c.get("aabbcc")).toBeNull();
    // The cleanup path should have removed both files; a re-put works cleanly.
    await c.put("aabbcc", makeEntry(new Uint8Array([5, 6, 7])));
    expect((await c.get("aabbcc"))!.payload).toEqual(new Uint8Array([5, 6, 7]));
  });

  it("returns null on corrupt JSON metadata", async () => {
    const c = filesystemCache(root);
    await c.put("aabbcc", makeEntry(new Uint8Array([1])));
    await writeFile(join(root, "aa", "aabbcc.meta"), "not json");
    expect(await c.get("aabbcc")).toBeNull();
  });
});

describe("filesystemCache — invalidate", () => {
  it("removes both files for a key", async () => {
    const c = filesystemCache(root);
    await c.put("aabbcc", makeEntry(new Uint8Array([1])));
    await c.invalidate("aabbcc");
    expect(await c.get("aabbcc")).toBeNull();
  });

  it("is a no-op for absent keys", async () => {
    const c = filesystemCache(root);
    await expect(c.invalidate("aabbcc")).resolves.toBeUndefined();
  });
});

describe("filesystemCache — invalidatePrefix", () => {
  it("removes keys starting with prefix, leaves others", async () => {
    const c = filesystemCache(root);
    await c.put("aabbcc", makeEntry(new Uint8Array([1])));
    await c.put("aadddd", makeEntry(new Uint8Array([2])));
    await c.put("bbeeff", makeEntry(new Uint8Array([3])));
    await c.invalidatePrefix?.("aa");
    expect(await c.get("aabbcc")).toBeNull();
    expect(await c.get("aadddd")).toBeNull();
    expect((await c.get("bbeeff"))!.payload).toEqual(new Uint8Array([3]));
  });

  it("empty prefix clears the cache and recreates the root", async () => {
    const c = filesystemCache(root);
    await c.put("aabbcc", makeEntry(new Uint8Array([1])));
    await c.invalidatePrefix?.("");
    expect(await c.get("aabbcc")).toBeNull();
    // A subsequent put should still work — the root was recreated.
    await c.put("aabbcc", makeEntry(new Uint8Array([2])));
    expect((await c.get("aabbcc"))!.payload).toEqual(new Uint8Array([2]));
  });

  it("invalidatePrefix is a no-op when root doesn't exist yet", async () => {
    const c = filesystemCache(root);
    await rm(root, { recursive: true, force: true });
    await expect(c.invalidatePrefix?.("aa")).resolves.toBeUndefined();
  });
});

describe("filesystemCache — gc", () => {
  it("removes entries older than the cutoff", async () => {
    const c = filesystemCache(root);
    const now = Date.now();
    await c.put("oldold", makeEntry(new Uint8Array([1]), () => now - 60_000));
    await c.put("newnew", makeEntry(new Uint8Array([2]), () => now));
    // Force the meta file's mtime back so the GC sees it as old.
    // (writtenMs in the JSON drives entry semantics; mtime on disk
    // drives gc — and we set them via a fresh write, so we have to
    // tweak mtime explicitly.)
    const { utimes } = await import("node:fs/promises");
    const past = (now - 60_000) / 1000;
    await utimes(join(root, "ol", "oldold.meta"), past, past);
    const removed = await c.gc(30_000);
    expect(removed).toBe(1);
    expect(await c.get("oldold")).toBeNull();
    expect((await c.get("newnew"))!.payload).toEqual(new Uint8Array([2]));
  });

  it("returns 0 when root doesn't exist", async () => {
    const c = filesystemCache(root);
    await rm(root, { recursive: true, force: true });
    expect(await c.gc(0)).toBe(0);
  });
});

describe("filesystemCache — dispose", () => {
  it("makes subsequent operations throw", async () => {
    const c = filesystemCache(root);
    await c.dispose();
    await expect(c.get("aabbcc")).rejects.toThrow(/disposed/);
    await expect(c.put("aabbcc", makeEntry(new Uint8Array([1])))).rejects.toThrow(/disposed/);
  });
});
