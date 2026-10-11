import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B8E.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["எ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
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
      "913a28e5058e441f6b6e1ed344f2d1f82c00d8b9af354d09f76b8684d5818f56",
    );
  });

  it("எ is one stroke in native writers' order (LipiTk: 81 of 88)", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl round the inner bowl",
      "climb the outer left side",
      "carry the top bar to its right end",
      "back to the upright, then down it",
    ]);
  });

  it("starts inside the bowl, turns it clockwise, and draws the upright down last", () => {
    const [bowl, left, bar, upright] = segs;
    // The inner end of the curl, at the left inside the bowl.
    expect(bowl.path[0].x).toBeLessThan(130);
    expect(bowl.path[0].y).toBeGreaterThan(200);
    expect(bowl.path[0].y).toBeLessThan(300);
    // Clockwise on the page: first right over the bowl's top, then down its
    // right side and back left along its foot.
    expect(bowl.path[1].x).toBeGreaterThan(bowl.path[0].x);
    expect(Math.max(...bowl.path.map((p) => p.x))).toBeGreaterThan(370);
    expect(Math.min(...bowl.path.map((p) => p.y))).toBeLessThan(40);
    expect(end(bowl).x).toBeLessThan(130);
    // Up the outer left side and along the top bar to its right end ...
    expect(end(left).y).toBeGreaterThan(450);
    expect(end(bar).x).toBeGreaterThan(740);
    // ... then back along the bar to the upright and down it.
    expect(upright.path[0]).toEqual(end(bar));
    const turn = upright.path.findIndex((p) => p.x === 569);
    expect(upright.path[turn].y).toBeGreaterThan(500);
    for (const p of upright.path.slice(turn)) expect(p.x).toBe(569);
    expect(end(upright).y).toBeLessThan(30);
  });

  it("keeps Frame 5's citation and says where the LipiTk counts override it", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 5.*எ.*p\. 193/i);
    expect(src.variation).toMatch(
      /Frame 5.*right upright last, upward from its foot, after a lift.*HP Labs India.*LipiTk 4\.0.*81 of the 88 stored prototypes of எ \(92%\) are one pen-down stroke.*clockwise.*overrides Frame 5's order and its lift.*Noto Sans Tamil.*no trace was copied.*varies by school/i,
    );
    expect(src.citation).toMatch(
      /reordered, and drawn in one stroke, after HP Labs India.*Tamil recognizer, class 6 \(எ\)/,
    );
  });
});
