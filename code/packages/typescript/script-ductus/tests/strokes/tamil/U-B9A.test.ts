import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B9A.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ச"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
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
      "0b654686eadc50501064657eb32f952e8e0fc62966a44ca97c1ee0e2674ecb43",
    );
  });

  it("ச follows its four movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "climb the left upright",
      "along the top and back",
      "down the stem, out right",
      "back, round the bowl",
    ]);
  });

  it("climbs, runs the top bar out and back, drops the stem, and comes back along the middle bar into the bowl", () => {
    const [climb, top, drop, bowl] = segs;
    expect(end(climb).y).toBeGreaterThan(climb.path[0].y + 150);
    expect(Math.max(...top.path.map((p) => p.x))).toBeGreaterThan(600);
    expect(end(top).x).toBeLessThan(500);
    // Movement 3 ends at the tip of the projecting middle bar ...
    expect(end(drop).x).toBeGreaterThan(600);
    // ... so movement 4 comes back along it before turning round the bowl.
    expect(bowl.path[1].x).toBeLessThan(bowl.path[0].x);
    expect(Math.min(...bowl.path.map((p) => p.y))).toBeLessThan(60);
    expect(Math.min(...bowl.path.map((p) => p.x))).toBeLessThan(120);
    // The bowl closes back at the inner crossing on the middle bar.
    const close = end(bowl);
    expect(
      drop.path.some((p) => Math.hypot(p.x - close.x, p.y - close.y) < 5),
    ).toBe(true);
  });

  it("ச's continuous order traces to Frame 3 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*ச.*p\. 191/i);
    expect(src.variation).toMatch(
      /Frame 3 numbers three upper-frame movements.*lower-left bowl.*one continuous stroke.*in order without lifting.*comes back along the bar to the inner crossing.*HP Labs India.*88% of the 176.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
