/**
 * Tests for sort -- sort lines of text files.
 *
 * We test the exported business logic functions: sortLines,
 * buildComparator, transformForComparison, parseHumanNumber,
 * getMonthValue, and versionCompare.
 */

import { describe, it, expect } from "vitest";
import {
  sortLines,
  buildComparator,
  transformForComparison,
  parseHumanNumber,
  getMonthValue,
  versionCompare,
  SortOptions,
} from "../src/sort.js";

// ---------------------------------------------------------------------------
// Helper: default sort options (all false).
// ---------------------------------------------------------------------------

function defaultOpts(overrides: Partial<SortOptions> = {}): SortOptions {
  return {
    reverse: false,
    numeric: false,
    humanNumeric: false,
    monthSort: false,
    generalNumeric: false,
    versionSort: false,
    unique: false,
    ignoreCase: false,
    dictionaryOrder: false,
    ignoreNonprinting: false,
    ignoreLeadingBlanks: false,
    stable: false,
    ...overrides,
  };
}

// ---------------------------------------------------------------------------
// sortLines: lexicographic (default) sort.
// ---------------------------------------------------------------------------

describe("sortLines", () => {
  it("should sort lines lexicographically by default", () => {
    const lines = ["banana", "apple", "cherry"];
    const result = sortLines(lines, defaultOpts());
    expect(result).toEqual(["apple", "banana", "cherry"]);
  });

  it("should handle empty input", () => {
    expect(sortLines([], defaultOpts())).toEqual([]);
  });

  it("should handle single-element input", () => {
    expect(sortLines(["hello"], defaultOpts())).toEqual(["hello"]);
  });

  it("should not mutate the original array", () => {
    const lines = ["banana", "apple"];
    sortLines(lines, defaultOpts());
    expect(lines).toEqual(["banana", "apple"]);
  });

  it("should handle duplicate lines", () => {
    const lines = ["b", "a", "b", "a"];
    const result = sortLines(lines, defaultOpts());
    expect(result).toEqual(["a", "a", "b", "b"]);
  });

  // -------------------------------------------------------------------------
  // Reverse sort (-r).
  // -------------------------------------------------------------------------

  it("should reverse sort with reverse flag", () => {
    const lines = ["apple", "cherry", "banana"];
    const result = sortLines(lines, defaultOpts({ reverse: true }));
    expect(result).toEqual(["cherry", "banana", "apple"]);
  });

  // -------------------------------------------------------------------------
  // Numeric sort (-n).
  // -------------------------------------------------------------------------

  it("should sort numerically with numeric flag", () => {
    const lines = ["10", "2", "1", "20"];
    const result = sortLines(lines, defaultOpts({ numeric: true }));
    expect(result).toEqual(["1", "2", "10", "20"]);
  });

  it("should treat non-numeric lines as 0 in numeric sort", () => {
    const lines = ["5", "abc", "2"];
    const result = sortLines(lines, defaultOpts({ numeric: true }));
    expect(result).toEqual(["abc", "2", "5"]);
  });

  it("should handle negative numbers in numeric sort", () => {
    const lines = ["3", "-1", "0", "2"];
    const result = sortLines(lines, defaultOpts({ numeric: true }));
    expect(result).toEqual(["-1", "0", "2", "3"]);
  });

  // -------------------------------------------------------------------------
  // Unique (-u).
  // -------------------------------------------------------------------------

  it("should remove duplicates with unique flag", () => {
    const lines = ["b", "a", "b", "c", "a"];
    const result = sortLines(lines, defaultOpts({ unique: true }));
    expect(result).toEqual(["a", "b", "c"]);
  });

  it("should remove case-insensitive duplicates with unique + ignoreCase", () => {
    const lines = ["Apple", "apple", "BANANA", "banana"];
    const result = sortLines(
      lines,
      defaultOpts({ unique: true, ignoreCase: true })
    );
    expect(result).toEqual(["Apple", "BANANA"]);
  });

  // -------------------------------------------------------------------------
  // Ignore case (-f).
  // -------------------------------------------------------------------------

  it("should sort case-insensitively with ignoreCase", () => {
    const lines = ["Banana", "apple", "Cherry"];
    const result = sortLines(lines, defaultOpts({ ignoreCase: true }));
    expect(result).toEqual(["apple", "Banana", "Cherry"]);
  });

  // -------------------------------------------------------------------------
  // Month sort (-M).
  // -------------------------------------------------------------------------

  it("should sort month abbreviations", () => {
    const lines = ["DEC", "JAN", "MAR", "FEB"];
    const result = sortLines(lines, defaultOpts({ monthSort: true }));
    expect(result).toEqual(["JAN", "FEB", "MAR", "DEC"]);
  });

  it("should treat unknown months as less than JAN", () => {
    const lines = ["JAN", "XYZ", "FEB"];
    const result = sortLines(lines, defaultOpts({ monthSort: true }));
    expect(result).toEqual(["XYZ", "JAN", "FEB"]);
  });

  // -------------------------------------------------------------------------
  // Human numeric sort (-h).
  // -------------------------------------------------------------------------

  it("should sort human-readable numbers", () => {
    const lines = ["1G", "2K", "3M", "500"];
    const result = sortLines(lines, defaultOpts({ humanNumeric: true }));
    expect(result).toEqual(["500", "2K", "3M", "1G"]);
  });

  // -------------------------------------------------------------------------
  // Version sort (-V).
  // -------------------------------------------------------------------------

  it("should sort version numbers naturally", () => {
    const lines = ["file10", "file2", "file1", "file20"];
    const result = sortLines(lines, defaultOpts({ versionSort: true }));
    expect(result).toEqual(["file1", "file2", "file10", "file20"]);
  });

  // -------------------------------------------------------------------------
  // Ignore leading blanks (-b).
  // -------------------------------------------------------------------------

  it("should ignore leading blanks when comparing", () => {
    const lines = ["  banana", "apple", "   cherry"];
    const result = sortLines(
      lines,
      defaultOpts({ ignoreLeadingBlanks: true })
    );
    expect(result).toEqual(["apple", "  banana", "   cherry"]);
  });

  // -------------------------------------------------------------------------
  // Dictionary order (-d).
  // -------------------------------------------------------------------------

  it("should only consider blanks and alphanumeric in dictionary mode", () => {
    const lines = ["b-c", "a.b", "d_e"];
    const result = sortLines(
      lines,
      defaultOpts({ dictionaryOrder: true })
    );
    // After removing non-alnum: "bc", "ab", "de"
    expect(result).toEqual(["a.b", "b-c", "d_e"]);
  });
});

// ---------------------------------------------------------------------------
// parseHumanNumber.
// ---------------------------------------------------------------------------

describe("parseHumanNumber", () => {
  it("should parse plain numbers", () => {
    expect(parseHumanNumber("100")).toBe(100);
    expect(parseHumanNumber("0")).toBe(0);
    expect(parseHumanNumber("3.14")).toBe(3.14);
  });

  it("should parse numbers with SI suffixes", () => {
    expect(parseHumanNumber("1K")).toBe(1000);
    expect(parseHumanNumber("2M")).toBe(2000000);
    expect(parseHumanNumber("1.5G")).toBe(1500000000);
  });

  it("should be case-insensitive for suffixes", () => {
    expect(parseHumanNumber("1k")).toBe(1000);
    expect(parseHumanNumber("2m")).toBe(2000000);
  });

  it("should return 0 for unparseable strings", () => {
    expect(parseHumanNumber("abc")).toBe(0);
    expect(parseHumanNumber("")).toBe(0);
  });

  it("should handle whitespace", () => {
    expect(parseHumanNumber("  100  ")).toBe(100);
    expect(parseHumanNumber("  2K  ")).toBe(2000);
  });
});

// ---------------------------------------------------------------------------
// getMonthValue.
// ---------------------------------------------------------------------------

describe("getMonthValue", () => {
  it("should return correct values for all months", () => {
    expect(getMonthValue("JAN")).toBe(1);
    expect(getMonthValue("FEB")).toBe(2);
    expect(getMonthValue("MAR")).toBe(3);
    expect(getMonthValue("APR")).toBe(4);
    expect(getMonthValue("MAY")).toBe(5);
    expect(getMonthValue("JUN")).toBe(6);
    expect(getMonthValue("JUL")).toBe(7);
    expect(getMonthValue("AUG")).toBe(8);
    expect(getMonthValue("SEP")).toBe(9);
    expect(getMonthValue("OCT")).toBe(10);
    expect(getMonthValue("NOV")).toBe(11);
    expect(getMonthValue("DEC")).toBe(12);
  });

  it("should be case-insensitive", () => {
    expect(getMonthValue("jan")).toBe(1);
    expect(getMonthValue("Jan")).toBe(1);
  });

  it("should return 0 for unknown strings", () => {
    expect(getMonthValue("XYZ")).toBe(0);
    expect(getMonthValue("")).toBe(0);
  });

  it("should handle leading whitespace", () => {
    expect(getMonthValue("  JAN")).toBe(1);
  });
});

// ---------------------------------------------------------------------------
// versionCompare.
// ---------------------------------------------------------------------------

describe("versionCompare", () => {
  it("should compare simple version numbers", () => {
    expect(versionCompare("file1", "file2")).toBeLessThan(0);
    expect(versionCompare("file10", "file2")).toBeGreaterThan(0);
    expect(versionCompare("file1", "file1")).toBe(0);
  });

  it("should handle multi-segment versions", () => {
    expect(versionCompare("v1.2.3", "v1.2.4")).toBeLessThan(0);
    expect(versionCompare("v1.10.0", "v1.2.0")).toBeGreaterThan(0);
  });

  it("should handle purely numeric strings", () => {
    expect(versionCompare("10", "2")).toBeGreaterThan(0);
    expect(versionCompare("1", "1")).toBe(0);
  });

  it("should handle empty strings", () => {
    expect(versionCompare("", "")).toBe(0);
    expect(versionCompare("a", "")).toBeGreaterThan(0);
    expect(versionCompare("", "a")).toBeLessThan(0);
  });
});

// ---------------------------------------------------------------------------
// transformForComparison.
// ---------------------------------------------------------------------------

describe("transformForComparison", () => {
  it("should return unchanged line with default options", () => {
    expect(transformForComparison("hello", defaultOpts())).toBe("hello");
  });

  it("should strip leading blanks", () => {
    expect(
      transformForComparison("  hello", defaultOpts({ ignoreLeadingBlanks: true }))
    ).toBe("hello");
  });

  it("should uppercase with ignoreCase", () => {
    expect(
      transformForComparison("Hello", defaultOpts({ ignoreCase: true }))
    ).toBe("HELLO");
  });

  it("should strip non-alnum with dictionaryOrder", () => {
    expect(
      transformForComparison("a-b.c", defaultOpts({ dictionaryOrder: true }))
    ).toBe("abc");
  });

  it("should strip non-printable with ignoreNonprinting", () => {
    expect(
      transformForComparison("a\x01b\x02c", defaultOpts({ ignoreNonprinting: true }))
    ).toBe("abc");
  });
});

// ---------------------------------------------------------------------------
// buildComparator.
// ---------------------------------------------------------------------------

describe("buildComparator", () => {
  it("should return a function", () => {
    const cmp = buildComparator(defaultOpts());
    expect(typeof cmp).toBe("function");
  });

  it("should compare lexicographically by default", () => {
    const cmp = buildComparator(defaultOpts());
    expect(cmp("apple", "banana")).toBeLessThan(0);
    expect(cmp("banana", "apple")).toBeGreaterThan(0);
    expect(cmp("apple", "apple")).toBe(0);
  });

  it("should reverse comparison with reverse flag", () => {
    const cmp = buildComparator(defaultOpts({ reverse: true }));
    expect(cmp("apple", "banana")).toBeGreaterThan(0);
    expect(cmp("banana", "apple")).toBeLessThan(0);
  });
});
