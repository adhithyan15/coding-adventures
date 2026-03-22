/**
 * Tests for hasher.ts -- SHA256 File Hashing
 *
 * These tests verify that the hasher:
 * - Produces consistent hashes for the same content
 * - Produces different hashes for different content
 * - Collects the right source files for each language
 * - Includes BUILD files
 * - Computes dependency hashes correctly
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import * as crypto from "node:crypto";
import {
  hashPackage,
  hashDeps,
  hashFile,
  collectSourceFiles,
  SOURCE_EXTENSIONS,
  SPECIAL_FILENAMES,
} from "../src/hasher.js";
import { DirectedGraph } from "../src/resolver.js";
import type { Package } from "../src/discovery.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-hasher-"));
}

function rmDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

function writeFile(filepath: string, content: string): void {
  fs.mkdirSync(path.dirname(filepath), { recursive: true });
  fs.writeFileSync(filepath, content, "utf-8");
}

function makePkg(
  pkgPath: string,
  language: string,
  name?: string,
): Package {
  return {
    name: name ?? `${language}/test-pkg`,
    path: pkgPath,
    buildCommands: ["echo test"],
    language,
  };
}

// ---------------------------------------------------------------------------
// Tests: hashFile
// ---------------------------------------------------------------------------

describe("hashFile", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should produce consistent hash for same content", () => {
    const filepath = path.join(tmpDir, "test.py");
    writeFile(filepath, "print('hello')\n");
    const hash1 = hashFile(filepath);
    const hash2 = hashFile(filepath);
    expect(hash1).toBe(hash2);
  });

  it("should produce different hash for different content", () => {
    const file1 = path.join(tmpDir, "a.py");
    const file2 = path.join(tmpDir, "b.py");
    writeFile(file1, "print('hello')");
    writeFile(file2, "print('world')");
    expect(hashFile(file1)).not.toBe(hashFile(file2));
  });

  it("should produce a valid SHA256 hex string", () => {
    const filepath = path.join(tmpDir, "test.py");
    writeFile(filepath, "content");
    const hash = hashFile(filepath);
    expect(hash).toMatch(/^[a-f0-9]{64}$/);
  });
});

// ---------------------------------------------------------------------------
// Tests: collectSourceFiles
// ---------------------------------------------------------------------------

describe("collectSourceFiles", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should collect Python source files", () => {
    writeFile(path.join(tmpDir, "BUILD"), "echo test\n");
    writeFile(path.join(tmpDir, "src", "main.py"), "print('hi')\n");
    writeFile(path.join(tmpDir, "pyproject.toml"), "[project]\n");
    writeFile(path.join(tmpDir, "README.md"), "# readme\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFiles(pkg);
    const names = files.map((f) => path.relative(tmpDir, f));

    expect(names).toContain("BUILD");
    expect(names).toContain(path.join("src", "main.py"));
    expect(names).toContain("pyproject.toml");
    expect(names).not.toContain("README.md");
  });

  it("should collect Go source files including go.mod", () => {
    writeFile(path.join(tmpDir, "BUILD"), "go build\n");
    writeFile(path.join(tmpDir, "main.go"), "package main\n");
    writeFile(path.join(tmpDir, "go.mod"), "module test\n");
    writeFile(path.join(tmpDir, "go.sum"), "checksum\n");

    const pkg = makePkg(tmpDir, "go");
    const files = collectSourceFiles(pkg);
    const names = files.map((f) => path.relative(tmpDir, f));

    expect(names).toContain("BUILD");
    expect(names).toContain("main.go");
    expect(names).toContain("go.mod");
    expect(names).toContain("go.sum");
  });

  it("should include BUILD variant files", () => {
    writeFile(path.join(tmpDir, "BUILD_mac"), "mac build\n");
    writeFile(path.join(tmpDir, "BUILD_linux"), "linux build\n");
    writeFile(path.join(tmpDir, "BUILD_windows"), "windows build\n");
    writeFile(path.join(tmpDir, "BUILD_mac_and_linux"), "unix build\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFiles(pkg);
    const names = files.map((f) => path.basename(f));

    expect(names).toContain("BUILD_mac");
    expect(names).toContain("BUILD_linux");
    expect(names).toContain("BUILD_windows");
    expect(names).toContain("BUILD_mac_and_linux");
  });

  it("should return sorted files for determinism", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "c.py"), "c\n");
    writeFile(path.join(tmpDir, "a.py"), "a\n");
    writeFile(path.join(tmpDir, "b.py"), "b\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFiles(pkg);
    const names = files.map((f) => path.relative(tmpDir, f));

    // Check they're sorted (localeCompare sorting).
    // Note: case-sensitive sort means uppercase comes after lowercase
    // on most systems, but localeCompare may vary. We just verify the
    // output is deterministic by checking it matches its own sort.
    const sortedNames = [...names].sort((a, b) => a.localeCompare(b));
    expect(names).toEqual(sortedNames);
  });
});

// ---------------------------------------------------------------------------
// Tests: hashPackage
// ---------------------------------------------------------------------------

describe("hashPackage", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should return consistent hash for same files", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "main.py"), "print('hi')\n");

    const pkg = makePkg(tmpDir, "python");
    expect(hashPackage(pkg)).toBe(hashPackage(pkg));
  });

  it("should change hash when file content changes", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "main.py"), "print('hello')\n");

    const pkg = makePkg(tmpDir, "python");
    const hash1 = hashPackage(pkg);

    writeFile(path.join(tmpDir, "main.py"), "print('world')\n");
    const hash2 = hashPackage(pkg);

    expect(hash1).not.toBe(hash2);
  });

  it("should return hash of empty string for package with no source files", () => {
    fs.mkdirSync(tmpDir, { recursive: true });

    const pkg = makePkg(tmpDir, "python");
    const expected = crypto.createHash("sha256").update("").digest("hex");
    expect(hashPackage(pkg)).toBe(expected);
  });
});

// ---------------------------------------------------------------------------
// Tests: hashDeps
// ---------------------------------------------------------------------------

describe("hashDeps", () => {
  it("should return empty hash for node with no dependencies", () => {
    const graph = new DirectedGraph();
    graph.addNode("A");

    const hashes = new Map([["A", "hash-a"]]);
    const expected = crypto.createHash("sha256").update("").digest("hex");
    expect(hashDeps("A", graph, hashes)).toBe(expected);
  });

  it("should return empty hash for unknown node", () => {
    const graph = new DirectedGraph();
    const expected = crypto.createHash("sha256").update("").digest("hex");
    expect(hashDeps("UNKNOWN", graph, new Map())).toBe(expected);
  });

  it("should incorporate dependency hashes", () => {
    const graph = new DirectedGraph();
    graph.addEdge("A", "B"); // A -> B means B depends on A

    const hashes = new Map([
      ["A", "hash-a"],
      ["B", "hash-b"],
    ]);

    // B's deps hash should include A's hash.
    const depsHash = hashDeps("B", graph, hashes);
    expect(depsHash).not.toBe(
      crypto.createHash("sha256").update("").digest("hex"),
    );
  });

  it("should produce different hashes when dependency changes", () => {
    const graph = new DirectedGraph();
    graph.addEdge("A", "B");

    const hashes1 = new Map([
      ["A", "hash-a-v1"],
      ["B", "hash-b"],
    ]);
    const hashes2 = new Map([
      ["A", "hash-a-v2"],
      ["B", "hash-b"],
    ]);

    expect(hashDeps("B", graph, hashes1)).not.toBe(
      hashDeps("B", graph, hashes2),
    );
  });
});

// ---------------------------------------------------------------------------
// Tests: Constants
// ---------------------------------------------------------------------------

describe("SOURCE_EXTENSIONS", () => {
  it("should include Python extensions", () => {
    expect(SOURCE_EXTENSIONS.python.has(".py")).toBe(true);
    expect(SOURCE_EXTENSIONS.python.has(".toml")).toBe(true);
  });

  it("should include Go extensions", () => {
    expect(SOURCE_EXTENSIONS.go.has(".go")).toBe(true);
  });

  it("should include TypeScript extensions", () => {
    expect(SOURCE_EXTENSIONS.typescript.has(".ts")).toBe(true);
    expect(SOURCE_EXTENSIONS.typescript.has(".json")).toBe(true);
  });

  it("should include Rust extensions", () => {
    expect(SOURCE_EXTENSIONS.rust.has(".rs")).toBe(true);
  });

  it("should include Elixir extensions", () => {
    expect(SOURCE_EXTENSIONS.elixir.has(".ex")).toBe(true);
    expect(SOURCE_EXTENSIONS.elixir.has(".exs")).toBe(true);
  });
});

describe("SPECIAL_FILENAMES", () => {
  it("should include Go special files", () => {
    expect(SPECIAL_FILENAMES.go.has("go.mod")).toBe(true);
    expect(SPECIAL_FILENAMES.go.has("go.sum")).toBe(true);
  });

  it("should include Ruby special files", () => {
    expect(SPECIAL_FILENAMES.ruby.has("Gemfile")).toBe(true);
  });
});
