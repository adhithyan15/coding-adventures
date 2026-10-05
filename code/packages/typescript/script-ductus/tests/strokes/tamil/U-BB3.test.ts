import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB3.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ள"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB3 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ள");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "28b515385790f0a89c75d4cf1e787189ae698fca13fbf4b4346f9ad8e8129cdf",
    );
  });

  it("ள follows Frame 12's six movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl round the bowl and up the outer loop",
      "curve over the top into the junction",
      "draw the adjoining stem down",
      "rise back up the same stem",
      "carry the top bar right",
      "back, then down the right upright",
    ]);
  });

  it("starts inside the loop, goes down and back up the middle stem, then down the right upright", () => {
    const [loop, , down, up, bar, right] = segs;
    expect(loop.path[0].x).toBeLessThan(150);
    expect(loop.path[0].y).toBeGreaterThan(200);
    expect(loop.path[0].y).toBeLessThan(330);
    expect(end(down).y).toBeLessThan(80);
    expect(end(up).y).toBeGreaterThan(480);
    expect(end(bar).x).toBeGreaterThan(950);
    expect(end(right).y).toBeLessThan(80);
    expect(Math.abs(end(right).x - 840)).toBeLessThan(30);
  });

  it("ள's continuous order traces to Frame 12", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 12.*ள.*p\. 195/i);
    expect(src.variation).toMatch(
      /Module 12 identifies ள as the retroflex lateral.*Frame 12 numbers six hand-movements.*one continuous stroke.*in order without lifting.*starts inside the loop.*climbs back up that shared stem.*HP Labs India.*90% of the 264.*single pen-down stroke.*varies by school/i,
    );
  });
});
