/**
 * Tests for grep -- print lines that match patterns.
 *
 * We test the exported business logic functions: buildRegex, grepLine,
 * extractMatches, grepLines, grepFile, and formatMatches.
 *
 * Most tests work on in-memory line arrays (via grepLines) so they
 * don't require filesystem access. The grepFile tests use a temporary
 * directory.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import {
  buildRegex,
  grepLine,
  extractMatches,
  grepLines,
  grepFile,
  formatMatches,
  GrepOptions,
  GrepMatch,
} from "../src/grep.js";

// ---------------------------------------------------------------------------
// Helpers: default options and temp directory.
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<GrepOptions> = {}): GrepOptions {
  return {
    fixedStrings: false,
    ignoreCase: false,
    invertMatch: false,
    wordRegexp: false,
    lineRegexp: false,
    lineNumber: false,
    count: false,
    filesWithMatches: false,
    filesWithoutMatch: false,
    onlyMatching: false,
    maxCount: null,
    ...overrides,
  };
}

let tmpDir: string;

beforeEach(() => {
  tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), "grep-test-"));
});

afterEach(() => {
  fs.rmSync(tmpDir, { recursive: true, force: true });
});

// ---------------------------------------------------------------------------
// buildRegex: constructing regex from pattern and options.
// ---------------------------------------------------------------------------

describe("buildRegex", () => {
  it("should build a basic regex", () => {
    const re = buildRegex("hello", defaultOpts());
    expect(re.test("hello world")).toBe(true);
    expect(re.test("goodbye")).toBe(false);
  });

  it("should build a case-insensitive regex with ignoreCase", () => {
    const re = buildRegex("hello", defaultOpts({ ignoreCase: true }));
    expect(re.test("HELLO world")).toBe(true);
    expect(re.test("Hello")).toBe(true);
  });

  it("should escape metacharacters with fixedStrings", () => {
    const re = buildRegex("a.b", defaultOpts({ fixedStrings: true }));
    expect(re.test("a.b")).toBe(true);
    expect(re.test("axb")).toBe(false); // . should not be wildcard
  });

  it("should escape all regex special chars with fixedStrings", () => {
    const re = buildRegex("[test]", defaultOpts({ fixedStrings: true }));
    expect(re.test("[test]")).toBe(true);
    expect(re.test("t")).toBe(false); // [test] should not be char class
  });

  it("should add word boundaries with wordRegexp", () => {
    const re = buildRegex("cat", defaultOpts({ wordRegexp: true }));
    expect(re.test("the cat sat")).toBe(true);
    expect(re.test("concatenate")).toBe(false);
  });

  it("should anchor to line with lineRegexp", () => {
    const re = buildRegex("hello", defaultOpts({ lineRegexp: true }));
    expect(re.test("hello")).toBe(true);
    expect(re.test("hello world")).toBe(false);
  });

  it("should combine fixedStrings and ignoreCase", () => {
    const re = buildRegex("A.B", defaultOpts({ fixedStrings: true, ignoreCase: true }));
    expect(re.test("a.b")).toBe(true);
    expect(re.test("A.B")).toBe(true);
    expect(re.test("axb")).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// grepLine: testing a single line against a pattern.
// ---------------------------------------------------------------------------

describe("grepLine", () => {
  it("should return true for matching lines", () => {
    const re = buildRegex("hello", defaultOpts());
    expect(grepLine("hello world", re, defaultOpts())).toBe(true);
  });

  it("should return false for non-matching lines", () => {
    const re = buildRegex("hello", defaultOpts());
    expect(grepLine("goodbye world", re, defaultOpts())).toBe(false);
  });

  it("should invert match with invertMatch", () => {
    const opts = defaultOpts({ invertMatch: true });
    const re = buildRegex("hello", opts);
    expect(grepLine("hello world", re, opts)).toBe(false);
    expect(grepLine("goodbye world", re, opts)).toBe(true);
  });

  it("should handle empty lines", () => {
    const re = buildRegex("hello", defaultOpts());
    expect(grepLine("", re, defaultOpts())).toBe(false);
  });

  it("should handle empty pattern (matches everything)", () => {
    const re = buildRegex("", defaultOpts());
    expect(grepLine("anything", re, defaultOpts())).toBe(true);
    expect(grepLine("", re, defaultOpts())).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// extractMatches: finding all matches in a line (-o mode).
// ---------------------------------------------------------------------------

describe("extractMatches", () => {
  it("should extract single match", () => {
    const re = buildRegex("hello", defaultOpts());
    const matches = extractMatches("say hello there", re);
    expect(matches).toEqual(["hello"]);
  });

  it("should extract multiple matches", () => {
    const re = buildRegex("a", defaultOpts());
    const matches = extractMatches("banana", re);
    expect(matches).toEqual(["a", "a", "a"]);
  });

  it("should return empty array for no matches", () => {
    const re = buildRegex("xyz", defaultOpts());
    const matches = extractMatches("hello world", re);
    expect(matches).toEqual([]);
  });

  it("should handle regex groups", () => {
    const re = buildRegex("\\d+", defaultOpts());
    const matches = extractMatches("abc 123 def 456", re);
    expect(matches).toEqual(["123", "456"]);
  });
});

// ---------------------------------------------------------------------------
// grepLines: searching across multiple lines.
// ---------------------------------------------------------------------------

describe("grepLines", () => {
  it("should find matching lines", () => {
    const lines = ["hello world", "goodbye world", "hello again"];
    const re = buildRegex("hello", defaultOpts());
    const results = grepLines(lines, re, defaultOpts());

    expect(results.length).toBe(2);
    expect(results[0].lineNumber).toBe(1);
    expect(results[0].line).toBe("hello world");
    expect(results[1].lineNumber).toBe(3);
    expect(results[1].line).toBe("hello again");
  });

  it("should return empty array when no matches", () => {
    const lines = ["aaa", "bbb", "ccc"];
    const re = buildRegex("xyz", defaultOpts());
    const results = grepLines(lines, re, defaultOpts());

    expect(results).toEqual([]);
  });

  it("should handle empty input", () => {
    const re = buildRegex("hello", defaultOpts());
    const results = grepLines([], re, defaultOpts());
    expect(results).toEqual([]);
  });

  it("should respect maxCount", () => {
    const lines = ["a1", "a2", "a3", "a4", "a5"];
    const re = buildRegex("a", defaultOpts());
    const results = grepLines(lines, re, defaultOpts({ maxCount: 2 }));

    expect(results.length).toBe(2);
    expect(results[0].line).toBe("a1");
    expect(results[1].line).toBe("a2");
  });

  it("should extract matches in onlyMatching mode", () => {
    const lines = ["abc 123 def", "no numbers here", "456 end"];
    const re = buildRegex("\\d+", defaultOpts());
    const results = grepLines(
      lines,
      re,
      defaultOpts({ onlyMatching: true })
    );

    expect(results.length).toBe(2);
    expect(results[0].matches).toEqual(["123"]);
    expect(results[1].matches).toEqual(["456"]);
  });

  it("should handle invert match across lines", () => {
    const lines = ["hello", "world", "hello again"];
    const re = buildRegex("hello", defaultOpts({ invertMatch: true }));
    const results = grepLines(
      lines,
      re,
      defaultOpts({ invertMatch: true })
    );

    expect(results.length).toBe(1);
    expect(results[0].line).toBe("world");
  });

  it("should use 1-based line numbers", () => {
    const lines = ["first", "second", "third"];
    const re = buildRegex("second", defaultOpts());
    const results = grepLines(lines, re, defaultOpts());

    expect(results[0].lineNumber).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// grepFile: searching in a file.
// ---------------------------------------------------------------------------

describe("grepFile", () => {
  it("should search within a file", () => {
    const filePath = path.join(tmpDir, "test.txt");
    fs.writeFileSync(filePath, "hello world\ngoodbye\nhello again\n");

    const re = buildRegex("hello", defaultOpts());
    const results = grepFile(filePath, re, defaultOpts());

    expect(results.length).toBe(2);
    expect(results[0].line).toBe("hello world");
    expect(results[1].line).toBe("hello again");
  });

  it("should throw for non-existent file", () => {
    const re = buildRegex("hello", defaultOpts());

    expect(() =>
      grepFile(path.join(tmpDir, "nope.txt"), re, defaultOpts())
    ).toThrow(/No such file or directory/);
  });

  it("should handle empty file", () => {
    const filePath = path.join(tmpDir, "empty.txt");
    fs.writeFileSync(filePath, "");

    const re = buildRegex("hello", defaultOpts());
    const results = grepFile(filePath, re, defaultOpts());

    expect(results).toEqual([]);
  });

  it("should handle file with trailing newline", () => {
    const filePath = path.join(tmpDir, "trailing.txt");
    fs.writeFileSync(filePath, "line1\nline2\n");

    const re = buildRegex("line", defaultOpts());
    const results = grepFile(filePath, re, defaultOpts());

    expect(results.length).toBe(2);
  });

  it("should handle file without trailing newline", () => {
    const filePath = path.join(tmpDir, "no-trailing.txt");
    fs.writeFileSync(filePath, "line1\nline2");

    const re = buildRegex("line", defaultOpts());
    const results = grepFile(filePath, re, defaultOpts());

    expect(results.length).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// formatMatches: formatting output.
// ---------------------------------------------------------------------------

describe("formatMatches", () => {
  const sampleMatches: GrepMatch[] = [
    { lineNumber: 1, line: "hello world", matches: [] },
    { lineNumber: 3, line: "hello again", matches: [] },
  ];

  it("should format normal output without filename", () => {
    const output = formatMatches(sampleMatches, "test.txt", defaultOpts(), false);

    expect(output).toEqual(["hello world", "hello again"]);
  });

  it("should format output with filename prefix", () => {
    const output = formatMatches(sampleMatches, "test.txt", defaultOpts(), true);

    expect(output).toEqual(["test.txt:hello world", "test.txt:hello again"]);
  });

  it("should format output with line numbers", () => {
    const output = formatMatches(
      sampleMatches,
      "test.txt",
      defaultOpts({ lineNumber: true }),
      false
    );

    expect(output).toEqual(["1:hello world", "3:hello again"]);
  });

  it("should format output with both filename and line number", () => {
    const output = formatMatches(
      sampleMatches,
      "test.txt",
      defaultOpts({ lineNumber: true }),
      true
    );

    expect(output).toEqual([
      "test.txt:1:hello world",
      "test.txt:3:hello again",
    ]);
  });

  it("should format count mode", () => {
    const output = formatMatches(
      sampleMatches,
      "test.txt",
      defaultOpts({ count: true }),
      false
    );

    expect(output).toEqual(["2"]);
  });

  it("should format count mode with filename", () => {
    const output = formatMatches(
      sampleMatches,
      "test.txt",
      defaultOpts({ count: true }),
      true
    );

    expect(output).toEqual(["test.txt:2"]);
  });

  it("should format only-matching mode", () => {
    const matchesWithExtracts: GrepMatch[] = [
      { lineNumber: 1, line: "abc 123 def", matches: ["123"] },
      { lineNumber: 2, line: "456 xyz 789", matches: ["456", "789"] },
    ];

    const output = formatMatches(
      matchesWithExtracts,
      "test.txt",
      defaultOpts({ onlyMatching: true }),
      false
    );

    expect(output).toEqual(["123", "456", "789"]);
  });

  it("should format only-matching with line numbers", () => {
    const matchesWithExtracts: GrepMatch[] = [
      { lineNumber: 5, line: "foo 42 bar", matches: ["42"] },
    ];

    const output = formatMatches(
      matchesWithExtracts,
      "test.txt",
      defaultOpts({ onlyMatching: true, lineNumber: true }),
      false
    );

    expect(output).toEqual(["5:42"]);
  });

  it("should handle empty matches array", () => {
    const output = formatMatches([], "test.txt", defaultOpts(), false);
    expect(output).toEqual([]);
  });

  it("should handle count mode with zero matches", () => {
    const output = formatMatches(
      [],
      "test.txt",
      defaultOpts({ count: true }),
      false
    );
    expect(output).toEqual(["0"]);
  });
});
