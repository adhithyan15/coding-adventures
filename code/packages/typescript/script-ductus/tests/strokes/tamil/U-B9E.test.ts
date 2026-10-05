import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B9E.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஞ"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B9E stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஞ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "f2e536d5ff832246e9d5af26a03f60a02a594518619f0c7212e34e395f616c17",
    );
  });

  it("Tamil ஞ follows Frame 8's eight movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl round the small inner loop",
      "up and over to the top bar",
      "carry the top bar right",
      "back, then down the stem",
      "on down the stem",
      "back up, round the right curve",
      "round the bottom bowl",
      "up the outer left side",
    ]);
  });

  it("spirals out of the inner loop into the bar, then climbs back up the stem into the outer bowl", () => {
    const [loop, rise, bar, back, down, outer, bottom, left] = segs;
    expect(loop.path[0].x).toBeLessThan(400);
    expect(loop.path[0].y).toBeGreaterThan(150);
    expect(loop.path[0].y).toBeLessThan(300);
    expect(end(rise).y).toBeGreaterThan(480);
    expect(end(bar).x).toBeGreaterThan(980);
    expect(Math.abs(end(back).x - 825)).toBeLessThan(20);
    expect(end(down).y).toBeLessThan(60);
    expect(outer.path[1].y).toBeGreaterThan(outer.path[0].y);
    expect(Math.max(...outer.path.map((p) => p.x))).toBeGreaterThan(1100);
    expect(Math.min(...bottom.path.map((p) => p.y))).toBeLessThan(-270);
    expect(end(left).y).toBeGreaterThan(450);
    expect(end(left).x).toBeLessThan(260);
  });

  it("ஞ's continuous order traces to Appendix I Frame 8", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(
      /Tamil Script Learners Manual.*Appendix I.*Frame 8.*ஞ.*p\. 194/i,
    );
    expect(src.variation).toMatch(
      /eight movements.*1–2.*left inner loop.*3.*top bar.*4–5.*central descent.*6–8.*outer bowl.*one continuous stroke.*in order without lifting.*starts inside the small loop.*climbs back up.*HP Labs India.*86% of the 140.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
