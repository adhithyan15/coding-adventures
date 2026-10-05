import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B8A.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஊ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B8A stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஊ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "2b2094c96c172fd20281e819a741096a7f7bef7c1b0193c875cae497f9b6eb3e",
    );
  });

  it("Tamil ஊ writes உ, lifts once, then writes ள in one stroke", () => {
    expect(penLifts(letter)).toBe(1);
    expect(letter.strokes).toHaveLength(2);
    expect(
      letter.strokes.map((stroke) =>
        stroke.segments.map((segment) => segment.label),
      ),
    ).toEqual([
      [
        "sweep outward around the spiral",
        "down the outer curve onto the baseline",
        "carry the baseline to the right",
      ],
      [
        "curl round the bowl and up the outer loop",
        "curve over the top into the junction",
        "draw the adjoining stem down",
        "rise back up the same stem",
        "carry the top bar right",
        "back, then down the right upright",
      ],
    ]);
    expect(letter.source.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(letter.source.variation).toMatch(
      /write உ first.*then write ள over it.*Frame 16.*three movements.*Frame 12.*six.*one continuous stroke.*lifts once.*starting inside its loop.*climbing back up the shared middle stem.*HP Labs India.*93% of the 73.*two pen-down strokes.*two-run learner order.*Noto Sans Tamil/i,
    );
  });

  it("starts உ inside its spiral and ள inside its loop, as each letter is written alone", () => {
    const [u, lla] = letter.strokes.map((stroke) => stroke.segments);
    expect(u[0].path[0].x).toBeGreaterThan(280);
    expect(u[0].path[0].x).toBeLessThan(360);
    expect(u[0].path[0].y).toBeGreaterThan(420);
    expect(end(u[2]).x).toBeGreaterThan(1450);
    expect(lla[0].path[0].x).toBeLessThan(800);
    expect(lla[0].path[0].y).toBeGreaterThan(300);
    expect(lla[0].path[0].y).toBeLessThan(400);
    // Down the shared middle stem and back up it, as ள does.
    const [, , down, up, bar, right] = lla;
    expect(end(down).y).toBeLessThan(200);
    expect(up.path[0]).toEqual(end(down));
    for (const p of up.path) expect(p.x).toBe(1117);
    expect(end(bar).x).toBeGreaterThan(1450);
    expect(right.path.at(-2)!.x).toBe(1337);
    expect(end(right).y).toBeLessThan(200);
  });

  it("ஊ's compositional order traces to Module 17 and its familiar components", () => {
    const src = letter.source;
    expect(src.url).toBe("https://sites.la.utexas.edu/tamilscript/frame-17/92");
    expect(src.citation).toMatch(
      /Module 17.*ஊ.*Frames 17, 16, and 12.*pp\. 195–196/i,
    );
    expect(src.variation).toMatch(
      /long ū.*write உ first.*then write ள over it.*two-run learner order.*Noto Sans Tamil.*varies by school/i,
    );
  });
});
