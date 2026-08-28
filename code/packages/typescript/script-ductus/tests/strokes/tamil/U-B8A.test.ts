import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B8A.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஊ"];
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B8A stroke evidence", () => {
  registerStrokeHonestyTests([letter], { ஊ: 0.9 });

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஊ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "96774e5bf1bd1f2c627558a5a9f06e160fdbdf3eddb201383ab7f98dde03b98f",
    );
  });

  it("Tamil ஊ writes familiar உ before the three-run ள overlay", () => {
    expect(penLifts(letter)).toBe(3);
    expect(letter.strokes).toHaveLength(4);
    expect(letter.strokes.map((stroke) => stroke.segments.length)).toEqual([
      3, 3, 2, 1,
    ]);
    expect(letter.source.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(letter.source.variation).toMatch(
      /write உ first.*then write ள over it.*Frame 16.*three movements joined.*Frame 12.*six movements.*three pen-down runs.*four-run learner order.*Noto Sans Tamil/i,
    );
  });

  it("ஊ's compositional order traces to Module 17 and its familiar components", () => {
    const src = letter.source;
    expect(src.url).toBe("https://sites.la.utexas.edu/tamilscript/frame-17/92");
    expect(src.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(src.variation).toMatch(
      /long ū.*write உ first.*then write ள over it.*four-run learner order.*Noto Sans Tamil.*varies by school/i,
    );
  });
});
