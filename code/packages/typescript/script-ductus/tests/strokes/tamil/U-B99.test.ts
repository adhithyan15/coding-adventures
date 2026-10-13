import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B99.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ங"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B99 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ங");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "32ac15adc47ec285a06447cad0fa7458c223d5fa86a96f02c3295dfa65c1a6ce",
    );
  });

  it("ங is one stroke in native writers' order (LipiTk: 92 of 108)", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "draw the left upright down",
      "climb back up it, then the top bar",
      "back along the bar, down the stem",
      "back up, then round the bowl",
      "left along the low bar to its end",
      "back along it and up the right upright",
    ]);
  });

  it("goes down the left upright and back up first, and up the right upright last", () => {
    const [left, climbAndBar, stem, bowl, lowOut, lowBackAndRight] = segs;
    // Down the tall left upright from its top ...
    expect(left.path[0].x).toBeLessThan(175);
    expect(left.path[0].y).toBeGreaterThan(480);
    expect(end(left).y).toBeLessThan(40);
    // ... back up it and out along the top bar.
    expect(Math.max(...climbAndBar.path.map((p) => p.y))).toBeGreaterThan(500);
    expect(end(climbAndBar).x).toBeGreaterThan(570);
    // Back along the bar to the inner stem and down to its free foot.
    expect(end(stem).x).toBe(388);
    expect(end(stem).y).toBeLessThan(170);
    // Back up, round the bowl, and down to the low bar.
    expect(Math.max(...bowl.path.map((p) => p.x))).toBeGreaterThan(660);
    expect(end(bowl).y).toBeLessThan(60);
    // Out to the low bar's left end under the stem, and back.
    expect(end(lowOut).x).toBeLessThan(370);
    // Up the right upright, ending at its top.
    const last = lowBackAndRight.path.slice(-8);
    for (const p of last) expect(p.x).toBe(912);
    expect(end(lowBackAndRight).y).toBeGreaterThan(500);
  });

  it("keeps Frame 2's citation and says where the LipiTk counts override it", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 2.*ங.*p\. 191/i);
    expect(src.variation).toMatch(
      /Frame 2.*detached part.*HP Labs India.*LipiTk 4\.0.*92 of the 108 stored prototypes of ங \(85%\) are one pen-down stroke.*69 \(75%\).*81 \(88%\).*overrides Frame 2's order and its lift.*Noto Sans Tamil.*no trace was copied.*varies by school/i,
    );
    expect(src.citation).toMatch(
      /reordered, and drawn in one stroke, after HP Labs India.*Tamil recognizer, class 13 \(ங\)/,
    );
  });
});
