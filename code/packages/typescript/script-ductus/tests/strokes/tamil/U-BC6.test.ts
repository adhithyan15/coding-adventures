import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BC6.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ெ"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BC6 stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ெ");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "5ab5031d2d7a9f36f0c5b14d8bb1bbe710d11f91b92184c32688aeea4b543eb4",
    );
  });

  it("ெ is one unbroken stroke of three movements", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["circle the small loop", "climb the outer curve", "over and down the right upright"]);
  });

  it("circles the small curl, climbs the outer curve and ends down the right upright", () => {
    const [curl, outer, upright] = segs;
    expect(curl.path[0].y).toBeLessThan(100);
    expect(Math.max(...curl.path.map((p) => p.y))).toBeLessThan(400);
    expect(Math.min(...outer.path.map((p) => p.x))).toBeLessThan(120);
    expect(end(outer).y).toBeGreaterThan(780);
    expect(end(upright).x).toBeGreaterThan(700);
    expect(end(upright).y).toBeLessThan(40);
  });

  it("ெ's order traces to native writers' pen traces in LipiTk class 41", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 41 \(ெ, e sign\).*native Tamil writers.*MIT licence/,
    );
    expect(src.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil.*varies by writer/);
  });
});
