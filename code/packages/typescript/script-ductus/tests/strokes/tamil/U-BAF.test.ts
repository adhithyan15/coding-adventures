import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BAF.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ய"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BAF stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ய");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "aecf2ac6ffde54981223a416023736d89d2d8cf884ecaa5a43ed974113b0ad0b",
    );
  });

  it("ய joins its hook, retraced center, base, and right upright without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(letter.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "down the left upright",
      "around the curved foot into the center",
      "up the central upright",
      "retrace down the central upright",
      "along the bottom",
      "up the right upright",
    ]);
  });

  it("ய's six-movement continuous order traces to Appendix I Frame 1", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 1.*ய.*p\. 190/i);
    expect(src.variation).toMatch(
      /six joined movements.*down the left.*central upright.*across the bottom.*up the right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  });
});
