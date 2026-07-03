import { describe, it, expect } from "vitest";
import {
  parseA1,
  printA1,
  columnToLetters,
  lettersToColumn,
  parseRange,
  normalizeRange,
  expandRange,
  addressKey,
} from "../src/address.js";

describe("column letters ↔ index (bijective base-26)", () => {
  it("maps the first 26 columns A..Z to 0..25", () => {
    expect(columnToLetters(0)).toBe("A");
    expect(columnToLetters(25)).toBe("Z");
    expect(lettersToColumn("A")).toBe(0);
    expect(lettersToColumn("Z")).toBe(25);
  });

  it("rolls over correctly past Z (AA=26, AB=27, AZ=51, BA=52)", () => {
    expect(columnToLetters(26)).toBe("AA");
    expect(columnToLetters(27)).toBe("AB");
    expect(columnToLetters(51)).toBe("AZ");
    expect(columnToLetters(52)).toBe("BA");
    expect(lettersToColumn("AA")).toBe(26);
    expect(lettersToColumn("AZ")).toBe(51);
    expect(lettersToColumn("BA")).toBe(52);
  });

  it("is a clean round-trip for a wide range of indices", () => {
    for (const col of [0, 1, 25, 26, 700, 16383]) {
      expect(lettersToColumn(columnToLetters(col))).toBe(col);
    }
  });

  it("is case-insensitive on input", () => {
    expect(lettersToColumn("aa")).toBe(26);
  });

  it("rejects bad input", () => {
    expect(() => columnToLetters(-1)).toThrow(RangeError);
    expect(() => columnToLetters(1.5)).toThrow(RangeError);
    expect(() => lettersToColumn("")).toThrow(SyntaxError);
    expect(() => lettersToColumn("A1")).toThrow(SyntaxError);
  });
});

describe("A1 parse/print", () => {
  it("parses a simple address", () => {
    expect(parseA1("A1")).toMatchObject({ col: 0, row: 0 });
    expect(parseA1("B7")).toMatchObject({ col: 1, row: 6 });
    expect(parseA1("AA10")).toMatchObject({ col: 26, row: 9 });
  });

  it("parses absolute $ flags", () => {
    expect(parseA1("$A$1")).toMatchObject({
      col: 0,
      row: 0,
      absoluteCol: true,
      absoluteRow: true,
    });
    expect(parseA1("$A1").absoluteCol).toBe(true);
    expect(parseA1("A$1").absoluteRow).toBe(true);
  });

  it("round-trips through print", () => {
    for (const s of ["A1", "B7", "AA10", "$A$1", "Z99"]) {
      expect(printA1(parseA1(s))).toBe(s);
    }
  });

  it("tolerates surrounding whitespace", () => {
    expect(parseA1("  C3 ")).toMatchObject({ col: 2, row: 2 });
  });

  it("rejects malformed addresses", () => {
    expect(() => parseA1("1A")).toThrow();
    expect(() => parseA1("A0")).toThrow();
    expect(() => parseA1("")).toThrow();
  });

  it("addressKey ignores $ flags (A1 and $A$1 are the same cell)", () => {
    expect(addressKey(parseA1("A1"))).toBe(addressKey(parseA1("$A$1")));
  });
});

describe("ranges", () => {
  it("parses a bare cell into a 1×1 range", () => {
    const r = parseRange("B2");
    expect(r.start).toMatchObject({ col: 1, row: 1 });
    expect(r.end).toMatchObject({ col: 1, row: 1 });
  });

  it("parses a multi-cell range", () => {
    const r = parseRange("A1:B3");
    expect(r.start).toMatchObject({ col: 0, row: 0 });
    expect(r.end).toMatchObject({ col: 1, row: 2 });
  });

  it("normalizes swapped corners", () => {
    const r = normalizeRange(parseRange("B3:A1"));
    expect(r.start).toMatchObject({ col: 0, row: 0 });
    expect(r.end).toMatchObject({ col: 1, row: 2 });
  });

  it("expands a range row-major", () => {
    const cells = expandRange(parseRange("A1:B2")).map(printA1);
    expect(cells).toEqual(["A1", "B1", "A2", "B2"]);
  });

  it("expands a single column", () => {
    const cells = expandRange(parseRange("A1:A3")).map(printA1);
    expect(cells).toEqual(["A1", "A2", "A3"]);
  });
});
