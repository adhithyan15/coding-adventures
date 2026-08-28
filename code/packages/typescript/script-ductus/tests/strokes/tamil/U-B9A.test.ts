import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B9A.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ச"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B9A stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ச");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "5877d5671da4a239903063f7903a060ea0af84a7e95f09d14cf10341f6cecf4b",
    );
  });

  it("ச joins its upper frame before lifting for the lower-left bowl", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 1,
    ]);
    expect(letter.strokes[0].segments.map((segment) => segment.label)).toEqual([
      "climb the left upright",
      "carry the top bar to the right",
      "drop the inner upright and carry right",
    ]);
    expect(letter.strokes[1].segments[0].label).toBe(
      "turn around and close the lower-left bowl",
    );
  });

  it("ச's two-run order traces to Frame 3 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*ச.*p\. 191/i);
    expect(src.variation).toMatch(
      /three joined upper-frame movements.*separate fourth movement.*lower-left bowl.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
  });
});
