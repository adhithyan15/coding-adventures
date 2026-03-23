/**
 * Tests for join -- join lines of two files on a common field.
 *
 * We test the exported business logic functions: splitFields, getKey,
 * buildOutputLine, buildUnpairedLine, and joinLines.
 *
 * All tests operate on in-memory string arrays, so no filesystem
 * access is needed.
 */

import { describe, it, expect } from "vitest";
import {
  splitFields,
  getKey,
  buildOutputLine,
  buildUnpairedLine,
  joinLines,
  JoinOptions,
} from "../src/join.js";

// ---------------------------------------------------------------------------
// Helper: default join options.
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<JoinOptions> = {}): JoinOptions {
  return {
    field1: 1,
    field2: 1,
    separator: null,
    unpaired: [],
    onlyUnpaired: null,
    empty: "",
    ignoreCase: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// splitFields: parsing lines into field arrays.
// ---------------------------------------------------------------------------

describe("splitFields", () => {
  it("should split on whitespace by default", () => {
    expect(splitFields("hello world foo", null)).toEqual([
      "hello",
      "world",
      "foo",
    ]);
  });

  it("should handle multiple whitespace characters", () => {
    expect(splitFields("a   b    c", null)).toEqual(["a", "b", "c"]);
  });

  it("should trim leading and trailing whitespace", () => {
    expect(splitFields("  hello world  ", null)).toEqual(["hello", "world"]);
  });

  it("should split on a custom separator", () => {
    expect(splitFields("a,b,c", ",")).toEqual(["a", "b", "c"]);
  });

  it("should preserve empty fields with custom separator", () => {
    expect(splitFields("a,,c", ",")).toEqual(["a", "", "c"]);
  });

  it("should handle single-field line", () => {
    expect(splitFields("hello", null)).toEqual(["hello"]);
  });

  it("should handle tab separator", () => {
    expect(splitFields("a\tb\tc", "\t")).toEqual(["a", "b", "c"]);
  });
});

// ---------------------------------------------------------------------------
// getKey: extracting the join key from fields.
// ---------------------------------------------------------------------------

describe("getKey", () => {
  it("should return the first field by default", () => {
    expect(getKey(["alice", "100", "NYC"], 1, false)).toBe("alice");
  });

  it("should return the specified field (1-based)", () => {
    expect(getKey(["alice", "100", "NYC"], 2, false)).toBe("100");
    expect(getKey(["alice", "100", "NYC"], 3, false)).toBe("NYC");
  });

  it("should return empty string for out-of-bounds field", () => {
    expect(getKey(["alice"], 5, false)).toBe("");
  });

  it("should return empty string for field 0", () => {
    expect(getKey(["alice"], 0, false)).toBe("");
  });

  it("should lowercase key when ignoreCase is true", () => {
    expect(getKey(["Alice", "100"], 1, true)).toBe("alice");
    expect(getKey(["HELLO"], 1, true)).toBe("hello");
  });

  it("should not lowercase when ignoreCase is false", () => {
    expect(getKey(["Alice"], 1, false)).toBe("Alice");
  });
});

// ---------------------------------------------------------------------------
// buildOutputLine: constructing joined output.
// ---------------------------------------------------------------------------

describe("buildOutputLine", () => {
  it("should build output with default separator (space)", () => {
    const result = buildOutputLine(
      "1",
      ["1", "Alice"],
      ["1", "Engineering"],
      1, 1, null
    );
    expect(result).toBe("1 Alice Engineering");
  });

  it("should use custom separator", () => {
    const result = buildOutputLine(
      "1",
      ["1", "Alice"],
      ["1", "Engineering"],
      1, 1, ","
    );
    expect(result).toBe("1,Alice,Engineering");
  });

  it("should exclude join field from both sides", () => {
    const result = buildOutputLine(
      "key",
      ["key", "a", "b"],
      ["key", "x", "y"],
      1, 1, null
    );
    expect(result).toBe("key a b x y");
  });

  it("should handle join on non-first field", () => {
    const result = buildOutputLine(
      "key",
      ["a", "key", "b"],
      ["x", "key", "y"],
      2, 2, null
    );
    // Non-key fields from file1: a, b; from file2: x, y
    expect(result).toBe("key a b x y");
  });

  it("should handle single-field records", () => {
    const result = buildOutputLine("key", ["key"], ["key"], 1, 1, null);
    expect(result).toBe("key");
  });
});

// ---------------------------------------------------------------------------
// buildUnpairedLine: constructing unmatched output.
// ---------------------------------------------------------------------------

describe("buildUnpairedLine", () => {
  it("should join fields with space by default", () => {
    expect(buildUnpairedLine(["a", "b", "c"], null)).toBe("a b c");
  });

  it("should use custom separator", () => {
    expect(buildUnpairedLine(["a", "b", "c"], ",")).toBe("a,b,c");
  });

  it("should handle single-field record", () => {
    expect(buildUnpairedLine(["hello"], null)).toBe("hello");
  });
});

// ---------------------------------------------------------------------------
// joinLines: the core merge-join algorithm.
// ---------------------------------------------------------------------------

describe("joinLines", () => {
  // -------------------------------------------------------------------------
  // Basic inner join.
  // -------------------------------------------------------------------------

  it("should join two sorted files on field 1 (inner join)", () => {
    const lines1 = ["1 Alice", "2 Bob", "3 Charlie"];
    const lines2 = ["1 Engineering", "3 Marketing"];

    const result = joinLines(lines1, lines2, defaultOpts());

    expect(result).toEqual(["1 Alice Engineering", "3 Charlie Marketing"]);
  });

  it("should handle identical files", () => {
    const lines = ["1 A", "2 B"];
    const result = joinLines(lines, lines, defaultOpts());
    expect(result).toEqual(["1 A A", "2 B B"]);
  });

  it("should handle completely disjoint files", () => {
    const lines1 = ["1 A", "3 C"];
    const lines2 = ["2 B", "4 D"];

    const result = joinLines(lines1, lines2, defaultOpts());
    expect(result).toEqual([]);
  });

  it("should handle empty first file", () => {
    const result = joinLines([], ["1 A"], defaultOpts());
    expect(result).toEqual([]);
  });

  it("should handle empty second file", () => {
    const result = joinLines(["1 A"], [], defaultOpts());
    expect(result).toEqual([]);
  });

  it("should handle both empty files", () => {
    const result = joinLines([], [], defaultOpts());
    expect(result).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Duplicate keys (cross product).
  // -------------------------------------------------------------------------

  it("should produce cross product for duplicate keys", () => {
    const lines1 = ["1 A1", "1 A2"];
    const lines2 = ["1 B1", "1 B2"];

    const result = joinLines(lines1, lines2, defaultOpts());

    // 2 x 2 = 4 output lines.
    expect(result).toEqual([
      "1 A1 B1",
      "1 A1 B2",
      "1 A2 B1",
      "1 A2 B2",
    ]);
  });

  it("should handle duplicates in only one file", () => {
    const lines1 = ["1 A", "1 B"];
    const lines2 = ["1 X"];

    const result = joinLines(lines1, lines2, defaultOpts());

    expect(result).toEqual(["1 A X", "1 B X"]);
  });

  // -------------------------------------------------------------------------
  // Join on different fields.
  // -------------------------------------------------------------------------

  it("should join on field 2 of file 1", () => {
    const lines1 = ["Alice 1", "Bob 2"];
    const lines2 = ["1 Engineering", "2 Marketing"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ field1: 2 })
    );

    expect(result).toEqual([
      "1 Alice Engineering",
      "2 Bob Marketing",
    ]);
  });

  it("should join on field 2 of both files", () => {
    const lines1 = ["X 1", "Y 2"];
    const lines2 = ["A 1", "B 2"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ field1: 2, field2: 2 })
    );

    expect(result).toEqual(["1 X A", "2 Y B"]);
  });

  // -------------------------------------------------------------------------
  // Custom separator.
  // -------------------------------------------------------------------------

  it("should use custom separator for splitting and output", () => {
    const lines1 = ["1,Alice", "2,Bob"];
    const lines2 = ["1,Engineering", "2,Marketing"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ separator: "," })
    );

    expect(result).toEqual([
      "1,Alice,Engineering",
      "2,Bob,Marketing",
    ]);
  });

  // -------------------------------------------------------------------------
  // Unpaired lines (-a flag: LEFT/RIGHT/FULL outer join).
  // -------------------------------------------------------------------------

  it("should print unpairable lines from file 1 with -a 1", () => {
    const lines1 = ["1 Alice", "2 Bob", "3 Charlie"];
    const lines2 = ["1 Engineering", "3 Marketing"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ unpaired: [1] })
    );

    expect(result).toEqual([
      "1 Alice Engineering",
      "2 Bob",
      "3 Charlie Marketing",
    ]);
  });

  it("should print unpairable lines from file 2 with -a 2", () => {
    const lines1 = ["1 Alice", "3 Charlie"];
    const lines2 = ["1 Engineering", "2 Marketing", "3 Sales"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ unpaired: [2] })
    );

    expect(result).toEqual([
      "1 Alice Engineering",
      "2 Marketing",
      "3 Charlie Sales",
    ]);
  });

  it("should print all unpairable lines with -a 1 -a 2 (full outer)", () => {
    const lines1 = ["1 A", "3 C"];
    const lines2 = ["2 B", "3 D"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ unpaired: [1, 2] })
    );

    expect(result).toEqual(["1 A", "2 B", "3 C D"]);
  });

  // -------------------------------------------------------------------------
  // Only unpaired (-v flag: anti join).
  // -------------------------------------------------------------------------

  it("should print only unpairable from file 1 with -v 1", () => {
    const lines1 = ["1 A", "2 B", "3 C"];
    const lines2 = ["1 X", "3 Z"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ onlyUnpaired: 1 })
    );

    expect(result).toEqual(["2 B"]);
  });

  it("should print only unpairable from file 2 with -v 2", () => {
    const lines1 = ["1 A", "3 C"];
    const lines2 = ["1 X", "2 Y", "3 Z"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ onlyUnpaired: 2 })
    );

    expect(result).toEqual(["2 Y"]);
  });

  // -------------------------------------------------------------------------
  // Case-insensitive join.
  // -------------------------------------------------------------------------

  it("should ignore case when comparing join fields", () => {
    const lines1 = ["Alice 100", "Bob 200"];
    const lines2 = ["alice Engineering", "bob Marketing"];

    const result = joinLines(
      lines1,
      lines2,
      defaultOpts({ ignoreCase: true })
    );

    expect(result.length).toBe(2);
    // The key value comes from file 1 (original case).
    expect(result[0]).toContain("Engineering");
    expect(result[1]).toContain("Marketing");
  });

  // -------------------------------------------------------------------------
  // Edge cases.
  // -------------------------------------------------------------------------

  it("should handle single-field records", () => {
    const lines1 = ["a", "b", "c"];
    const lines2 = ["b", "c", "d"];

    const result = joinLines(lines1, lines2, defaultOpts());

    expect(result).toEqual(["b", "c"]);
  });

  it("should handle many fields", () => {
    const lines1 = ["key f1 f2 f3"];
    const lines2 = ["key g1 g2"];

    const result = joinLines(lines1, lines2, defaultOpts());

    expect(result).toEqual(["key f1 f2 f3 g1 g2"]);
  });
});
