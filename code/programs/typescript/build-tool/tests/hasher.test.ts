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
  collectSourceFilesGlob,
  SOURCE_EXTENSIONS,
  SPECIAL_FILENAMES,
} from "../src/hasher.js";
import { DirectedGraph } from "../src/resolver.js";
import type { Package } from "../src/discovery.js";

type SourceCollectionFixture = {
  input: {
    options: {
      candidates: Array<{
        path: string;
        kind: "file" | "symlink" | "reparse_point";
        content_hex?: string;
      }>;
    };
  };
  expected: {
    result: {
      files: Array<{ path: string }>;
    };
  };
};

const SOURCE_COLLECTION_FIXTURES = [
  "source-collection-extension.json",
  "source-collection-declared.json",
] as const;

const EXPECTED_EXCLUDED_COMPONENTS = [
  ".build",
  ".cargo",
  ".claude",
  ".dart_tool",
  ".git",
  ".gradle",
  ".hg",
  ".mypy_cache",
  ".pytest_cache",
  ".ruff_cache",
  ".stack-work",
  ".svn",
  ".tox",
  ".venv",
  "Pods",
  "__pycache__",
  "_build",
  "build",
  "cover",
  "deps",
  "dist",
  "dist-newstyle",
  "gradle-build",
  "node_modules",
  "target",
  "vendor",
] as const;

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

function makePkg(pkgPath: string, language: string, name?: string): Package {
  return {
    name: name ?? `${language}/test-pkg`,
    path: pkgPath,
    buildCommands: ["echo test"],
    language,
  };
}

function readSourceCollectionFixture(
  filename: (typeof SOURCE_COLLECTION_FIXTURES)[number],
): SourceCollectionFixture {
  const fixtureUrl = new URL(
    `../../../../specs/fixtures/build-tool-v1/cases/${filename}`,
    import.meta.url,
  );
  return JSON.parse(
    fs.readFileSync(fixtureUrl, "utf-8"),
  ) as SourceCollectionFixture;
}

function projectFixturePath(filepath: string): string {
  return filepath.replace(/\.mli?$/, ".ts");
}

function materializeProjectedFixture(
  root: string,
  fixture: SourceCollectionFixture,
): void {
  for (const candidate of fixture.input.options.candidates) {
    if (candidate.kind !== "file") continue;
    if (!/^(?:excluded-\d+|case\/|near\/)/.test(candidate.path)) continue;
    writeFile(
      path.join(root, ...projectFixturePath(candidate.path).split("/")),
      Buffer.from(candidate.content_hex ?? "", "hex").toString("utf-8"),
    );
  }
}

function projectedExpectedPaths(fixture: SourceCollectionFixture): string[] {
  return fixture.expected.result.files
    .map(({ path: filepath }) => projectFixturePath(filepath))
    .filter((filepath) => /^(?:case\/|near\/)/.test(filepath))
    .sort((a, b) => a.localeCompare(b));
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

  it.each(SOURCE_COLLECTION_FIXTURES)(
    "projects %s exact generated-directory pruning into extension collection",
    (fixtureFilename) => {
      const fixture = readSourceCollectionFixture(fixtureFilename);
      materializeProjectedFixture(tmpDir, fixture);

      const actual = collectSourceFiles(makePkg(tmpDir, "typescript"))
        .map((filepath) =>
          path.relative(tmpDir, filepath).split(path.sep).join("/"),
        )
        .sort((a, b) => a.localeCompare(b));

      expect(actual).toEqual(projectedExpectedPaths(fixture));
    },
  );

  it("keeps the neutral fixture's exact 26-component exclusion registry", () => {
    for (const fixtureFilename of SOURCE_COLLECTION_FIXTURES) {
      const fixture = readSourceCollectionFixture(fixtureFilename);
      const excluded = fixture.input.options.candidates
        .filter(({ path: filepath }) => filepath.startsWith("excluded-"))
        .map(({ path: filepath }) => filepath.split("/")[1])
        .sort((a, b) => a.localeCompare(b));

      expect(excluded).toEqual(
        [...EXPECTED_EXCLUDED_COMPONENTS].sort((a, b) => a.localeCompare(b)),
      );
    }
  });

  it("keeps discovery-only directory names eligible in both source modes", () => {
    writeFile(
      path.join(tmpDir, "specs", "contract.ts"),
      "export const contract = true;\n",
    );
    const pkg = makePkg(tmpDir, "typescript");

    for (const files of [
      collectSourceFiles(pkg),
      collectSourceFilesGlob(pkg, ["**/*.ts"]),
    ]) {
      expect(
        files.map((filepath) => path.relative(tmpDir, filepath)),
      ).toContain(path.join("specs", "contract.ts"));
    }
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

// ---------------------------------------------------------------------------
// Tests: collectSourceFilesGlob
// ---------------------------------------------------------------------------

describe("collectSourceFilesGlob", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should collect files matching glob patterns", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "src", "main.py"), "print('hi')\n");
    writeFile(path.join(tmpDir, "src", "utils.py"), "pass\n");
    writeFile(path.join(tmpDir, "README.md"), "# docs\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFilesGlob(pkg, ["src/*.py"]);

    const basenames = files.map((f) => path.basename(f));
    expect(basenames).toContain("main.py");
    expect(basenames).toContain("utils.py");
    expect(basenames).toContain("BUILD"); // Always included
    expect(basenames).not.toContain("README.md");
  });

  it("should always include BUILD files regardless of patterns", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "src", "main.py"), "pass\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFilesGlob(pkg, ["src/*.py"]);
    const basenames = files.map((f) => path.basename(f));
    expect(basenames).toContain("BUILD");
  });

  it("should return only BUILD when no patterns match", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "src", "main.py"), "pass\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFilesGlob(pkg, ["nonexistent/*.rs"]);
    expect(files.length).toBe(1);
    expect(path.basename(files[0])).toBe("BUILD");
  });

  it("should handle empty patterns array", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "src", "main.py"), "pass\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFilesGlob(pkg, []);
    expect(files.length).toBe(1);
    expect(path.basename(files[0])).toBe("BUILD");
  });

  it("should return sorted files", () => {
    writeFile(path.join(tmpDir, "BUILD"), "test\n");
    writeFile(path.join(tmpDir, "c.py"), "pass\n");
    writeFile(path.join(tmpDir, "a.py"), "pass\n");
    writeFile(path.join(tmpDir, "b.py"), "pass\n");

    const pkg = makePkg(tmpDir, "python");
    const files = collectSourceFilesGlob(pkg, ["*.py"]);
    const basenames = files.map((f) => path.basename(f));
    // Sorted by relative path: BUILD (uppercase) sorts after lowercase on
    // most systems using localeCompare, so verify sorted order.
    expect(basenames).toEqual(["a.py", "b.py", "BUILD", "c.py"]);
  });

  it.each(SOURCE_COLLECTION_FIXTURES)(
    "projects %s exact generated-directory pruning into declared-source collection",
    (fixtureFilename) => {
      const fixture = readSourceCollectionFixture(fixtureFilename);
      materializeProjectedFixture(tmpDir, fixture);

      const actual = collectSourceFilesGlob(makePkg(tmpDir, "typescript"), [
        "**/*.ts",
      ])
        .map((filepath) =>
          path.relative(tmpDir, filepath).split(path.sep).join("/"),
        )
        .sort((a, b) => a.localeCompare(b));

      expect(actual).toEqual(projectedExpectedPaths(fixture));
    },
  );

  it("does not follow directory symlinks or Windows junctions", ({ skip }) => {
    const outside = makeTempDir();
    const linked = path.join(tmpDir, "linked");
    try {
      writeFile(
        path.join(outside, "external.ts"),
        "export const outside = true;\n",
      );
      try {
        fs.symlinkSync(
          outside,
          linked,
          process.platform === "win32" ? "junction" : "dir",
        );
      } catch (error) {
        const code = (error as NodeJS.ErrnoException).code;
        if (
          process.platform === "win32" &&
          (code === "EPERM" || code === "EACCES")
        ) {
          skip("this Windows host does not permit a directory link fixture");
          return;
        }
        throw error;
      }

      const pkg = makePkg(tmpDir, "typescript");
      expect(collectSourceFiles(pkg)).toEqual([]);
      expect(collectSourceFilesGlob(pkg, ["**/*.ts"])).toEqual([]);
    } finally {
      rmDir(outside);
    }
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

describe("walkFiles error-handling branch", () => {
  it("returns an empty list when the package directory does not exist", () => {
    // walkFiles wraps `readdirSync` in try/catch — exercises the catch branch.
    const ghost = path.join(
      os.tmpdir(),
      "build-tool-hasher-ghost-" + Date.now(),
    );
    const pkg = makePkg(ghost, "python");
    expect(collectSourceFilesGlob(pkg, ["**/*"])).toEqual([]);
  });
});
