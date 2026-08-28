import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B9F.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ட"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B9F stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ட");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "c97c378853a8287721b78d10ffe7814fc4a6a6c4727dabaae75b95df8c4c0134",
    );
  });

  it("ட descends and turns along its foot without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(letter.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "down the left upright",
      "along the long rightward foot",
    ]);
  });

  it("ட's continuous order traces to Frame 1 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 1.*ட.*p\. 190/i);
    expect(src.variation).toMatch(
      /left descent.*rightward foot.*two joined movements.*Module 1 identifies.*top-to-bottom.*left-to-right.*varies by school.*continuous order.*Noto Sans Tamil/i,
    );
  });
});
