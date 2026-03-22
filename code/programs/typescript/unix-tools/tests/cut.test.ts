/**
 * Tests for cut -- remove sections from each line of files.
 *
 * We test the exported business logic functions: parseRangeList,
 * isSelected, cutByChars, cutByFields, and cutLine.
 */

import { describe, it, expect } from "vitest";
import {
  parseRangeList,
  isSelected,
  cutByChars,
  cutByFields,
  cutLine,
  CutOptions,
  Range,
} from "../src/cut.js";

// ---------------------------------------------------------------------------
// parseRangeList.
// ---------------------------------------------------------------------------

describe("parseRangeList", () => {
  it("should parse a single number", () => {
    const ranges = parseRangeList("3");
    expect(ranges).toEqual([{ start: 3, end: 3 }]);
  });

  it("should parse a range", () => {
    const ranges = parseRangeList("1-5");
    expect(ranges).toEqual([{ start: 1, end: 5 }]);
  });

  it("should parse open-ended range (N-)", () => {
    const ranges = parseRangeList("3-");
    expect(ranges).toEqual([{ start: 3, end: Infinity }]);
  });

  it("should parse open-start range (-N)", () => {
    const ranges = parseRangeList("-5");
    expect(ranges).toEqual([{ start: 1, end: 5 }]);
  });

  it("should parse comma-separated values", () => {
    const ranges = parseRangeList("1,3,5");
    expect(ranges).toEqual([
      { start: 1, end: 1 },
      { start: 3, end: 3 },
      { start: 5, end: 5 },
    ]);
  });

  it("should parse mixed ranges and numbers", () => {
    const ranges = parseRangeList("1-3,5,7-");
    expect(ranges).toEqual([
      { start: 1, end: 3 },
      { start: 5, end: 5 },
      { start: 7, end: Infinity },
    ]);
  });

  it("should sort ranges by start position", () => {
    const ranges = parseRangeList("5,1,3");
    expect(ranges[0].start).toBe(1);
    expect(ranges[1].start).toBe(3);
    expect(ranges[2].start).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// isSelected.
// ---------------------------------------------------------------------------

describe("isSelected", () => {
  const ranges: Range[] = [
    { start: 1, end: 3 },
    { start: 5, end: 5 },
    { start: 7, end: Infinity },
  ];

  it("should return true for positions in range", () => {
    expect(isSelected(1, ranges)).toBe(true);
    expect(isSelected(2, ranges)).toBe(true);
    expect(isSelected(3, ranges)).toBe(true);
    expect(isSelected(5, ranges)).toBe(true);
    expect(isSelected(7, ranges)).toBe(true);
    expect(isSelected(100, ranges)).toBe(true);
  });

  it("should return false for positions outside ranges", () => {
    expect(isSelected(4, ranges)).toBe(false);
    expect(isSelected(6, ranges)).toBe(false);
    expect(isSelected(0, ranges)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// cutByChars.
// ---------------------------------------------------------------------------

describe("cutByChars", () => {
  it("should select specific characters", () => {
    expect(cutByChars("abcdefgh", "1,3,5", false)).toBe("ace");
  });

  it("should select a range of characters", () => {
    expect(cutByChars("abcdefgh", "2-5", false)).toBe("bcde");
  });

  it("should select from position to end", () => {
    expect(cutByChars("abcdefgh", "6-", false)).toBe("fgh");
  });

  it("should select from start to position", () => {
    expect(cutByChars("abcdefgh", "-3", false)).toBe("abc");
  });

  it("should handle complement mode", () => {
    expect(cutByChars("abcdefgh", "1-3", true)).toBe("defgh");
  });

  it("should handle positions beyond line length", () => {
    expect(cutByChars("abc", "1-5", false)).toBe("abc");
  });

  it("should handle empty line", () => {
    expect(cutByChars("", "1-5", false)).toBe("");
  });

  it("should handle output delimiter between ranges", () => {
    expect(cutByChars("abcdefgh", "1-3,6-8", false, ",")).toBe("abc,fgh");
  });
});

// ---------------------------------------------------------------------------
// cutByFields.
// ---------------------------------------------------------------------------

describe("cutByFields", () => {
  const baseOpts: CutOptions = {
    fields: "2",
    delimiter: "\t",
    onlyDelimited: false,
    complement: false,
  };

  it("should extract a single field", () => {
    expect(cutByFields("a\tb\tc", baseOpts)).toBe("b");
  });

  it("should extract multiple fields", () => {
    const opts = { ...baseOpts, fields: "1,3" };
    expect(cutByFields("a\tb\tc", opts)).toBe("a\tc");
  });

  it("should return full line if no delimiter found", () => {
    expect(cutByFields("abcdef", baseOpts)).toBe("abcdef");
  });

  it("should suppress lines without delimiter when onlyDelimited is set", () => {
    const opts = { ...baseOpts, onlyDelimited: true };
    expect(cutByFields("abcdef", opts)).toBeNull();
  });

  it("should use custom delimiter", () => {
    const opts = { ...baseOpts, delimiter: "," };
    expect(cutByFields("a,b,c", opts)).toBe("b");
  });

  it("should use output delimiter", () => {
    const opts = { ...baseOpts, fields: "1,3", outputDelimiter: ":" };
    expect(cutByFields("a\tb\tc", opts)).toBe("a:c");
  });

  it("should complement field selection", () => {
    const opts = { ...baseOpts, fields: "2", complement: true };
    expect(cutByFields("a\tb\tc", opts)).toBe("a\tc");
  });

  it("should handle field range", () => {
    const opts = { ...baseOpts, fields: "2-3" };
    expect(cutByFields("a\tb\tc\td", opts)).toBe("b\tc");
  });
});

// ---------------------------------------------------------------------------
// cutLine.
// ---------------------------------------------------------------------------

describe("cutLine", () => {
  it("should dispatch to cutByFields when fields option is set", () => {
    const opts: CutOptions = {
      fields: "1",
      delimiter: ",",
      onlyDelimited: false,
      complement: false,
    };
    expect(cutLine("hello,world", opts)).toBe("hello");
  });

  it("should dispatch to cutByChars when characters option is set", () => {
    const opts: CutOptions = {
      characters: "1-5",
      delimiter: "\t",
      onlyDelimited: false,
      complement: false,
    };
    expect(cutLine("hello world", opts)).toBe("hello");
  });

  it("should dispatch to cutByChars when bytes option is set", () => {
    const opts: CutOptions = {
      bytes: "1-3",
      delimiter: "\t",
      onlyDelimited: false,
      complement: false,
    };
    expect(cutLine("abcdef", opts)).toBe("abc");
  });
});
