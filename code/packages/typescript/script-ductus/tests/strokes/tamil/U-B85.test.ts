import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-B85.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["அ"];
const segs = letter.strokes.flatMap((stroke) => stroke.segments);
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-B85 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("அ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "da3b843757ff7e9ac16e197644782d0c8593d7c98262a1bbb5fedacb85e45c13",
    );
  });

  it("அ follows Frame 4's movements without lifting (LipiTk: 104 of 114 one stroke)", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual([
      "curl around the upper loop",
      "sweep down the outer curve",
      "turn around the lower loop",
      "carry the horizontal to the right",
      "climb the right upright to its top",
      "draw the right upright down",
    ]);
  });

  it("climbs the right upright from the end of the horizontal, then draws it down", () => {
    const [curl, , , horizontal, climb, down] = segs;
    // The pen starts at the tip inside the upper curl.
    expect(curl.path[0].y).toBeGreaterThan(480);
    expect(climb.path[0]).toEqual(end(horizontal));
    // Up the upright to its top ...
    for (const p of climb.path.slice(1)) expect(p.x).toBe(990);
    expect(end(climb).y).toBeGreaterThan(500);
    // ... and straight back down over that ink to the foot.
    expect(down.path[0]).toEqual(end(climb));
    for (const p of down.path) expect(p.x).toBe(990);
    expect(end(down).y).toBeLessThan(-100);
  });

  it("cites Frame 4 for the order and the LipiTk counts for the one stroke", () => {
    const src = letter.source;
    expect(src.url).toContain("tamilscript");
    expect(src.citation).toMatch(/Appendix I.*Frame 4.*அ/);
    expect(
      src.variation,
      "must not present one order as the only order",
    ).toMatch(/variation|no single/i);
    expect(src.variation).toMatch(
      /Frame 4.*not evidence of one.*HP Labs India.*LipiTk 4\.0.*hpl-tamil-iso-char.*104 of the 114 stored prototypes of அ \(91%\) are one pen-down stroke.*95 of them \(91%\).*up the right upright.*overrides the old lift.*Noto Sans Tamil.*only counts and shares are cited.*no trace was copied/i,
    );
    expect(src.citation).toMatch(
      /drawn in one stroke after HP Labs India.*Tamil recognizer, class 0 \(அ\)/,
    );
  });
});
