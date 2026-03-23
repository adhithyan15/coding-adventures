/**
 * Tests for cache.ts -- Build Cache Management
 *
 * These tests verify that the cache:
 * - Loads and saves JSON cache files
 * - Correctly determines when packages need rebuilding
 * - Records build results
 * - Handles missing/malformed cache files gracefully
 * - Uses atomic writes (write to temp, rename)
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { BuildCache } from "../src/cache.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-cache-"));
}

function rmDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

// ---------------------------------------------------------------------------
// Tests: needsBuild
// ---------------------------------------------------------------------------

describe("BuildCache.needsBuild", () => {
  it("should return true for unknown package", () => {
    const cache = new BuildCache();
    expect(cache.needsBuild("python/unknown", "hash1", "dhash1")).toBe(true);
  });

  it("should return false for cached package with matching hashes", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "success");
    expect(cache.needsBuild("python/pkg", "hash1", "dhash1")).toBe(false);
  });

  it("should return true when package hash changes", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "success");
    expect(cache.needsBuild("python/pkg", "hash2", "dhash1")).toBe(true);
  });

  it("should return true when deps hash changes", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "success");
    expect(cache.needsBuild("python/pkg", "hash1", "dhash2")).toBe(true);
  });

  it("should return true when last build failed", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "failed");
    // Even with matching hashes, a failed build should be retried.
    expect(cache.needsBuild("python/pkg", "hash1", "dhash1")).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// Tests: record
// ---------------------------------------------------------------------------

describe("BuildCache.record", () => {
  it("should store entry that can be retrieved", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "success");

    const entries = cache.entries;
    expect(entries.has("python/pkg")).toBe(true);
    expect(entries.get("python/pkg")?.packageHash).toBe("hash1");
    expect(entries.get("python/pkg")?.depsHash).toBe("dhash1");
    expect(entries.get("python/pkg")?.status).toBe("success");
  });

  it("should overwrite existing entry", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "failed");
    cache.record("python/pkg", "hash2", "dhash2", "success");

    const entry = cache.entries.get("python/pkg");
    expect(entry?.packageHash).toBe("hash2");
    expect(entry?.status).toBe("success");
  });

  it("should set lastBuilt timestamp", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "success");

    const entry = cache.entries.get("python/pkg");
    expect(entry?.lastBuilt).toBeTruthy();
    // Should be a valid ISO date string.
    expect(new Date(entry!.lastBuilt).getTime()).not.toBeNaN();
  });
});

// ---------------------------------------------------------------------------
// Tests: load and save
// ---------------------------------------------------------------------------

describe("BuildCache.load", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should load a valid cache file", () => {
    const cachePath = path.join(tmpDir, ".build-cache.json");
    const data = {
      "python/logic-gates": {
        package_hash: "abc123",
        deps_hash: "def456",
        last_built: "2024-01-15T10:30:00.000Z",
        status: "success",
      },
    };
    fs.writeFileSync(cachePath, JSON.stringify(data), "utf-8");

    const cache = new BuildCache();
    cache.load(cachePath);

    expect(cache.needsBuild("python/logic-gates", "abc123", "def456")).toBe(
      false,
    );
  });

  it("should handle non-existent file gracefully", () => {
    const cache = new BuildCache();
    cache.load(path.join(tmpDir, "nonexistent.json"));
    expect(cache.entries.size).toBe(0);
  });

  it("should handle malformed JSON gracefully", () => {
    const cachePath = path.join(tmpDir, "bad.json");
    fs.writeFileSync(cachePath, "not valid json{{{", "utf-8");

    const cache = new BuildCache();
    cache.load(cachePath);
    expect(cache.entries.size).toBe(0);
  });

  it("should skip malformed entries", () => {
    const cachePath = path.join(tmpDir, "partial.json");
    const data = {
      "python/good": {
        package_hash: "abc",
        deps_hash: "def",
        last_built: "2024-01-15T10:30:00.000Z",
        status: "success",
      },
      "python/bad": {
        package_hash: "abc",
        // Missing deps_hash, last_built, status
      },
    };
    fs.writeFileSync(cachePath, JSON.stringify(data), "utf-8");

    const cache = new BuildCache();
    cache.load(cachePath);
    expect(cache.entries.size).toBe(1);
    expect(cache.entries.has("python/good")).toBe(true);
  });
});

describe("BuildCache.save", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should save and reload cache", () => {
    const cachePath = path.join(tmpDir, ".build-cache.json");

    const cache1 = new BuildCache();
    cache1.record("python/pkg-a", "hash1", "dhash1", "success");
    cache1.record("python/pkg-b", "hash2", "dhash2", "failed");
    cache1.save(cachePath);

    const cache2 = new BuildCache();
    cache2.load(cachePath);

    expect(cache2.needsBuild("python/pkg-a", "hash1", "dhash1")).toBe(false);
    expect(cache2.needsBuild("python/pkg-b", "hash2", "dhash2")).toBe(true); // failed
  });

  it("should write valid JSON", () => {
    const cachePath = path.join(tmpDir, ".build-cache.json");

    const cache = new BuildCache();
    cache.record("python/pkg", "hash1", "dhash1", "success");
    cache.save(cachePath);

    const text = fs.readFileSync(cachePath, "utf-8");
    const data = JSON.parse(text);
    expect(data["python/pkg"]).toBeDefined();
    expect(data["python/pkg"].package_hash).toBe("hash1");
  });

  it("should sort entries by name in saved file", () => {
    const cachePath = path.join(tmpDir, ".build-cache.json");

    const cache = new BuildCache();
    cache.record("python/z-pkg", "h1", "d1", "success");
    cache.record("python/a-pkg", "h2", "d2", "success");
    cache.save(cachePath);

    const text = fs.readFileSync(cachePath, "utf-8");
    const keys = Object.keys(JSON.parse(text));
    expect(keys[0]).toBe("python/a-pkg");
    expect(keys[1]).toBe("python/z-pkg");
  });

  it("should not leave temp file after save", () => {
    const cachePath = path.join(tmpDir, ".build-cache.json");

    const cache = new BuildCache();
    cache.record("python/pkg", "h1", "d1", "success");
    cache.save(cachePath);

    const tmpFile = path.join(tmpDir, ".build-cache.json.tmp");
    expect(fs.existsSync(tmpFile)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// Tests: entries property
// ---------------------------------------------------------------------------

describe("BuildCache.entries", () => {
  it("should return a copy (not the internal map)", () => {
    const cache = new BuildCache();
    cache.record("python/pkg", "h1", "d1", "success");

    const entries = cache.entries;
    entries.delete("python/pkg");

    // Internal state should be unchanged.
    expect(cache.entries.has("python/pkg")).toBe(true);
  });
});
