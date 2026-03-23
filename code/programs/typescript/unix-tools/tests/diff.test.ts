/**
 * Tests for diff -- compare files line by line.
 *
 * We test the exported business logic functions: normalizeLine,
 * computeLcsTable, computeDiff, formatNormal, formatUnified,
 * formatContext, diffLines, and diffDirectories.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  normalizeLine,
  computeLcsTable,
  computeDiff,
  formatNormal,
  formatUnified,
  formatContext,
  diffLines,
  diffDirectories,
  DiffOptions,
  DiffEdit,
} from "../src/diff.js";

// ---------------------------------------------------------------------------
// Helpers: default options and temp directory.
// ---------------------------------------------------------------------------

let tmpDir: string;

function defaultOpts(overrides: Partial<DiffOptions> = {}): DiffOptions {
  return {
    contextLines: 3,
    format: "normal",
    ignoreCase: false,
    ignoreSpaceChange: false,
    ignoreAllSpace: false,
    ignoreBlankLines: false,
    brief: false,
    recursive: false,
    ...overrides,
  };
}

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "diff-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// normalizeLine: line normalization for comparison.
// ---------------------------------------------------------------------------

describe("normalizeLine", () => {
  it("should return the line unchanged with default options", () => {
    expect(normalizeLine("Hello World", defaultOpts())).toBe("Hello World");
  });

  it("should lowercase with ignoreCase", () => {
    expect(normalizeLine("Hello WORLD", defaultOpts({ ignoreCase: true }))).toBe("hello world");
  });

  it("should remove all whitespace with ignoreAllSpace", () => {
    expect(normalizeLine("a  b  c", defaultOpts({ ignoreAllSpace: true }))).toBe("abc");
  });

  it("should collapse whitespace with ignoreSpaceChange", () => {
    expect(normalizeLine("a  b   c", defaultOpts({ ignoreSpaceChange: true }))).toBe("a b c");
  });

  it("should trim with ignoreSpaceChange", () => {
    expect(normalizeLine("  hello  ", defaultOpts({ ignoreSpaceChange: true }))).toBe("hello");
  });

  it("should handle empty line", () => {
    expect(normalizeLine("", defaultOpts())).toBe("");
  });

  it("should combine ignoreCase and ignoreSpaceChange", () => {
    expect(normalizeLine("  Hello  WORLD  ", defaultOpts({
      ignoreCase: true,
      ignoreSpaceChange: true,
    }))).toBe("hello world");
  });
});

// ---------------------------------------------------------------------------
// computeLcsTable: LCS dynamic programming table.
// ---------------------------------------------------------------------------

describe("computeLcsTable", () => {
  it("should compute LCS for identical sequences", () => {
    const dp = computeLcsTable(["a", "b", "c"], ["a", "b", "c"]);
    expect(dp[3][3]).toBe(3);
  });

  it("should compute LCS for completely different sequences", () => {
    const dp = computeLcsTable(["a", "b"], ["c", "d"]);
    expect(dp[2][2]).toBe(0);
  });

  it("should compute LCS for partially matching sequences", () => {
    const dp = computeLcsTable(["a", "b", "c", "d"], ["a", "c", "d", "e"]);
    // LCS is ["a", "c", "d"] = length 3.
    expect(dp[4][4]).toBe(3);
  });

  it("should handle empty sequences", () => {
    const dp = computeLcsTable([], ["a", "b"]);
    expect(dp[0][2]).toBe(0);

    const dp2 = computeLcsTable(["a"], []);
    expect(dp2[1][0]).toBe(0);
  });

  it("should handle both empty", () => {
    const dp = computeLcsTable([], []);
    expect(dp[0][0]).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// computeDiff: computing edit operations.
// ---------------------------------------------------------------------------

describe("computeDiff", () => {
  it("should return no edits for identical files", () => {
    const edits = computeDiff(["a", "b", "c"], ["a", "b", "c"], defaultOpts());
    expect(edits).toHaveLength(0);
  });

  it("should detect a simple change", () => {
    const edits = computeDiff(["a", "b", "c"], ["a", "B", "c"], defaultOpts());
    expect(edits.length).toBeGreaterThan(0);
    // There should be a change involving "b" -> "B".
    const changeEdit = edits.find(e => e.type === "change" || e.type === "delete" || e.type === "add");
    expect(changeEdit).toBeDefined();
  });

  it("should detect additions", () => {
    const edits = computeDiff(["a", "c"], ["a", "b", "c"], defaultOpts());
    expect(edits.length).toBeGreaterThan(0);
  });

  it("should detect deletions", () => {
    const edits = computeDiff(["a", "b", "c"], ["a", "c"], defaultOpts());
    expect(edits.length).toBeGreaterThan(0);
  });

  it("should handle completely different files", () => {
    const edits = computeDiff(["a", "b"], ["c", "d"], defaultOpts());
    expect(edits.length).toBeGreaterThan(0);
  });

  it("should handle empty first file", () => {
    const edits = computeDiff([], ["a", "b"], defaultOpts());
    expect(edits.length).toBeGreaterThan(0);
  });

  it("should handle empty second file", () => {
    const edits = computeDiff(["a", "b"], [], defaultOpts());
    expect(edits.length).toBeGreaterThan(0);
  });

  it("should respect ignoreCase", () => {
    const edits = computeDiff(
      ["Hello", "World"],
      ["hello", "world"],
      defaultOpts({ ignoreCase: true })
    );
    expect(edits).toHaveLength(0);
  });

  it("should respect ignoreAllSpace", () => {
    const edits = computeDiff(
      ["a  b  c"],
      ["a b c"],
      defaultOpts({ ignoreAllSpace: true })
    );
    expect(edits).toHaveLength(0);
  });
});

// ---------------------------------------------------------------------------
// formatNormal: normal diff output.
// ---------------------------------------------------------------------------

describe("formatNormal", () => {
  it("should return empty string for no edits", () => {
    expect(formatNormal([])).toBe("");
  });

  it("should format a change edit", () => {
    const edits: DiffEdit[] = [{
      type: "change",
      startA: 2, endA: 2,
      startB: 2, endB: 2,
      linesA: ["old"],
      linesB: ["new"],
    }];

    const result = formatNormal(edits);
    expect(result).toContain("2c2");
    expect(result).toContain("< old");
    expect(result).toContain("---");
    expect(result).toContain("> new");
  });

  it("should format an add edit", () => {
    const edits: DiffEdit[] = [{
      type: "add",
      startA: 2, endA: 2,
      startB: 3, endB: 4,
      linesA: [],
      linesB: ["new1", "new2"],
    }];

    const result = formatNormal(edits);
    expect(result).toContain("a");
    expect(result).toContain("> new1");
    expect(result).toContain("> new2");
  });

  it("should format a delete edit", () => {
    const edits: DiffEdit[] = [{
      type: "delete",
      startA: 3, endA: 4,
      startB: 2, endB: 2,
      linesA: ["deleted1", "deleted2"],
      linesB: [],
    }];

    const result = formatNormal(edits);
    expect(result).toContain("d");
    expect(result).toContain("< deleted1");
    expect(result).toContain("< deleted2");
  });

  it("should format multi-line ranges", () => {
    const edits: DiffEdit[] = [{
      type: "change",
      startA: 2, endA: 4,
      startB: 2, endB: 3,
      linesA: ["old1", "old2", "old3"],
      linesB: ["new1", "new2"],
    }];

    const result = formatNormal(edits);
    expect(result).toContain("2,4c2,3");
  });
});

// ---------------------------------------------------------------------------
// formatUnified: unified diff output.
// ---------------------------------------------------------------------------

describe("formatUnified", () => {
  it("should return empty string for no edits", () => {
    expect(formatUnified([], [], [], "a", "b", 3)).toBe("");
  });

  it("should include file headers", () => {
    const edits: DiffEdit[] = [{
      type: "change",
      startA: 1, endA: 1,
      startB: 1, endB: 1,
      linesA: ["old"],
      linesB: ["new"],
    }];

    const result = formatUnified(edits, ["old"], ["new"], "file1.txt", "file2.txt", 3);
    expect(result).toContain("--- file1.txt");
    expect(result).toContain("+++ file2.txt");
  });

  it("should include @@ hunk headers", () => {
    const edits: DiffEdit[] = [{
      type: "change",
      startA: 1, endA: 1,
      startB: 1, endB: 1,
      linesA: ["old"],
      linesB: ["new"],
    }];

    const result = formatUnified(edits, ["old"], ["new"], "a", "b", 3);
    expect(result).toContain("@@");
  });

  it("should show deleted lines with - prefix", () => {
    const edits: DiffEdit[] = [{
      type: "delete",
      startA: 1, endA: 1,
      startB: 0, endB: 0,
      linesA: ["deleted"],
      linesB: [],
    }];

    const result = formatUnified(edits, ["deleted"], [], "a", "b", 0);
    expect(result).toContain("-deleted");
  });

  it("should show added lines with + prefix", () => {
    const edits: DiffEdit[] = [{
      type: "add",
      startA: 0, endA: 0,
      startB: 1, endB: 1,
      linesA: [],
      linesB: ["added"],
    }];

    const result = formatUnified(edits, [], ["added"], "a", "b", 0);
    expect(result).toContain("+added");
  });
});

// ---------------------------------------------------------------------------
// formatContext: context diff output.
// ---------------------------------------------------------------------------

describe("formatContext", () => {
  it("should return empty string for no edits", () => {
    expect(formatContext([], [], [], "a", "b", 3)).toBe("");
  });

  it("should include file headers with *** and ---", () => {
    const edits: DiffEdit[] = [{
      type: "change",
      startA: 1, endA: 1,
      startB: 1, endB: 1,
      linesA: ["old"],
      linesB: ["new"],
    }];

    const result = formatContext(edits, ["old"], ["new"], "file1.txt", "file2.txt", 3);
    expect(result).toContain("*** file1.txt");
    expect(result).toContain("--- file2.txt");
    expect(result).toContain("***************");
  });
});

// ---------------------------------------------------------------------------
// diffLines: high-level diff between line arrays.
// ---------------------------------------------------------------------------

describe("diffLines", () => {
  it("should return empty string for identical files", () => {
    const result = diffLines(["a", "b", "c"], ["a", "b", "c"], defaultOpts());
    expect(result).toBe("");
  });

  it("should produce normal format output by default", () => {
    const result = diffLines(["a", "old", "c"], ["a", "new", "c"], defaultOpts());
    expect(result).toContain("<");
    expect(result).toContain(">");
  });

  it("should produce unified format when requested", () => {
    const result = diffLines(
      ["a", "old", "c"],
      ["a", "new", "c"],
      defaultOpts({ format: "unified" })
    );
    expect(result).toContain("---");
    expect(result).toContain("+++");
    expect(result).toContain("@@");
  });

  it("should produce context format when requested", () => {
    const result = diffLines(
      ["a", "old", "c"],
      ["a", "new", "c"],
      defaultOpts({ format: "context" })
    );
    expect(result).toContain("***");
    expect(result).toContain("---");
  });

  it("should produce brief output when requested", () => {
    const result = diffLines(
      ["a", "old"],
      ["a", "new"],
      defaultOpts({ brief: true }),
      "file1",
      "file2"
    );
    expect(result).toContain("Files file1 and file2 differ");
  });

  it("should produce empty output for identical files in brief mode", () => {
    const result = diffLines(
      ["a", "b"],
      ["a", "b"],
      defaultOpts({ brief: true }),
      "file1",
      "file2"
    );
    expect(result).toBe("");
  });

  it("should handle empty files", () => {
    const result = diffLines([], ["a", "b"], defaultOpts());
    expect(result.length).toBeGreaterThan(0);
  });

  it("should ignore case when requested", () => {
    const result = diffLines(
      ["Hello", "World"],
      ["hello", "world"],
      defaultOpts({ ignoreCase: true })
    );
    expect(result).toBe("");
  });
});

// ---------------------------------------------------------------------------
// diffDirectories: recursive directory comparison.
// ---------------------------------------------------------------------------

describe("diffDirectories", () => {
  it("should report files only in one directory", () => {
    const dirA = path.join(tmpDir, "dirA");
    const dirB = path.join(tmpDir, "dirB");
    fs.mkdirSync(dirA);
    fs.mkdirSync(dirB);

    fs.writeFileSync(path.join(dirA, "only-in-a.txt"), "content");
    fs.writeFileSync(path.join(dirB, "only-in-b.txt"), "content");

    const result = diffDirectories(dirA, dirB, defaultOpts());

    expect(result).toContain("Only in");
    expect(result).toContain("only-in-a.txt");
    expect(result).toContain("only-in-b.txt");
  });

  it("should compare matching files", () => {
    const dirA = path.join(tmpDir, "dirA");
    const dirB = path.join(tmpDir, "dirB");
    fs.mkdirSync(dirA);
    fs.mkdirSync(dirB);

    fs.writeFileSync(path.join(dirA, "same.txt"), "hello");
    fs.writeFileSync(path.join(dirB, "same.txt"), "hello");

    const result = diffDirectories(dirA, dirB, defaultOpts());

    // Identical files produce no output.
    expect(result).toBe("");
  });

  it("should show differences in matching files", () => {
    const dirA = path.join(tmpDir, "dirA");
    const dirB = path.join(tmpDir, "dirB");
    fs.mkdirSync(dirA);
    fs.mkdirSync(dirB);

    fs.writeFileSync(path.join(dirA, "differ.txt"), "old content");
    fs.writeFileSync(path.join(dirB, "differ.txt"), "new content");

    const result = diffDirectories(dirA, dirB, defaultOpts());

    expect(result.length).toBeGreaterThan(0);
  });

  it("should handle empty directories", () => {
    const dirA = path.join(tmpDir, "dirA");
    const dirB = path.join(tmpDir, "dirB");
    fs.mkdirSync(dirA);
    fs.mkdirSync(dirB);

    const result = diffDirectories(dirA, dirB, defaultOpts());
    expect(result).toBe("");
  });
});
