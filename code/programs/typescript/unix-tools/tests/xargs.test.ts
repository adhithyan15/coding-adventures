/**
 * Tests for xargs -- build and execute command lines from standard input.
 *
 * We test the exported business logic functions: parseItems,
 * buildBatches, buildCommands, and runXargs.
 */

import { describe, it, expect } from "vitest";
import {
  parseItems,
  buildBatches,
  buildCommands,
  runXargs,
  XargsOptions,
} from "../src/xargs.js";

// ---------------------------------------------------------------------------
// Helper: default xargs options (no flags set).
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<XargsOptions> = {}): XargsOptions {
  return {
    nullDelimiter: false,
    delimiter: null,
    maxArgs: 0,
    replaceStr: null,
    verbose: false,
    noRunIfEmpty: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// parseItems: input parsing tests.
// ---------------------------------------------------------------------------

describe("parseItems", () => {
  it("should split on whitespace by default", () => {
    const items = parseItems("hello world foo", defaultOpts());
    expect(items).toEqual(["hello", "world", "foo"]);
  });

  it("should handle multiple whitespace types", () => {
    const items = parseItems("hello\tworld\nfoo", defaultOpts());
    expect(items).toEqual(["hello", "world", "foo"]);
  });

  it("should handle leading and trailing whitespace", () => {
    const items = parseItems("  hello world  ", defaultOpts());
    expect(items).toEqual(["hello", "world"]);
  });

  it("should handle empty input", () => {
    const items = parseItems("", defaultOpts());
    expect(items).toEqual([]);
  });

  it("should handle whitespace-only input", () => {
    const items = parseItems("   \t\n  ", defaultOpts());
    expect(items).toEqual([]);
  });

  it("should respect single quotes", () => {
    const items = parseItems("'hello world' foo", defaultOpts());
    expect(items).toEqual(["hello world", "foo"]);
  });

  it("should respect double quotes", () => {
    const items = parseItems('"hello world" foo', defaultOpts());
    expect(items).toEqual(["hello world", "foo"]);
  });

  it("should handle backslash escaping", () => {
    const items = parseItems("hello\\ world foo", defaultOpts());
    expect(items).toEqual(["hello world", "foo"]);
  });

  it("should handle mixed quotes", () => {
    const items = parseItems("'single' \"double\" plain", defaultOpts());
    expect(items).toEqual(["single", "double", "plain"]);
  });

  // --- NUL delimiter mode (-0) -----------------------------------------

  it("should split on NUL in null mode", () => {
    const items = parseItems("hello\0world\0foo", defaultOpts({ nullDelimiter: true }));
    expect(items).toEqual(["hello", "world", "foo"]);
  });

  it("should handle trailing NUL", () => {
    const items = parseItems("hello\0world\0", defaultOpts({ nullDelimiter: true }));
    expect(items).toEqual(["hello", "world"]);
  });

  it("should preserve spaces in NUL mode", () => {
    const items = parseItems("hello world\0foo bar\0", defaultOpts({ nullDelimiter: true }));
    expect(items).toEqual(["hello world", "foo bar"]);
  });

  // --- Custom delimiter (-d) -------------------------------------------

  it("should split on custom delimiter", () => {
    const items = parseItems("hello,world,foo", defaultOpts({ delimiter: "," }));
    expect(items).toEqual(["hello", "world", "foo"]);
  });

  it("should handle custom delimiter with empty items", () => {
    const items = parseItems("hello,,foo", defaultOpts({ delimiter: "," }));
    expect(items).toEqual(["hello", "foo"]);
  });

  it("should split on newline as custom delimiter", () => {
    const items = parseItems("hello\nworld\nfoo", defaultOpts({ delimiter: "\n" }));
    expect(items).toEqual(["hello", "world", "foo"]);
  });
});

// ---------------------------------------------------------------------------
// buildBatches: batching items for execution.
// ---------------------------------------------------------------------------

describe("buildBatches", () => {
  it("should put all items in one batch when maxArgs is 0", () => {
    const batches = buildBatches(["a", "b", "c", "d"], 0);
    expect(batches).toEqual([["a", "b", "c", "d"]]);
  });

  it("should split items into batches of maxArgs", () => {
    const batches = buildBatches(["a", "b", "c", "d"], 2);
    expect(batches).toEqual([["a", "b"], ["c", "d"]]);
  });

  it("should handle uneven splits", () => {
    const batches = buildBatches(["a", "b", "c", "d", "e"], 2);
    expect(batches).toEqual([["a", "b"], ["c", "d"], ["e"]]);
  });

  it("should handle maxArgs larger than item count", () => {
    const batches = buildBatches(["a", "b"], 10);
    expect(batches).toEqual([["a", "b"]]);
  });

  it("should handle single item per batch", () => {
    const batches = buildBatches(["a", "b", "c"], 1);
    expect(batches).toEqual([["a"], ["b"], ["c"]]);
  });

  it("should return empty array for empty input", () => {
    const batches = buildBatches([], 2);
    expect(batches).toEqual([]);
  });

  it("should return empty array for empty input with maxArgs 0", () => {
    const batches = buildBatches([], 0);
    expect(batches).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// buildCommands: building command strings.
// ---------------------------------------------------------------------------

describe("buildCommands", () => {
  it("should append items to base command", () => {
    const cmds = buildCommands(["echo"], ["hello", "world"], null);
    expect(cmds).toEqual(["echo hello world"]);
  });

  it("should handle empty items", () => {
    const cmds = buildCommands(["echo"], [], null);
    expect(cmds).toEqual(["echo"]);
  });

  it("should handle command with initial args", () => {
    const cmds = buildCommands(["grep", "-l"], ["pattern", "file"], null);
    expect(cmds).toEqual(["grep -l pattern file"]);
  });

  it("should quote items with spaces", () => {
    const cmds = buildCommands(["echo"], ["hello world", "foo"], null);
    expect(cmds).toEqual(['echo "hello world" foo']);
  });

  // --- Replacement mode (-I) -------------------------------------------

  it("should replace placeholder with each item", () => {
    const cmds = buildCommands(["echo", "item: {}"], ["foo", "bar"], "{}");
    expect(cmds).toHaveLength(2);
    expect(cmds[0]).toContain("foo");
    expect(cmds[1]).toContain("bar");
  });

  it("should replace all occurrences of placeholder", () => {
    const cmds = buildCommands(["cp", "{}", "/backup/{}"], ["file.txt"], "{}");
    expect(cmds).toHaveLength(1);
    expect(cmds[0]).toContain("file.txt");
  });
});

// ---------------------------------------------------------------------------
// runXargs: end-to-end pipeline tests.
// ---------------------------------------------------------------------------

describe("runXargs", () => {
  it("should execute echo with items", () => {
    const result = runXargs("hello world", ["echo"], defaultOpts());
    expect(result.exitCode).toBe(0);
    expect(result.output.trim()).toBe("hello world");
  });

  it("should handle empty input with noRunIfEmpty", () => {
    const result = runXargs("", ["echo"], defaultOpts({ noRunIfEmpty: true }));
    expect(result.exitCode).toBe(0);
    expect(result.output).toBe("");
  });

  it("should batch with maxArgs", () => {
    const result = runXargs("a b c d", ["echo"], defaultOpts({ maxArgs: 2 }));
    expect(result.exitCode).toBe(0);
    // Should have two lines: "a b" and "c d".
    const lines = result.output.trim().split("\n");
    expect(lines).toHaveLength(2);
    expect(lines[0]).toBe("a b");
    expect(lines[1]).toBe("c d");
  });

  it("should handle replacement mode", () => {
    const result = runXargs(
      "foo\nbar",
      ["echo", "item: {}"],
      defaultOpts({ replaceStr: "{}" })
    );
    expect(result.exitCode).toBe(0);
    const lines = result.output.trim().split("\n");
    expect(lines).toHaveLength(2);
  });

  it("should handle NUL-delimited input", () => {
    const result = runXargs(
      "hello\0world",
      ["echo"],
      defaultOpts({ nullDelimiter: true })
    );
    expect(result.exitCode).toBe(0);
    expect(result.output.trim()).toBe("hello world");
  });

  it("should handle custom delimiter", () => {
    const result = runXargs(
      "hello,world",
      ["echo"],
      defaultOpts({ delimiter: "," })
    );
    expect(result.exitCode).toBe(0);
    expect(result.output.trim()).toBe("hello world");
  });

  it("should return non-zero exit code for failed commands", () => {
    const result = runXargs(
      "nonexistent_file",
      ["cat"],
      defaultOpts()
    );
    expect(result.exitCode).not.toBe(0);
  });

  it("should handle empty input with replacement mode", () => {
    const result = runXargs(
      "",
      ["echo", "{}"],
      defaultOpts({ replaceStr: "{}" })
    );
    expect(result.exitCode).toBe(0);
    expect(result.output).toBe("");
  });
});
