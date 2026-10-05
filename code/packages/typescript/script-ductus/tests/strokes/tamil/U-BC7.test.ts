import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BC7.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ே"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BC7 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ே");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "c5300e116c2fae6703cd5b9f0b6fdc28f0c5f50a5ca5bfe934806118c6517706",
    );
  });

  it("ே is one unbroken stroke of three movements", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["circle the small upper loop", "sweep left and down the big curve", "circle up into the lower loop"]);
  });

  it("circles the upper loop, sweeps down the big curve and curls into the lower loop", () => {
    const [upper, curve, lower] = segs;
    expect(upper.path[0].y).toBeGreaterThan(700);
    expect(upper.path[2].x).toBeLessThan(upper.path[0].x);
    expect(Math.min(...curve.path.map((p) => p.x))).toBeLessThan(120);
    expect(end(curve).y).toBeLessThan(60);
    expect(Math.max(...lower.path.map((p) => p.y))).toBeLessThan(350);
    expect(end(lower).y).toBeLessThan(100);
  });

  it("ே's order traces to native writers' pen traces in LipiTk class 42", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 42 \(ே, ee sign\).*native Tamil writers.*MIT licence/,
    );
    expect(src.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil.*varies by writer/);
  });
});
