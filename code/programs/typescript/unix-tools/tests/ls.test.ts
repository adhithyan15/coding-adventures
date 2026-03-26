/**
 * Tests for ls -- list directory contents.
 *
 * We test the exported business logic functions: listDirectory,
 * formatSize, formatPermissions, formatDate, formatEntry, and the
 * DirEntry/ListOptions types.
 *
 * All tests use a temporary directory to avoid depending on the
 * real filesystem layout.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  listDirectory,
  formatSize,
  formatPermissions,
  formatDate,
  formatEntry,
  ListOptions,
  DirEntry,
} from "../src/ls.js";

// ---------------------------------------------------------------------------
// Helpers: temp directory management and default options.
// ---------------------------------------------------------------------------

let tmpDir: string;

function defaultOpts(overrides: Partial<ListOptions> = {}): ListOptions {
  return {
    all: false,
    almostAll: false,
    long: false,
    humanReadable: false,
    reverse: false,
    sortBySize: false,
    sortByTime: false,
    sortByExtension: false,
    unsorted: false,
    classify: false,
    onePerLine: false,
    ...overrides,
  };
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "ls-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// formatSize: human-readable file sizes.
// ---------------------------------------------------------------------------

describe("formatSize", () => {
  it("should return raw number for small sizes", () => {
    expect(formatSize(0)).toBe("0");
    expect(formatSize(500)).toBe("500");
    expect(formatSize(1023)).toBe("1023");
  });

  it("should format kilobytes", () => {
    expect(formatSize(1024)).toBe("1.0K");
    expect(formatSize(2048)).toBe("2.0K");
  });

  it("should format megabytes", () => {
    expect(formatSize(1048576)).toBe("1.0M");
    expect(formatSize(1572864)).toBe("1.5M");
  });

  it("should format gigabytes", () => {
    expect(formatSize(1073741824)).toBe("1.0G");
  });

  it("should handle intermediate values", () => {
    // 1536 bytes = 1.5K
    expect(formatSize(1536)).toBe("1.5K");
  });
});

// ---------------------------------------------------------------------------
// formatPermissions: Unix permission strings.
// ---------------------------------------------------------------------------

describe("formatPermissions", () => {
  it("should format directory permissions", () => {
    expect(formatPermissions(0o755, true, false)).toBe("drwxr-xr-x");
  });

  it("should format regular file permissions", () => {
    expect(formatPermissions(0o644, false, false)).toBe("-rw-r--r--");
  });

  it("should format symlink permissions", () => {
    expect(formatPermissions(0o777, false, true)).toBe("lrwxrwxrwx");
  });

  it("should format no permissions", () => {
    expect(formatPermissions(0o000, false, false)).toBe("----------");
  });

  it("should format executable file", () => {
    expect(formatPermissions(0o755, false, false)).toBe("-rwxr-xr-x");
  });

  it("should format write-only file", () => {
    expect(formatPermissions(0o200, false, false)).toBe("--w-------");
  });

  it("should format read-only file", () => {
    expect(formatPermissions(0o444, false, false)).toBe("-r--r--r--");
  });
});

// ---------------------------------------------------------------------------
// formatDate: ls-style date formatting.
// ---------------------------------------------------------------------------

describe("formatDate", () => {
  it("should format recent dates with time", () => {
    const recent = new Date();
    recent.setDate(recent.getDate() - 1); // yesterday
    const result = formatDate(recent);

    // Should contain month abbreviation and time (HH:MM).
    expect(result).toMatch(/[A-Z][a-z]{2}\s+\d{1,2}\s+\d{2}:\d{2}/);
  });

  it("should format old dates with year", () => {
    const old = new Date("2020-06-15T10:30:00");
    const result = formatDate(old);

    // Should contain year instead of time.
    expect(result).toContain("2020");
    expect(result).toContain("Jun");
  });

  it("should handle Jan 1 date", () => {
    const date = new Date("2020-01-01T00:00:00");
    const result = formatDate(date);
    expect(result).toContain("Jan");
  });
});

// ---------------------------------------------------------------------------
// listDirectory: directory listing.
// ---------------------------------------------------------------------------

describe("listDirectory", () => {
  it("should list files in a directory", () => {
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "aaa");
    fs.writeFileSync(path.join(tmpDir, "b.txt"), "bbb");

    const entries = listDirectory(tmpDir, defaultOpts());

    const names = entries.map((e) => e.name);
    expect(names).toContain("a.txt");
    expect(names).toContain("b.txt");
  });

  it("should sort entries alphabetically by default", () => {
    fs.writeFileSync(path.join(tmpDir, "c.txt"), "c");
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "a");
    fs.writeFileSync(path.join(tmpDir, "b.txt"), "b");

    const entries = listDirectory(tmpDir, defaultOpts());
    const names = entries.map((e) => e.name);

    expect(names).toEqual(["a.txt", "b.txt", "c.txt"]);
  });

  it("should hide hidden files by default", () => {
    fs.writeFileSync(path.join(tmpDir, ".hidden"), "secret");
    fs.writeFileSync(path.join(tmpDir, "visible.txt"), "public");

    const entries = listDirectory(tmpDir, defaultOpts());
    const names = entries.map((e) => e.name);

    expect(names).not.toContain(".hidden");
    expect(names).toContain("visible.txt");
  });

  it("should show hidden files with -a flag", () => {
    fs.writeFileSync(path.join(tmpDir, ".hidden"), "secret");
    fs.writeFileSync(path.join(tmpDir, "visible.txt"), "public");

    const entries = listDirectory(tmpDir, defaultOpts({ all: true }));
    const names = entries.map((e) => e.name);

    expect(names).toContain(".");
    expect(names).toContain("..");
    expect(names).toContain(".hidden");
    expect(names).toContain("visible.txt");
  });

  it("should show hidden files but not . and .. with -A flag", () => {
    fs.writeFileSync(path.join(tmpDir, ".hidden"), "secret");
    fs.writeFileSync(path.join(tmpDir, "visible.txt"), "public");

    const entries = listDirectory(tmpDir, defaultOpts({ almostAll: true }));
    const names = entries.map((e) => e.name);

    expect(names).not.toContain(".");
    expect(names).not.toContain("..");
    expect(names).toContain(".hidden");
    expect(names).toContain("visible.txt");
  });

  it("should handle empty directory", () => {
    const entries = listDirectory(tmpDir, defaultOpts());
    expect(entries).toEqual([]);
  });

  it("should throw for non-existent directory", () => {
    expect(() =>
      listDirectory(path.join(tmpDir, "nope"), defaultOpts())
    ).toThrow(/cannot access/);
  });

  // -------------------------------------------------------------------------
  // Reverse sort.
  // -------------------------------------------------------------------------

  it("should reverse sort order with reverse flag", () => {
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "a");
    fs.writeFileSync(path.join(tmpDir, "b.txt"), "b");
    fs.writeFileSync(path.join(tmpDir, "c.txt"), "c");

    const entries = listDirectory(tmpDir, defaultOpts({ reverse: true }));
    const names = entries.map((e) => e.name);

    expect(names).toEqual(["c.txt", "b.txt", "a.txt"]);
  });

  // -------------------------------------------------------------------------
  // Sort by size.
  // -------------------------------------------------------------------------

  it("should sort by size with sortBySize flag (largest first)", () => {
    fs.writeFileSync(path.join(tmpDir, "small.txt"), "a");
    fs.writeFileSync(path.join(tmpDir, "big.txt"), "aaaaaaaaaa");
    fs.writeFileSync(path.join(tmpDir, "medium.txt"), "aaaa");

    const entries = listDirectory(tmpDir, defaultOpts({ sortBySize: true }));
    const names = entries.map((e) => e.name);

    expect(names[0]).toBe("big.txt");
    expect(names[names.length - 1]).toBe("small.txt");
  });

  // -------------------------------------------------------------------------
  // Sort by extension.
  // -------------------------------------------------------------------------

  it("should sort by extension with sortByExtension flag", () => {
    fs.writeFileSync(path.join(tmpDir, "file.c"), "c");
    fs.writeFileSync(path.join(tmpDir, "file.a"), "a");
    fs.writeFileSync(path.join(tmpDir, "file.b"), "b");

    const entries = listDirectory(
      tmpDir,
      defaultOpts({ sortByExtension: true })
    );
    const names = entries.map((e) => e.name);

    expect(names).toEqual(["file.a", "file.b", "file.c"]);
  });

  // -------------------------------------------------------------------------
  // Unsorted (-U).
  // -------------------------------------------------------------------------

  it("should not sort when unsorted flag is set", () => {
    fs.writeFileSync(path.join(tmpDir, "z.txt"), "z");
    fs.writeFileSync(path.join(tmpDir, "a.txt"), "a");

    const entries = listDirectory(tmpDir, defaultOpts({ unsorted: true }));

    // Just verify we get the right count -- order depends on fs.
    expect(entries.length).toBe(2);
  });

  // -------------------------------------------------------------------------
  // DirEntry metadata.
  // -------------------------------------------------------------------------

  it("should populate DirEntry metadata correctly", () => {
    fs.writeFileSync(path.join(tmpDir, "test.txt"), "hello");

    const entries = listDirectory(tmpDir, defaultOpts());
    const entry = entries.find((e) => e.name === "test.txt");

    expect(entry).toBeDefined();
    expect(entry!.size).toBe(5);
    expect(entry!.isDirectory).toBe(false);
    expect(entry!.isSymlink).toBe(false);
    expect(entry!.mtime).toBeInstanceOf(Date);
  });

  it("should identify directories in DirEntry", () => {
    fs.mkdirSync(path.join(tmpDir, "subdir"));

    const entries = listDirectory(tmpDir, defaultOpts());
    const entry = entries.find((e) => e.name === "subdir");

    expect(entry).toBeDefined();
    expect(entry!.isDirectory).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// formatEntry: formatted output lines.
// ---------------------------------------------------------------------------

describe("formatEntry", () => {
  it("should format a short entry (just the name)", () => {
    const entry: DirEntry = {
      name: "file.txt",
      fullPath: "/tmp/file.txt",
      size: 100,
      mtime: new Date(),
      isDirectory: false,
      isSymlink: false,
      mode: 0o644,
      nlink: 1,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(entry, defaultOpts());
    expect(result).toBe("file.txt");
  });

  it("should append / for directories in classify mode", () => {
    const entry: DirEntry = {
      name: "subdir",
      fullPath: "/tmp/subdir",
      size: 0,
      mtime: new Date(),
      isDirectory: true,
      isSymlink: false,
      mode: 0o755,
      nlink: 2,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(entry, defaultOpts({ classify: true }));
    expect(result).toBe("subdir/");
  });

  it("should append @ for symlinks in classify mode", () => {
    const entry: DirEntry = {
      name: "link",
      fullPath: "/tmp/link",
      size: 10,
      mtime: new Date(),
      isDirectory: false,
      isSymlink: true,
      mode: 0o777,
      nlink: 1,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(entry, defaultOpts({ classify: true }));
    expect(result).toBe("link@");
  });

  it("should append * for executable files in classify mode", () => {
    const entry: DirEntry = {
      name: "script.sh",
      fullPath: "/tmp/script.sh",
      size: 50,
      mtime: new Date(),
      isDirectory: false,
      isSymlink: false,
      mode: 0o755,
      nlink: 1,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(entry, defaultOpts({ classify: true }));
    expect(result).toBe("script.sh*");
  });

  it("should not append classifier when classify is false", () => {
    const entry: DirEntry = {
      name: "subdir",
      fullPath: "/tmp/subdir",
      size: 0,
      mtime: new Date(),
      isDirectory: true,
      isSymlink: false,
      mode: 0o755,
      nlink: 2,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(entry, defaultOpts({ classify: false }));
    expect(result).toBe("subdir");
  });

  it("should produce long format with -l flag", () => {
    const entry: DirEntry = {
      name: "file.txt",
      fullPath: "/tmp/file.txt",
      size: 1024,
      mtime: new Date(),
      isDirectory: false,
      isSymlink: false,
      mode: 0o644,
      nlink: 1,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(entry, defaultOpts({ long: true }));

    // Long format should contain permission string.
    expect(result).toContain("-rw-r--r--");
    expect(result).toContain("file.txt");
  });

  it("should show human-readable sizes in long format", () => {
    const entry: DirEntry = {
      name: "big.dat",
      fullPath: "/tmp/big.dat",
      size: 1048576, // 1 MB
      mtime: new Date(),
      isDirectory: false,
      isSymlink: false,
      mode: 0o644,
      nlink: 1,
      uid: 1000,
      gid: 1000,
    };

    const result = formatEntry(
      entry,
      defaultOpts({ long: true, humanReadable: true })
    );

    expect(result).toContain("1.0M");
  });
});
