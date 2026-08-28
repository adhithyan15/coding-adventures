import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BAA.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ப"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BAA stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ப");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "624d2325815e10fd32a13ed79019c2f20c745b10df6feb10615e6b65d80aaad7",
    );
  });

  it("ப descends, crosses the bottom, and rises without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(letter.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "down the left upright",
      "along the bottom",
      "up the right upright",
    ]);
  });

  it("ப's continuous order traces to Frame 1 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript/category/3-moduals/module-01");
    expect(src.citation).toMatch(/Tamil Script Learners Manual.*Frame 1.*ப/i);
    expect(src.variation).toMatch(
      /left-to-right.*top-to-bottom.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  });
});
