/**
 * Tests for paste -- merge lines of files.
 *
 * We test the exported business logic functions: parseDelimiters,
 * pasteParallel, pasteSerial, and pasteLines.
 */

import { describe, it, expect } from "vitest";
import {
  parseDelimiters,
  pasteParallel,
  pasteSerial,
  pasteLines,
} from "../src/paste.js";

// ---------------------------------------------------------------------------
// parseDelimiters.
// ---------------------------------------------------------------------------

describe("parseDelimiters", () => {
  it("should parse a single character", () => {
    expect(parseDelimiters(",")).toEqual([","]);
  });

  it("should parse multiple characters", () => {
    expect(parseDelimiters(",;")).toEqual([",", ";"]);
  });

  it("should handle \\n escape for newline", () => {
    expect(parseDelimiters("\\n")).toEqual(["\n"]);
  });

  it("should handle \\t escape for tab", () => {
    expect(parseDelimiters("\\t")).toEqual(["\t"]);
  });

  it("should handle \\\\ escape for backslash", () => {
    expect(parseDelimiters("\\\\")).toEqual(["\\"]);
  });

  it("should handle \\0 escape for empty string", () => {
    expect(parseDelimiters("\\0")).toEqual([""]);
  });

  it("should handle mixed escapes and characters", () => {
    expect(parseDelimiters(",\\n:")).toEqual([",", "\n", ":"]);
  });

  it("should handle empty string", () => {
    expect(parseDelimiters("")).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// pasteParallel.
// ---------------------------------------------------------------------------

describe("pasteParallel", () => {
  it("should merge two equal-length arrays", () => {
    const inputs = [
      ["a", "b", "c"],
      ["1", "2", "3"],
    ];
    const result = pasteParallel(inputs, ["\t"]);
    expect(result).toEqual(["a\t1", "b\t2", "c\t3"]);
  });

  it("should handle arrays of different lengths", () => {
    const inputs = [
      ["a", "b"],
      ["1", "2", "3"],
    ];
    const result = pasteParallel(inputs, ["\t"]);
    expect(result).toEqual(["a\t1", "b\t2", "\t3"]);
  });

  it("should cycle through delimiters", () => {
    const inputs = [
      ["a", "b"],
      ["1", "2"],
      ["x", "y"],
    ];
    const result = pasteParallel(inputs, [",", ":"]);
    expect(result).toEqual(["a,1:x", "b,2:y"]);
  });

  it("should handle single input", () => {
    const inputs = [["a", "b", "c"]];
    const result = pasteParallel(inputs, ["\t"]);
    expect(result).toEqual(["a", "b", "c"]);
  });

  it("should handle empty input", () => {
    expect(pasteParallel([], ["\t"])).toEqual([]);
  });

  it("should handle empty arrays", () => {
    const inputs = [[], ["1", "2"]];
    const result = pasteParallel(inputs, ["\t"]);
    expect(result).toEqual(["\t1", "\t2"]);
  });
});

// ---------------------------------------------------------------------------
// pasteSerial.
// ---------------------------------------------------------------------------

describe("pasteSerial", () => {
  it("should join all lines from each file into one line", () => {
    const inputs = [
      ["a", "b", "c"],
      ["1", "2", "3"],
    ];
    const result = pasteSerial(inputs, ["\t"]);
    expect(result).toEqual(["a\tb\tc", "1\t2\t3"]);
  });

  it("should cycle delimiters within a single file", () => {
    const inputs = [["a", "b", "c", "d"]];
    const result = pasteSerial(inputs, [",", ":"]);
    expect(result).toEqual(["a,b:c,d"]);
  });

  it("should handle single-line files", () => {
    const inputs = [["hello"], ["world"]];
    const result = pasteSerial(inputs, ["\t"]);
    expect(result).toEqual(["hello", "world"]);
  });

  it("should handle empty input", () => {
    const inputs = [[]];
    const result = pasteSerial(inputs, ["\t"]);
    expect(result).toEqual([""]);
  });
});

// ---------------------------------------------------------------------------
// pasteLines (top-level dispatcher).
// ---------------------------------------------------------------------------

describe("pasteLines", () => {
  it("should use parallel mode by default", () => {
    const inputs = [["a", "b"], ["1", "2"]];
    const result = pasteLines(inputs, "\t", false);
    expect(result).toEqual(["a\t1", "b\t2"]);
  });

  it("should use serial mode when serial is true", () => {
    const inputs = [["a", "b", "c"]];
    const result = pasteLines(inputs, "\t", true);
    expect(result).toEqual(["a\tb\tc"]);
  });

  it("should parse delimiter string", () => {
    const inputs = [["a", "b"], ["1", "2"]];
    const result = pasteLines(inputs, ",", false);
    expect(result).toEqual(["a,1", "b,2"]);
  });

  it("should handle empty delimiter via \\0 escape", () => {
    const inputs = [["a", "b"], ["1", "2"]];
    const result = pasteLines(inputs, "\\0", false);
    expect(result).toEqual(["a1", "b2"]);
  });
});
