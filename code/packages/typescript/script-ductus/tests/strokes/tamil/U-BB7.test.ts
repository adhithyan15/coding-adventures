import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB7.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஷ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB7 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஷ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "c3fb1fcfb894b2ef67fad4c3f8490a4b50b7a28452f1bd885dcb636c4c5ed5c1",
    );
  });

  it("ஷ draws Narale's four parts in one stroke (LipiTk: 175 of 188)", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl round the small inner loop",
      "round the bottom and up the outer left side",
      "arch over the top and down to the bar",
      "carry the bar to the right",
      "up the right end and back over to the loop",
      "round the loop and down the tail",
    ]);
  });

  it("curls out of the inner loop, crosses the bar, and ends at the foot of the tail", () => {
    const [inner, outer, arch, bar, turn, loop] = segs;
    // The tip inside the small loop, turning clockwise on the page: up and
    // right over the loop first.
    expect(inner.path[0].x).toBeLessThan(130);
    expect(inner.path[0].y).toBeGreaterThan(190);
    expect(inner.path[0].y).toBeLessThan(230);
    expect(inner.path[1].y).toBeGreaterThan(inner.path[0].y);
    expect(inner.path[1].x).toBeGreaterThan(inner.path[0].x);
    // Round the bottom and up the outer side, over the top to the bar.
    expect(Math.min(...outer.path.map((p) => p.x))).toBeLessThan(110);
    expect(Math.max(...arch.path.map((p) => p.y))).toBeGreaterThan(520);
    // Left to right along the bar, up its right end and back over.
    expect(end(bar).x).toBeGreaterThan(1200);
    expect(Math.max(...turn.path.map((p) => p.y))).toBeGreaterThan(260);
    expect(end(turn).x).toBeLessThan(900);
    // Round the loop and down the tail: the stroke ends at its lowest point.
    expect(Math.max(...loop.path.map((p) => p.y))).toBeGreaterThan(520);
    expect(end(loop).y).toBeLessThan(-240);
  });

  it("keeps Narale's citation and says where the LipiTk counts override it", () => {
    expect(letter.source.url).toBe(
      "https://tamilnavarasam.in/Books/Others/Tamil_eng_hindi.pdf",
    );
    expect(letter.source.citation).toMatch(
      /Narale.*Learn Tamil Through English\/Hindi.*Third Tamil Granthakshar ஷ.*p\. 13/i,
    );
    expect(letter.source.variation).toMatch(
      /numbers ஷ's four parts.*not evidence of a lift.*HP Labs India.*LipiTk 4\.0.*175 of the 188 stored prototypes of ஷ \(93%\) are one pen-down stroke.*170 of the 175 \(97%\).*clockwise.*overrides the four runs.*Noto Sans Tamil.*no trace was copied.*varies by school/i,
    );
    expect(letter.source.citation).toMatch(
      /drawn in one stroke after HP Labs India.*Tamil recognizer, class 31 \(ஷ\)/,
    );
  });
});
