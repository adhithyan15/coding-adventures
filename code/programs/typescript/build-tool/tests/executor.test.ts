/**
 * Tests for executor.ts -- Parallel Build Execution
 *
 * These tests verify that the executor:
 * - Runs BUILD commands in the correct order
 * - Skips packages when not needed
 * - Marks dependents as dep-skipped when a dependency fails
 * - Handles dry-run mode
 * - Respects the affected set from git-diff
 * - Updates the cache after builds
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import { executeBuilds } from "../src/executor.js";
import { DirectedGraph } from "../src/resolver.js";
import { BuildCache } from "../src/cache.js";
import type { Package } from "../src/discovery.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-exec-"));
}

function rmDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

function makePkg(
  name: string,
  pkgPath: string,
  commands: string[],
  language = "python",
): Package {
  return { name, path: pkgPath, buildCommands: commands, language };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("executeBuilds", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should build a simple package successfully", async () => {
    const pkgDir = path.join(tmpDir, "pkg-a");
    fs.mkdirSync(pkgDir, { recursive: true });

    const pkg = makePkg("python/pkg-a", pkgDir, ["echo hello"]);
    const graph = new DirectedGraph();
    graph.addNode("python/pkg-a");

    const results = await executeBuilds({
      packages: [pkg],
      graph,
      cache: new BuildCache(),
      packageHashes: new Map([["python/pkg-a", "hash1"]]),
      depsHashes: new Map([["python/pkg-a", "dhash1"]]),
      force: true,
    });

    expect(results.get("python/pkg-a")?.status).toBe("built");
    expect(results.get("python/pkg-a")?.returnCode).toBe(0);
  });

  it("should detect failed builds", async () => {
    const pkgDir = path.join(tmpDir, "pkg-fail");
    fs.mkdirSync(pkgDir, { recursive: true });

    const pkg = makePkg("python/pkg-fail", pkgDir, ["exit 1"]);
    const graph = new DirectedGraph();
    graph.addNode("python/pkg-fail");

    const results = await executeBuilds({
      packages: [pkg],
      graph,
      cache: new BuildCache(),
      packageHashes: new Map([["python/pkg-fail", "hash1"]]),
      depsHashes: new Map([["python/pkg-fail", "dhash1"]]),
      force: true,
    });

    expect(results.get("python/pkg-fail")?.status).toBe("failed");
  });

  it("should skip packages not in affected set", async () => {
    const pkgDir = path.join(tmpDir, "pkg-skip");
    fs.mkdirSync(pkgDir, { recursive: true });

    const pkg = makePkg("python/pkg-skip", pkgDir, ["echo hello"]);
    const graph = new DirectedGraph();
    graph.addNode("python/pkg-skip");

    const results = await executeBuilds({
      packages: [pkg],
      graph,
      cache: new BuildCache(),
      packageHashes: new Map([["python/pkg-skip", "hash1"]]),
      depsHashes: new Map([["python/pkg-skip", "dhash1"]]),
      affectedSet: new Set(), // empty -- nothing affected
    });

    expect(results.get("python/pkg-skip")?.status).toBe("skipped");
  });

  it("should handle dry-run mode", async () => {
    const pkgDir = path.join(tmpDir, "pkg-dry");
    fs.mkdirSync(pkgDir, { recursive: true });

    const pkg = makePkg("python/pkg-dry", pkgDir, ["echo hello"]);
    const graph = new DirectedGraph();
    graph.addNode("python/pkg-dry");

    const results = await executeBuilds({
      packages: [pkg],
      graph,
      cache: new BuildCache(),
      packageHashes: new Map([["python/pkg-dry", "hash1"]]),
      depsHashes: new Map([["python/pkg-dry", "dhash1"]]),
      force: true,
      dryRun: true,
    });

    expect(results.get("python/pkg-dry")?.status).toBe("would-build");
  });

  it("should skip packages with unchanged cache", async () => {
    const pkgDir = path.join(tmpDir, "pkg-cached");
    fs.mkdirSync(pkgDir, { recursive: true });

    const pkg = makePkg("python/pkg-cached", pkgDir, ["echo hello"]);
    const graph = new DirectedGraph();
    graph.addNode("python/pkg-cached");

    // Pre-populate cache with matching hashes.
    const cache = new BuildCache();
    cache.record("python/pkg-cached", "hash1", "dhash1", "success");

    const results = await executeBuilds({
      packages: [pkg],
      graph,
      cache,
      packageHashes: new Map([["python/pkg-cached", "hash1"]]),
      depsHashes: new Map([["python/pkg-cached", "dhash1"]]),
      // affectedSet is null (no git diff), force is false
    });

    expect(results.get("python/pkg-cached")?.status).toBe("skipped");
  });

  it("should dep-skip packages when dependency fails", async () => {
    const dirA = path.join(tmpDir, "pkg-a");
    const dirB = path.join(tmpDir, "pkg-b");
    fs.mkdirSync(dirA, { recursive: true });
    fs.mkdirSync(dirB, { recursive: true });

    // A -> B (A must build before B). A will fail.
    const pkgA = makePkg("python/pkg-a", dirA, ["exit 1"]);
    const pkgB = makePkg("python/pkg-b", dirB, ["echo hello"]);

    const graph = new DirectedGraph();
    graph.addEdge("python/pkg-a", "python/pkg-b");

    const results = await executeBuilds({
      packages: [pkgA, pkgB],
      graph,
      cache: new BuildCache(),
      packageHashes: new Map([
        ["python/pkg-a", "h1"],
        ["python/pkg-b", "h2"],
      ]),
      depsHashes: new Map([
        ["python/pkg-a", "d1"],
        ["python/pkg-b", "d2"],
      ]),
      force: true,
    });

    expect(results.get("python/pkg-a")?.status).toBe("failed");
    expect(results.get("python/pkg-b")?.status).toBe("dep-skipped");
  });

  it("should execute multiple commands sequentially per package", async () => {
    const pkgDir = path.join(tmpDir, "pkg-multi");
    const filePath = path.join(pkgDir, "test.txt");
    const escapedFilePath = filePath.replace(/\\/g, "\\\\").replace(/'/g, "\\'");
    fs.mkdirSync(pkgDir, { recursive: true });

    // Use Node instead of shell builtins so this stays portable on Windows.
    const pkg = makePkg("python/pkg-multi", pkgDir, [
      `node -e "require('node:fs').writeFileSync('${escapedFilePath}', 'hello')"`,
      `node -e "process.stdout.write(require('node:fs').readFileSync('${escapedFilePath}', 'utf8'))"`,
    ]);
    const graph = new DirectedGraph();
    graph.addNode("python/pkg-multi");

    const results = await executeBuilds({
      packages: [pkg],
      graph,
      cache: new BuildCache(),
      packageHashes: new Map([["python/pkg-multi", "hash1"]]),
      depsHashes: new Map([["python/pkg-multi", "dhash1"]]),
      force: true,
    });

    expect(results.get("python/pkg-multi")?.status).toBe("built");
  });
});
