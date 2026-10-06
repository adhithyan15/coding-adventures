import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import { DUCTUS, penLifts, type Point } from "../../../src/strokes";
import { entry } from "../../../src/strokes/tamil/U-BBE.ts";
import { registerStrokeHonestyTests } from "../../support/stroke-honesty";

const letter = DUCTUS["ா"];
const segs = letter.strokes[0].segments;
const end = (segment: { path: Point[] }): Point => segment.path.at(-1)!;
const sha256 = (value: string): string =>
  createHash("sha256").update(value).digest("hex");

describe("Tamil U-BBE stroke evidence", () => {
  registerStrokeHonestyTests([letter]);

  it("assembles the glyph-owned tuple without copying its letter object", () => {
    expect(entry[0]).toBe("ா");
    expect(letter).toBe(entry[1]);
  });

  it("preserves the exact glyph-owned data", () => {
    expect(sha256(JSON.stringify(letter))).toBe(
      "ce35c1032d3b92eba9d91eb3e0fcab0157d97ccdbf6550714e89addfc327e4d4",
    );
  });

  it("ா is one unbroken stroke of three movements", () => {
    expect(penLifts(letter)).toBe(0);
    expect(letter.strokes).toHaveLength(1);
    expect(segs.map((segment) => segment.label)).toEqual(["down the left upright", "back up and along the top", "back, then down the stem"]);
  });

  it("draws the left upright down, climbs back to the top bar, and ends down the right upright", () => {
    const [down, upAndAcross, stem] = segs;
    expect(end(down).y).toBeLessThan(down.path[0].y - 250);
    expect(Math.max(...upAndAcross.path.map((p) => p.y))).toBeGreaterThan(500);
    expect(end(upAndAcross).x).toBeGreaterThan(550);
    expect(end(stem).x).toBeGreaterThan(down.path[0].x + 200);
    expect(end(stem).y).toBeLessThan(40);
  });

  it("ா's order traces to native writers' pen traces in LipiTk class 36", () => {
    const src = letter.source;
    expect(src.url).toBe("https://lipitk.sourceforge.net/lipi-reco.htm");
    expect(src.citation).toMatch(
      /HP Labs India.*Lipi Indic Character Recognizers 4\.0.*Tamil recognizer.*class 36 \(ா, aa sign\).*native Tamil writers.*MIT licence/,
    );
    expect(src.variation).toMatch(/one pen-down stroke.*scaled to a square.*Noto Sans Tamil.*varies by writer/);
  });
});
