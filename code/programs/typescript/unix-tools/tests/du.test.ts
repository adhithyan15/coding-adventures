/**
 * Tests for du -- estimate file space usage.
 *
 * We test the exported business logic functions: formatDuSize,
 * shouldExclude, and diskUsage. We create temporary directories
 * with known file sizes to verify the calculations.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  formatDuSize,
  shouldExclude,
  diskUsage,
  DuOptions,
  DuEntry,
} from "../src/du.js";

// ---------------------------------------------------------------------------
// Helper: default DuOptions.
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<DuOptions> = {}): DuOptions {
  return {
    all: false,
    humanReadable: false,
    si: false,
    summarize: false,
    total: false,
    maxDepth: -1,
    dereference: false,
    exclude: [],
    nullTerminated: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// formatDuSize.
// ---------------------------------------------------------------------------

describe("formatDuSize", () => {
  it("should show bytes in 1K-blocks by default", () => {
    // 2048 bytes => 2 (1K-blocks).
    expect(formatDuSize(2048, false, false)).toBe("2");
  });

  it("should round up to next 1K-block", () => {
    // 1500 bytes => ceil(1500/1024) = 2.
    expect(formatDuSize(1500, false, false)).toBe("2");
  });

  it("should show 0 for 0 bytes", () => {
    expect(formatDuSize(0, false, false)).toBe("0");
  });

  it("should show human-readable format for KB", () => {
    // 500 bytes => "500B" (less than 1024).
    expect(formatDuSize(500, true, false)).toBe("500B");
  });

  it("should show human-readable format for MB", () => {
    // 2 * 1024 * 1024 bytes = 2MB.
    expect(formatDuSize(2 * 1024 * 1024, true, false)).toBe("2.0M");
  });

  it("should show human-readable format for GB", () => {
    // 1.5 * 1024^3 bytes.
    expect(formatDuSize(1.5 * 1024 * 1024 * 1024, true, false)).toBe("1.5G");
  });

  it("should use powers of 1000 with si flag", () => {
    // 2,000,000 bytes in SI = 2.0M.
    expect(formatDuSize(2000000, true, true)).toBe("2.0M");
  });
});

// ---------------------------------------------------------------------------
// shouldExclude.
// ---------------------------------------------------------------------------

describe("shouldExclude", () => {
  it("should match exact names", () => {
    expect(shouldExclude("node_modules", ["node_modules"])).toBe(true);
  });

  it("should not match non-matching names", () => {
    expect(shouldExclude("src", ["node_modules"])).toBe(false);
  });

  it("should match suffix patterns", () => {
    expect(shouldExclude("file.log", ["*.log"])).toBe(true);
  });

  it("should match prefix patterns", () => {
    expect(shouldExclude("test_data", ["test*"])).toBe(true);
  });

  it("should match substring patterns", () => {
    expect(shouldExclude("my_temp_file", ["*temp*"])).toBe(true);
  });

  it("should return false for empty pattern list", () => {
    expect(shouldExclude("anything", [])).toBe(false);
  });

  it("should match against multiple patterns", () => {
    expect(shouldExclude("file.log", ["*.txt", "*.log"])).toBe(true);
    expect(shouldExclude("file.txt", ["*.txt", "*.log"])).toBe(true);
    expect(shouldExclude("file.md", ["*.txt", "*.log"])).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// diskUsage (integration test with temp directory).
// ---------------------------------------------------------------------------

describe("diskUsage", () => {
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "du-test-"));
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  it("should return non-zero size for a directory with files", () => {
    // Create a file with known content.
    fs.writeFileSync(path.join(tmpDir, "file.txt"), "hello world\n");

    const entries: DuEntry[] = [];
    const total = diskUsage(tmpDir, defaultOpts(), entries, 0);

    expect(total).toBeGreaterThan(0);
    expect(entries.length).toBeGreaterThan(0);
  });

  it("should include the directory itself in entries", () => {
    fs.writeFileSync(path.join(tmpDir, "file.txt"), "test");

    const entries: DuEntry[] = [];
    diskUsage(tmpDir, defaultOpts(), entries, 0);

    const dirEntry = entries.find((e) => e.path === tmpDir);
    expect(dirEntry).toBeTruthy();
  });

  it("should include files in -a mode", () => {
    fs.writeFileSync(path.join(tmpDir, "file.txt"), "test content");

    const entries: DuEntry[] = [];
    diskUsage(tmpDir, defaultOpts({ all: true }), entries, 0);

    const fileEntry = entries.find((e) =>
      e.path.endsWith("file.txt")
    );
    expect(fileEntry).toBeTruthy();
  });

  it("should not include files in default mode (only directories)", () => {
    fs.writeFileSync(path.join(tmpDir, "file.txt"), "test");

    const entries: DuEntry[] = [];
    diskUsage(tmpDir, defaultOpts(), entries, 0);

    const fileEntry = entries.find((e) =>
      e.path.endsWith("file.txt")
    );
    expect(fileEntry).toBeUndefined();
  });

  it("should handle nested directories", () => {
    const subDir = path.join(tmpDir, "sub");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(subDir, "nested.txt"), "nested content");

    const entries: DuEntry[] = [];
    diskUsage(tmpDir, defaultOpts(), entries, 0);

    // Should have entries for both directories.
    expect(entries.length).toBeGreaterThanOrEqual(2);
  });

  it("should only show top-level with summarize mode", () => {
    const subDir = path.join(tmpDir, "sub");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(subDir, "file.txt"), "test");

    const entries: DuEntry[] = [];
    diskUsage(tmpDir, defaultOpts({ summarize: true }), entries, 0);

    // Should only have the top-level entry.
    expect(entries.length).toBe(1);
    expect(entries[0].path).toBe(tmpDir);
  });

  it("should respect maxDepth", () => {
    const subDir = path.join(tmpDir, "sub");
    const subSubDir = path.join(subDir, "subsub");
    fs.mkdirSync(subDir);
    fs.mkdirSync(subSubDir);
    fs.writeFileSync(path.join(subSubDir, "file.txt"), "test");

    const entries: DuEntry[] = [];
    diskUsage(tmpDir, defaultOpts({ maxDepth: 1 }), entries, 0);

    // Should have top-level and sub, but not subsub.
    const paths = entries.map((e) => e.path);
    expect(paths).toContain(tmpDir);
    expect(paths).toContain(subDir);
    expect(paths).not.toContain(subSubDir);
  });

  it("should exclude files matching patterns", () => {
    fs.writeFileSync(path.join(tmpDir, "keep.txt"), "keep");
    fs.writeFileSync(path.join(tmpDir, "skip.log"), "skip");

    const entries: DuEntry[] = [];
    const opts = defaultOpts({ all: true, exclude: ["*.log"] });
    diskUsage(tmpDir, opts, entries, 0);

    const logEntry = entries.find((e) => e.path.endsWith(".log"));
    expect(logEntry).toBeUndefined();
  });

  it("should handle empty directory", () => {
    const entries: DuEntry[] = [];
    const total = diskUsage(tmpDir, defaultOpts(), entries, 0);

    // Empty directory still has some size (directory metadata).
    expect(entries.length).toBe(1);
    expect(entries[0].path).toBe(tmpDir);
  });
});
