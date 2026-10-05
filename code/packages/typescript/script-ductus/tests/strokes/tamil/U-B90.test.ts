import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B90.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஐ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B90 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஐ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "c6838f95d6eab0c3db61d4bb1052c971bb044621b2f2273227bc3b5f721e7b2b",
    );
  });

  it("ஐ follows the animation's five parts without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "spiral out, then down the upright",
      "draw the upright back up",
      "round the right loop and back left",
      "circle the left bowl to the centre",
      "down the stem, round the right bowl",
    ]);
  });

  it("curls out of the spiral, runs down the upright and back up, and comes back down the centre stem", () => {
    const [spiral, upright, upperRight, lowerLeft, lowerRight] = segs;
    // The spiral's inner end is a free end, so the pen starts there.
    expect(spiral.path[0].x).toBeGreaterThan(220);
    expect(spiral.path[0].x).toBeLessThan(300);
    expect(spiral.path[0].y).toBeGreaterThan(440);
    expect(Math.min(...spiral.path.map((p) => p.x))).toBeLessThan(130);
    // It runs down the central upright to that upright's free foot ...
    expect(Math.abs(end(spiral).x - 556)).toBeLessThan(10);
    expect(end(spiral).y).toBeLessThan(240);
    // ... and movement 2 draws the upright back up.
    for (const p of upright.path) expect(Math.abs(p.x - 556)).toBeLessThan(10);
    expect(end(upright).y).toBeGreaterThan(390);
    expect(Math.max(...upperRight.path.map((p) => p.x))).toBeGreaterThan(900);
    expect(end(upperRight).x).toBeLessThan(220);
    expect(Math.min(...lowerLeft.path.map((p) => p.y))).toBeLessThan(-270);
    // The lower-left bowl ends at the top of the short centre stem ...
    expect(Math.abs(end(lowerLeft).x - 556)).toBeLessThan(10);
    expect(end(lowerLeft).y).toBeGreaterThan(-40);
    // ... so the last part comes back down it into the lower-right bowl.
    expect(lowerRight.path[1].y).toBeLessThan(lowerRight.path[0].y);
    expect(Math.max(...lowerRight.path.map((p) => p.x))).toBeGreaterThan(900);
    expect(end(lowerRight).y).toBeGreaterThan(0);
  });

  it("ஐ's continuous order traces to the Commons animation and Frame 11", () => {
    expect(letter.source.url).toBe(
      "https://commons.wikimedia.org/wiki/File:Writing_Tamil_10.gif",
    );
    expect(letter.source.citation).toMatch(
      /Info-farmer.*Writing Tamil 10.*ஐ.*CC BY-SA 3\.0.*Frame 11.*p\. 194/i,
    );
    expect(letter.source.variation).toMatch(
      /13-frame.*five parts.*spiral.*upright drawn upward.*upper-right loop.*lower-left bowl.*lower-right bowl.*not evidence of pen lifts.*seven movements.*one continuous stroke.*in order without lifting.*starts inside the spiral.*draws it back up.*comes back down that stem.*HP Labs India.*99% of the 169.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
