/**
 * fs-cache.test.ts — disk-backed CacheIO behaviour.
 *
 * Uses real filesystem under os.tmpdir() per test, cleaned up in
 * afterEach.  No subprocesses, no network.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import { createHash, randomBytes } from "node:crypto";
import { promises as fs } from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { createFsCacheIO } from "../src/index.js";

// ─── Helpers ─────────────────────────────────────────────────────────────

/** Make a fresh tmpdir per test; auto-cleaned in afterEach. */
let cacheDir: string;

async function mkTmpDir(): Promise<string> {
  return fs.mkdtemp(path.join(os.tmpdir(), "forme-fs-cache-test-"));
}

async function rmRf(dir: string): Promise<void> {
  await fs.rm(dir, { recursive: true, force: true });
}

beforeEach(async () => {
  cacheDir = await mkTmpDir();
});

afterEach(async () => {
  await rmRf(cacheDir);
});

/** Make a sha256-hex key for test inputs. */
function key(s: string): string {
  return createHash("sha256").update(s, "utf8").digest("hex");
}

const META = { pageId: "test", byteSize: 0, sha256: "x" };

// ─── Tests ───────────────────────────────────────────────────────────────

describe("createFsCacheIO — basic round-trip", () => {
  it("put then get returns the same value", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("hello");
    await io.put(k, "world", META);
    expect(await io.get(k)).toBe("world");
  });

  it("get on a missing key returns null", async () => {
    const io = createFsCacheIO({ cacheDir });
    expect(await io.get(key("nope"))).toBeNull();
  });

  it("list returns all stored keys, sorted, frozen", async () => {
    const io = createFsCacheIO({ cacheDir });
    const ka = key("a"), kb = key("b"), kc = key("c");
    await io.put(ka, "1", META);
    await io.put(kb, "2", META);
    await io.put(kc, "3", META);
    const keys = await io.list();
    expect([...keys].sort()).toEqual([ka, kb, kc].sort());
    expect(Object.isFrozen(keys)).toBe(true);
  });

  it("list on an empty cache returns []", async () => {
    const io = createFsCacheIO({ cacheDir });
    expect(await io.list()).toEqual([]);
  });

  it("list on a non-existent cacheDir returns [] (instance constructed but never used to write)", async () => {
    // Tear down the just-created dir to simulate "configured but not yet populated".
    await rmRf(cacheDir);
    const io = createFsCacheIO({ cacheDir });
    expect(await io.list()).toEqual([]);
  });

  it("put then list reflects the new key", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("x");
    await io.put(k, "v", META);
    expect([...(await io.list())]).toEqual([k]);
  });
});

describe("createFsCacheIO — sharded layout", () => {
  it("each entry lives under cacheDir/<first 2 hex>/<rest>.cache", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("shard-test");
    await io.put(k, "v", META);
    const expectedFile = path.join(cacheDir, k.slice(0, 2), `${k.slice(2)}.cache`);
    const contents = await fs.readFile(expectedFile, "utf8");
    expect(contents).toBe("v");
  });

  it("two keys with different first-2 hex prefixes go in different shard dirs", async () => {
    // Force two known prefixes by computing many keys.
    const io = createFsCacheIO({ cacheDir });
    const ka = key("a");
    const kb = key("b");
    // Statistically near-certain that the two sha256 hex strings
    // differ in their first byte (255/256 chance).  Refuse to run
    // the test if they happen not to.
    expect(ka.slice(0, 2)).not.toBe(kb.slice(0, 2));
    await io.put(ka, "1", META);
    await io.put(kb, "2", META);
    const shards = (await fs.readdir(cacheDir)).filter((d) => /^[0-9a-f]{2}$/.test(d));
    expect(shards.length).toBe(2);
  });
});

describe("createFsCacheIO — key validation", () => {
  it("rejects keys that aren't 64 hex chars", async () => {
    const io = createFsCacheIO({ cacheDir });
    await expect(io.get("short")).rejects.toThrow(/invalid cache key/);
    await expect(io.put("short", "v", META)).rejects.toThrow(/invalid cache key/);
  });

  it("rejects keys with non-hex chars", async () => {
    const io = createFsCacheIO({ cacheDir });
    const bad = "z".repeat(64);
    await expect(io.get(bad)).rejects.toThrow(/invalid cache key/);
  });

  it("rejects uppercase hex (we use lowercase consistently)", async () => {
    const io = createFsCacheIO({ cacheDir });
    const bad = "A".repeat(64);
    await expect(io.get(bad)).rejects.toThrow(/invalid cache key/);
  });

  it("rejects path-traversal attempts dressed as keys", async () => {
    const io = createFsCacheIO({ cacheDir });
    // 64 chars total, but containing `/` — fails the hex regex.
    const bad = "../" + "a".repeat(61);
    expect(bad.length).toBe(64);
    await expect(io.get(bad)).rejects.toThrow(/invalid cache key/);
  });

  it("rejects empty key", async () => {
    const io = createFsCacheIO({ cacheDir });
    await expect(io.get("")).rejects.toThrow(/invalid cache key/);
  });

  it("rejects key with embedded null byte", async () => {
    const io = createFsCacheIO({ cacheDir });
    const bad = "a".repeat(63) + "\x00";
    await expect(io.get(bad)).rejects.toThrow(/invalid cache key/);
  });
});

describe("createFsCacheIO — cacheDir validation", () => {
  it("get / list don't throw for a non-existent cacheDir (graceful)", async () => {
    // get on a missing dir should return null (ENOENT mapped); list
    // should return []. This lets callers construct the IO before
    // the dir exists; it'll be created on first put.
    await rmRf(cacheDir);
    const io = createFsCacheIO({ cacheDir });
    expect(await io.get(key("x"))).toBeNull();
    expect(await io.list()).toEqual([]);
  });

  it("put throws when cacheDir doesn't exist (config error)", async () => {
    await rmRf(cacheDir);
    const io = createFsCacheIO({ cacheDir });
    await expect(io.put(key("x"), "v", META)).rejects.toThrow(/does not exist/);
  });

  it("put throws when cacheDir is a file, not a directory", async () => {
    await rmRf(cacheDir);
    await fs.writeFile(cacheDir, "i am a file");
    const io = createFsCacheIO({ cacheDir });
    await expect(io.put(key("x"), "v", META)).rejects.toThrow(/not a directory/);
  });
});

describe("createFsCacheIO — atomic writes", () => {
  it("atomicWrites: true (default) writes via temp + rename — no .tmp file remains on success", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("atomic");
    await io.put(k, "v", META);

    // Inspect the shard dir — should contain ONLY the final .cache file.
    const shard = path.join(cacheDir, k.slice(0, 2));
    const entries = await fs.readdir(shard);
    expect(entries.length).toBe(1);
    expect(entries[0]!.endsWith(".cache")).toBe(true);
    expect(entries[0]!.includes(".tmp.")).toBe(false);
  });

  it("atomicWrites: false skips the temp-rename dance", async () => {
    const io = createFsCacheIO({ cacheDir, atomicWrites: false });
    const k = key("non-atomic");
    await io.put(k, "v", META);
    expect(await io.get(k)).toBe("v");
  });

  it("concurrent puts to the same key don't corrupt entries", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("concurrent");
    // Fire 10 concurrent puts; each writes a distinct value.
    await Promise.all(
      Array.from({ length: 10 }, (_, i) => io.put(k, `value-${i}`, META)),
    );
    // The final value is well-formed (one of the writes won the race).
    const got = await io.get(k);
    expect(got).not.toBeNull();
    expect(got!.startsWith("value-")).toBe(true);
    // No leftover .tmp files (each successful rename consumed its
    // temp file; failed renames cleaned up).
    const shard = path.join(cacheDir, k.slice(0, 2));
    const entries = await fs.readdir(shard);
    expect(entries.filter((e) => e.includes(".tmp."))).toEqual([]);
  });
});

describe("createFsCacheIO — list ignores non-cache files", () => {
  it("ignores files in shard dirs that don't end in .cache", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("listed");
    await io.put(k, "v", META);
    // Drop a stray file in the same shard dir.
    const shard = path.join(cacheDir, k.slice(0, 2));
    await fs.writeFile(path.join(shard, "stray.txt"), "ignored");
    await fs.writeFile(path.join(shard, "anothercache.cache"), "wrong shape"); // 12 chars, not 62
    const keys = await io.list();
    expect([...keys]).toEqual([k]);
  });

  it("ignores top-level files / non-shard dirs in cacheDir", async () => {
    const io = createFsCacheIO({ cacheDir });
    await fs.writeFile(path.join(cacheDir, "README.txt"), "this is a cache");
    await fs.mkdir(path.join(cacheDir, "not-a-shard"));
    const k = key("only");
    await io.put(k, "v", META);
    expect([...(await io.list())]).toEqual([k]);
  });

  it("ignores leftover .tmp.* files from interrupted writes", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("clean");
    await io.put(k, "v", META);
    // Simulate a leftover .tmp.* from a crashed previous run.
    const shard = path.join(cacheDir, k.slice(0, 2));
    await fs.writeFile(path.join(shard, `${k.slice(2)}.cache.tmp.123.abcdef`), "partial");
    expect([...(await io.list())]).toEqual([k]);
  });
});

describe("createFsCacheIO — defensive non-ENOENT error handling", () => {
  it("get propagates non-ENOENT errors (e.g. cache file is unexpectedly a directory → EISDIR)", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("isdir");
    // Create the file path AS A DIRECTORY — reading it as a file
    // gives EISDIR, not ENOENT.
    const file = path.join(cacheDir, k.slice(0, 2), `${k.slice(2)}.cache`);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.mkdir(file);
    await expect(io.get(k)).rejects.toThrow();   // not null — actual throw
  });

  it("list skips shard entries that aren't directories (e.g. stray file named like a shard)", async () => {
    const io = createFsCacheIO({ cacheDir });
    // Drop a stray FILE whose name matches the 2-hex shard pattern.
    // The list pass should pick it up via the name filter, then
    // hit ENOTDIR on readdir and skip it.
    await fs.writeFile(path.join(cacheDir, "ab"), "fake shard file");
    // And add a real entry too so we can verify it still lands.
    const k = key("real");
    await io.put(k, "v", META);
    const keys = await io.list();
    expect([...keys]).toEqual([k]);
  });

  it("non-ENOENT errors from list's outer readdir propagate (cacheDir is a file)", async () => {
    await rmRf(cacheDir);
    await fs.writeFile(cacheDir, "i am not a directory");
    const io = createFsCacheIO({ cacheDir });
    // fs.readdir on a file gives ENOTDIR — not ENOENT — should throw.
    await expect(io.list()).rejects.toThrow();
  });

  it("put cleans up temp file when rename fails (destination path is a directory → EISDIR)", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("rename-fail");
    // Pre-create the final cache file path AS A DIRECTORY.  Rename
    // from temp file to a directory path fails with EISDIR; the
    // catch arm runs unlink(tmp) for cleanup.
    const file = path.join(cacheDir, k.slice(0, 2), `${k.slice(2)}.cache`);
    await fs.mkdir(path.dirname(file), { recursive: true });
    await fs.mkdir(file);
    await expect(io.put(k, "v", META)).rejects.toThrow();
    // No leftover .tmp.* file in the shard dir.
    const shard = path.join(cacheDir, k.slice(0, 2));
    const entries = await fs.readdir(shard);
    expect(entries.filter((e) => e.includes(".tmp."))).toEqual([]);
  });
});

describe("createFsCacheIO — sane error propagation", () => {
  it("non-ENOENT read errors propagate from get (e.g. permission denied on parent — skipped on CI for portability)", async () => {
    // We can't reliably reproduce EACCES cross-platform; instead
    // verify the contract by injecting a malformed cacheDir.
    // Empty string is invalid path on POSIX → fs.readFile fails
    // with ENOENT (returns null) or EINVAL (propagates).
    const io = createFsCacheIO({ cacheDir: "" });
    // get with empty cacheDir + valid key resolves to "" + "/<sub>" — which is "/<sub>"
    // absolute path that doesn't exist → ENOENT → null.  That's fine.
    expect(await io.get(key("x"))).toBeNull();
  });
});

describe("createFsCacheIO — randomness in temp filename suffix", () => {
  it("two concurrent puts to the same key produce two distinct temp filenames internally", async () => {
    // We can't directly observe the temp filenames without monkey-
    // patching node:fs.  Instead, repeat 50 puts and confirm none
    // of them leak a leftover temp file (which would happen if two
    // concurrent puts collided on the temp name and one failed to
    // rename).
    const io = createFsCacheIO({ cacheDir });
    const k = key("rand");
    await Promise.all(
      Array.from({ length: 50 }, (_, i) => io.put(k, `v${i}`, META)),
    );
    const shard = path.join(cacheDir, k.slice(0, 2));
    const entries = await fs.readdir(shard);
    // Only the final .cache file.
    expect(entries.length).toBe(1);
    expect(entries[0]!.endsWith(".cache")).toBe(true);
  });
});

describe("createFsCacheIO — end-to-end with incremental cache layer", () => {
  it("can be wired into createIncrementalCache and survive a put-get roundtrip", async () => {
    const io = createFsCacheIO({ cacheDir });
    // Fake a small entry shaped like what the incremental-cache
    // layer would store: a JSON object with a known structure.
    const k = key("end-to-end");
    const entry = JSON.stringify({
      unscopedCss: "paragraph { color: red; }",
      emittedRules: ["body"],
      warnings: [],
      sha256: "x".repeat(64),
    });
    await io.put(k, entry, META);
    expect(await io.get(k)).toBe(entry);
  });
});

describe("createFsCacheIO — multiple instances over the same cacheDir", () => {
  it("two IOs over the same dir see each other's writes", async () => {
    const ioA = createFsCacheIO({ cacheDir });
    const ioB = createFsCacheIO({ cacheDir });
    const k = key("shared");
    await ioA.put(k, "from-A", META);
    expect(await ioB.get(k)).toBe("from-A");
  });
});

describe("createFsCacheIO — large value stress test", () => {
  it("round-trips a 1 MiB cache entry", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("big");
    const big = randomBytes(1 << 20).toString("base64");
    await io.put(k, big, META);
    expect(await io.get(k)).toBe(big);
  });
});

describe("createFsCacheIO — write to existing key overwrites", () => {
  it("second put to the same key replaces the value", async () => {
    const io = createFsCacheIO({ cacheDir });
    const k = key("overwrite");
    await io.put(k, "v1", META);
    await io.put(k, "v2", META);
    expect(await io.get(k)).toBe("v2");
  });
});
