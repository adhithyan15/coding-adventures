/**
 * Tests for discovery.ts -- Package Discovery
 *
 * These tests verify that the package discovery system correctly:
 * - Walks directory trees recursively
 * - Finds BUILD files and registers packages
 * - Skips known non-source directories
 * - Infers languages from directory paths
 * - Builds qualified package names
 * - Handles platform-specific BUILD files (BUILD_mac, BUILD_linux,
 *   BUILD_windows, BUILD_mac_and_linux)
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as path from "node:path";
import * as os from "node:os";
import {
  discoverPackages,
  readLines,
  inferLanguage,
  inferPackageName,
  getBuildFile,
  SKIP_DIRS,
} from "../src/discovery.js";

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

/** Create a temporary directory for test fixtures. */
function makeTempDir(): string {
  return fs.mkdtempSync(path.join(os.tmpdir(), "build-tool-test-"));
}

/** Recursively remove a directory. */
function rmDir(dir: string): void {
  fs.rmSync(dir, { recursive: true, force: true });
}

/** Create a file with content, creating parent directories as needed. */
function writeFile(filepath: string, content: string): void {
  fs.mkdirSync(path.dirname(filepath), { recursive: true });
  fs.writeFileSync(filepath, content, "utf-8");
}

// ---------------------------------------------------------------------------
// Tests: readLines
// ---------------------------------------------------------------------------

describe("readLines", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should return non-blank, non-comment lines", () => {
    const filepath = path.join(tmpDir, "BUILD");
    writeFile(
      filepath,
      "# This is a comment\nnpm install\n\n# Another comment\nnpx vitest run\n",
    );
    const lines = readLines(filepath);
    expect(lines).toEqual(["npm install", "npx vitest run"]);
  });

  it("should return empty array for non-existent file", () => {
    const lines = readLines(path.join(tmpDir, "NONEXISTENT"));
    expect(lines).toEqual([]);
  });

  it("should strip leading and trailing whitespace", () => {
    const filepath = path.join(tmpDir, "BUILD");
    writeFile(filepath, "  npm install  \n  npx vitest run  \n");
    const lines = readLines(filepath);
    expect(lines).toEqual(["npm install", "npx vitest run"]);
  });

  it("should return empty array for file with only comments and blanks", () => {
    const filepath = path.join(tmpDir, "BUILD");
    writeFile(filepath, "# comment\n\n# another\n\n");
    const lines = readLines(filepath);
    expect(lines).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Tests: inferLanguage
// ---------------------------------------------------------------------------

describe("inferLanguage", () => {
  it("should detect Python from path", () => {
    expect(inferLanguage("/repo/code/packages/python/logic-gates")).toBe(
      "python",
    );
  });

  it("should detect Ruby from path", () => {
    expect(inferLanguage("/repo/code/packages/ruby/logic_gates")).toBe("ruby");
  });

  it("should detect Go from path", () => {
    expect(inferLanguage("/repo/code/programs/go/build-tool")).toBe("go");
  });

  it("should detect Rust from path", () => {
    expect(inferLanguage("/repo/code/packages/rust/arithmetic")).toBe("rust");
  });

  it("should detect TypeScript from path", () => {
    expect(inferLanguage("/repo/code/programs/typescript/build-tool")).toBe(
      "typescript",
    );
  });

  it("should detect Elixir from path", () => {
    expect(inferLanguage("/repo/code/programs/elixir/build-tool")).toBe(
      "elixir",
    );
  });

  it("should return unknown for unrecognized paths", () => {
    expect(inferLanguage("/repo/code/packages/haskell/something")).toBe(
      "unknown",
    );
  });
});

// ---------------------------------------------------------------------------
// Tests: inferPackageName
// ---------------------------------------------------------------------------

describe("inferPackageName", () => {
  it("should build qualified name from language and directory", () => {
    expect(
      inferPackageName("/repo/code/packages/python/logic-gates", "python"),
    ).toBe("python/logic-gates");
  });

  it("should handle nested paths", () => {
    expect(
      inferPackageName("/a/b/c/programs/go/build-tool", "go"),
    ).toBe("go/build-tool");
  });
});

// ---------------------------------------------------------------------------
// Tests: getBuildFile
// ---------------------------------------------------------------------------

describe("getBuildFile", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should return generic BUILD file", () => {
    writeFile(path.join(tmpDir, "BUILD"), "echo hello\n");
    const result = getBuildFile(tmpDir, "darwin");
    expect(result).toBe(path.join(tmpDir, "BUILD"));
  });

  it("should return null when no BUILD file exists", () => {
    const result = getBuildFile(tmpDir, "darwin");
    expect(result).toBeNull();
  });

  it("should prefer BUILD_mac on darwin", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_mac"), "mac specific\n");
    const result = getBuildFile(tmpDir, "darwin");
    expect(result).toBe(path.join(tmpDir, "BUILD_mac"));
  });

  it("should prefer BUILD_linux on linux", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_linux"), "linux specific\n");
    const result = getBuildFile(tmpDir, "linux");
    expect(result).toBe(path.join(tmpDir, "BUILD_linux"));
  });

  it("should prefer BUILD_windows on win32", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_windows"), "windows specific\n");
    const result = getBuildFile(tmpDir, "win32");
    expect(result).toBe(path.join(tmpDir, "BUILD_windows"));
  });

  it("should use BUILD_mac_and_linux on darwin when no BUILD_mac exists", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_mac_and_linux"), "unix shared\n");
    const result = getBuildFile(tmpDir, "darwin");
    expect(result).toBe(path.join(tmpDir, "BUILD_mac_and_linux"));
  });

  it("should use BUILD_mac_and_linux on linux when no BUILD_linux exists", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_mac_and_linux"), "unix shared\n");
    const result = getBuildFile(tmpDir, "linux");
    expect(result).toBe(path.join(tmpDir, "BUILD_mac_and_linux"));
  });

  it("should NOT use BUILD_mac_and_linux on win32", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_mac_and_linux"), "unix shared\n");
    const result = getBuildFile(tmpDir, "win32");
    expect(result).toBe(path.join(tmpDir, "BUILD"));
  });

  it("should prefer BUILD_mac over BUILD_mac_and_linux on darwin", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_mac"), "mac specific\n");
    writeFile(path.join(tmpDir, "BUILD_mac_and_linux"), "unix shared\n");
    const result = getBuildFile(tmpDir, "darwin");
    expect(result).toBe(path.join(tmpDir, "BUILD_mac"));
  });

  it("should prefer BUILD_linux over BUILD_mac_and_linux on linux", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    writeFile(path.join(tmpDir, "BUILD_linux"), "linux specific\n");
    writeFile(path.join(tmpDir, "BUILD_mac_and_linux"), "unix shared\n");
    const result = getBuildFile(tmpDir, "linux");
    expect(result).toBe(path.join(tmpDir, "BUILD_linux"));
  });

  it("should fall back to BUILD on win32 when no BUILD_windows", () => {
    writeFile(path.join(tmpDir, "BUILD"), "generic\n");
    const result = getBuildFile(tmpDir, "win32");
    expect(result).toBe(path.join(tmpDir, "BUILD"));
  });
});

// ---------------------------------------------------------------------------
// Tests: discoverPackages
// ---------------------------------------------------------------------------

describe("discoverPackages", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = makeTempDir();
  });

  afterEach(() => {
    rmDir(tmpDir);
  });

  it("should discover a single package", () => {
    const pkgDir = path.join(tmpDir, "packages", "python", "logic-gates");
    writeFile(path.join(pkgDir, "BUILD"), "python -m pip install .\n");

    const packages = discoverPackages(tmpDir);
    expect(packages).toHaveLength(1);
    expect(packages[0].name).toBe("python/logic-gates");
    expect(packages[0].path).toBe(pkgDir);
    expect(packages[0].buildCommands).toEqual(["python -m pip install ."]);
    expect(packages[0].language).toBe("python");
  });

  it("should discover multiple packages sorted by name", () => {
    writeFile(
      path.join(tmpDir, "packages", "python", "b-pkg", "BUILD"),
      "echo b\n",
    );
    writeFile(
      path.join(tmpDir, "packages", "python", "a-pkg", "BUILD"),
      "echo a\n",
    );

    const packages = discoverPackages(tmpDir);
    expect(packages).toHaveLength(2);
    expect(packages[0].name).toBe("python/a-pkg");
    expect(packages[1].name).toBe("python/b-pkg");
  });

  it("should skip directories in SKIP_DIRS", () => {
    writeFile(
      path.join(tmpDir, "node_modules", "some-dep", "BUILD"),
      "echo skip\n",
    );
    writeFile(
      path.join(tmpDir, "packages", "python", "real-pkg", "BUILD"),
      "echo real\n",
    );

    const packages = discoverPackages(tmpDir);
    expect(packages).toHaveLength(1);
    expect(packages[0].name).toBe("python/real-pkg");
  });

  it("should stop recursing when BUILD file found", () => {
    const parentDir = path.join(tmpDir, "packages", "python", "parent");
    writeFile(path.join(parentDir, "BUILD"), "echo parent\n");
    writeFile(path.join(parentDir, "sub", "BUILD"), "echo child\n");

    const packages = discoverPackages(tmpDir);
    expect(packages).toHaveLength(1);
    expect(packages[0].name).toBe("python/parent");
  });

  it("should return empty array for empty directory", () => {
    const packages = discoverPackages(tmpDir);
    expect(packages).toHaveLength(0);
  });

  it("should use platform-specific BUILD file", () => {
    const pkgDir = path.join(tmpDir, "packages", "python", "my-pkg");
    writeFile(path.join(pkgDir, "BUILD"), "generic cmd\n");
    writeFile(path.join(pkgDir, "BUILD_mac"), "mac cmd\n");

    const packages = discoverPackages(tmpDir, "darwin");
    expect(packages).toHaveLength(1);
    expect(packages[0].buildCommands).toEqual(["mac cmd"]);
  });

  it("should detect multiple languages", () => {
    writeFile(
      path.join(tmpDir, "packages", "python", "py-pkg", "BUILD"),
      "echo py\n",
    );
    writeFile(
      path.join(tmpDir, "packages", "ruby", "rb-pkg", "BUILD"),
      "echo rb\n",
    );
    writeFile(
      path.join(tmpDir, "programs", "go", "go-tool", "BUILD"),
      "echo go\n",
    );

    const packages = discoverPackages(tmpDir);
    expect(packages).toHaveLength(3);
    expect(packages.map((p) => p.language).sort()).toEqual([
      "go",
      "python",
      "ruby",
    ]);
  });
});

// ---------------------------------------------------------------------------
// Tests: SKIP_DIRS constant
// ---------------------------------------------------------------------------

describe("SKIP_DIRS", () => {
  it("should contain common skip directories", () => {
    expect(SKIP_DIRS.has(".git")).toBe(true);
    expect(SKIP_DIRS.has("node_modules")).toBe(true);
    expect(SKIP_DIRS.has("__pycache__")).toBe(true);
    expect(SKIP_DIRS.has(".venv")).toBe(true);
    expect(SKIP_DIRS.has("target")).toBe(true);
    expect(SKIP_DIRS.has(".claude")).toBe(true);
  });
});
