/**
 * Tests for tar -- an archiving utility.
 *
 * We test the exported business logic functions: createHeaderBlock,
 * parseHeaderBlock, createArchive, listArchive, extractArchive,
 * and stripPathComponents.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  createHeaderBlock,
  parseHeaderBlock,
  createArchive,
  listArchive,
  extractArchive,
  stripPathComponents,
  TarHeader,
} from "../src/tar.js";

// ---------------------------------------------------------------------------
// Helpers: temp directory management.
// ---------------------------------------------------------------------------

let tmpDir: string;

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "tar-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// createHeaderBlock / parseHeaderBlock: round-trip tests.
// ---------------------------------------------------------------------------

describe("createHeaderBlock", () => {
  it("should create a 512-byte block", () => {
    const header: TarHeader = {
      name: "test.txt",
      mode: 0o644,
      uid: 1000,
      gid: 1000,
      size: 100,
      mtime: 1700000000,
      typeflag: "0",
      linkname: "",
    };

    const block = createHeaderBlock(header);
    expect(block.length).toBe(512);
  });

  it("should contain the filename", () => {
    const header: TarHeader = {
      name: "hello.txt",
      mode: 0o644,
      uid: 0,
      gid: 0,
      size: 0,
      mtime: 0,
      typeflag: "0",
      linkname: "",
    };

    const block = createHeaderBlock(header);
    expect(block.toString("ascii", 0, 9)).toBe("hello.txt");
  });

  it("should contain the ustar magic string", () => {
    const header: TarHeader = {
      name: "test",
      mode: 0o755,
      uid: 0,
      gid: 0,
      size: 0,
      mtime: 0,
      typeflag: "0",
      linkname: "",
    };

    const block = createHeaderBlock(header);
    const magic = block.toString("ascii", 257, 262);
    expect(magic).toBe("ustar");
  });
});

describe("parseHeaderBlock", () => {
  it("should return null for all-zero block (end-of-archive)", () => {
    const block = Buffer.alloc(512, 0);
    expect(parseHeaderBlock(block)).toBeNull();
  });

  it("should round-trip a header correctly", () => {
    const original: TarHeader = {
      name: "test.txt",
      mode: 0o644,
      uid: 1000,
      gid: 100,
      size: 42,
      mtime: 1700000000,
      typeflag: "0",
      linkname: "",
    };

    const block = createHeaderBlock(original);
    const parsed = parseHeaderBlock(block);

    expect(parsed).not.toBeNull();
    expect(parsed!.name).toBe("test.txt");
    expect(parsed!.mode).toBe(0o644);
    expect(parsed!.uid).toBe(1000);
    expect(parsed!.gid).toBe(100);
    expect(parsed!.size).toBe(42);
    expect(parsed!.mtime).toBe(1700000000);
    expect(parsed!.typeflag).toBe("0");
  });

  it("should handle directory entries", () => {
    const header: TarHeader = {
      name: "subdir/",
      mode: 0o755,
      uid: 0,
      gid: 0,
      size: 0,
      mtime: 1700000000,
      typeflag: "5",
      linkname: "",
    };

    const block = createHeaderBlock(header);
    const parsed = parseHeaderBlock(block);

    expect(parsed!.name).toBe("subdir/");
    expect(parsed!.typeflag).toBe("5");
    expect(parsed!.size).toBe(0);
  });

  it("should handle long filenames (up to 100 chars)", () => {
    const longName = "a".repeat(99) + ".";
    const header: TarHeader = {
      name: longName,
      mode: 0o644,
      uid: 0,
      gid: 0,
      size: 0,
      mtime: 0,
      typeflag: "0",
      linkname: "",
    };

    const block = createHeaderBlock(header);
    const parsed = parseHeaderBlock(block);

    expect(parsed!.name).toBe(longName);
  });
});

// ---------------------------------------------------------------------------
// stripPathComponents: path manipulation.
// ---------------------------------------------------------------------------

describe("stripPathComponents", () => {
  it("should strip leading components", () => {
    expect(stripPathComponents("a/b/c/file.txt", 2)).toBe("c/file.txt");
  });

  it("should return empty string when stripping all components", () => {
    expect(stripPathComponents("a/b", 2)).toBe("");
  });

  it("should return name unchanged with count 0", () => {
    expect(stripPathComponents("a/b/c", 0)).toBe("a/b/c");
  });

  it("should strip one component", () => {
    expect(stripPathComponents("dir/file.txt", 1)).toBe("file.txt");
  });

  it("should handle names without separators", () => {
    expect(stripPathComponents("file.txt", 1)).toBe("");
  });

  it("should handle trailing slashes", () => {
    expect(stripPathComponents("a/b/c/", 1)).toBe("b/c");
  });

  it("should return empty for over-stripping", () => {
    expect(stripPathComponents("a", 5)).toBe("");
  });
});

// ---------------------------------------------------------------------------
// createArchive / listArchive: creating and listing archives.
// ---------------------------------------------------------------------------

describe("createArchive", () => {
  it("should create an archive with a single file", () => {
    fs.writeFileSync(path.join(tmpDir, "hello.txt"), "Hello, world!");

    const result = createArchive(["hello.txt"], tmpDir);

    expect(result.buffer.length).toBeGreaterThan(0);
    expect(result.entries).toContain("hello.txt");
  });

  it("should create an archive with multiple files", () => {
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "AAA");
    fs.writeFileSync(path.join(tmpDir, "b.txt"), "BBB");

    const result = createArchive(["a.txt", "b.txt"], tmpDir);

    expect(result.entries).toContain("a.txt");
    expect(result.entries).toContain("b.txt");
  });

  it("should include directories recursively", () => {
    const subDir = path.join(tmpDir, "subdir");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(subDir, "nested.txt"), "nested content");

    const result = createArchive(["subdir"], tmpDir);

    expect(result.entries.some(e => e.includes("subdir"))).toBe(true);
    expect(result.entries.some(e => e.includes("nested.txt"))).toBe(true);
  });

  it("should handle empty files", () => {
    fs.writeFileSync(path.join(tmpDir, "empty.txt"), "");

    const result = createArchive(["empty.txt"], tmpDir);

    expect(result.entries).toContain("empty.txt");
  });

  it("should end with two zero blocks", () => {
    fs.writeFileSync(path.join(tmpDir, "test.txt"), "test");

    const result = createArchive(["test.txt"], tmpDir);
    const buf = result.buffer;

    // The last 1024 bytes should be all zeros.
    const lastTwo = buf.subarray(buf.length - 1024);
    let allZero = true;
    for (let i = 0; i < lastTwo.length; i++) {
      if (lastTwo[i] !== 0) { allZero = false; break; }
    }
    expect(allZero).toBe(true);
  });
});

describe("listArchive", () => {
  it("should list entries from an archive", () => {
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "AAA");
    fs.writeFileSync(path.join(tmpDir, "b.txt"), "BBB");

    const archive = createArchive(["a.txt", "b.txt"], tmpDir);
    const entries = listArchive(archive.buffer);

    expect(entries).toHaveLength(2);
    expect(entries[0].name).toBe("a.txt");
    expect(entries[1].name).toBe("b.txt");
  });

  it("should list directories and their contents", () => {
    const subDir = path.join(tmpDir, "mydir");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(subDir, "file.txt"), "content");

    const archive = createArchive(["mydir"], tmpDir);
    const entries = listArchive(archive.buffer);

    expect(entries.some(e => e.name === "mydir/")).toBe(true);
    expect(entries.some(e => e.name === "mydir/file.txt")).toBe(true);
  });

  it("should report correct file sizes", () => {
    fs.writeFileSync(path.join(tmpDir, "sized.txt"), "12345");

    const archive = createArchive(["sized.txt"], tmpDir);
    const entries = listArchive(archive.buffer);

    expect(entries[0].size).toBe(5);
  });

  it("should handle empty archive (just zero blocks)", () => {
    const emptyArchive = Buffer.alloc(1024, 0);
    const entries = listArchive(emptyArchive);

    expect(entries).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// extractArchive: extracting from archives.
// ---------------------------------------------------------------------------

describe("extractArchive", () => {
  it("should extract a single file", () => {
    // Create archive.
    fs.writeFileSync(path.join(tmpDir, "hello.txt"), "Hello, world!");
    const archive = createArchive(["hello.txt"], tmpDir);

    // Extract to a different directory.
    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);

    const extracted = extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: false,
      stripComponents: 0,
      verbose: false,
      filterFiles: [],
    });

    expect(extracted).toContain("hello.txt");

    const content = fs.readFileSync(path.join(extractDir, "hello.txt"), "utf-8");
    expect(content).toBe("Hello, world!");
  });

  it("should extract directories", () => {
    const subDir = path.join(tmpDir, "mydir");
    fs.mkdirSync(subDir);
    fs.writeFileSync(path.join(subDir, "file.txt"), "content");

    const archive = createArchive(["mydir"], tmpDir);

    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);

    extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: false,
      stripComponents: 0,
      verbose: false,
      filterFiles: [],
    });

    expect(fs.existsSync(path.join(extractDir, "mydir"))).toBe(true);
    expect(fs.existsSync(path.join(extractDir, "mydir", "file.txt"))).toBe(true);
  });

  it("should respect keepOldFiles", () => {
    // Create archive.
    fs.writeFileSync(path.join(tmpDir, "file.txt"), "new content");
    const archive = createArchive(["file.txt"], tmpDir);

    // Create existing file in extract directory.
    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);
    fs.writeFileSync(path.join(extractDir, "file.txt"), "old content");

    extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: true,
      stripComponents: 0,
      verbose: false,
      filterFiles: [],
    });

    const content = fs.readFileSync(path.join(extractDir, "file.txt"), "utf-8");
    expect(content).toBe("old content");
  });

  it("should strip path components", () => {
    const nested = path.join(tmpDir, "a", "b");
    fs.mkdirSync(nested, { recursive: true });
    fs.writeFileSync(path.join(nested, "file.txt"), "deep content");

    const archive = createArchive(["a"], tmpDir);

    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);

    extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: false,
      stripComponents: 1,
      verbose: false,
      filterFiles: [],
    });

    // "a/b/file.txt" with strip 1 becomes "b/file.txt".
    expect(fs.existsSync(path.join(extractDir, "b", "file.txt"))).toBe(true);
  });

  it("should filter files during extraction", () => {
    fs.writeFileSync(path.join(tmpDir, "keep.txt"), "keep me");
    fs.writeFileSync(path.join(tmpDir, "skip.txt"), "skip me");

    const archive = createArchive(["keep.txt", "skip.txt"], tmpDir);

    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);

    extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: false,
      stripComponents: 0,
      verbose: false,
      filterFiles: ["keep.txt"],
    });

    expect(fs.existsSync(path.join(extractDir, "keep.txt"))).toBe(true);
    expect(fs.existsSync(path.join(extractDir, "skip.txt"))).toBe(false);
  });

  it("should handle multiple files", () => {
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "AAA");
    fs.writeFileSync(path.join(tmpDir, "b.txt"), "BBB");
    fs.writeFileSync(path.join(tmpDir, "c.txt"), "CCC");

    const archive = createArchive(["a.txt", "b.txt", "c.txt"], tmpDir);

    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);

    const extracted = extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: false,
      stripComponents: 0,
      verbose: false,
      filterFiles: [],
    });

    expect(extracted).toHaveLength(3);
    expect(fs.readFileSync(path.join(extractDir, "a.txt"), "utf-8")).toBe("AAA");
    expect(fs.readFileSync(path.join(extractDir, "b.txt"), "utf-8")).toBe("BBB");
    expect(fs.readFileSync(path.join(extractDir, "c.txt"), "utf-8")).toBe("CCC");
  });

  it("should round-trip create + extract", () => {
    // Create files with various sizes.
    fs.writeFileSync(path.join(tmpDir, "small.txt"), "small");
    fs.writeFileSync(path.join(tmpDir, "medium.txt"), "x".repeat(1000));
    fs.writeFileSync(path.join(tmpDir, "empty.txt"), "");

    const archive = createArchive(["small.txt", "medium.txt", "empty.txt"], tmpDir);

    const extractDir = path.join(tmpDir, "extracted");
    fs.mkdirSync(extractDir);

    extractArchive(archive.buffer, extractDir, {
      preservePermissions: false,
      keepOldFiles: false,
      stripComponents: 0,
      verbose: false,
      filterFiles: [],
    });

    expect(fs.readFileSync(path.join(extractDir, "small.txt"), "utf-8")).toBe("small");
    expect(fs.readFileSync(path.join(extractDir, "medium.txt"), "utf-8")).toBe("x".repeat(1000));
    expect(fs.readFileSync(path.join(extractDir, "empty.txt"), "utf-8")).toBe("");
  });
});
