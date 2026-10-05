import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BA8.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ந"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BA8 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ந");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "0c1593cb5e9321d4b67fd08f03010131f27d330a3b29f7888d871a5c68a19b25",
    );
  });

  it("ந follows Frame 5's six movements without lifting", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "draw the left upright upward",
      "carry the top bar right",
      "return left to the middle",
      "descend the middle upright",
      "back up, round the right bowl",
      "sweep the low tail left",
    ]);
  });

  it("climbs the left upright, descends the middle one, and climbs back into the bowl", () => {
    const [climb, , ret, descend, bowl] = segs;
    expect(end(climb).y).toBeGreaterThan(climb.path[0].y + 300);
    expect(end(ret).x).toBeLessThan(ret.path[0].x - 100);
    expect(end(descend).y).toBeLessThan(80);
    expect(bowl.path[1].y).toBeGreaterThan(bowl.path[0].y);
    expect(Math.max(...bowl.path.map((p) => p.x))).toBeGreaterThan(650);
  });

  it("ந's continuous order traces to Frame 5's first row", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 5.*ந.*p\. 193/i);
    expect(src.variation).toMatch(
      /Module 5 identifies.*voiced dental nasal.*extended final curve may be omitted.*Frame 5 numbers six hand-movements.*one continuous stroke.*in order without lifting.*climbs back up the middle upright.*HP Labs India.*85% of the 224.*single pen-down stroke.*varies by school.*Noto Sans Tamil/i,
    );
  });
});
