import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BB0.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ர"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BB0 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ர");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "a7e0e81c893ff9aea522a1bf46d91e2d75fda17a46956a63ca29479bd20c700c",
    );
  });

  it("ர follows its four movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "down the left upright",
      "back up and along the top",
      "back, then down the stem",
      "into the angled tail",
    ]);
  });

  it("goes down the left upright, back up it, across, and down the middle", () => {
    const [down, up, middle, tail] = segs;
    expect(end(down).y).toBeLessThan(down.path[0].y - 300);
    const top = up.path.reduce((a, p) => (p.y > a.y ? p : a));
    expect(top.x).toBeLessThan(200);
    expect(end(up).x).toBeGreaterThan(500);
    expect(end(middle).y).toBeLessThan(60);
    expect(end(tail).y).toBeLessThan(-150);
  });

  it("ர's continuous order traces to Frame 3 of the UT Austin primer", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*ர.*p\. 191/i);
    expect(src.variation).toMatch(
      /Frame 3 identifies ர.*angular short fourth movement.*one continuous stroke.*in order without lifting.*runs down the left upright.*climbs back up.*HP Labs India.*90% of the 189.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
