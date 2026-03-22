/**
 * Tests for comm -- compare two sorted files line by line.
 *
 * We test the exported `compareSorted` function, which performs the
 * merge-like comparison of two sorted line arrays.
 */

import { describe, it, expect } from "vitest";
import { compareSorted } from "../src/comm.js";

// ---------------------------------------------------------------------------
// Basic three-column output.
// ---------------------------------------------------------------------------

describe("compareSorted", () => {
  it("should produce three columns for sorted inputs", () => {
    const lines1 = ["apple", "banana", "cherry"];
    const lines2 = ["banana", "cherry", "date"];
    const result = compareSorted(lines1, lines2, [false, false, false]);

    // apple is unique to file1 (column 1, no indent).
    // banana is common (column 3, two tabs).
    // cherry is common (column 3, two tabs).
    // date is unique to file2 (column 2, one tab).
    expect(result).toEqual([
      "apple",
      "\t\tbanana",
      "\t\tcherry",
      "\tdate",
    ]);
  });

  it("should handle identical files", () => {
    const lines = ["a", "b", "c"];
    const result = compareSorted(lines, lines, [false, false, false]);
    expect(result).toEqual(["\t\ta", "\t\tb", "\t\tc"]);
  });

  it("should handle completely disjoint files", () => {
    const lines1 = ["a", "c"];
    const lines2 = ["b", "d"];
    const result = compareSorted(lines1, lines2, [false, false, false]);
    expect(result).toEqual(["a", "\tb", "c", "\td"]);
  });

  it("should handle empty first file", () => {
    const result = compareSorted([], ["a", "b"], [false, false, false]);
    expect(result).toEqual(["\ta", "\tb"]);
  });

  it("should handle empty second file", () => {
    const result = compareSorted(["a", "b"], [], [false, false, false]);
    expect(result).toEqual(["a", "b"]);
  });

  it("should handle both empty files", () => {
    const result = compareSorted([], [], [false, false, false]);
    expect(result).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Column suppression.
  // -------------------------------------------------------------------------

  it("should suppress column 1 with suppress[0]", () => {
    const lines1 = ["a", "b"];
    const lines2 = ["b", "c"];
    const result = compareSorted(lines1, lines2, [true, false, false]);
    // Column 1 (unique to file1) is suppressed.
    // Column 2 shifts left: no indent (was one tab).
    // Column 3 shifts left: one tab (was two tabs).
    expect(result).toEqual(["\tb", "c"]);
  });

  it("should suppress column 2 with suppress[1]", () => {
    const lines1 = ["a", "b"];
    const lines2 = ["b", "c"];
    const result = compareSorted(lines1, lines2, [false, true, false]);
    // Column 2 (unique to file2) is suppressed.
    // Column 1 unchanged: no indent.
    // Column 3 unchanged: one tab (col2 suppressed, so only 1 preceding column).
    expect(result).toEqual(["a", "\tb"]);
  });

  it("should suppress column 3 with suppress[2]", () => {
    const lines1 = ["a", "b"];
    const lines2 = ["b", "c"];
    const result = compareSorted(lines1, lines2, [false, false, true]);
    expect(result).toEqual(["a", "\tc"]);
  });

  it("should show only common lines with -12 (suppress col 1 and 2)", () => {
    const lines1 = ["a", "b", "c"];
    const lines2 = ["b", "c", "d"];
    const result = compareSorted(lines1, lines2, [true, true, false]);
    expect(result).toEqual(["b", "c"]);
  });

  it("should show only unique-to-file1 with -23", () => {
    const lines1 = ["a", "b", "c"];
    const lines2 = ["b", "c", "d"];
    const result = compareSorted(lines1, lines2, [false, true, true]);
    expect(result).toEqual(["a"]);
  });

  it("should show only unique-to-file2 with -13", () => {
    const lines1 = ["a", "b", "c"];
    const lines2 = ["b", "c", "d"];
    const result = compareSorted(lines1, lines2, [true, false, true]);
    expect(result).toEqual(["d"]);
  });

  it("should show nothing with -123", () => {
    const lines1 = ["a", "b"];
    const lines2 = ["b", "c"];
    const result = compareSorted(lines1, lines2, [true, true, true]);
    expect(result).toEqual([]);
  });

  // -------------------------------------------------------------------------
  // Custom delimiter.
  // -------------------------------------------------------------------------

  it("should use custom output delimiter", () => {
    const lines1 = ["a", "b"];
    const lines2 = ["b", "c"];
    const result = compareSorted(lines1, lines2, [false, false, false], ",");
    expect(result).toEqual(["a", ",,b", ",c"]);
  });

  // -------------------------------------------------------------------------
  // Edge cases.
  // -------------------------------------------------------------------------

  it("should handle single-element arrays", () => {
    const result = compareSorted(["a"], ["a"], [false, false, false]);
    expect(result).toEqual(["\t\ta"]);
  });

  it("should handle duplicate lines in both files", () => {
    const lines1 = ["a", "a", "b"];
    const lines2 = ["a", "b", "b"];
    const result = compareSorted(lines1, lines2, [false, false, false]);
    // First "a" matches "a" => common.
    // Second "a" < "b" => unique to file1.
    // "b" == "b" => common.
    // Extra "b" in file2 => unique to file2.
    expect(result).toEqual(["\t\ta", "a", "\t\tb", "\tb"]);
  });
});
