import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B86.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ஆ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B86 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ஆ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "40d7a6def49d026f06886088fd59f68e2928afded0b76be4b6f3307640efdabc",
    );
  });

  it("ஆ follows Frame 4's movements without lifting (LipiTk: 28 of 29 one stroke)", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl around the upper loop",
      "sweep down the outer curve",
      "turn around the lower loop",
      "carry the horizontal to the right",
      "climb the right upright to its top",
      "draw the right upright down",
      "loop the long-vowel tail to the left",
    ]);
  });

  it("climbs the right upright from the horizontal, comes down it and runs on into the loop", () => {
    const [curl, , , horizontal, climb, down, loop] = segs;
    expect(curl.path[0].y).toBeGreaterThan(480);
    expect(climb.path[0]).toEqual(end(horizontal));
    for (const p of climb.path.slice(1)) expect(p.x).toBe(990);
    expect(end(climb).y).toBeGreaterThan(500);
    expect(down.path[0]).toEqual(end(climb));
    for (const p of down.path) expect(p.x).toBe(990);
    expect(loop.path[0]).toEqual(end(down));
    expect(Math.max(...loop.path.map((p) => p.x))).toBeGreaterThan(1200);
    expect(Math.min(...loop.path.map((p) => p.y))).toBeLessThan(-250);
  });

  it("cites Frame 4 for the order and the LipiTk counts for the one stroke", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*ஆ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
    expect(src.variation).toMatch(
      /Frame 4.*not evidence of one.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*28 of the 29 stored prototypes of ஆ \(97%\) are one pen-down stroke.*26 of them \(93%\).*up the right upright.*overrides the old lift.*Noto Sans Tamil.*no trace was copied/i,
    );
    expect(src.citation).toMatch(
      /drawn in one stroke after HP Labs India.*Tamil recognizer, class 1 \(ஆ\)/,
    );
  });
});
