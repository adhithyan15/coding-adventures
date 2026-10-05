import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B95.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["க"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B95 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("க");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "081c17666e2beaa83e56e90d55373ed367707fc72c3930aea15f7d7f85602e61",
    );
  });

  it("க is one stroke: frame, lower-left bowl, then across into the right bowl", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "climb the left upright",
      "carry the top bar right and back",
      "drop the inner upright",
      "round the lower-left bowl",
      "return up the outer left side",
      "cross into the right bowl",
    ]);
  });

  it("climbs the left upright, drops the inner one, and crosses the middle bar into the right bowl", () => {
    const [climb, , drop, , ret, cross] = segs;
    expect(climb.path[0].x).toBeLessThan(250);
    expect(climb.path[0].y).toBeGreaterThan(250);
    expect(climb.path[0].y).toBeLessThan(350);
    expect(end(climb).y).toBeGreaterThan(climb.path[0].y + 150);
    expect(end(drop).y).toBeLessThan(drop.path[0].y - 150);
    // The bowl's return ends back at the middle left, where the climb began.
    expect(Math.hypot(end(ret).x - climb.path[0].x, end(ret).y - climb.path[0].y)).toBeLessThan(5);
    // Movement 6 starts there and runs right along the middle bar first.
    expect(cross.path[2].x).toBeGreaterThan(cross.path[0].x + 100);
    expect(Math.abs(cross.path[2].y - cross.path[0].y)).toBeLessThan(20);
  });

  it("க's continuous order traces to Appendix I Frame 3", () => {
    const src = letter.source;
    expect(src.url).toContain(
      "tamilscript/files/2009/08/hw_lettersinstructions.pdf",
    );
    expect(src.citation).toMatch(/Appendix I.*Frame 3.*க.*p\. 191/i);
    expect(src.variation).toMatch(
      /Frame 3 numbers six hand-movements.*one continuous stroke.*six movements in order without lifting.*crosses the middle bar.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*88% of the 203.*single pen-down stroke/i,
    );
    // The note must not present one order as the only order.
    expect(src.variation).toMatch(/no single/i);
  });
});
