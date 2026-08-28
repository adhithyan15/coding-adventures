import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { verifiedLetterFont } from "../../../src/scriptdata";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B8E.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["எ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B8E stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("எ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "726d39c401d3b7efc11f01136aa93dedcc721aff5bccf84892693185d5cde1af",
    );
  });

  it("uses the verified Tamil font", () => {
    expect(verifiedLetterFont("எ", letter.source.url)).toBe(
      "_fonts/NotoSansTamil-Static.ttf",
    );
  });

  it("Tamil எ keeps its six-movement body separate from the right upright", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(
      letter.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "climb the outer left side",
        "carry the top bar to the right",
        "retrace left and drop the inner upright",
        "turn left into the inner spiral",
        "sweep around the broad outer curve",
        "carry the lower foot right",
      ],
      ["draw the separate right upright up"],
    ]);
  });

  it("எ's two-run stroke order traces to Frame 5's second row", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 5.*எ.*p\. 193/);
    expect(src.variation).toMatch(
      /first six movements.*connected body.*upward right upright.*movement 7.*one lift.*varies by school.*two-run order.*Noto Sans Tamil/i,
    );
  });
});
